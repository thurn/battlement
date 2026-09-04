mod app_support;

use std::{
  cell::{Cell, RefCell},
  num::NonZeroU32,
  rc::Rc,
  sync::{Arc, Mutex},
};

use battlement::{
  CameraState, CommandBody, GameObject, GameObjectKind, LowerLimit, ObjectId, PreparedAsset, Prop,
  Scene, SceneId, SemanticRole, SessionId, Snapshot, UiDocument, UiDocumentState, UiElement,
  UpperLimit, VisualElementUpdate,
};
use battlement_fake::client::FakeClient;
use battlement_reactant::{
  announcement::use_announce,
  app::App,
  app_context::{AppHandle, use_app},
  component::{Component, memo},
  hooks,
  host::{
    ButtonHost, DropdownField, GroupBox, Label, MinMaxSlider, PopupWindow, ProgressBar,
    RadioButton, RadioButtonGroup, RepeatButton, SliderInt, TabView, TextElement, TextField,
    ToggleButtonGroup, ToggleHost, View,
  },
  portal::create_portal,
  render::Render,
  runtime::Reactant,
  semantics::{SemanticDescription, SemanticName, SemanticProps},
};
use trox::{Bundle, DiagnosticCode, LocalizedString, Localizer, SourceLocale, tx};

#[derive(Clone, Copy)]
struct IdleSpawner;

struct LocalizedFixture {
  mounts: Rc<Cell<u32>>,
  portal: battlement_reactant::portal::PortalTarget,
}

struct AppFixture {
  replacement: Rc<RefCell<Option<Localizer>>>,
}

struct StaleHandleFixture {
  stale: Rc<RefCell<Option<AppHandle>>>,
}

impl PartialEq for LocalizedFixture {
  fn eq(&self, other: &Self) -> bool {
    Rc::ptr_eq(&self.mounts, &other.mounts) && self.portal == other.portal
  }
}

impl battlement_reactant::executor::Spawner for IdleSpawner {
  fn spawn(
    &self,
    _future: battlement_reactant::executor::BoxFuture<'static, ()>,
  ) -> battlement_reactant::executor::SpawnedTask {
    panic!("localization fixture does not spawn tasks")
  }
}

impl Component for LocalizedFixture {
  fn render(&self) -> impl Render {
    let mounts = Rc::clone(&self.mounts);
    let _ = hooks::use_state_with(move || {
      mounts.set(mounts.get() + 1);
      7_u8
    });
    View::new()
      .child(
        Label::new(message()).semantic(
          SemanticProps::new(SemanticRole::StaticText)
            .name(SemanticName::text(message()))
            .description(SemanticDescription::text(message())),
        ),
      )
      .child(create_portal(
        Label::new(message()).semantic(
          SemanticProps::new(SemanticRole::StaticText).name(SemanticName::text(message())),
        ),
        self.portal.clone(),
      ))
      .child(View::new().portal_target(self.portal.clone()))
  }
}

impl Component for AppFixture {
  fn render(&self) -> impl Render {
    let app = use_app();
    let announce = use_announce();
    let replacement = Rc::clone(&self.replacement);
    View::new().child((
      Label::new(message()).name("localized-value"),
      ButtonHost::new(message())
        .name("replace-localizer")
        .on_click(move || {
          announce.send(message());
          app.set_localizer(self::source_localizer());
          app.set_localizer(
            replacement
              .borrow_mut()
              .take()
              .expect("one queued replacement"),
          );
          announce.send(message());
        }),
    ))
  }
}

impl Component for StaleHandleFixture {
  fn render(&self) -> impl Render {
    let app = use_app();
    if self.stale.borrow().is_none() {
      *self.stale.borrow_mut() = Some(app);
    }
    let announce = use_announce();
    let stale = Rc::clone(&self.stale);
    View::new().child((
      Label::new(message()).name("localized-value"),
      ButtonHost::new(message())
        .name("use-stale-handle")
        .on_click(move || {
          stale
            .borrow()
            .as_ref()
            .expect("initial session handle")
            .set_localizer(self::target_localizer());
          announce.send(message());
        }),
    ))
  }
}

#[test]
fn source_bundle_lowers_every_product_copy_property_category() {
  let document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.set_source_bundle(self::source_bundle());
  reactant.register_root(document.clone(), |_| self::property_catalog());
  let (snapshot, commit) = reactant
    .begin_session(&mut ())
    .unwrap()
    .into_parts(self::snapshot(&document));
  let serialized = serde_json::to_string(&snapshot.ui).unwrap();

  for copy in [
    "Source text",
    "Source label",
    "Source placeholder",
    "Source choice",
    "Source progress",
  ] {
    assert!(serialized.contains(copy), "missing localized copy: {copy}");
  }
  assert!(serialized.contains("User value"));
  let groups = commit.into_groups();
  assert_eq!(self::semantic_labels(&groups), ["Source text"]);
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn direct_replacement_crosses_memo_boundaries_without_remounting() {
  let document = self::document();
  let mounts = Rc::new(Cell::new(0));
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.set_source_bundle(self::source_bundle());
  let portal = reactant.create_portal_target();
  let fixture_mounts = Rc::clone(&mounts);
  reactant.register_root(document.clone(), move |_| {
    memo(LocalizedFixture {
      mounts: Rc::clone(&fixture_mounts),
      portal: portal.clone(),
    })
  });
  let (snapshot, commit) = reactant
    .begin_session(&mut ())
    .unwrap()
    .into_parts(self::snapshot(&document));
  let source_ids = self::label_ids(&snapshot.ui[0]);
  assert_eq!(source_ids.len(), 2);
  assert_eq!(mounts.get(), 1);
  let _ = commit.into_groups();

  let groups = reactant
    .replace_localizer(&mut (), self::target_localizer())
    .unwrap()
    .into_groups();
  assert_eq!(mounts.get(), 1);
  assert!(groups.iter().flatten().all(|body| !matches!(
    body,
    CommandBody::VisualElementCreate(_) | CommandBody::VisualElementDestroy(_)
  )));
  let updated = self::translated_label_ids(&groups);
  assert_eq!(updated.len(), 2);
  assert!(updated.iter().all(|id| source_ids.contains(id)));
  assert_eq!(
    self::semantic_labels(&groups),
    ["Target text", "Target text"]
  );
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn app_handle_uses_the_last_replacement_and_preserves_announcement_order() {
  let replacement = Rc::new(RefCell::new(Some(self::target_localizer())));
  let app = App::new("app/content").ui(AppFixture { replacement });
  let root = app.root_document().root_id;
  let mut client = FakeClient::connect(app, app_support::catalog());
  let button = app_support::named(&mut client, root, "replace-localizer");
  client.clear_commands();
  client.ui().click(button);

  assert_eq!(
    app_support::text(&mut client, root, "localized-value"),
    "Target text"
  );
  assert_eq!(self::announcements(&client), ["Source text", "Target text"]);
}

#[test]
fn stale_app_handle_cannot_change_announcement_localization() {
  let stale = Rc::new(RefCell::new(None));
  let app = App::new("app/content").ui(StaleHandleFixture {
    stale: Rc::clone(&stale),
  });
  let root = app.root_document().root_id;
  let mut client = FakeClient::connect(app, app_support::catalog());
  client.reconnect();
  let button = app_support::named(&mut client, root, "use-stale-handle");
  client.clear_commands();
  client.ui().click(button);

  assert_eq!(
    app_support::text(&mut client, root, "localized-value"),
    "Source text"
  );
  assert_eq!(self::announcements(&client), ["Source text"]);
}

#[test]
fn localized_presentation_uses_source_development_by_default() {
  let document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_| Label::new(message()));
  let (snapshot, commit) = reactant
    .begin_session(&mut ())
    .unwrap()
    .into_parts(self::snapshot(&document));

  assert!(
    serde_json::to_string(&snapshot.ui)
      .unwrap()
      .contains("Source text")
  );
  let _ = commit.into_groups();
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn localized_sparse_properties_preserve_unset_set_and_reset() {
  let document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.set_source_bundle(self::source_bundle());
  reactant.register_root(document.clone(), |text: &Prop<LocalizedString>| {
    Label::new(message()).text(text.clone())
  });
  let mut text = Prop::Unset;
  let (snapshot, commit) = reactant
    .begin_session(&mut text)
    .unwrap()
    .into_parts(self::snapshot(&document));
  assert!(
    !serde_json::to_string(&snapshot.ui)
      .unwrap()
      .contains("Source text")
  );
  let _ = commit.into_groups();

  text = Prop::Set(message());
  assert!(
    self::label_text_props(&reactant.refresh(&mut text).unwrap().into_groups())
      .contains(&Prop::Set("Source text".to_owned()))
  );
  text = Prop::Reset;
  assert!(
    self::label_text_props(&reactant.refresh(&mut text).unwrap().into_groups())
      .contains(&Prop::Reset)
  );
  let _ = reactant.shutdown(&mut text).into_groups();
}

#[test]
fn reactant_preserves_trox_diagnostics_and_source_fallback() {
  let diagnostics = Arc::new(Mutex::new(Vec::new()));
  let captured = Arc::clone(&diagnostics);
  let mut unrelated = self::source_bundle();
  unrelated.entries.clear();
  unrelated.source_catalog_fingerprint = "0".repeat(64);
  let localizer = Localizer::new(unrelated, self::source_bundle())
    .unwrap()
    .with_diagnostic_hook(move |diagnostic| captured.lock().unwrap().push(diagnostic.code));
  let document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.set_localizer(localizer);
  reactant.register_root(document.clone(), |_| Label::new(message()));
  let (snapshot, commit) = reactant
    .begin_session(&mut ())
    .unwrap()
    .into_parts(self::snapshot(&document));

  assert!(
    serde_json::to_string(&snapshot.ui)
      .unwrap()
      .contains("Source text")
  );
  let diagnostics = diagnostics.lock().unwrap();
  assert!(diagnostics.contains(&DiagnosticCode::CatalogMismatch));
  assert!(diagnostics.contains(&DiagnosticCode::MissingMessage));
  drop(diagnostics);
  let _ = commit.into_groups();
  let _ = reactant.shutdown(&mut ()).into_groups();
}

fn message() -> LocalizedString {
  tx(
    "Source text",
    "Localized text used by the Reactant runtime replacement fixture.",
  )
}

fn property_catalog() -> impl Render {
  View::new()
    .child((
      Label::new(message()),
      TextElement::new(message()),
      TextField::new()
        .label(tx(
          "Source label",
          "Localized label used by the Reactant property fixture.",
        ))
        .placeholder(tx(
          "Source placeholder",
          "Localized placeholder used by the Reactant property fixture.",
        ))
        .value("User value"),
      ToggleHost::new().text(message()),
      RadioButton::new().text(message()),
      (
        RadioButtonGroup::new()
          .label(message())
          .choices([tx(
            "Source choice",
            "Localized choice used by the Reactant property fixture.",
          )])
          .selected_index(0),
        ToggleButtonGroup::new()
          .label(message())
          .child(ButtonHost::new(message())),
        DropdownField::new()
          .label(message())
          .choices([tx(
            "Source choice",
            "Localized choice used by the Reactant property fixture.",
          )])
          .selection(
            0,
            tx(
              "Source choice",
              "Localized choice used by the Reactant property fixture.",
            ),
          ),
        ButtonHost::new(message()),
      ),
    ))
    .child((
      RepeatButton::new(message(), 100, NonZeroU32::new(25).unwrap()),
      GroupBox::new().text(message()).child(Label::new(message())),
      PopupWindow::new()
        .text(message())
        .child(Label::new(message())),
      battlement_reactant::host::SliderHost::new()
        .label(message())
        .low_value(0.0)
        .high_value(10.0)
        .value(5.0),
      SliderInt::new()
        .label(message())
        .low_value(0)
        .high_value(10)
        .value(5),
      MinMaxSlider::new()
        .label(message())
        .low_limit(LowerLimit::Inclusive(0.0))
        .high_limit(UpperLimit::Inclusive(10.0))
        .min_value(2.0)
        .max_value(8.0),
      ProgressBar::new()
        .title(tx(
          "Source progress",
          "Localized progress title used by the Reactant property fixture.",
        ))
        .low_value(0.0)
        .high_value(10.0)
        .value(5.0),
      TabView::new()
        .selected_tab_index(0)
        .child(battlement_reactant::host::TabHost::new(message()).child(Label::new(message()))),
      Label::new(message()).semantic(
        SemanticProps::new(SemanticRole::StaticText)
          .name(SemanticName::text(message()))
          .description(SemanticDescription::text(message())),
      ),
    ))
}

fn source_bundle() -> Bundle {
  Bundle::from_canonical_json(include_str!("localization/bundles/en-US.trox.json"))
    .expect("valid localization test source bundle")
}

fn target_localizer() -> Localizer {
  Localizer::new(
    Bundle::from_canonical_json(include_str!("localization/bundles/fr.trox.json"))
      .expect("valid localization test target bundle"),
    source_bundle(),
  )
  .expect("compatible localization test bundles")
}

fn source_localizer() -> Localizer {
  Localizer::for_source(SourceLocale::new("en-US").expect("valid source locale"))
    .expect("valid source-development localizer")
}

fn document() -> UiDocument {
  UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4())
}

fn snapshot(document: &UiDocument) -> Snapshot {
  let camera_id = ObjectId::new_v4();
  Snapshot::new(
    SessionId::new_v4(),
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(SceneId::new_v4(), "test/scene")],
    vec![
      GameObject::new(camera_id, CameraState::new()),
      GameObject::new(
        document.document_id,
        GameObjectKind::UiDocument(UiDocumentState::new(document.root_id)),
      ),
    ],
    camera_id,
  )
}

fn label_ids(document: &UiDocument) -> Vec<ObjectId> {
  fn collect(node: &battlement::UiNode, ids: &mut Vec<ObjectId>) {
    if matches!(node.element, UiElement::Label(_)) {
      ids.push(node.object_id);
    }
    for child in &node.children {
      collect(child, ids);
    }
  }

  let mut ids = Vec::new();
  for child in &document.children {
    collect(child, &mut ids);
  }
  ids
}

fn translated_label_ids(groups: &[Vec<CommandBody>]) -> Vec<ObjectId> {
  groups
    .iter()
    .flatten()
    .filter_map(|body| match body {
      CommandBody::VisualElementUpdate(update) => match update.as_ref() {
        VisualElementUpdate::Properties { object_id, element } => match element.as_ref() {
          UiElement::Label(label) if label.text == Prop::Set("Target text".to_owned()) => {
            Some(*object_id)
          }
          _ => None,
        },
        _ => None,
      },
      _ => None,
    })
    .collect()
}

fn label_text_props(groups: &[Vec<CommandBody>]) -> Vec<Prop<String>> {
  groups
    .iter()
    .flatten()
    .filter_map(|body| match body {
      CommandBody::VisualElementUpdate(update) => match update.as_ref() {
        VisualElementUpdate::Properties { element, .. } => match element.as_ref() {
          UiElement::Label(label) => Some(label.text.clone()),
          _ => None,
        },
        _ => None,
      },
      _ => None,
    })
    .collect()
}

fn semantic_labels(groups: &[Vec<CommandBody>]) -> Vec<String> {
  groups
    .iter()
    .flatten()
    .find_map(|body| match body {
      CommandBody::AccessibilityUpdate(update) => update.snapshot.as_ref().map(|snapshot| {
        snapshot
          .nodes
          .iter()
          .filter_map(|node| node.label.clone())
          .collect()
      }),
      _ => None,
    })
    .unwrap_or_default()
}

fn announcements(client: &FakeClient<App>) -> Vec<String> {
  client
    .commands()
    .iter()
    .filter_map(|command| match &command.command.body {
      CommandBody::AccessibilityUpdate(update) if !update.announcements.is_empty() => {
        Some(update.announcements.clone())
      }
      _ => None,
    })
    .flatten()
    .collect()
}

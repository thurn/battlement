use std::{
  cell::Cell,
  num::NonZeroU32,
  panic::{self, AssertUnwindSafe},
  rc::Rc,
  slice,
  sync::Arc,
};

use battlement::{
  Box, Button, CameraState, ClientMessage, Command, Connect, DropdownField, GameObject,
  GameObjectKind, GroupBox, Image, Label, LowerLimit, MinMaxSlider, ObjectId, PanelScaleMode,
  PanelSettings, ParentScene, PopupWindow, PreparedAsset, ProgressBar, RadioButton,
  RadioButtonGroup, RepeatButton, Response, ResponseMessage, Scene, SceneId, ScrollView, Scroller,
  SessionId, Slider, SliderInt, Snapshot, Tab, TabView, TextElement, TextField, Toggle,
  ToggleButtonGroup, UiDocument, UiDocumentState, UiElementKind, UiEventKind, UiEventSubscription,
  UiNode, UpperLimit, VisualElement,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_native::{Engine, EngineError};
use battlement_reactant::{
  component::Component,
  executor::{BoxFuture, SpawnedTask, Spawner},
  primitive::ContainerRenderExt,
  props::Missing,
  render::{Node, Render},
  runtime::Reactant,
};
use uuid::Uuid;

struct IdleSpawner;

struct SessionEngine<G: 'static> {
  game: G,
  reactant: Reactant<G>,
  document: UiDocument,
  recorded: Rc<std::cell::RefCell<Option<Response>>>,
}

struct RequiredOptions {
  emphasized: bool,
}

struct GeneratedCard<Title = Missing, Child = Missing> {
  required: (Title, Child),
  optional: RequiredOptions,
}

struct ManualCard<Title = Missing, Child = Missing> {
  required: (Title, Child),
  optional: RequiredOptions,
}

battlement_reactant::required_props!(GeneratedCard, title: String, child: Label);

impl GeneratedCard<Missing, Missing> {
  fn new() -> Self {
    Self {
      required: (Missing, Missing),
      optional: RequiredOptions { emphasized: false },
    }
  }
}

impl<Title, Child> GeneratedCard<Title, Child> {
  fn emphasized(mut self, value: bool) -> Self {
    self.optional.emphasized = value;
    self
  }
}

impl Component for GeneratedCard<String, Label> {
  fn render(&self) -> impl Render {
    card_tree(&self.required.0, &self.required.1, self.optional.emphasized)
  }
}

impl ManualCard<Missing, Missing> {
  fn new() -> Self {
    Self {
      required: (Missing, Missing),
      optional: RequiredOptions { emphasized: false },
    }
  }
}

impl<Child> ManualCard<Missing, Child> {
  fn title(self, value: String) -> ManualCard<String, Child> {
    ManualCard {
      required: (value, self.required.1),
      optional: self.optional,
    }
  }
}

impl<Title> ManualCard<Title, Missing> {
  fn child(self, value: Label) -> ManualCard<Title, Label> {
    ManualCard {
      required: (self.required.0, value),
      optional: self.optional,
    }
  }
}

impl<Title, Child> ManualCard<Title, Child> {
  fn emphasized(mut self, value: bool) -> Self {
    self.optional.emphasized = value;
    self
  }
}

impl Component for ManualCard<String, Label> {
  fn render(&self) -> impl Render {
    card_tree(&self.required.0, &self.required.1, self.optional.emphasized)
  }
}

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

impl<G: 'static> Engine for SessionEngine<G> {
  type ActionPayload = ();
  type ErrorCode = ();
  type Command = Command;

  fn connect(&mut self, _message: Connect) -> Result<Response, EngineError> {
    let response = self
      .reactant
      .begin_session(&mut self.game)
      .expect("fixture render succeeds")
      .into_response(snapshot(
        SessionId::new_v4(),
        slice::from_ref(&self.document),
      ));
    self.recorded.replace(Some(response.clone()));
    Ok(response)
  }

  fn submit(&mut self, _message: ClientMessage<(), ()>) -> Result<Response, EngineError> {
    Err(EngineError::new("fixture does not accept actions"))
  }

  fn poll(&mut self) -> Result<Option<Response>, EngineError> {
    Ok(None)
  }
}

#[test]
fn generated_and_handwritten_required_props_render_equivalent_fake_trees() {
  let document = document();
  let root_id = document.root_id;
  let recorded = Rc::new(std::cell::RefCell::new(None));
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_: &()| {
    (
      GeneratedCard::new()
        .emphasized(true)
        .child(Label::new("body"))
        .title("Citadel".to_owned()),
      ManualCard::new()
        .title("Citadel".to_owned())
        .emphasized(true)
        .child(Label::new("body")),
    )
  });
  let engine = SessionEngine {
    game: (),
    reactant,
    document,
    recorded,
  };

  let mut client = FakeClient::connect(engine, catalog());

  let cards = client.ui().element(root_id).children().to_vec();
  assert_eq!(cards.len(), 2);
  for card in cards {
    let (kind, classes, children) = {
      let ui = client.ui();
      let rendered = ui.element(card);
      (
        rendered.kind(),
        rendered.classes().map(<[String]>::to_vec),
        rendered.children().to_vec(),
      )
    };
    assert_eq!(kind, UiElementKind::VisualElement);
    assert_eq!(classes, Some(vec!["emphasized".to_owned()]));
    assert_eq!(client.ui().element(children[0]).text(), Some("Citadel"));
    assert_eq!(client.ui().element(children[1]).text(), Some("body"));
  }
}

#[test]
fn fake_client_receives_every_primitive_and_legal_child_family() {
  let document = document();
  let root_id = document.root_id;
  let recorded = Rc::new(std::cell::RefCell::new(None));
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_: &()| primitive_catalog());
  let engine = SessionEngine {
    game: (),
    reactant,
    document,
    recorded: Rc::clone(&recorded),
  };

  let mut client = FakeClient::connect(engine, catalog());

  let response = recorded.borrow();
  let ResponseMessage::Snapshot(rendered) =
    &response.as_ref().expect("response recorded").messages[0]
  else {
    panic!("session response did not begin with a snapshot");
  };
  let mut rendered_nodes = Vec::new();
  collect_nodes(&rendered.ui[0].children, &mut rendered_nodes);
  for (object_id, kind) in &rendered_nodes {
    assert_eq!(client.ui().element(*object_id).kind(), *kind);
  }
  assert_eq!(
    client.ui().element(root_id).children(),
    &[rendered.ui[0].children[0].object_id]
  );
  for kind in every_kind() {
    assert!(
      rendered_nodes.iter().any(|(_, actual)| *actual == kind),
      "catalog omitted {kind:?}"
    );
  }
  let root = &rendered.ui[0].children[0];
  assert_eq!(root.element.kind(), UiElementKind::VisualElement);
  assert_eq!(root.children[0].element.kind(), UiElementKind::Box);
  assert_eq!(root.children[0].children.len(), 1);
  let toggle_group = root
    .children
    .iter()
    .find(|node| node.element.kind() == UiElementKind::ToggleButtonGroup)
    .expect("toggle group rendered");
  assert_eq!(
    toggle_group.children[0].element.kind(),
    UiElementKind::Button
  );
  let tab_view = root
    .children
    .iter()
    .find(|node| node.element.kind() == UiElementKind::TabView)
    .expect("tab view rendered");
  assert_eq!(tab_view.children[0].element.kind(), UiElementKind::Tab);
  assert_eq!(
    tab_view.children[0].children[0].element.kind(),
    UiElementKind::Label
  );
}

#[test]
fn authored_subscriptions_leave_the_committed_fake_hierarchy_unchanged() {
  let authored = Rc::new(Cell::new(false));
  let document = document();
  let root_id = document.root_id;
  let recorded = Rc::new(std::cell::RefCell::new(None));
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), {
    let authored = Rc::clone(&authored);
    move |_: &()| {
      if authored.get() {
        Label::new("stable")
          .events([UiEventKind::Click])
          .event_subscriptions([UiEventSubscription::target(UiEventKind::Focus)])
      } else {
        Label::new("stable")
      }
    }
  });
  let engine = SessionEngine {
    game: (),
    reactant,
    document,
    recorded,
  };
  let mut client = FakeClient::connect(engine, catalog());
  let before = client.ui().element(root_id).children().to_vec();

  authored.set(true);
  assert!(panic::catch_unwind(AssertUnwindSafe(|| client.reconnect())).is_err());

  assert_eq!(client.ui().element(root_id).children(), before);
  assert_eq!(client.ui().element(before[0]).text(), Some("stable"));
}

fn primitive_catalog() -> impl battlement_reactant::render::Render {
  VisualElement::new().name("catalog").children(vec![
    Node::new(Box::new().name("box").child(Label::new("box child"))),
    Node::new(Label::new("label")),
    Node::new(TextElement::new("text element")),
    Node::new(TextField::new().label("text field").value("value")),
    Node::new(Toggle::new().text("toggle").value(true)),
    Node::new(RadioButton::new().text("radio").value(false)),
    Node::new(
      RadioButtonGroup::new()
        .label("quality")
        .choices(["low", "high"])
        .selected_index(0),
    ),
    Node::new(
      ToggleButtonGroup::new()
        .label("alignment")
        .child(Button::new("left")),
    ),
    Node::new(
      DropdownField::new()
        .label("mode")
        .choices(["one"])
        .selection(0, "one"),
    ),
    Node::new(Button::new("button")),
    Node::new(RepeatButton::new(
      "repeat",
      100,
      NonZeroU32::new(25).expect("nonzero interval"),
    )),
    Node::new(
      GroupBox::new()
        .text("group")
        .child(Label::new("group child")),
    ),
    Node::new(
      PopupWindow::new()
        .text("popup")
        .child(Label::new("popup child")),
    ),
    Node::new(ScrollView::new().child(Label::new("scroll child"))),
    Node::new(Scroller::new().low_value(0.0).high_value(10.0).value(5.0)),
    Node::new(Slider::new().low_value(0.0).high_value(10.0).value(5.0)),
    Node::new(SliderInt::new().low_value(0).high_value(10).value(5)),
    Node::new(
      MinMaxSlider::new()
        .low_limit(LowerLimit::Inclusive(0.0))
        .high_limit(UpperLimit::Inclusive(10.0))
        .min_value(2.0)
        .max_value(8.0),
    ),
    Node::new(
      ProgressBar::new()
        .title("progress")
        .low_value(0.0)
        .high_value(10.0)
        .value(5.0),
    ),
    Node::new(
      TabView::new().selected_tab_index(0).child(
        Tab::new("tab")
          .closeable(true)
          .child(Label::new("tab child")),
      ),
    ),
    Node::new(Image::new().name("image")),
  ])
}

fn card_tree(title: &str, child: &Label, emphasized: bool) -> impl Render {
  let card = if emphasized {
    VisualElement::new().class("emphasized")
  } else {
    VisualElement::new()
  };
  card.child(Label::new(title)).child(child.clone())
}

fn every_kind() -> [UiElementKind; 23] {
  [
    UiElementKind::VisualElement,
    UiElementKind::Box,
    UiElementKind::Label,
    UiElementKind::TextElement,
    UiElementKind::TextField,
    UiElementKind::Toggle,
    UiElementKind::RadioButton,
    UiElementKind::RadioButtonGroup,
    UiElementKind::ToggleButtonGroup,
    UiElementKind::DropdownField,
    UiElementKind::Button,
    UiElementKind::RepeatButton,
    UiElementKind::GroupBox,
    UiElementKind::PopupWindow,
    UiElementKind::ScrollView,
    UiElementKind::Scroller,
    UiElementKind::Slider,
    UiElementKind::SliderInt,
    UiElementKind::MinMaxSlider,
    UiElementKind::ProgressBar,
    UiElementKind::Tab,
    UiElementKind::TabView,
    UiElementKind::Image,
  ]
}

fn collect_nodes(nodes: &[UiNode], output: &mut Vec<(ObjectId, UiElementKind)>) {
  for node in nodes {
    output.push((node.object_id, node.element.kind()));
    collect_nodes(&node.children, output);
  }
}

fn document() -> UiDocument {
  UiDocument::with_root_id(object_id(10), object_id(11))
}

fn snapshot(session_id: SessionId, documents: &[UiDocument]) -> Snapshot {
  let camera_id = object_id(1);
  let mut objects = vec![GameObject::new(camera_id, CameraState::new())];
  objects.extend(documents.iter().map(|document| {
    GameObject::new(
      document.document_id,
      GameObjectKind::UiDocument(
        UiDocumentState::new(document.root_id)
          .panel_settings(PanelSettings::new().scale_mode(PanelScaleMode::ConstantPixelSize)),
      ),
    )
    .parent_scene(ParentScene::Persistent)
  }));
  Snapshot::new(
    session_id,
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(SceneId::new_v4(), "test/scene")],
    objects,
    camera_id,
  )
}

fn catalog() -> Arc<FakeAssetCatalog> {
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene("test/scene");
  Arc::new(catalog)
}

fn object_id(value: u128) -> ObjectId {
  ObjectId::from_uuid(Uuid::from_u128(value)).expect("fixture ID is nonzero")
}

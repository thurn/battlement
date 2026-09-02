use std::panic::{self, AssertUnwindSafe};

use battlement::{
  AccessibilitySnapshot, CameraState, ClickEvent, CommandBody, GameObject, GameObjectKind,
  ObjectId, PopupKind, PreparedAsset, Scene, SceneId, SemanticRole, SessionId, Snapshot,
  UiAccessibilityAction, UiAccessibilityActionEvent, UiDocument, UiDocumentState, UiEvent,
  UiEventBody,
};
use battlement_reactant::{
  accessibility_popup::{self, PopupButtonOptions},
  component::Component,
  element_ref,
  executor::{BoxFuture, SpawnedTask, Spawner},
  host::{Button, Label, View},
  render::Render,
  runtime::Reactant,
  semantics::{AccessibleName, SemanticProps, SemanticVisibility},
};

#[derive(Clone, Default)]
struct Game {
  changed: bool,
  expanded: bool,
  presses: usize,
}

struct Fixture(Game);
struct IdleSpawner;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

impl Component for Fixture {
  fn render(&self) -> impl Render {
    let title = element_ref::use_element_ref();
    let value = element_ref::use_element_ref();
    let selected = if self.0.changed { " Low " } else { " High " };
    let behavior = accessibility_popup::use_popup_button(PopupButtonOptions {
      name: AccessibleName::LabelledBy(vec![title.clone(), value.clone()]),
      popup: PopupKind::ListBox,
      expanded: self.0.expanded,
      is_disabled: false,
      on_press: |game: &mut Game| game.presses += 1,
    });
    View::new().child((
      Label::new(" Quality ").element_ref(title).semantic(
        SemanticProps::new(SemanticRole::StaticText)
          .name(AccessibleName::text(" Quality "))
          .visibility(SemanticVisibility::NameSourceOnly),
      ),
      Button::new("")
        .semantic(behavior.semantic)
        .focus_props(behavior.focus)
        .interaction_props(behavior.interaction)
        .child(
          Label::new(selected).element_ref(value).semantic(
            SemanticProps::new(SemanticRole::StaticText)
              .name(AccessibleName::text(selected))
              .visibility(SemanticVisibility::NameSourceOnly),
          ),
        ),
    ))
  }
}

#[test]
fn popup_button_keeps_one_host_and_controlled_context_across_updates_and_activation() {
  let document = UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4());
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), |game: &Game| Fixture(game.clone()));
  let mut game = Game::default();
  let (initial, commit) = runtime
    .begin_session(&mut game)
    .unwrap()
    .into_parts(self::snapshot(&document));
  let groups = commit.into_groups();
  let button = self::assert_context(&groups, "Quality High", false);
  let child = initial.ui[0].children[0].children[1].children[0].object_id;
  for (changed, expanded) in [(true, false), (true, true), (true, false)] {
    game.changed = changed;
    game.expanded = expanded;
    let groups = runtime.refresh(&mut game).unwrap().into_groups();
    assert_eq!(
      self::assert_context(&groups, "Quality Low", expanded),
      button
    );
    assert!(!groups.iter().flatten().any(|body| matches!(
      body,
      CommandBody::VisualElementCreate(_) | CommandBody::VisualElementDestroy(_)
    )));
    assert_eq!(game.presses, 0);
  }
  let events = [
    UiEvent::click(button, ClickEvent::NavigationSubmit),
    UiEvent::click(child, ClickEvent::NavigationSubmit),
    UiEvent::new(
      button,
      true,
      false,
      UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
        backend_generation: 1,
        action: UiAccessibilityAction::Activate,
      }),
    ),
  ];
  for (index, event) in events.into_iter().enumerate() {
    let _ = runtime
      .dispatch(&mut game, event)
      .unwrap()
      .into_commit()
      .into_groups();
    assert_eq!(game.presses, index + 1);
    assert!(!game.expanded);
  }
  let _ = runtime.shutdown(&mut game).into_groups();
}

#[test]
fn malformed_popup_declarations_fail_as_developer_errors() {
  for (role, popup, expanded) in [
    (SemanticRole::Button, Some(PopupKind::ListBox), None),
    (SemanticRole::Button, None, Some(false)),
    (SemanticRole::Link, Some(PopupKind::ListBox), Some(false)),
  ] {
    let mut runtime = Reactant::new(IdleSpawner);
    runtime.register_root(
      UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4()),
      move |_game: &Game| {
        let mut behavior = accessibility_popup::use_popup_button(PopupButtonOptions {
          name: AccessibleName::text("Options"),
          popup: PopupKind::ListBox,
          expanded: false,
          is_disabled: false,
          on_press: |_: &mut Game| {},
        });
        behavior.semantic.role = role;
        behavior.semantic.state.popup = popup;
        behavior.semantic.state.expanded = expanded;
        Button::new("Options")
          .semantic(behavior.semantic)
          .focus_props(behavior.focus)
          .interaction_props(behavior.interaction)
      },
    );
    assert!(
      panic::catch_unwind(AssertUnwindSafe(|| {
        let _ = runtime.begin_session(&mut Game::default());
      }))
      .is_err()
    );
  }
}

fn assert_context(groups: &[Vec<CommandBody>], label: &str, expanded: bool) -> ObjectId {
  let snapshot: &AccessibilitySnapshot = groups
    .iter()
    .flatten()
    .find_map(|body| match body {
      CommandBody::AccessibilityUpdate(update) => update.snapshot.as_ref(),
      _ => None,
    })
    .unwrap();
  assert_eq!(snapshot.nodes.len(), 1);
  let node = &snapshot.nodes[0];
  assert_eq!(node.role, SemanticRole::Button);
  assert_eq!(node.label.as_deref(), Some(label));
  assert_eq!(node.state.popup, Some(PopupKind::ListBox));
  assert_eq!(node.state.expanded, Some(expanded));
  assert!(node.actions.activate);
  node.object_id
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

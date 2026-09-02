use battlement::{
  AccessibilityAction, AccessibilityUpdate, CameraState, CommandBody, GameObject, ObjectId,
  PreparedAsset, Scene, SceneId, SemanticRole, SessionId, Snapshot, UiAccessibilityAction,
  UiAccessibilityActionEvent, UiDocument, UiDocumentState, UiEvent, UiEventBody,
  UiEventDisposition,
};
use battlement_reactant::{
  accessibility::{ButtonOptions, use_button},
  component::Component,
  element_ref::use_element_ref,
  executor::{BoxFuture, SpawnedTask, Spawner},
  host::{Label, View},
  render::Render,
  runtime::Reactant,
  semantics::{AccessibleName, SemanticProps, SemanticVisibility, text},
};

#[derive(Default)]
struct Game {
  presses: usize,
}

#[derive(Clone, Copy)]
struct IdleSpawner;

#[derive(Clone, Copy)]
struct NameSourceFixture;

impl Component for NameSourceFixture {
  fn render(&self) -> impl Render {
    let source = use_element_ref();
    let mut button = use_button(ButtonOptions {
      name: text("ignored"),
      is_disabled: false,
      on_press: |_game: &mut Game| {},
    });
    button.semantic.name = Some(AccessibleName::LabelledBy(source.clone()));
    View::new().child((
      Label::new("Account settings").element_ref(source).semantic(
        SemanticProps::new(SemanticRole::StaticText)
          .name(AccessibleName::text("Account settings"))
          .visibility(SemanticVisibility::NameSourceOnly),
      ),
      View::new()
        .semantic(button.semantic)
        .focus_props(button.focus)
        .interaction_props(button.interaction),
    ))
  }
}

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

#[test]
fn complete_snapshot_resolves_contents_and_prunes_hidden_subtrees() {
  let document = document();
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), |_game: &Game| {
    let mut button = use_button(ButtonOptions {
      name: text("ignored"),
      is_disabled: false,
      on_press: |_game: &mut Game| {},
    });
    button.semantic.name = Some(AccessibleName::Contents);
    View::new()
      .semantic(SemanticProps::new(SemanticRole::Group))
      .child((View::new()
        .semantic(button.semantic)
        .focus_props(button.focus)
        .interaction_props(button.interaction)
        .child((
          Label::new("Save").semantic(
            SemanticProps::new(SemanticRole::StaticText)
              .name(AccessibleName::Text(text(" Save   changes "))),
          ),
          View::new()
            .semantic(
              SemanticProps::new(SemanticRole::Group).visibility(SemanticVisibility::Hidden),
            )
            .child(
              Label::new("secret").semantic(
                SemanticProps::new(SemanticRole::StaticText)
                  .name(AccessibleName::Text(text("Secret"))),
              ),
            ),
        )),))
  });
  let mut game = Game::default();
  let groups = runtime
    .begin_session(&mut game)
    .expect("semantic render succeeds")
    .into_parts(snapshot(&document))
    .1
    .into_groups();
  let update = accessibility_update(&groups);
  let snapshot = update.snapshot.as_ref().expect("complete snapshot");
  assert_eq!(snapshot.nodes.len(), 3);
  let button = snapshot
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Button)
    .expect("button node");
  assert_eq!(button.label.as_deref(), Some("Save changes"));
  assert!(
    snapshot
      .nodes
      .iter()
      .all(|node| node.label.as_deref() != Some("Secret"))
  );
  let _ = runtime.shutdown(&mut game).into_groups();
}

#[test]
fn name_source_only_hosts_resolve_without_becoming_nodes() {
  let document = document();
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), |_game: &Game| NameSourceFixture);
  let mut game = Game::default();
  let groups = runtime
    .begin_session(&mut game)
    .expect("semantic render succeeds")
    .into_parts(snapshot(&document))
    .1
    .into_groups();
  let snapshot = accessibility_update(&groups).snapshot.as_ref().unwrap();
  assert_eq!(snapshot.nodes.len(), 1);
  assert_eq!(snapshot.nodes[0].label.as_deref(), Some("Account settings"));
  let _ = runtime.shutdown(&mut game).into_groups();
}

#[test]
fn accessibility_activation_uses_the_ordinary_logical_event_path() {
  let document = document();
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), |_game: &Game| {
    let button = use_button(ButtonOptions {
      name: text("Save changes"),
      is_disabled: false,
      on_press: |game: &mut Game| game.presses += 1,
    });
    View::new()
      .semantic(button.semantic)
      .focus_props(button.focus)
      .interaction_props(button.interaction)
  });
  let mut game = Game::default();
  let groups = runtime
    .begin_session(&mut game)
    .expect("semantic render succeeds")
    .into_parts(snapshot(&document))
    .1
    .into_groups();
  let target = accessibility_update(&groups)
    .snapshot
    .as_ref()
    .unwrap()
    .roots[0];
  let result = runtime
    .dispatch(
      &mut game,
      UiEvent::new(
        target,
        true,
        false,
        UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
          backend_generation: 1,
          action: UiAccessibilityAction::Activate,
        }),
      ),
    )
    .expect("accessibility action renders");
  assert_eq!(result.disposition(), UiEventDisposition::PreventDefault);
  assert_eq!(game.presses, 1);
  assert!(
    result
      .into_groups()
      .iter()
      .flatten()
      .all(|command| !matches!(command, CommandBody::AccessibilityUpdate(_)))
  );
  let _ = runtime.shutdown(&mut game).into_groups();
}

#[test]
fn protocol_round_trips_direct_actions_and_complete_snapshot() {
  let update = AccessibilityUpdate {
    snapshot: Some(battlement::AccessibilitySnapshot {
      commit_sequence: 7,
      roots: vec![],
      nodes: vec![],
    }),
    announcements: vec!["Saved".to_owned()],
  };
  let bytes = battlement::json::to_vec(&CommandBody::AccessibilityUpdate(update.clone())).unwrap();
  assert_eq!(
    battlement::json::from_slice::<CommandBody>(&bytes).unwrap(),
    CommandBody::AccessibilityUpdate(update)
  );
  let action = AccessibilityAction::Scroll(battlement::AccessibilityScrollDirection::Forward);
  let bytes = battlement::json::to_vec(&action).unwrap();
  assert_eq!(
    battlement::json::from_slice::<AccessibilityAction>(&bytes).unwrap(),
    action
  );
}

fn accessibility_update(groups: &[Vec<CommandBody>]) -> &AccessibilityUpdate {
  groups
    .iter()
    .flatten()
    .find_map(|body| match body {
      CommandBody::AccessibilityUpdate(update) => Some(update),
      _ => None,
    })
    .expect("accessibility replacement command")
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
        battlement::GameObjectKind::UiDocument(UiDocumentState::new(document.root_id)),
      ),
    ],
    camera_id,
  )
}

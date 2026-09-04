mod runtime_support;

use battlement::{
  CameraState, CommandBody, GameObject, GameObjectKind, ObjectId, PreparedAsset, Scene, SceneId,
  SessionId, Snapshot, UiDocument, UiDocumentState, application::ApplicationState,
};
use battlement_reactant::{
  accessibility, application,
  component::{self, Component},
  executor::{BoxFuture, SpawnedTask, Spawner},
  host::Label,
  render::Render,
};

struct IdleSpawner;

#[derive(PartialEq)]
struct ActivityLabel(&'static str);

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

impl Component for ActivityLabel {
  fn render(&self) -> impl Render {
    let activity = if application::use_application_state().is_active() {
      "active"
    } else {
      "inactive"
    };
    let label = format!("{} {activity}", self.0);
    Label::new(trox::ls(label.clone())).semantic(accessibility::use_static_text(trox::ls(label)))
  }
}

#[test]
fn lifecycle_context_updates_memoized_consumers_and_preserves_preview_overrides() {
  let document = UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4());
  let mut runtime = runtime_support::reactant(IdleSpawner);
  runtime.register_root(document.clone(), |state: &ApplicationState| {
    application::provider(*state).child((
      component::memo(ActivityLabel("host")),
      application::provider(ApplicationState::default())
        .child(component::memo(ActivityLabel("preview"))),
    ))
  });
  let mut state = ApplicationState::default();
  let camera = ObjectId::new_v4();
  let snapshot = Snapshot::new(
    SessionId::new_v4(),
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(SceneId::new_v4(), "test/scene")],
    vec![
      GameObject::new(camera, CameraState::new()),
      GameObject::new(
        document.document_id,
        GameObjectKind::UiDocument(UiDocumentState::new(document.root_id)),
      ),
    ],
    camera,
  );
  let initial = runtime
    .begin_session(&mut state)
    .unwrap()
    .into_parts(snapshot)
    .1
    .into_groups();
  assert_eq!(self::labels(&initial), ["host active", "preview active"]);
  state.focused = false;
  assert_eq!(
    self::labels(&runtime.refresh(&mut state).unwrap().into_groups()),
    ["host inactive", "preview active"]
  );
  state = ApplicationState {
    focused: true,
    paused: true,
  };
  assert!(
    runtime
      .refresh(&mut state)
      .unwrap()
      .into_groups()
      .is_empty()
  );
  state.paused = false;
  assert_eq!(
    self::labels(&runtime.refresh(&mut state).unwrap().into_groups()),
    ["host active", "preview active"]
  );
  let _ = runtime.shutdown(&mut state).into_groups();
}

fn labels(groups: &[Vec<CommandBody>]) -> Vec<String> {
  groups
    .iter()
    .flatten()
    .find_map(|body| match body {
      CommandBody::AccessibilityUpdate(update) => update.snapshot.as_ref().map(|snapshot| {
        snapshot
          .nodes
          .iter()
          .map(|node| node.label.clone().unwrap())
          .collect()
      }),
      _ => None,
    })
    .expect("semantic replacement")
}

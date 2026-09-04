mod runtime_support;

use std::{panic, panic::AssertUnwindSafe};
use trox::ls;

use battlement::{
  ActionId, CameraState, Command, CommandBody, GameObject, GameObjectKind, ObjectId,
  PanelScaleMode, PanelSettings, ParentScene, PreparedAsset, Response, ResponseMessage, Scene,
  SceneId, SessionId, Snapshot, UiDocument, UiDocumentState, VisualElementUpdate,
};
use battlement_reactant::{
  executor::{BoxFuture, SpawnedTask, Spawner},
  render::{Either, Render},
  runtime::{Reactant, ResponseReactantExt},
};

struct IdleSpawner;

struct PanickingCommand;

struct Game {
  alternate_kind: bool,
  left: &'static str,
  right: &'static str,
}

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

impl From<Command> for PanickingCommand {
  fn from(_value: Command) -> Self {
    panic!("custom conversion failed");
  }
}

#[test]
fn dependency_groups_preserve_barriers_and_parallelize_independent_patches() {
  let (mut reactant, mut game, _) = self::active_runtime();
  game.alternate_kind = true;
  game.left = "changed left";
  game.right = "changed right";

  let groups = reactant
    .refresh(&mut game)
    .expect("changed render succeeds")
    .into_groups();
  let destroy_group = self::group_with(&groups, |body| {
    matches!(body, CommandBody::VisualElementDestroy(_))
  });
  let create_group = self::group_with(&groups, |body| {
    matches!(body, CommandBody::VisualElementCreate(_))
  });
  let property_groups = groups
    .iter()
    .enumerate()
    .flat_map(|(index, group)| {
      group.iter().filter_map(move |body| {
        matches!(
          body,
          CommandBody::VisualElementUpdate(value)
            if matches!(value.as_ref(), VisualElementUpdate::Properties { .. })
        )
        .then_some(index)
      })
    })
    .collect::<Vec<_>>();

  assert_ne!(destroy_group, create_group);
  assert_eq!(property_groups.len(), 2);
  assert_eq!(property_groups[0], property_groups[1]);
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn all_commit_consumers_acknowledge_their_delivery_receipt() {
  let (mut reactant, mut game, session_id) = self::active_runtime();
  game.left = "groups";
  let groups = reactant
    .refresh(&mut game)
    .expect("group render succeeds")
    .into_groups();
  assert!(!groups.is_empty());
  assert!(
    reactant
      .refresh(&mut game)
      .expect("receipt clears")
      .is_empty()
  );

  game.left = "batch";
  let batch = reactant
    .refresh(&mut game)
    .expect("batch render succeeds")
    .into_batch(session_id)
    .expect("nonempty commit creates a batch");
  assert!(!batch.groups.is_empty());
  assert!(
    batch
      .groups
      .iter()
      .flat_map(|group| &group.commands)
      .all(|command| command.blocking)
  );
  assert!(
    reactant
      .refresh(&mut game)
      .expect("receipt clears")
      .is_empty()
  );

  game.left = "response";
  let response = Response::<Command>::empty(session_id).append_reactant(
    reactant
      .refresh(&mut game)
      .expect("response render succeeds"),
  );
  assert_eq!(response.messages.len(), 1);
  assert!(
    reactant
      .refresh(&mut game)
      .expect("receipt clears")
      .is_empty()
  );

  game.left = "action";
  let action_id = ActionId::new_v4();
  let response = Response::<Command>::empty(session_id).append_reactant_for_action(
    action_id,
    reactant.refresh(&mut game).expect("action render succeeds"),
  );
  let ResponseMessage::Batch(batch) = &response.messages[0] else {
    panic!("Reactant response did not append a batch");
  };
  assert_eq!(batch.caused_by_action_id, Some(action_id));
  assert!(
    reactant
      .refresh(&mut game)
      .expect("receipt clears")
      .is_empty()
  );
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn empty_commits_add_no_batch() {
  let (mut reactant, mut game, session_id) = self::active_runtime();
  let response = Response::<Command>::empty(session_id).append_reactant(
    reactant
      .refresh(&mut game)
      .expect("unchanged render succeeds"),
  );
  assert!(response.messages.is_empty());
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn dropping_nonempty_commit_panics_and_poison_is_reported_later() {
  let (mut reactant, mut game, _) = self::active_runtime();
  game.left = "undelivered";
  let commit = reactant
    .refresh(&mut game)
    .expect("changed render succeeds");
  let dropped = panic::catch_unwind(AssertUnwindSafe(|| drop(commit)))
    .expect_err("dropping a nonempty commit must panic");
  let message = self::panic_message(dropped);
  assert_eq!(
    message,
    "a nonempty Reactant commit was dropped without delivery"
  );
  assert!(!message.contains("retry"));

  let later = panic::catch_unwind(AssertUnwindSafe(|| reactant.poll(&mut game)));
  assert!(later.is_err());
}

#[test]
fn reentry_with_an_outstanding_receipt_panics_and_poisoned_drop_is_quiet() {
  let (mut reactant, mut game, _) = self::active_runtime();
  game.left = "pending";
  let commit = reactant
    .refresh(&mut game)
    .expect("changed render succeeds");
  let reentry = panic::catch_unwind(AssertUnwindSafe(|| reactant.poll(&mut game)));
  assert!(reentry.is_err());
  assert!(panic::catch_unwind(AssertUnwindSafe(|| drop(commit))).is_ok());
}

#[test]
fn failed_custom_command_conversion_poison_is_reported_on_the_next_entry() {
  let (mut reactant, mut game, session_id) = self::active_runtime();
  game.left = "conversion";
  let commit = reactant
    .refresh(&mut game)
    .expect("changed render succeeds");
  let conversion = panic::catch_unwind(AssertUnwindSafe(|| {
    Response::<PanickingCommand>::empty(session_id).append_reactant(commit)
  }));
  assert!(conversion.is_err());

  let later = panic::catch_unwind(AssertUnwindSafe(|| reactant.poll(&mut game)));
  assert!(later.is_err());
}

fn view(game: &Game) -> impl Render + use<> {
  let replaceable = if game.alternate_kind {
    Either::right(battlement_reactant::host::Box::new())
  } else {
    Either::left(battlement_reactant::host::View::new())
  };
  battlement_reactant::host::View::new().child((
    replaceable,
    battlement_reactant::host::Label::new(ls(game.left)),
    battlement_reactant::host::Label::new(ls(game.right)),
  ))
}

fn active_runtime() -> (Reactant<Game>, Game, SessionId) {
  let document = UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4());
  let mut game = Game {
    alternate_kind: false,
    left: "left",
    right: "right",
  };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), self::view);
  let snapshot = self::snapshot(&document);
  let session_id = snapshot.session_id;
  let _response = reactant
    .begin_session(&mut game)
    .expect("initial render succeeds")
    .into_response(snapshot);
  (reactant, game, session_id)
}

fn group_with(groups: &[Vec<CommandBody>], predicate: impl Fn(&CommandBody) -> bool) -> usize {
  groups
    .iter()
    .position(|group| group.iter().any(&predicate))
    .expect("expected mutation is present")
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
  payload
    .downcast_ref::<String>()
    .cloned()
    .or_else(|| {
      payload
        .downcast_ref::<&str>()
        .map(|value| (*value).to_owned())
    })
    .expect("panic payload is a message")
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
        GameObjectKind::UiDocument(UiDocumentState::new(document.root_id).panel_settings(
          PanelSettings::new().scale_mode(PanelScaleMode::ConstantLogicalPixelSize),
        )),
      )
      .parent_scene(ParentScene::Persistent),
    ],
    camera_id,
  )
}

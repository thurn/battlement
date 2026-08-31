use std::{any::Any, panic::AssertUnwindSafe};

use battlement::{
  CameraState, GameObject, ObjectId, PreparedAsset, Scene, SceneId, SessionId, Snapshot,
};
use battlement_reactant::{
  __register_generated_asset,
  asset_generator::{AssetRegistration, LogicalRect, LogicalSize},
  executor::{BoxFuture, SpawnedTask, Spawner},
  runtime::Reactant,
};

const ADDRESS: &str = "battlement-reactant/generated/0000000000000000000000000000000000000000000000000000000000000000.png";

__register_generated_asset!(AssetRegistration::__new(
  ADDRESS,
  LogicalSize::new(20.0, 10.0),
  LogicalRect::new(0.0, 0.0, 20.0, 10.0),
  None,
  "fixture::FIRST",
));

__register_generated_asset!(AssetRegistration::__new(
  ADDRESS,
  LogicalSize::new(30.0, 10.0),
  LogicalRect::new(0.0, 0.0, 30.0, 10.0),
  None,
  "fixture::SECOND",
));

struct IdleSpawner;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

#[test]
fn conflicting_linked_metadata_names_both_sources_and_values() {
  let mut game = ();
  let mut reactant = Reactant::new(IdleSpawner);
  let panic = std::panic::catch_unwind(AssertUnwindSafe(|| {
    let _ = reactant
      .begin_session(&mut game)
      .expect("conflict fixture renders")
      .into_parts(self::snapshot());
  }))
  .expect_err("conflicting linked registrations were accepted");
  let message = self::panic_message(panic);
  for text in [
    ADDRESS,
    "fixture::FIRST",
    "fixture::SECOND",
    "canvas",
    "subject",
    "slices",
  ] {
    assert!(message.contains(text), "panic omitted {text}: {message}");
  }
}

fn snapshot() -> Snapshot {
  let camera_id = ObjectId::new_v4();
  Snapshot::new(
    SessionId::new_v4(),
    vec![PreparedAsset::scene("test/scene")],
    vec![Scene::new(SceneId::new_v4(), "test/scene")],
    vec![GameObject::new(camera_id, CameraState::new())],
    camera_id,
  )
}

fn panic_message(panic: Box<dyn Any + Send>) -> String {
  panic
    .downcast_ref::<String>()
    .cloned()
    .or_else(|| {
      panic
        .downcast_ref::<&str>()
        .map(|value| (*value).to_owned())
    })
    .expect("panic carries text")
}

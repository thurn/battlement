use std::{any::Any, panic::AssertUnwindSafe};

use battlement::{
  CameraState, GameObject, ObjectId, PreparedAsset, Scene, SceneId, SessionId, Snapshot, UiDocument,
};
use battlement_reactant::{
  asset_generator,
  executor::{BoxFuture, SpawnedTask, Spawner},
  host::View,
  runtime::Reactant,
};

asset_generator::generate! {
  @background RESERVED {
    @canvas 20px 10px;
    background: linear-gradient(red, blue);
  }
}

struct IdleSpawner;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

#[test]
fn every_caller_prepared_case_is_rejected_inside_the_generated_prefix() {
  let address = RESERVED.texture_address().as_str().to_owned();
  for (case, asset) in [
    ("Scene", PreparedAsset::scene(address.clone())),
    ("Prefab", PreparedAsset::prefab(address.clone())),
    (
      "ParticleEffect",
      PreparedAsset::particle_effect(address.clone()),
    ),
    ("Material", PreparedAsset::material(address.clone())),
    ("Texture", PreparedAsset::texture(address.clone())),
    ("Sprite", PreparedAsset::sprite(address.clone())),
    ("VectorImage", PreparedAsset::vector_image(address.clone())),
    (
      "RenderTexture",
      PreparedAsset::render_texture(address.clone()),
    ),
    ("AudioClip", PreparedAsset::audio_clip(address.clone())),
    (
      "TextMeshProFont",
      PreparedAsset::text_mesh_pro_font(address.clone()),
    ),
    ("UiFont", PreparedAsset::ui_font(address.clone())),
  ] {
    let message = self::conversion_panic(asset);
    assert!(
      message.contains(&format!("PreparedAsset::{case}")),
      "{message}"
    );
    assert!(message.contains(&address), "{message}");
    assert!(message.contains("RESERVED"), "{message}");
    assert!(message.contains("exclusively owns"), "{message}");
  }
}

fn conversion_panic(asset: PreparedAsset) -> String {
  let document = UiDocument::new(ObjectId::new_v4());
  let mut game = ();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_| View::new());
  let panic = std::panic::catch_unwind(AssertUnwindSafe(|| {
    let mut snapshot = self::snapshot();
    snapshot.prepared_assets.push(asset);
    let _ = reactant
      .begin_session(&mut game)
      .expect("reserved-address fixture renders")
      .into_parts(snapshot);
  }))
  .expect_err("reserved generated address was accepted");
  self::panic_message(panic)
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

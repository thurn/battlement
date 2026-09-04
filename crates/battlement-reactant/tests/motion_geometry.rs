mod runtime_support;

use battlement::{
  CameraState, GameObject, GameObjectKind, MotionEventBatch, MotionGestureEvent,
  MotionGestureEventKind, MotionGestureVector, MotionPointerDevice, MotionSequence, ObjectId,
  ParentScene, PreparedAsset, Prop, Scene, SceneId, SessionId, Snapshot, UiDocument,
  UiDocumentState, UiVisualElementProperties,
};
use battlement_reactant::{
  component::Component,
  element_ref,
  executor::{BoxFuture, SpawnedTask, Spawner},
  geometry,
  host::{Label, View},
  render::Render,
};
use trox::ls;

struct IdleSpawner;

struct GeometryOnHover {
  hovered: bool,
}

#[test]
fn a_native_motion_callback_can_rerender_a_geometry_consumer() {
  let document = UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4());
  let camera = ObjectId::new_v4();
  let mut runtime = runtime_support::reactant(IdleSpawner);
  runtime.register_root(document.clone(), |hovered: &bool| GeometryOnHover {
    hovered: *hovered,
  });
  let mut hovered = false;
  let (snapshot, initial_commit) =
    runtime
      .begin_session(&mut hovered)
      .unwrap()
      .into_parts(Snapshot::new(
        SessionId::new_v4(),
        vec![PreparedAsset::scene("test/scene")],
        vec![Scene::new(SceneId::new_v4(), "test/scene")],
        vec![
          GameObject::new(camera, CameraState::new()),
          GameObject::new(
            document.document_id,
            GameObjectKind::UiDocument(UiDocumentState::new(document.root_id)),
          )
          .parent_scene(ParentScene::Persistent),
        ],
        camera,
      ));
  let _ = initial_commit.into_groups();
  let Prop::Set(descriptor) = &snapshot.ui[0].children[0].element.visual_element().motion else {
    panic!("hover subscription requires a motion descriptor");
  };
  let commit = runtime
    .motion_events(
      &mut hovered,
      MotionEventBatch {
        first_sequence: MotionSequence(0),
        last_sequence: MotionSequence(0),
        events: Vec::new(),
        samples: Vec::new(),
        value_samples: Vec::new(),
        playback_events: Vec::new(),
        gesture_events: vec![MotionGestureEvent {
          descriptor_id: descriptor.descriptor_id,
          generation: descriptor.generation,
          kind: MotionGestureEventKind::HoverStart,
          pointer_id: 0,
          device: MotionPointerDevice::Mouse,
          point: MotionGestureVector::default(),
          delta: MotionGestureVector::default(),
          offset: MotionGestureVector::default(),
          velocity: MotionGestureVector::default(),
          axis: None,
          momentum_generation: 0,
          constrained: false,
        }],
      },
    )
    .unwrap();
  assert!(hovered);
  assert!(
    serde_json::to_string(&commit.into_groups())
      .unwrap()
      .contains("Hover observed")
  );
  let _ = runtime.shutdown(&mut hovered).into_groups();
}

impl Component for GeometryOnHover {
  fn render(&self) -> impl Render {
    let target = element_ref::use_element_ref();
    let _measurement = geometry::use_geometry(target.clone());
    View::new()
      .element_ref(target)
      .on_hover_start(|hovered: &mut bool, _| *hovered = true)
      .child(Label::new(ls(if self.hovered {
        "Hover observed"
      } else {
        "Waiting for hover"
      })))
  }
}

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    panic!("fixture has no asynchronous resources")
  }
}

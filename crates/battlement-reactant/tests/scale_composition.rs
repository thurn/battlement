mod runtime_support;

use battlement::{
  CameraState, GameObject, GameObjectKind, MotionProperty, MotionValue, ObjectId, PanelScaleMode,
  PanelSettings, ParentScene, PreparedAsset, Prop, Scene, SceneId, SessionId, Snapshot, UiDocument,
  UiDocumentState, UiVisualElementProperties,
};
use battlement_reactant::{
  executor::{BoxFuture, SpawnedTask, Spawner},
  prelude::*,
  runtime::Reactant,
};

struct IdleSpawner;
impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

struct ScaleCompositionContract;

impl Component for ScaleCompositionContract {
  fn render(&self) -> impl Render {
    let factor = use_motion_value(1.012_f32);
    View::new().child((
      View::new().animate(StyleTarget::new().scale(0.955).scale_factor(factor.clone())),
      View::new().animate(StyleTarget::new().scale_factor(factor).scale(0.955)),
    ))
  }
}

#[test]
fn composed_scale_keeps_local_target_in_either_authoring_order() {
  let document = document();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), |(): &()| ScaleCompositionContract);
  let rendered = start(&mut reactant, &mut (), &document);
  let children = &rendered.children[0].children;
  for child in children {
    let Prop::Set(descriptor) = &child.element.visual_element().motion else {
      panic!("motion missing")
    };
    descriptor.validate().unwrap();
    assert!(
      descriptor.slots[0]
        .target
        .tracks
        .iter()
        .any(|track| track.property == MotionProperty::Scale
          && track.values == [MotionValue::Vector2([0.955, 0.955])])
    );
    assert_eq!(descriptor.value_bindings.len(), 1);
    assert_eq!(
      descriptor.value_bindings[0].composition,
      battlement::MotionBindingComposition::Compose
    );
  }
  let _ = reactant.shutdown(&mut ()).into_groups();
}

fn start<G: 'static>(
  reactant: &mut Reactant<G>,
  game: &mut G,
  document: &UiDocument,
) -> UiDocument {
  let (snapshot, commit) = reactant
    .begin_session(game)
    .unwrap()
    .into_parts(snapshot(document));
  let _ = commit.into_groups();
  snapshot
    .ui
    .into_iter()
    .find(|value| value.document_id == document.document_id)
    .unwrap()
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
        GameObjectKind::UiDocument(UiDocumentState::new(document.root_id).panel_settings(
          PanelSettings::new().scale_mode(PanelScaleMode::ConstantLogicalPixelSize),
        )),
      )
      .parent_scene(ParentScene::Persistent),
    ],
    camera_id,
  )
}

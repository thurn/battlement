use battlement::{
  CameraState, CommandBody, GameObject, GameObjectKind, MotionColor, MotionDescriptor, MotionLayer,
  MotionLength, MotionProperty, MotionValue, ObjectId, PanelScaleMode, PanelSettings, ParentScene,
  PreparedAsset, Prop, Scene, SceneId, SessionId, Snapshot, UiDocument, UiDocumentState,
  UiVisualElementProperties, VisualElementUpdate,
};
use battlement_reactant::{
  executor::{BoxFuture, SpawnedTask, Spawner},
  host::View,
  motion::MotionStyle,
  paint::{PaintFill, PaintStyle},
  runtime::{Reactant, ReactantCommit},
};

struct IdleSpawner;

#[test]
fn static_paint_updates_and_removal_preserve_the_host_and_gesture_generation() {
  let document = UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4());
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), |color: &Option<MotionColor>| {
    let paint = color.map_or_else(PaintStyle::new, |color| {
      PaintStyle::new()
        .background(PaintFill::Color(color))
        .clip_polygon(
          [[0., 0.], [100., 0.], [50., 100.]].map(|point| point.map(MotionLength::percent)),
        )
    });
    View::new()
      .paint(paint)
      .while_focus_visible(MotionStyle::new().opacity(0.8))
  });
  let mut color = Some(MotionColor::new(1., 0., 0., 1.));
  let (initial, commit) = runtime
    .begin_session(&mut color)
    .unwrap()
    .into_parts(self::snapshot(&document));
  let _ = commit.into_groups();
  let host = &initial.ui[0].children[0];
  let Prop::Set(first) = &host.element.visual_element().motion else {
    panic!("paint descriptor missing")
  };
  assert!(first.initial.is_none());
  assert_eq!(first.slots.len(), 1);
  assert_eq!(first.slots[0].layer, MotionLayer::FocusVisible);
  assert_eq!(first.static_baseline.len(), 2);
  color = Some(MotionColor::new(0., 1., 0., 1.));
  let (updated_host, next) = self::paint_update(runtime.refresh(&mut color).unwrap());
  assert_eq!(updated_host, host.object_id);
  assert_eq!(next.generation, first.generation);
  assert_eq!(next.slots, first.slots);
  assert_eq!(
    next.static_baseline[0].property,
    MotionProperty::BackgroundColor
  );
  assert_eq!(
    next.static_baseline[0].value,
    MotionValue::Color(color.unwrap())
  );
  color = None;
  let (updated_host, cleared) = self::paint_update(runtime.refresh(&mut color).unwrap());
  assert_eq!(updated_host, host.object_id);
  assert_eq!(cleared.generation, first.generation);
  assert_eq!(cleared.slots, first.slots);
  assert!(cleared.static_baseline.is_empty());
  assert!(runtime.refresh(&mut color).unwrap().is_empty());
  let _ = runtime.shutdown(&mut color).into_groups();
}

#[test]
fn static_only_paint_has_no_animation_slots_or_entrance_target() {
  let document = UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4());
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), |_: &()| {
    View::new()
      .paint(PaintStyle::new().background(PaintFill::Color(MotionColor::new(1., 0., 0., 1.))))
  });
  let (initial, commit) = runtime
    .begin_session(&mut ())
    .unwrap()
    .into_parts(self::snapshot(&document));
  let _ = commit.into_groups();
  let Prop::Set(descriptor) = &initial.ui[0].children[0].element.visual_element().motion else {
    panic!("paint descriptor missing")
  };
  assert!(descriptor.slots.is_empty());
  assert!(descriptor.initial.is_none());
  let _ = runtime.shutdown(&mut ()).into_groups();
}

fn paint_update(commit: ReactantCommit) -> (ObjectId, MotionDescriptor) {
  let commands = commit
    .into_groups()
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
  let [CommandBody::VisualElementUpdate(update)] = commands.as_slice() else {
    panic!("one sparse update expected")
  };
  let VisualElementUpdate::Properties { object_id, element } = update.as_ref() else {
    panic!("property update expected")
  };
  let Prop::Set(value) = &element.visual_element().motion else {
    panic!("paint update missing")
  };
  (*object_id, value.clone())
}

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
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

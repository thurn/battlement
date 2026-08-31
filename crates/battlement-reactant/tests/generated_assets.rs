use std::slice;

use battlement::{
  BackgroundPositionKeyword, BackgroundRepeatMode, BackgroundSize, BackgroundSource, CameraState,
  GameObject, GameObjectKind, ImageSource, ObjectId, PanelScaleMode, PanelSettings, ParentScene,
  PreparedAsset, Prop, ResponseMessage, Scene, SceneId, SessionId, Snapshot, StyleValue,
  UiDocument, UiDocumentState, UiElement, UiElementKind, UiVisualElementProperties,
};
use battlement_reactant::{
  asset_generator::{self, LogicalInsets, LogicalRect, LogicalSize},
  executor::{BoxFuture, SpawnedTask, Spawner},
  host::View,
  runtime::Reactant,
};
use uuid::Uuid;

asset_generator::generate! {
  @background PANEL {
    @canvas 20px 10px;
    @subject 1px 2px 17px 7px;
    background: linear-gradient(red, blue);
  }
}

asset_generator::generate! {
  @nine-slice FRAME {
    @canvas 30px 18px;
    @slices 2px 3px 4px 5px;
    @raster-scale 4;
    border: 1px dashed red;
  }
}

asset_generator::generate! {
  @text-image TITLE {
    @canvas 80px 24px;
    @font-file unity("Assets/title.ttf");
    content: "Ready";
    font-size: 16px;
    text-shadow: 1px 2px red, 2px 3px blue;
  }
}

struct IdleSpawner;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

#[test]
fn macro_emits_copyable_typed_handles_and_unique_registrations() {
  fn copy<T: Copy>(value: T) -> T {
    value
  }

  assert_eq!(copy(PANEL).canvas_size(), LogicalSize::new(20.0, 10.0));
  assert_eq!(
    PANEL.subject_bounds(),
    LogicalRect::new(1.0, 2.0, 17.0, 7.0)
  );
  assert_eq!(FRAME.slice_insets(), LogicalInsets::new(2.0, 3.0, 4.0, 5.0));
  assert_eq!(TITLE.canvas_size(), LogicalSize::new(80.0, 24.0));

  let mut registrations = asset_generator::registrations().collect::<Vec<_>>();
  registrations.sort_by_key(|value| value.address);
  assert_eq!(registrations.len(), 3);
  assert!(
    registrations
      .windows(2)
      .all(|pair| pair[0].address != pair[1].address)
  );
  assert!(registrations.iter().all(|value| {
    value.address.starts_with("battlement-reactant/generated/")
      && value.address.ends_with(".png")
      && value.address.len() == "battlement-reactant/generated/".len() + 64 + 4
  }));
}

#[test]
fn generated_background_styles_are_paint_only() {
  let style = PANEL.background_style();
  assert!(matches!(
    style.background_image,
    Prop::Set(StyleValue::Value(BackgroundSource::Texture(ref address)))
      if address == &PANEL.texture_address()
  ));
  assert!(matches!(
    style.background_position_x,
    Prop::Set(StyleValue::Value(value)) if value.keyword == BackgroundPositionKeyword::Left
  ));
  assert!(matches!(
    style.background_position_y,
    Prop::Set(StyleValue::Value(value)) if value.keyword == BackgroundPositionKeyword::Top
  ));
  assert!(matches!(
    style.background_repeat,
    Prop::Set(StyleValue::Value(value))
      if value.x == BackgroundRepeatMode::NoRepeat && value.y == BackgroundRepeatMode::NoRepeat
  ));
  assert!(matches!(
    style.background_size,
    Prop::Set(StyleValue::Value(BackgroundSize::Axes { .. }))
  ));
  assert!(matches!(style.width, Prop::Unset));
  assert!(matches!(style.height, Prop::Unset));

  let sliced = FRAME.background_style();
  assert!(matches!(
    sliced.unity_slice_top,
    Prop::Set(StyleValue::Value(value)) if value == 8
  ));
  assert!(matches!(
    sliced.unity_slice_right,
    Prop::Set(StyleValue::Value(value)) if value == 12
  ));
  assert!(matches!(
    sliced.unity_slice_scale,
    Prop::Set(StyleValue::Value(value)) if value.0 == 0.25
  ));
  assert!(matches!(sliced.width, Prop::Unset));
  assert!(matches!(sliced.height, Prop::Unset));
}

#[test]
fn generated_image_lowers_to_exactly_one_native_image_host() {
  let document = UiDocument::new(ObjectId::new_v4());
  let root_id = document.root_id;
  let mut game = ();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_| PANEL.image());
  let rendered = reactant
    .begin_session(&mut game)
    .expect("generated image render succeeds")
    .into_response(snapshot(SessionId::new_v4(), slice::from_ref(&document)));
  let rendered = rendered
    .messages
    .into_iter()
    .find_map(|message| match message {
      ResponseMessage::Snapshot(value) => Some(value),
      ResponseMessage::Batch(_) => None,
    })
    .expect("initial render produces a snapshot");

  let root = rendered
    .ui
    .iter()
    .find(|value| value.root_id == root_id)
    .expect("registered document exists");
  assert_eq!(root.children.len(), 1);
  assert_eq!(root.children[0].element.kind(), UiElementKind::Image);
  assert!(root.children[0].children.is_empty());
  let UiElement::Image(image) = &root.children[0].element else {
    unreachable!("kind checked above");
  };
  assert_eq!(
    image.source,
    Prop::Set(ImageSource::Texture(PANEL.texture_address()))
  );
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn generated_style_is_independent_of_host_method_order() {
  let first_document = UiDocument::new(ObjectId::new_v4());
  let second_document = UiDocument::new(ObjectId::new_v4());
  let mut game = ();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(first_document.clone(), |_| {
    View::new().style(PANEL.background_style()).name("panel")
  });
  reactant.register_root(second_document.clone(), |_| {
    View::new().name("panel").style(PANEL.background_style())
  });
  let rendered = reactant
    .begin_session(&mut game)
    .expect("generated style render succeeds")
    .into_response(snapshot(
      SessionId::new_v4(),
      &[first_document.clone(), second_document.clone()],
    ));
  let rendered = rendered
    .messages
    .into_iter()
    .find_map(|message| match message {
      ResponseMessage::Snapshot(value) => Some(value),
      ResponseMessage::Batch(_) => None,
    })
    .expect("initial render produces a snapshot");
  let element = |root_id| {
    rendered
      .ui
      .iter()
      .find(|value| value.root_id == root_id)
      .expect("registered document exists")
      .children[0]
      .element
      .visual_element()
  };

  assert_eq!(
    element(first_document.root_id),
    element(second_document.root_id)
  );
  let _ = reactant.shutdown(&mut game).into_groups();
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
    vec![
      PreparedAsset::Scene("test/scene".into()),
      PreparedAsset::Texture(PANEL.texture_address()),
    ],
    vec![Scene::new(SceneId::new_v4(), "test/scene")],
    objects,
    camera_id,
  )
}

fn object_id(value: u128) -> ObjectId {
  ObjectId::from_uuid(Uuid::from_u128(value)).expect("fixture ID is nonzero")
}

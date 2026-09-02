use std::{collections::VecDeque, slice, sync::Arc};

use battlement::{
  BackgroundPositionKeyword, BackgroundRepeatMode, BackgroundSize, BackgroundSource, CameraState,
  ClientMessage, Command, Connect, GameObject, GameObjectKind, ImageSource, ObjectId,
  PanelScaleMode, PanelSettings, ParentScene, PreparedAsset, Prop, Response, ResponseMessage,
  Scene, SceneId, SessionId, Snapshot, StyleValue, UiDocument, UiDocumentState, UiElement,
  UiElementKind, UiEventAction, UiEventResponse, UiVisualElementProperties,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_native::{Engine, EngineError};
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
  @background PANEL_DUPLICATE {
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

struct SnapshotEngine {
  responses: VecDeque<Response>,
}

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

impl Engine for SnapshotEngine {
  type ActionPayload = ();
  type ErrorCode = ();
  type Command = Command;

  fn connect(&mut self, _message: Connect) -> Result<Response, EngineError> {
    Ok(self.responses.pop_front().expect("fixture has a snapshot"))
  }

  fn submit(&mut self, _message: ClientMessage<(), ()>) -> Result<Response, EngineError> {
    Err(EngineError::new("fixture does not accept actions"))
  }

  fn submit_ui_event(
    &mut self,
    message: UiEventAction,
  ) -> Result<UiEventResponse<Self::Command>, EngineError> {
    Ok(UiEventResponse::from_event(
      &message.event,
      Response::empty(message.session_id),
    ))
  }

  fn poll(&mut self) -> Result<Option<Response>, EngineError> {
    Ok(self.responses.pop_front())
  }
}

#[test]
fn macro_emits_copyable_typed_handles_and_linked_registrations() {
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
  assert_eq!(registrations.len(), 4);
  assert_eq!(PANEL.texture_address(), PANEL_DUPLICATE.texture_address());
  assert_eq!(
    registrations
      .iter()
      .filter(|value| value.address == PANEL.texture_address().as_str())
      .count(),
    2
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
  let response = reactant
    .begin_session(&mut game)
    .expect("generated image render succeeds")
    .into_response(snapshot(SessionId::new_v4(), slice::from_ref(&document)));
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene("test/scene");
  catalog.add_textures([
    PANEL.texture_address(),
    FRAME.texture_address(),
    TITLE.texture_address(),
  ]);
  let client = FakeClient::connect(
    SnapshotEngine {
      responses: [response.clone()].into(),
    },
    Arc::new(catalog),
  );
  assert_eq!(
    client.world().prepared_assets(),
    self::expected_prepared_assets()
  );
  let rendered = response
    .messages
    .into_iter()
    .find_map(|message| match message {
      ResponseMessage::Snapshot(value) => Some(value),
      ResponseMessage::Batch(_) => None,
    })
    .expect("initial render produces a snapshot");
  assert_eq!(rendered.prepared_assets, self::expected_prepared_assets());

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
fn every_conversion_restores_the_sorted_generated_union_and_preserves_callers() {
  let document = UiDocument::new(ObjectId::new_v4());
  let mut game = ();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_| View::new());

  for _ in 0..2 {
    let mut state = snapshot(SessionId::new_v4(), slice::from_ref(&document));
    state
      .prepared_assets
      .push(PreparedAsset::sprite("caller/icon"));
    let (state, commit) = reactant
      .begin_session(&mut game)
      .expect("generated catalog session renders")
      .into_parts(state);
    let mut expected = self::expected_prepared_assets();
    expected.insert(1, PreparedAsset::sprite("caller/icon"));
    assert_eq!(state.prepared_assets, expected);
    let _ = commit.into_groups();
  }

  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn fake_client_applies_the_generated_union_on_authoritative_replacement() {
  let document = UiDocument::new(ObjectId::new_v4());
  let mut game = ();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_| View::new());
  let mut response = |reactant: &mut Reactant<()>, state| {
    reactant
      .begin_session(&mut game)
      .expect("fake-client fixture renders")
      .into_response(state)
  };
  let session_id = SessionId::new_v4();
  let mut initial = snapshot(session_id, slice::from_ref(&document));
  initial
    .prepared_assets
    .push(PreparedAsset::sprite("caller/icon"));
  let initial = response(&mut reactant, initial);
  let mut replacement = snapshot(session_id, slice::from_ref(&document));
  replacement
    .prepared_assets
    .push(PreparedAsset::material("caller/material"));
  let replacement = response(&mut reactant, replacement);
  let _ = reactant.shutdown(&mut game).into_groups();

  let mut catalog = self::catalog();
  catalog.add_sprite("caller/icon");
  catalog.add_material("caller/material");
  let mut client = FakeClient::connect(
    SnapshotEngine {
      responses: [initial, replacement].into(),
    },
    Arc::new(catalog),
  );
  let mut expected = self::expected_prepared_assets();
  expected.insert(1, PreparedAsset::sprite("caller/icon"));
  assert_eq!(client.world().prepared_assets(), expected);
  client.poll();
  expected[1] = PreparedAsset::material("caller/material");
  assert_eq!(client.world().prepared_assets(), expected);
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
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(SceneId::new_v4(), "test/scene")],
    objects,
    camera_id,
  )
}

fn expected_prepared_assets() -> Vec<PreparedAsset> {
  let mut addresses = [
    PANEL.texture_address(),
    FRAME.texture_address(),
    TITLE.texture_address(),
  ];
  addresses.sort_by(|left, right| left.as_str().cmp(right.as_str()));
  let mut prepared = vec![PreparedAsset::Scene("test/scene".into())];
  prepared.extend(addresses.into_iter().map(PreparedAsset::Texture));
  prepared
}

fn catalog() -> FakeAssetCatalog {
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene("test/scene");
  catalog.add_textures([
    PANEL.texture_address(),
    FRAME.texture_address(),
    TITLE.texture_address(),
  ]);
  catalog
}

fn object_id(value: u128) -> ObjectId {
  ObjectId::from_uuid(Uuid::from_u128(value)).expect("fixture ID is nonzero")
}

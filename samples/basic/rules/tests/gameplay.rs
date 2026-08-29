use battlement::{
  DragMode, GameObjectKind, PointerButton, PointerEvent, PreparedAsset, ScreenPosition, Vector3,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient, client::PointerInput};
use battlement_rules::{
  BLUE_MATERIAL, BasicEngine, CONTENT_SCENE, CUBE_IDS, FONT, STATUS_ID, VisualState,
  WHITE_MATERIAL, YELLOW_MATERIAL,
};

#[test]
fn initial_world_contains_interactive_cubes_and_prepared_assets() {
  let client = self::client();

  self::assert_status_contains(&client, VisualState::Connected.registry_key());

  assert_eq!(client.world().object_count(), 8);
  assert_eq!(
    client
      .world()
      .objects()
      .filter(|object| matches!(object.kind(), GameObjectKind::Cube { .. }))
      .count(),
    3
  );
  for cube_id in CUBE_IDS {
    let cube = client.assert_object(cube_id);
    assert_eq!(
      cube.pointer_events(),
      &[PointerEvent::Enter, PointerEvent::Exit, PointerEvent::Click]
    );
    assert_eq!(
      cube
        .material(0)
        .expect("cube should have a material")
        .as_str(),
      WHITE_MATERIAL
    );
  }
  assert_eq!(
    client.assert_object(CUBE_IDS[0]).drag_mode(),
    Some(DragMode::SnapToPointer)
  );
  assert_eq!(
    client.assert_object(CUBE_IDS[1]).drag_mode(),
    Some(DragMode::PreserveOffset)
  );
  assert_eq!(client.assert_object(CUBE_IDS[2]).drag_mode(), None);
  for asset in [
    PreparedAsset::scene(CONTENT_SCENE),
    PreparedAsset::material(WHITE_MATERIAL),
    PreparedAsset::material(YELLOW_MATERIAL),
    PreparedAsset::material(BLUE_MATERIAL),
    PreparedAsset::text_mesh_pro_font(FONT),
  ] {
    assert!(client.world().is_prepared(&asset));
  }
  self::assert_status_contains(&client, "initial snapshot");
}

#[test]
fn hovering_a_cube_updates_its_material_and_visible_status() {
  let mut client = self::client();

  client.move_pointer(Some(CUBE_IDS[0]), self::pointer_input());

  self::assert_status_contains(&client, VisualState::Hovered.registry_key());
  self::assert_material(&client, CUBE_IDS[0], YELLOW_MATERIAL);
  self::assert_status_contains(&client, "pointer enter");
  self::assert_status_contains(&client, "response: immediate");

  client.move_pointer(None, self::pointer_input());

  self::assert_status_contains(&client, VisualState::HoverRestored.registry_key());
  self::assert_material(&client, CUBE_IDS[0], WHITE_MATERIAL);
  self::assert_status_contains(&client, "pointer exit");
}

#[test]
fn clicking_a_cube_moves_it_and_updates_visible_status() {
  let mut client = self::client();

  client.click(CUBE_IDS[2]);

  self::assert_status_contains(&client, VisualState::ClickPlaced.registry_key());
  client.assert_world_position(CUBE_IDS[2], Vector3::new(2.0, 0.0, 2.0), 1e-9);
  self::assert_status_contains(&client, "pointer click");
  self::assert_status_contains(&client, "500 ms move tween");

  client.click(CUBE_IDS[2]);

  self::assert_status_contains(&client, VisualState::ClickRestored.registry_key());
  client.assert_world_position(CUBE_IDS[2], Vector3::new(2.0, 0.0, 0.0), 1e-9);
}

#[test]
fn dragging_a_cube_commits_its_world_position_and_updates_status() {
  let mut client = self::client();
  let destination = Vector3::new(-0.75, 0.0, 1.5);

  client.drag_start(CUBE_IDS[0], self::pointer_input());
  self::assert_status_contains(&client, VisualState::DragInFlight.registry_key());
  self::assert_status_contains(&client, "drag start");
  self::assert_status_contains(&client, "local pointer capture");

  client.drag_end(CUBE_IDS[0], self::pointer_input(), destination);

  self::assert_status_contains(&client, VisualState::DragPlaced.registry_key());
  client.assert_world_position(CUBE_IDS[0], destination, 1e-9);
  self::assert_status_contains(&client, "drag end");
  self::assert_status_contains(&client, "commit world position");
}

#[test]
fn first_action_queues_one_visible_polled_change_on_another_cube() {
  let mut client = self::client();
  client.poll();
  self::assert_material(&client, CUBE_IDS[2], WHITE_MATERIAL);

  client.move_pointer(Some(CUBE_IDS[0]), self::pointer_input());
  client.poll();

  self::assert_material(&client, CUBE_IDS[2], BLUE_MATERIAL);
  self::assert_status_contains(&client, "cube C → blue");
  self::assert_status_contains(&client, "response: polled");
  let before = client.world().clone();

  client.poll();

  assert_eq!(client.world(), &before);
}

#[test]
fn visual_state_enum_matches_the_ditto_registry() {
  assert_eq!(
    battlement_rules::DITTO_VISUAL_STATE_REGISTRY
      .matches("[[states]]")
      .count(),
    VisualState::ALL.len()
  );
  for state in VisualState::ALL {
    assert!(
      battlement_rules::DITTO_VISUAL_STATE_REGISTRY
        .contains(&format!("key = \"{}\"", state.registry_key()))
    );
  }
}

fn client() -> FakeClient<BasicEngine> {
  FakeClient::connect(
    battlement_rules::create_engine().expect("engine should be created"),
    self::asset_catalog(),
  )
}

fn asset_catalog() -> FakeAssetCatalog {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene(CONTENT_SCENE);
  for material in [WHITE_MATERIAL, YELLOW_MATERIAL, BLUE_MATERIAL] {
    assets.add_material(material);
  }
  assets.add_text_mesh_pro_font(FONT);
  assets
}

fn pointer_input() -> PointerInput {
  PointerInput {
    pointer_id: 0,
    screen_position: ScreenPosition::default(),
    world_hit: Vector3::default(),
    button: PointerButton::Left,
  }
}

fn assert_material(
  client: &FakeClient<BasicEngine>,
  cube_id: battlement::ObjectId,
  expected: &str,
) {
  assert_eq!(
    client
      .assert_object(cube_id)
      .material(0)
      .expect("cube should have a material")
      .as_str(),
    expected
  );
}

fn assert_status_contains(client: &FakeClient<BasicEngine>, expected: &str) {
  let text = client
    .assert_object(STATUS_ID)
    .text()
    .expect("status should be text");
  assert!(
    text.text.contains(expected),
    "expected status {text:?} to contain {expected:?}"
  );
}

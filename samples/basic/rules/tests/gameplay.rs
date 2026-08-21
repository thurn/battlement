use masonry::{
    GameObjectKind, PointerButton, PointerEvent, PreparedAsset, ScreenPosition, Vector3,
};
use masonry_fake::{assets::FakeAssetCatalog, client::FakeClient, client::PointerInput};
use masonry_rules::{
    BLUE_MATERIAL, BasicEngine, CONTENT_SCENE, CUBE_IDS, FONT, GRAY_MATERIAL, STATUS_ID,
    YELLOW_MATERIAL,
};

#[test]
fn initial_world_contains_interactive_cubes_and_prepared_assets() {
    let client = self::client();

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
            cube.material(0)
                .expect("cube should have a material")
                .as_str(),
            GRAY_MATERIAL
        );
    }
    for asset in [
        PreparedAsset::scene(CONTENT_SCENE),
        PreparedAsset::material(GRAY_MATERIAL),
        PreparedAsset::material(YELLOW_MATERIAL),
        PreparedAsset::material(BLUE_MATERIAL),
        PreparedAsset::font(FONT),
    ] {
        assert!(client.world().is_prepared(&asset));
    }
    self::assert_status_contains(&client, "initial snapshot");
}

#[test]
fn hovering_a_cube_updates_its_material_and_visible_status() {
    let mut client = self::client();

    client.move_pointer(Some(CUBE_IDS[0]), self::pointer_input());

    self::assert_material(&client, CUBE_IDS[0], YELLOW_MATERIAL);
    self::assert_status_contains(&client, "pointer enter");
    self::assert_status_contains(&client, "response: immediate");

    client.move_pointer(None, self::pointer_input());

    self::assert_material(&client, CUBE_IDS[0], GRAY_MATERIAL);
    self::assert_status_contains(&client, "pointer exit");
}

#[test]
fn clicking_a_cube_moves_it_and_updates_visible_status() {
    let mut client = self::client();

    client.click(CUBE_IDS[1]);

    client.assert_world_position(CUBE_IDS[1], Vector3::new(0.0, 0.0, 2.0), 1e-9);
    self::assert_status_contains(&client, "pointer click");
    self::assert_status_contains(&client, "500 ms move tween");

    client.click(CUBE_IDS[1]);

    client.assert_world_position(CUBE_IDS[1], Vector3::new(0.0, 0.0, 0.0), 1e-9);
}

#[test]
fn first_action_queues_one_visible_polled_change_on_another_cube() {
    let mut client = self::client();
    client.poll();
    self::assert_material(&client, CUBE_IDS[2], GRAY_MATERIAL);

    client.move_pointer(Some(CUBE_IDS[0]), self::pointer_input());
    client.poll();

    self::assert_material(&client, CUBE_IDS[2], BLUE_MATERIAL);
    self::assert_status_contains(&client, "cube C → blue");
    self::assert_status_contains(&client, "response: polled");
    let before = client.world().clone();

    client.poll();

    assert_eq!(client.world(), &before);
}

fn client() -> FakeClient<BasicEngine> {
    FakeClient::connect(
        masonry_rules::create_engine().expect("engine should be created"),
        self::asset_catalog(),
    )
}

fn asset_catalog() -> FakeAssetCatalog {
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene(CONTENT_SCENE);
    for material in [GRAY_MATERIAL, YELLOW_MATERIAL, BLUE_MATERIAL] {
        assets.add_material(material);
    }
    assets.add_font(FONT);
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

fn assert_material(client: &FakeClient<BasicEngine>, cube_id: masonry::ObjectId, expected: &str) {
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

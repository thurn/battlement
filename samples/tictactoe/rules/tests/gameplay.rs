use std::time::Duration;

use battlement::{
    ImageFit, ImageState, ObjectId, PointerEvent, PreparedAsset, TextureAddress, Vector3,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient, time::ManualClock};
use battlement_rules::{
    BOARD_ID, BOARD_TEXTURE, CONTENT_SCENE, FONT, O_TEXTURE, STATUS_ID, TicTacToeEngine, X_TEXTURE,
};

const BOARD_CENTER_Y: f64 = -0.7;
const CELL_SIZE: f64 = 1.92;

#[test]
fn initial_world_contains_clickable_board_and_prepared_art() {
    let (client, _) = self::client(0);
    let board = client.assert_object(BOARD_ID);

    assert_eq!(client.world().object_count(), 4);
    assert_eq!(board.pointer_events(), &[PointerEvent::Click]);
    assert_eq!(
        board
            .image()
            .expect("board should be an image")
            .texture
            .as_str(),
        BOARD_TEXTURE
    );
    for address in [BOARD_TEXTURE, X_TEXTURE, O_TEXTURE] {
        assert!(
            client
                .world()
                .is_prepared(&PreparedAsset::Texture(TextureAddress::new(address)))
        );
    }
    client.assert_text(STATUS_ID, "Your turn — click an empty square");
}

#[test]
fn board_hits_create_player_marks_in_row_major_cells() {
    for (index, expected) in [
        (0, Vector3::new(-1.92, 1.22, -0.05)),
        (4, Vector3::new(0.0, -0.7, -0.05)),
        (8, Vector3::new(1.92, -2.62, -0.05)),
    ] {
        let (mut client, _) = self::client(index as u64);
        let before = client.checkpoint();

        self::click_cell(&mut client, index);

        let marker_id = client.assert_one_object_created_since(before);
        client.assert_world_position(marker_id, expected, 1e-9);
        self::assert_mark(&client, marker_id, X_TEXTURE);
    }

    let (mut client, _) = self::client(9);
    let before = client.world().clone();
    client.click_at(BOARD_ID, Vector3::new(3.6, 0.0, 0.0));
    assert_eq!(client.world(), &before);
}

#[test]
fn player_move_is_immediate_and_ai_move_appears_at_the_deadline() {
    let (mut client, clock) = self::client(7);
    let before_player = client.checkpoint();

    self::click_cell(&mut client, 4);

    let player_marker = client.assert_one_object_created_since(before_player);
    self::assert_mark(&client, player_marker, X_TEXTURE);
    client.assert_text(STATUS_ID, "Computer thinking…");
    assert!(!client.world().input_enabled());

    clock.advance(Duration::from_millis(499));
    client.poll();
    assert_eq!(self::marker_ids(&client), vec![player_marker]);
    client.assert_text(STATUS_ID, "Computer thinking…");

    let before_ai = client.checkpoint();
    clock.advance(Duration::from_millis(1));
    client.poll();
    let ai_marker = client.assert_one_object_created_since(before_ai);
    self::assert_mark(&client, ai_marker, O_TEXTURE);
    client.assert_text(STATUS_ID, "Your turn — click an empty square");
    assert!(client.world().input_enabled());
}

#[test]
fn occupied_cell_leaves_the_visible_world_unchanged() {
    let (mut client, clock) = self::client(7);
    self::click_cell(&mut client, 4);
    clock.advance(Duration::from_millis(500));
    client.poll();
    let before = client.world().clone();

    self::click_cell(&mut client, 4);

    assert_eq!(client.world(), &before);
}

#[test]
fn completed_round_reports_the_outcome_and_resets_on_the_next_click() {
    let (mut client, clock) = self::client(7);
    self::play_round(&mut client, &clock, &[0, 1, 2]);
    let status = client
        .assert_object(STATUS_ID)
        .text()
        .expect("status should be text")
        .text
        .clone();
    let markers = self::marker_ids(&client);

    assert!(
        status.contains("win") || status.contains("Draw"),
        "expected a completed round, got {status:?}",
    );

    self::click_cell(&mut client, 4);

    client.assert_text(STATUS_ID, "Your turn — click an empty square");
    assert!(client.world().input_enabled());
    for marker_id in markers {
        client.assert_object_absent(marker_id);
    }
}

#[test]
fn winning_rows_columns_and_diagonals_are_reported_in_the_world() {
    for cells in [[0, 1, 2], [0, 3, 6], [0, 4, 8], [2, 4, 6]] {
        let (mut client, clock) = self::client(7);
        self::play_round(&mut client, &clock, &cells);

        client.assert_text(STATUS_ID, "You win! Click the board to play again.");
        assert!(client.world().input_enabled());
    }
}

#[test]
fn computer_win_and_draw_are_reported_in_the_world() {
    for (seed, cells, expected) in [
        (
            3,
            [0, 2, 3, 7, 8],
            "Computer wins. Click the board to play again.",
        ),
        (0, [0, 2, 3, 7, 8], "Draw! Click the board to play again."),
    ] {
        let (mut client, clock) = self::client(seed);
        self::play_round(&mut client, &clock, &cells);

        client.assert_text(STATUS_ID, expected);
        assert!(client.world().input_enabled());
    }
}

fn play_round(client: &mut FakeClient<TicTacToeEngine>, clock: &ManualClock, cells: &[usize]) {
    for cell in cells {
        let status = client
            .assert_object(STATUS_ID)
            .text()
            .expect("status should be text")
            .text
            .as_str();
        if status.contains("win") || status.contains("Draw") {
            return;
        }
        self::click_cell(client, *cell);
        if !client.world().input_enabled() {
            clock.advance(Duration::from_millis(500));
            client.poll();
        }
    }
}

fn client(seed: u64) -> (FakeClient<TicTacToeEngine>, ManualClock) {
    FakeClient::connect_clocked(
        |clock| battlement_rules::create_seeded_engine(seed, move || clock.now()),
        self::asset_catalog(),
    )
}

fn asset_catalog() -> FakeAssetCatalog {
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene(CONTENT_SCENE);
    assets.add_textures([BOARD_TEXTURE, X_TEXTURE, O_TEXTURE]);
    assets.add_font(FONT);
    assets
}

fn click_cell(client: &mut FakeClient<TicTacToeEngine>, index: usize) {
    client.click_at(BOARD_ID, self::cell_position(index));
}

fn cell_position(index: usize) -> Vector3 {
    let row = index / 3;
    let column = index % 3;
    Vector3::new(
        (column as f64 - 1.0) * CELL_SIZE,
        BOARD_CENTER_Y + (1.0 - row as f64) * CELL_SIZE,
        0.0,
    )
}

fn marker_ids(client: &FakeClient<TicTacToeEngine>) -> Vec<ObjectId> {
    client
        .world()
        .images()
        .filter_map(|(object, image)| {
            let texture = image.texture.as_str();
            (texture == X_TEXTURE || texture == O_TEXTURE).then_some(object.id())
        })
        .collect()
}

fn assert_mark(client: &FakeClient<TicTacToeEngine>, object_id: ObjectId, texture: &str) {
    client.assert_image(
        object_id,
        &ImageState {
            fit: ImageFit::Contain,
            ..ImageState::new(texture, 2.25, 2.25)
        },
    );
}

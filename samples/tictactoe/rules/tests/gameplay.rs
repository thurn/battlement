use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
    time::{Duration, Instant},
};

use masonry::{
    GameObjectKind, ImageFit, ImageState, ObjectId, PointerButton, PointerEvent, PreparedAsset,
    ScreenPosition, TextureAddress, Vector3,
};
use masonry_fake::{
    assets::FakeAssetCatalog,
    client::{FakeClient, PointerInput},
    time::ManualClock,
};
use masonry_rules::TicTacToeEngine;

const CONTENT_SCENE: &str = "tictactoe/content";
const BOARD_TEXTURE: &str = "tictactoe/board";
const X_TEXTURE: &str = "tictactoe/x";
const O_TEXTURE: &str = "tictactoe/o";
const FONT: &str = "tictactoe/font";
const BOARD_ID: ObjectId = masonry::object_id!("c8c9e10d-585b-45f4-ac19-b76746ed2d25");
const STATUS_ID: ObjectId = masonry::object_id!("9b10a4a0-1367-46a8-9a2c-7c29eef033b1");
const BOARD_CENTER_Y: f64 = -0.7;
const CELL_SIZE: f64 = 1.92;

#[test]
fn initial_world_contains_clickable_board_and_prepared_art() {
    let client = self::client(0);
    let board = client.assert_object(BOARD_ID);

    assert_eq!(client.world().objects().count(), 4);
    assert_eq!(board.pointer_events(), &[PointerEvent::Click]);
    assert!(matches!(
        board.kind(),
        GameObjectKind::Image { image } if image.texture.as_str() == BOARD_TEXTURE
    ));
    for address in [BOARD_TEXTURE, X_TEXTURE, O_TEXTURE] {
        assert!(
            client
                .world()
                .prepared_assets()
                .contains(&PreparedAsset::Texture(TextureAddress::new(address)),)
        );
    }
    assert_eq!(
        self::status_text(&client),
        "Your turn — click an empty square"
    );
}

#[test]
fn board_hits_create_player_marks_in_row_major_cells() {
    for (index, expected) in [
        (0, Vector3::new(-1.92, 1.22, -0.05)),
        (4, Vector3::new(0.0, -0.7, -0.05)),
        (8, Vector3::new(1.92, -2.62, -0.05)),
    ] {
        let mut client = self::client(index as u64);

        self::click_cell(&mut client, index);

        let marker_id = self::latest_created_object(&client);
        client.assert_world_position(marker_id, expected, 1e-9);
        self::assert_mark(&client, marker_id, X_TEXTURE);
    }

    let mut client = self::client(9);
    self::click_at(&mut client, Vector3::new(3.6, 0.0, 0.0));
    assert!(self::marker_ids(&client).is_empty());
    assert_eq!(
        self::status_text(&client),
        "Your turn — click an empty square"
    );
    assert!(client.world().input_enabled());
}

#[test]
fn player_move_is_immediate_and_ai_move_appears_at_the_deadline() {
    let mut client = self::client(7);

    self::click_cell(&mut client, 4);

    let player_marker = self::latest_created_object(&client);
    self::assert_mark(&client, player_marker, X_TEXTURE);
    assert_eq!(self::status_text(&client), "Computer thinking…");
    assert!(!client.world().input_enabled());

    client.advance(Duration::from_millis(499));
    client.poll();
    assert_eq!(self::marker_ids(&client), vec![player_marker]);
    assert_eq!(self::status_text(&client), "Computer thinking…");

    client.advance(Duration::from_millis(1));
    client.poll();
    let ai_marker = self::latest_created_object(&client);
    assert_ne!(ai_marker, player_marker);
    self::assert_mark(&client, ai_marker, O_TEXTURE);
    assert_eq!(
        self::status_text(&client),
        "Your turn — click an empty square"
    );
    assert!(client.world().input_enabled());
}

#[test]
fn occupied_cell_leaves_the_visible_world_unchanged() {
    let mut client = self::client(7);
    self::click_cell(&mut client, 4);
    let player_marker = self::latest_created_object(&client);
    client.advance(Duration::from_millis(500));
    client.poll();
    let status = self::status_text(&client).to_owned();
    let marker_kind = client.assert_object(player_marker).kind().clone();

    self::click_cell(&mut client, 4);

    assert_eq!(client.assert_object(player_marker).kind(), &marker_kind);
    assert_eq!(self::status_text(&client), status);
    assert!(client.world().input_enabled());
}

#[test]
fn completed_round_reports_the_outcome_and_resets_on_the_next_click() {
    let mut client = self::client(7);
    self::play_round(&mut client, &[0, 1, 2]);
    let status = self::status_text(&client).to_owned();
    let markers = self::marker_ids(&client);

    assert!(
        status.contains("win") || status.contains("Draw"),
        "expected a completed round, got {status:?}",
    );

    self::click_cell(&mut client, 4);

    assert_eq!(
        self::status_text(&client),
        "Your turn — click an empty square"
    );
    assert!(client.world().input_enabled());
    for marker_id in markers {
        client.assert_object_absent(marker_id);
    }
}

#[test]
fn winning_rows_columns_and_diagonals_are_reported_in_the_world() {
    for cells in [[0, 1, 2], [0, 3, 6], [0, 4, 8], [2, 4, 6]] {
        let mut client = self::client(7);
        self::play_round(&mut client, &cells);

        assert_eq!(
            self::status_text(&client),
            "You win! Click the board to play again."
        );
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
        let mut client = self::client(seed);
        self::play_round(&mut client, &cells);

        assert_eq!(self::status_text(&client), expected);
        assert!(client.world().input_enabled());
    }
}

fn play_round(client: &mut TestClient, cells: &[usize]) {
    for cell in cells {
        if self::status_text(client).contains("win") || self::status_text(client).contains("Draw") {
            return;
        }
        self::click_cell(client, *cell);
        if !client.world().input_enabled() {
            client.advance(Duration::from_millis(500));
            client.poll();
        }
    }
}

struct TestClient {
    client: FakeClient<TicTacToeEngine>,
    clock: ManualClock,
}

impl TestClient {
    fn advance(&self, duration: Duration) {
        self.clock.advance(duration);
    }
}

impl Deref for TestClient {
    type Target = FakeClient<TicTacToeEngine>;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for TestClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

fn client(seed: u64) -> TestClient {
    let clock = ManualClock::new(Instant::now());
    let engine_clock = clock.clone();
    TestClient {
        client: FakeClient::connect(
            masonry_rules::create_seeded_engine(seed, move || engine_clock.now()),
            Arc::new(self::asset_catalog()),
        ),
        clock,
    }
}

fn asset_catalog() -> FakeAssetCatalog {
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene(CONTENT_SCENE);
    assets.add_texture(BOARD_TEXTURE);
    assets.add_texture(X_TEXTURE);
    assets.add_texture(O_TEXTURE);
    assets.add_font(FONT);
    assets
}

fn click_cell(client: &mut FakeClient<TicTacToeEngine>, index: usize) {
    self::click_at(client, self::cell_position(index));
}

fn click_at(client: &mut FakeClient<TicTacToeEngine>, world_hit: Vector3) {
    let input = PointerInput {
        pointer_id: 0,
        screen_position: ScreenPosition::default(),
        world_hit,
        button: PointerButton::Left,
    };
    let board_id = BOARD_ID;
    client.move_pointer(Some(board_id), input);
    client.pointer_down(board_id, input);
    client.pointer_up(board_id, input);
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
        .objects()
        .filter_map(|object| match object.kind() {
            GameObjectKind::Image { image }
                if image.texture.as_str() == X_TEXTURE || image.texture.as_str() == O_TEXTURE =>
            {
                Some(object.id())
            }
            _ => None,
        })
        .collect()
}

fn latest_created_object(client: &FakeClient<TicTacToeEngine>) -> ObjectId {
    self::marker_ids(client)
        .last()
        .copied()
        .expect("a marker should exist in the simulated world")
}

fn assert_mark(client: &FakeClient<TicTacToeEngine>, object_id: ObjectId, texture: &str) {
    client.assert_object_kind(
        object_id,
        &GameObjectKind::Image {
            image: ImageState {
                fit: ImageFit::Contain,
                ..ImageState::new(texture, 2.25, 2.25)
            },
        },
    );
}

fn status_text(client: &FakeClient<TicTacToeEngine>) -> &str {
    let GameObjectKind::Text { text } = client.assert_object(STATUS_ID).kind() else {
        panic!("status should be text");
    };
    &text.text
}

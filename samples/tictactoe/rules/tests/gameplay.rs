use std::time::Duration;

use battlement::{GameObjectKind, ImageFit, ImageState, ObjectId, PanelPoint, Vector3};
use battlement_fake::assets::FakeAssetCatalog;
use battlement_rules::{
  BOARD_TEXTURE, CONTENT_SCENE, DITTO_SEED, FONT, O_TEXTURE, VisualState, X_TEXTURE,
};
use reactant_testing::Display;

const BOARD_CENTER_Y: f64 = -0.7;
const CELL_SIZE: f64 = 1.92;
const TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn exported_application_mounts_a_snapshot() {
  let display = Display::mount(battlement_rules::application, self::asset_catalog());
  assert!(display.objects().count() >= 1);
}

#[test]
fn initial_world_contains_board_text_and_row_major_hit_regions() {
  let mut display = self::display(0);
  self::synchronize_initial(&mut display);

  assert!(self::image_ids(&display, BOARD_TEXTURE).len() == 1);
  self::assert_text(&display, "TIC TAC TOE — ROUND 1");
  self::assert_text(&display, "Your turn — click an empty square");
  for expected in [
    Vector3::new(-1.92, 1.22, 0.0),
    Vector3::new(0.0, -0.7, 0.0),
    Vector3::new(1.92, -2.62, 0.0),
  ] {
    let hit = self::hit_region_at(&display, expected);
    self::assert_position(&display, hit, expected);
  }
}

#[test]
fn committed_board_hits_create_stable_player_marks_in_row_major_cells() {
  for index in [0, 4, 8] {
    let mut display = self::display(index as u64);
    self::synchronize_initial(&mut display);
    self::click_cell(&mut display, index);
    let marker = self::wait_for_human_move(&mut display, index);

    self::assert_mark(&display, marker, X_TEXTURE);
    self::assert_position(&display, marker, self::cell_position(index, -0.05));
  }

  let mut display = self::display(9);
  self::synchronize_initial(&mut display);
  self::click_world(&mut display, Vector3::new(3.3, BOARD_CENTER_Y, 0.0));
  assert!(self::mark_ids(&display).is_empty());
  self::assert_text(&display, "Your turn — click an empty square");
}

#[test]
fn player_move_is_immediate_and_ai_move_follows_after_thinking() {
  let mut display = self::display(DITTO_SEED);
  self::synchronize_initial(&mut display);
  self::click_cell(&mut display, 4);
  let player = self::wait_for_human_move(&mut display, 4);

  self::assert_mark(&display, player, X_TEXTURE);
  self::assert_text(&display, "Computer thinking…");
  assert_eq!(display.presentation_time(), Duration::ZERO);
  assert_eq!(display.frame(), 0);

  self::synchronize_action(&mut display);
  assert!(self::image_ids(&display, O_TEXTURE).is_empty());
  display.settle();
  let ai = self::image_ids(&display, O_TEXTURE);
  assert_eq!(ai.len(), 1);
  self::assert_mark(&display, ai[0], O_TEXTURE);
  self::assert_text(&display, "Your turn — click an empty square");
  assert_eq!(self::mark_at(&display, 4, X_TEXTURE), Some(player));
}

#[test]
fn occupied_and_outside_hits_leave_the_visible_world_unchanged() {
  let mut display = self::display(DITTO_SEED);
  self::synchronize_initial(&mut display);
  self::play_turn(&mut display, 4);
  let marks = self::mark_ids(&display);

  self::click_cell(&mut display, 4);
  self::click_world(&mut display, Vector3::new(3.3, BOARD_CENTER_Y, 0.0));

  assert_eq!(self::mark_ids(&display), marks);
  self::assert_text(&display, "Your turn — click an empty square");
}

#[test]
fn completed_round_reports_the_outcome_and_resets_on_the_next_hit() {
  let mut display = self::display(DITTO_SEED);
  self::synchronize_initial(&mut display);
  self::play_round(&mut display, &[0, 1, 2]);
  let status = self::status_text(&display);
  assert!(status.contains("win") || status.contains("Draw"));
  let marks = self::mark_ids(&display);

  self::click_world(&mut display, Vector3::new(3.3, BOARD_CENTER_Y, 0.0));
  self::synchronize_action(&mut display);

  self::assert_text(&display, "TIC TAC TOE — ROUND 2");
  self::assert_text(&display, "Your turn — click an empty square");
  for marker in marks {
    assert!(display.object(marker).is_none());
  }
}

#[test]
fn winning_rows_columns_and_diagonals_are_reported_in_the_world() {
  for cells in [[0, 1, 2], [0, 3, 6], [0, 4, 8], [2, 4, 6]] {
    let mut display = self::display(DITTO_SEED);
    self::synchronize_initial(&mut display);
    self::play_round(&mut display, &cells);
    self::assert_text(&display, "You win! Click the board to play again.");
  }
}

#[test]
fn computer_win_draw_and_default_seed_outcomes_are_preserved() {
  for (seed, cells, expected) in [
    (
      3,
      &[0, 2, 3, 7, 8][..],
      "Computer wins. Click the board to play again.",
    ),
    (
      0,
      &[0, 2, 3, 7, 8][..],
      "Draw! Click the board to play again.",
    ),
    (
      DITTO_SEED,
      &[2, 1, 0][..],
      "You win! Click the board to play again.",
    ),
    (
      DITTO_SEED,
      &[6, 5, 0][..],
      "Computer wins. Click the board to play again.",
    ),
    (
      DITTO_SEED,
      &[8, 4, 3, 7, 2][..],
      "Draw! Click the board to play again.",
    ),
  ] {
    let mut display = self::display(seed);
    self::synchronize_initial(&mut display);
    self::play_round(&mut display, cells);
    self::assert_text(&display, expected);
  }
}

#[test]
fn navigation_activation_dispatches_the_same_typed_cell_action() {
  let mut display = self::display(DITTO_SEED);
  self::synchronize_initial(&mut display);
  display.navigate(battlement::NavigationDirection::Right);
  let focused = display
    .focused()
    .expect("a board cell should receive focus");
  assert!(matches!(
    display.object(focused).expect("focused object").kind(),
    GameObjectKind::BoxHitRegion { .. }
  ));
  let position = display.world_point(focused, Vector3::ZERO);
  let index = (0..9)
    .find(|index| self::near(position, self::cell_position(*index, 0.0)))
    .expect("focus should belong to a board cell");

  display.activate_focused();
  self::wait_for_human_move(&mut display, index);
  assert!(self::mark_at(&display, index, X_TEXTURE).is_some());
}

#[test]
fn reconnect_resets_round_state_and_deterministic_visual_registry_is_complete() {
  let mut display = self::display(DITTO_SEED);
  self::synchronize_initial(&mut display);
  self::play_turn(&mut display, 4);
  assert!(!self::mark_ids(&display).is_empty());

  display.reconnect();
  self::synchronize_initial(&mut display);
  assert!(self::mark_ids(&display).is_empty());
  self::assert_text(&display, "TIC TAC TOE — ROUND 1");

  assert_eq!(
    battlement_rules::DITTO_VISUAL_STATE_REGISTRY
      .matches("[[states]]")
      .count(),
    1
  );
  assert!(
    battlement_rules::DITTO_VISUAL_STATE_REGISTRY.contains(&format!(
      "key = \"{}\"",
      VisualState::HumanMove.registry_key()
    ))
  );
}

fn display(seed: u64) -> Display {
  Display::mount(
    move || battlement_rules::create_seeded_application(seed, std::time::Instant::now),
    self::asset_catalog(),
  )
}

fn asset_catalog() -> FakeAssetCatalog {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene(CONTENT_SCENE);
  assets.add_textures([BOARD_TEXTURE, X_TEXTURE, O_TEXTURE]);
  assets.add_text_mesh_pro_font(FONT);
  assets
}

fn synchronize_initial(display: &mut Display) {
  display.poll();
  for _ in 0..4 {
    if display.game_status::<battlement_rules::TicTacToe>() == Some(reactant::GameStatus::Ready) {
      return;
    }
    display.poll();
  }
  panic!("initial game presentation did not become ready");
}

fn wait_for_human_move(display: &mut Display, cell: usize) -> ObjectId {
  for _ in 0..4 {
    if let Some(marker) = self::mark_at(display, cell, X_TEXTURE)
      && self::status_text(display) == "Computer thinking…"
    {
      return marker;
    }
    assert!(display.wait_for_game_output::<battlement_rules::TicTacToe>(TIMEOUT));
    display.poll();
  }
  panic!("human checkpoint did not become visible");
}

fn synchronize_action(display: &mut Display) {
  for _ in 0..8 {
    if display.game_status::<battlement_rules::TicTacToe>() == Some(reactant::GameStatus::Ready) {
      assert!(display.wait_for_game_worker::<battlement_rules::TicTacToe>(TIMEOUT));
      return;
    }
    assert!(display.wait_for_game_output::<battlement_rules::TicTacToe>(TIMEOUT));
    display.poll();
  }
  panic!("game output did not reach the completed-action boundary");
}

fn play_turn(display: &mut Display, cell: usize) {
  self::click_cell(display, cell);
  self::wait_for_human_move(display, cell);
  self::synchronize_action(display);
  display.settle();
}

fn play_round(display: &mut Display, cells: &[usize]) {
  for cell in cells {
    if self::terminal(display) {
      return;
    }
    self::click_cell(display, *cell);
    self::synchronize_action(display);
    if self::status_text(display) == "Computer thinking…" {
      display.settle();
    }
  }
}

fn terminal(display: &Display) -> bool {
  let status = self::status_text(display);
  status.contains("win") || status.contains("Draw")
}

fn click_cell(display: &mut Display, index: usize) {
  self::click_world(display, self::cell_position(index, 0.0));
}

fn click_world(display: &mut Display, position: Vector3) {
  let pixels_per_world_unit = 1080.0 / 11.2;
  display.click_at(PanelPoint::new(
    960.0 + position.x * pixels_per_world_unit,
    540.0 - position.y * pixels_per_world_unit,
  ));
}

fn cell_position(index: usize, z: f64) -> Vector3 {
  let row = index / 3;
  let column = index % 3;
  Vector3::new(
    (column as f64 - 1.0) * CELL_SIZE,
    BOARD_CENTER_Y + (1.0 - row as f64) * CELL_SIZE,
    z,
  )
}

fn assert_text(display: &Display, expected: &str) {
  let y = if expected.starts_with("TIC TAC TOE") {
    4.7
  } else {
    3.75
  };
  assert_eq!(self::text_at(display, y), expected);
}

fn status_text(display: &Display) -> &str {
  self::text_at(display, 3.75)
}

fn text_at(display: &Display, y: f64) -> &str {
  display
    .texts()
    .filter(|(object, _)| {
      let position = display.world_point(object.id(), Vector3::ZERO);
      (position.y - y).abs() < 1e-9
    })
    .last()
    .map(|(_, text)| text.text.as_str())
    .expect("positioned text")
}

fn image_ids(display: &Display, texture: &str) -> Vec<ObjectId> {
  display
    .images()
    .filter_map(|(object, image)| (image.texture.as_str() == texture).then_some(object.id()))
    .collect()
}

fn mark_at(display: &Display, index: usize, texture: &str) -> Option<ObjectId> {
  let expected = self::cell_position(index, -0.05);
  self::image_ids(display, texture)
    .into_iter()
    .find(|id| self::near(display.world_point(*id, Vector3::ZERO), expected))
}

fn hit_region_at(display: &Display, expected: Vector3) -> ObjectId {
  display
    .objects()
    .filter(|object| matches!(object.kind(), GameObjectKind::BoxHitRegion { .. }))
    .find(|object| self::near(display.world_point(object.id(), Vector3::ZERO), expected))
    .map(|object| object.id())
    .expect("row-major hit region")
}

fn assert_mark(display: &Display, id: ObjectId, texture: &str) {
  let GameObjectKind::Image { image } = display.object(id).expect("mark object").kind() else {
    panic!("mark is not an image")
  };
  assert_eq!(
    *image,
    ImageState {
      fit: ImageFit::Contain,
      ..ImageState::new(texture, 2.25, 2.25)
    }
  );
}

fn assert_position(display: &Display, id: ObjectId, expected: Vector3) {
  assert!(self::near(display.world_point(id, Vector3::ZERO), expected));
}

fn near(actual: Vector3, expected: Vector3) -> bool {
  (actual.x - expected.x).abs() < 1e-9
    && (actual.y - expected.y).abs() < 1e-9
    && (actual.z - expected.z).abs() < 1e-9
}

fn mark_ids(display: &Display) -> Vec<ObjectId> {
  self::image_ids(display, X_TEXTURE)
    .into_iter()
    .chain(self::image_ids(display, O_TEXTURE))
    .collect()
}

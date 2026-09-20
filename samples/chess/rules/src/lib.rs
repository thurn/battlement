//! Native Reactant rules engine for the standalone chess sample.

mod ai;
pub mod assets;
pub mod audio;
mod chess_board;
mod chess_prompt;
mod chess_ui_state;
mod cursor;
mod diagnostics;
mod motion;
mod persistence;
mod position;
mod promotion_dialog;
mod reactant_app;
mod reactant_effects;
mod reactant_game;
mod reactant_input;
mod reactant_view;
pub mod visual_state;

pub use position::ChessPiece;
pub use reactant_app::ReactantChessApp;
/// Compatibility name for the sample's exported Reactant engine.
pub type ChessEngine = ReactantChessApp;

use std::time::Duration;

use battlement::{
  AudioClipAddress, GridLayout, ObjectId, PrefabAddress, Quaternion, Vector3, object_id,
};
use battlement_native::EngineError;
use cozy_chess::{Board, Color, File, Move, Piece, Rank, Square};

use crate::assets::{black, music, white};
pub(crate) use crate::audio::{
  CAPTURE_SOUNDS, CASTLE_SOUND, CHECK_SOUND, DRAW_SOUND, DROP_SOUNDS, INVALID_DROP_SOUND,
  PLAYER_LOSS_SOUND, PLAYER_WIN_SOUND, PROMOTION_SOUND, RESET_SOUND, VOLUME_DOWN_SOUND,
  VOLUME_UP_SOUND,
};

pub(crate) const AI_THINK_TIME: Duration = Duration::from_secs(2);
pub(crate) const HIGHLIGHT_HEIGHT: f64 = 0.02;
pub(crate) const HIGHLIGHT_SCALE: f64 = 0.09;
pub(crate) const CAMERA_BUTTON_DEPTH: f64 = 1.5;
pub(crate) const CAMERA_VERTICAL_FOV_RADIANS: f64 = std::f64::consts::PI / 3.0;
pub(crate) const REFRESH_BUTTON_SIZE: f64 = 0.16;
pub(crate) const REFRESH_BUTTON_MARGIN: f64 = 0.12;
pub(crate) const CAMERA_ROTATION: Quaternion =
  Quaternion::new(0.58184814, -0.001219943, 0.0008727778, 0.813296);

/// Addresses of all chess-piece prefabs.
pub const PIECE_PREFABS: [PrefabAddress; 12] = [
  white::PAWN,
  white::ROOK,
  white::KNIGHT,
  white::BISHOP,
  white::QUEEN,
  white::KING,
  black::PAWN,
  black::ROOK,
  black::KNIGHT,
  black::BISHOP,
  black::QUEEN,
  black::KING,
];
/// Machine-readable registry consumed by the Ditto coverage checker.
pub const DITTO_VISUAL_STATE_REGISTRY: &str = include_str!("../../ditto-visual-states.toml");
/// Background-music playlist order.
pub const MUSIC_TRACKS: [AudioClipAddress; 4] = [
  music::CRITICAL,
  music::SWITCH_WITH_ME,
  music::BREAKBEAT_CHIPS,
  music::DRAG_AND_DREAD,
];
/// Delay from the start of “Critical” to its first beat.
pub const CRITICAL_FIRST_BEAT_OFFSET_MS: u64 = 80;
/// Beat interval of “Critical”.
pub const CRITICAL_BEAT_INTERVAL_MS: u64 = 570;
/// Number of “Critical” beats used for the piece spawn-in sequence.
pub const PIECE_SPAWN_BEAT_COUNT: usize = 8;
/// Lifetime of each piece spawn effect.
pub const PIECE_SPAWN_EFFECT_LIFETIME_MS: u64 = 1_000;
/// Duration of the piece spawn-in sequence in milliseconds.
pub const PIECE_SPAWN_SEQUENCE_DURATION_MS: u64 = CRITICAL_FIRST_BEAT_OFFSET_MS
  + (PIECE_SPAWN_BEAT_COUNT - 1) as u64 * CRITICAL_BEAT_INTERVAL_MS
  + PIECE_SPAWN_EFFECT_LIFETIME_MS;
/// Stable identity of the Play button.
pub const PLAY_BUTTON_ID: ObjectId = object_id!("4cf7cb75-ec8f-44ec-88c9-c83ca3869f43");
/// Root of the Reactant chess document for public display scenarios.
pub const REACTANT_CHESS_ROOT_ID: ObjectId = visual_state::ROOT_ID;
/// Stable identity of the new-game refresh button.
pub const REFRESH_BUTTON_ID: ObjectId = object_id!("35b288b3-6d72-48af-aeb9-e8f11d63e3ea");

/// Creates the engine used by the native sample.
pub fn create_engine() -> Result<ReactantChessApp, EngineError> {
  if let Ok(name) = std::env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE") {
    return ReactantChessApp::named_review(&name).map_err(EngineError::new);
  }
  if std::env::var("BATTLEMENT_DITTO_ACTIVE").as_deref() == Ok("1") {
    return Ok(ReactantChessApp::with_think_time(Duration::ZERO));
  }
  Ok(ReactantChessApp::new())
}

/// Creates the sample engine with a caller-owned monotonic clock.
pub fn create_engine_with_clock(now: impl Fn() -> std::time::Instant + 'static) -> ChessEngine {
  ReactantChessApp::with_think_time_and_clock(AI_THINK_TIME, now)
}

/// Creates the sample engine with a custom AI budget.
pub fn create_engine_with_think_time(think_time: Duration) -> ChessEngine {
  ReactantChessApp::with_think_time(think_time)
}

/// Creates the sample engine with deterministic presentation randomness.
pub fn create_seeded_engine(seed: u64) -> ChessEngine {
  ReactantChessApp::with_seed(seed)
}

/// Creates an already-started sample engine from a FEN position.
pub fn create_engine_with_position(
  fen: &str,
  think_time: Duration,
) -> Result<ChessEngine, EngineError> {
  ReactantChessApp::with_starting_position(fen, think_time).map_err(EngineError::new)
}

fn legal_destinations(board: &Board, from: Square) -> Vec<Square> {
  let mut destinations = Vec::new();
  board.generate_moves_for(from.bitboard(), |moves| {
    destinations.extend(
      moves
        .into_iter()
        .map(|movement| visible_destination(board, movement)),
    );
    false
  });
  destinations.sort_unstable();
  destinations.dedup();
  destinations
}

fn player_moves(board: &Board, from: Square, target: Square) -> Vec<Move> {
  if board.side_to_move() != Color::White || board.color_on(from) != Some(Color::White) {
    return Vec::new();
  }
  let mut candidates = Vec::new();
  board.generate_moves_for(from.bitboard(), |moves| {
    candidates.extend(
      moves
        .into_iter()
        .filter(|movement| visible_destination(board, *movement) == target),
    );
    false
  });
  candidates
}

fn visible_destination(board: &Board, movement: Move) -> Square {
  let color = board.color_on(movement.from);
  if board.piece_on(movement.from) != Some(Piece::King) || board.color_on(movement.to) != color {
    return movement.to;
  }
  Square::new(
    if movement.to.file() > movement.from.file() {
      File::G
    } else {
      File::C
    },
    movement.from.rank(),
  )
}

fn square_at(position: Vector3) -> Option<Square> {
  if !(-4.0..4.0).contains(&position.x) || !(-4.0..4.0).contains(&position.z) {
    return None;
  }
  let file = (position.x + 3.5).round() as usize;
  let rank = (position.z + 3.5).round() as usize;
  Some(Square::new(File::index(file), Rank::index(rank)))
}

fn square_position(square: Square) -> Vector3 {
  GridLayout::centered(
    Vector3::ZERO,
    8,
    8,
    Vector3::new(1.0, 0.0, 0.0),
    Vector3::new(0.0, 0.0, 1.0),
  )
  .position(square.file() as u32, square.rank() as u32)
}

fn piece_entity_id(index: usize, generation: u32) -> ObjectId {
  format!("43000000-0000-4000-83{generation:02x}-{index:012x}")
    .parse()
    .expect("generated Chess piece ID is valid")
}

fn address(color: Color, piece: Piece) -> PrefabAddress {
  match (color, piece) {
    (Color::White, Piece::Pawn) => white::PAWN,
    (Color::White, Piece::Rook) => white::ROOK,
    (Color::White, Piece::Knight) => white::KNIGHT,
    (Color::White, Piece::Bishop) => white::BISHOP,
    (Color::White, Piece::Queen) => white::QUEEN,
    (Color::White, Piece::King) => white::KING,
    (Color::Black, Piece::Pawn) => black::PAWN,
    (Color::Black, Piece::Rook) => black::ROOK,
    (Color::Black, Piece::Knight) => black::KNIGHT,
    (Color::Black, Piece::Bishop) => black::BISHOP,
    (Color::Black, Piece::Queen) => black::QUEEN,
    (Color::Black, Piece::King) => black::KING,
  }
}

battlement_native::export_deterministic_engine!(
  create_engine,
  clock = virtualized,
  randomness = seeded,
  external_state = isolated,
  persistent_state = reset,
  input = semantic,
  visible_output = flatbuffers,
);

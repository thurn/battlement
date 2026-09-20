//! Native Reactant rules engine for the standalone chess sample.

mod ai;
pub mod assets;
pub mod audio;
pub mod chess_board;
pub mod chess_prompt;
pub mod chess_ui_state;
pub mod cursor;
pub mod motion;
pub mod persistence;
pub mod position;
pub mod promotion_dialog;
pub mod reactant_effects;
pub mod reactant_game;
pub mod reactant_input;
pub mod reactant_view;
pub mod visual_state;

pub use position::ChessPiece;
pub use reactant_game::{ChessGame, ChessState};
use std::time::Duration;

use battlement::{
  AudioClipAddress, GridLayout, ObjectId, PrefabAddress, Quaternion, Vector3, object_id,
};
use cozy_chess::{Board, Color, File, Move, Piece, Rank, Square};

use crate::assets::{black, music, white};
pub use crate::audio::{
  CAPTURE_SOUNDS, CASTLE_SOUND, CHECK_SOUND, DRAW_SOUND, DROP_SOUNDS, INVALID_DROP_SOUND,
  PLAYER_LOSS_SOUND, PLAYER_WIN_SOUND, PROMOTION_SOUND, RESET_SOUND, VOLUME_DOWN_SOUND,
  VOLUME_UP_SOUND,
};

/// Default time budget for the embedded computer opponent.
pub const AI_THINK_TIME: Duration = Duration::from_secs(2);
/// Small lift that keeps legal-target highlights above the board surface.
pub const HIGHLIGHT_HEIGHT: f64 = 0.02;
/// World-space scale applied to legal-target highlight planes.
pub const HIGHLIGHT_SCALE: f64 = 0.09;
/// Distance from the camera used to place the world-space refresh button.
pub const CAMERA_BUTTON_DEPTH: f64 = 1.5;
/// Camera field of view used by the viewport-relative button calculation.
pub const CAMERA_VERTICAL_FOV_RADIANS: f64 = std::f64::consts::PI / 3.0;
/// World-space width and height of the refresh button.
pub const REFRESH_BUTTON_SIZE: f64 = 0.16;
/// View-frustum inset around the refresh button.
pub const REFRESH_BUTTON_MARGIN: f64 = 0.12;
/// Shared rotation that makes world-space sprites face the sample camera.
pub const CAMERA_ROTATION: Quaternion =
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

/// Creates the component-first application used by the native sample.
///
/// Environment-driven fixture selection lives only at this exported boundary.
/// The component tree therefore receives ordinary typed configuration and does
/// not need test-specific conditionals scattered through its renders.
pub fn application() -> reactant::Application {
  if let Ok(name) = std::env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE") {
    return review_application(&name).unwrap_or_else(|error| panic!("{error}"));
  }
  let think_time = if std::env::var("BATTLEMENT_DITTO_ACTIVE").as_deref() == Ok("1") {
    Duration::ZERO
  } else {
    AI_THINK_TIME
  };
  application_with_think_time(think_time)
}

/// Creates the sample application with a custom AI budget.
///
/// Supplying a duration is useful for tests and embedding: the same rules and UI
/// can run instantly or spend more time searching without changing components.
pub fn application_with_think_time(think_time: Duration) -> reactant::Application {
  configured_application(
    Board::default(),
    None,
    visual_state::VisualState::Title,
    false,
    think_time,
    Some(43),
    true,
  )
}

/// Creates the application with deterministic presentation randomness.
///
/// Random sound variants belong to presentation context rather than logical
/// state. Seeding that context makes captures reproducible for recorded scenarios.
pub fn seeded_application(seed: u64) -> reactant::Application {
  configured_application(
    Board::default(),
    None,
    visual_state::VisualState::Title,
    false,
    AI_THINK_TIME,
    Some(seed),
    true,
  )
}

/// Creates a title-screen application whose game starts from a FEN position.
///
/// This constructor preserves the normal user journey while making a particular
/// rules position available after Play is activated.
pub fn application_with_position(
  fen: &str,
  think_time: Duration,
) -> Result<reactant::Application, String> {
  let board = fen
    .parse::<Board>()
    .map_err(|error| format!("invalid chess position: {error}"))?;
  Ok(configured_application(
    board,
    None,
    visual_state::VisualState::Title,
    false,
    think_time,
    Some(43),
    false,
  ))
}

/// Creates an already-started application from a FEN position.
///
/// Starting inside the game is useful for host integrations that already own
/// session selection, and for examples that focus on board interaction.
pub fn application_at_position(
  fen: &str,
  think_time: Duration,
) -> Result<reactant::Application, String> {
  let board = fen
    .parse::<Board>()
    .map_err(|error| format!("invalid chess position: {error}"))?;
  Ok(configured_application(
    board.clone(),
    Some(reactant_game::ChessState::new(board)),
    visual_state::VisualState::Resumed,
    true,
    think_time,
    Some(43),
    false,
  ))
}

/// Funnels exported constructors into one typed component configuration.
///
/// A single assembly path is a useful Reactant pattern: variants differ in data,
/// while document ownership, hooks, and component structure remain identical.
fn configured_application(
  starting_board: Board,
  initial_state: Option<reactant_game::ChessState>,
  visual_state: visual_state::VisualState,
  origin_saved: bool,
  think_time: Duration,
  seed: Option<u64>,
  load_persistence: bool,
) -> reactant::Application {
  reactant_view::application(reactant_view::ChessConfig {
    starting_board,
    initial_state,
    visual_state,
    origin_saved,
    think_time,
    seed,
    load_persistence,
  })
}

/// Builds a deterministic semantic fixture through the production app assembly.
fn review_application(name: &str) -> Result<reactant::Application, String> {
  let (board, state) = if name == "paused" {
    (Board::default(), visual_state::VisualState::Paused)
  } else {
    let fixture = visual_state::semantic_fixture(name)
      .ok_or_else(|| format!("unknown Reactant Chess app fixture {name:?}"))?;
    (fixture.board, fixture.state)
  };
  let initial = (state != visual_state::VisualState::Title)
    .then(|| reactant_game::ChessState::new(board.clone()));
  Ok(configured_application(
    board,
    initial,
    state,
    state == visual_state::VisualState::Resumed,
    Duration::ZERO,
    Some(43),
    false,
  ))
}

/// Returns unique visible targets for highlights and accessible move buttons.
///
/// Promotions collapse to one square and cozy-chess's castling encoding is
/// translated to the king's visible destination before deduplication.
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

/// Resolves a player-visible target back into every matching legal rules move.
///
/// Several candidates are retained for promotion so the rules action can ask a
/// typed prompt instead of making the view invent a special command.
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

/// Maps engine encodings to the square a player sees the moving piece occupy.
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

/// Converts a world-space drag endpoint into a square inside the board bounds.
fn square_at(position: Vector3) -> Option<Square> {
  if !(-4.0..4.0).contains(&position.x) || !(-4.0..4.0).contains(&position.z) {
    return None;
  }
  let file = (position.x + 3.5).round() as usize;
  let rank = (position.z + 3.5).round() as usize;
  Some(Square::new(File::index(file), Rank::index(rank)))
}

/// Converts a logical square to its centered world-space board position.
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

/// Creates a stable piece identity that also encodes its reference-table slot.
///
/// The generation changes when a session is replaced, forcing a clean remount;
/// the index stays stable across moves so Motion can keep targeting the object.
fn piece_entity_id(index: usize, generation: u32) -> ObjectId {
  format!("43000000-0000-4000-83{generation:02x}-{index:012x}")
    .parse()
    .expect("generated Chess piece ID is valid")
}

/// Selects the Unity prefab for one logical piece without leaking assets into rules code.
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

reactant::export_application!(application);

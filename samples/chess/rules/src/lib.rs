//! Native rules engine for the standalone chess sample.

mod ai;
pub mod assets;
pub mod audio;
mod cursor;
mod diagnostics;
mod input;
mod native;
mod persistence;
pub mod visual_state;

use std::{
  array,
  path::PathBuf,
  sync::mpsc::{self, Receiver, TryRecvError},
  time::{Duration, Instant},
};

use battlement::{
  ActionId, AudioClipAddress, GridLayout, ObjectId, PrefabAddress, Quaternion, SceneId, SessionId,
  Vector3, object_id, scene_id,
};
use battlement_native::{
  ConnectView, CoreClientMessageView, Engine, EngineError, EngineResponse, FlatBufferSubmitError,
  UiEventActionView, UiEventResult, threading::AdaptiveThreadPool,
};
use cozy_chess::{Board, Color, File, GameStatus, Move, Piece, Rank, Square};
use fastrand::Rng;
use tracing::info;

use crate::assets::{black, music, white};
use crate::audio::{
  CAPTURE_SOUNDS, CASTLE_SOUND, CHECK_SOUND, DRAW_SOUND, DROP_SOUNDS, INVALID_DROP_SOUND,
  MusicPlaylist, PLAYER_LOSS_SOUND, PLAYER_WIN_SOUND, PROMOTION_SOUND, RESET_SOUND,
  VOLUME_DOWN_SOUND, VOLUME_UP_SOUND,
};
use crate::input::RestartShortcut;
use crate::visual_state::VisualState;

const SCENE_ID: SceneId = scene_id!("36630324-bd92-4497-b328-3599930dffa9");
const AI_THINK_TIME: Duration = Duration::from_secs(2);
const MUSIC_VOLUME_STEP: f64 = 0.1;
const HIGHLIGHT_HEIGHT: f64 = 0.02;
const HIGHLIGHT_SCALE: f64 = 0.09;
const CAMERA_BUTTON_DEPTH: f64 = 1.5;
const CAMERA_VERTICAL_FOV_RADIANS: f64 = std::f64::consts::PI / 3.0;
const REFRESH_BUTTON_SIZE: f64 = 0.16;
const REFRESH_BUTTON_MARGIN: f64 = 0.12;
const CAPTURE_EFFECT_LIFETIME_MS: u64 = 2_000;
const ZERO_BUDGET_AI_OBSERVATION_POLLS: u8 = 128;
const CAMERA_ROTATION: Quaternion =
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
/// Stable identity of the new-game refresh button.
pub const REFRESH_BUTTON_ID: ObjectId = object_id!("35b288b3-6d72-48af-aeb9-e8f11d63e3ea");
/// Native chess rules engine with a parallel computer opponent.
pub struct ChessEngine {
  thread_pool: AdaptiveThreadPool,
  session_id: SessionId,
  starting_board: Board,
  board: Board,
  objects: [Option<ObjectId>; 64],
  piece_generation: u32,
  highlight_ids: [ObjectId; 64],
  cursor: Square,
  cursor_visible: bool,
  selected: Option<Square>,
  pause_open: bool,
  confirm_new_game: bool,
  started: bool,
  ai_move: Option<PendingAi>,
  ai_poll_deferrals: u8,
  think_time: Duration,
  music: MusicPlaylist,
  restart_shortcut: RestartShortcut,
  persistent_data_path: Option<PathBuf>,
  screen_aspect: f64,
  diagnostics_enabled: bool,
  rng: Rng,
  now: Box<dyn Fn() -> Instant>,
  visual_state: VisualState,
  semantic_fixture: Option<VisualState>,
  deterministic_runtime: bool,
}

/// Creates the engine used by the native sample.
pub fn create_engine() -> Result<ChessEngine, EngineError> {
  let deterministic_runtime = std::env::var("BATTLEMENT_DITTO_ACTIVE").as_deref() == Ok("1");
  let engine = match std::env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").ok() {
    Some(name) => self::engine_for_fixture(
      visual_state::semantic_fixture(&name)
        .unwrap_or_else(|| panic!("unknown Chess semantic fixture {name:?}")),
    )?,
    None if deterministic_runtime => {
      let epoch = Instant::now();
      self::engine_for_board(
        Board::default(),
        Duration::ZERO,
        Rng::with_seed(43),
        move || epoch,
      )?
    }
    None => self::engine_for_board(Board::default(), AI_THINK_TIME, Rng::new(), Instant::now)?,
  };
  info!(
    ai_think_time_ms = engine.think_time.as_millis() as u64,
    "Chess rules engine created"
  );
  Ok(engine)
}

/// Creates a chess engine driven by a caller-supplied clock.
pub fn create_engine_with_clock(now: impl Fn() -> Instant + 'static) -> ChessEngine {
  self::engine_for_board(Board::default(), AI_THINK_TIME, Rng::new(), now)
    .expect("thread pool should initialize")
}

/// Creates an engine with a custom AI budget for simulations and tests.
pub fn create_engine_with_think_time(think_time: Duration) -> ChessEngine {
  self::engine_for_board(Board::default(), think_time, Rng::new(), Instant::now)
    .expect("thread pool should initialize")
}

/// Creates a deterministic engine for spawn-sequence simulations.
pub fn create_seeded_engine(seed: u64) -> ChessEngine {
  self::engine_for_board(
    Board::default(),
    AI_THINK_TIME,
    Rng::with_seed(seed),
    Instant::now,
  )
  .expect("thread pool should initialize")
}

/// Creates an engine from a FEN position for fake-client simulations.
pub fn create_engine_with_position(
  fen: &str,
  think_time: Duration,
) -> Result<ChessEngine, EngineError> {
  self::engine_for_board(
    fen
      .parse()
      .map_err(|error| EngineError::new(format!("invalid chess position: {error}")))?,
    think_time,
    Rng::new(),
    Instant::now,
  )
}

fn engine_for_board(
  board: Board,
  think_time: Duration,
  rng: Rng,
  now: impl Fn() -> Instant + 'static,
) -> Result<ChessEngine, EngineError> {
  self::engine_for_board_with_fixture(board, think_time, rng, now, None)
}

fn engine_for_fixture(fixture: visual_state::SemanticFixture) -> Result<ChessEngine, EngineError> {
  self::engine_for_board_with_fixture(
    fixture.board,
    Duration::ZERO,
    Rng::with_seed(43),
    Instant::now,
    Some(fixture.state),
  )
}

fn engine_for_board_with_fixture(
  board: Board,
  think_time: Duration,
  rng: Rng,
  now: impl Fn() -> Instant + 'static,
  semantic_fixture: Option<VisualState>,
) -> Result<ChessEngine, EngineError> {
  Ok(ChessEngine {
    thread_pool: AdaptiveThreadPool::new()?,
    session_id: SessionId::new_v4(),
    starting_board: board.clone(),
    objects: [None; 64],
    piece_generation: 0,
    highlight_ids: array::from_fn(self::highlight_id),
    cursor: cursor::START,
    cursor_visible: false,
    selected: None,
    pause_open: false,
    confirm_new_game: false,
    board,
    started: false,
    ai_move: None,
    ai_poll_deferrals: 0,
    think_time,
    music: MusicPlaylist::new(),
    restart_shortcut: RestartShortcut::new(),
    persistent_data_path: None,
    screen_aspect: 16.0 / 9.0,
    diagnostics_enabled: false,
    rng,
    now: Box::new(now),
    visual_state: VisualState::Title,
    semantic_fixture,
    deterministic_runtime: std::env::var("BATTLEMENT_DITTO_ACTIVE").as_deref() == Ok("1"),
  })
}

impl Engine for ChessEngine {
  const WIRE_CONTRACT_DIGEST_C: &'static [u8; 65] = battlement_native::WIRE_CONTRACT_DIGEST_C;

  fn connect(&mut self, message: ConnectView<'_>) -> Result<EngineResponse, EngineError> {
    self.connect_state(message);
    native::snapshot(self)
  }

  fn submit(&mut self, bytes: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError> {
    let message = CoreClientMessageView::read(bytes).map_err(|error| {
      FlatBufferSubmitError::invalid_argument(format!("invalid client message: {error}"))
    })?;
    let CoreClientMessageView::Action(action) = message else {
      return EngineResponse::empty(*self.session_id.as_uuid().as_bytes())
        .map_err(FlatBufferSubmitError::engine);
    };
    if action.session_id() != *self.session_id.as_uuid().as_bytes() {
      return Err(FlatBufferSubmitError::engine(EngineError::new(
        "Chess action session mismatch",
      )));
    }
    let action_id = ActionId::from_bytes(action.action_id()).expect("validated action UUID");
    self
      .submit_action_view(action_id, action.body())
      .map_err(FlatBufferSubmitError::engine)
  }

  fn submit_ui_event(
    &mut self,
    action: UiEventActionView<'_>,
  ) -> Result<UiEventResult, EngineError> {
    if action.session_id() != *self.session_id.as_uuid().as_bytes() {
      return Err(EngineError::new("UI event session mismatch"));
    }
    Ok(UiEventResult {
      disposition: if action.default_prevented() {
        battlement::UiEventDisposition::PreventDefault
      } else {
        battlement::UiEventDisposition::Continue
      },
      response: EngineResponse::empty(*self.session_id.as_uuid().as_bytes())?,
    })
  }

  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
    if !self.started {
      return Ok(None);
    }
    if let Some(mv) = self.poll_ai_move() {
      let mut board_after = self.board.clone();
      board_after.play_unchecked(mv);
      let metadata = if self.diagnostics_enabled {
        match board_after.status() {
          GameStatus::Won => vec![("chess.game_status", "won")],
          GameStatus::Drawn => vec![("chess.game_status", "drawn")],
          GameStatus::Ongoing => Vec::new(),
        }
      } else {
        Vec::new()
      };
      let session_id = self.session_id;
      let response = native::batch_response_with_metadata(
        session_id,
        None,
        battlement_native::NativeBatchStart::AfterEarlierBlockingWork,
        &metadata,
        |message| {
          let mut groups = native::apply_move(self, message, mv, true)?;
          let last = groups.last_mut().expect("an AI move has a final group");
          last.push(
            message
              .set_local_scale(
                *battlement::CommandId::new_v4().as_uuid().as_bytes(),
                true,
                *cursor::EFFECT_ID.as_uuid().as_bytes(),
                [1.0, 1.0, 1.0],
              )
              .map_err(native::protocol)?,
          );
          last.push(
            message
              .set_input_enabled(
                *battlement::CommandId::new_v4().as_uuid().as_bytes(),
                true,
                true,
              )
              .map_err(native::protocol)?,
          );
          Ok(groups)
        },
      )?;
      return Ok(Some(response));
    }
    self.music.poll(self.session_id, (self.now)())
  }
}

impl ChessEngine {
  fn connect_state(&mut self, message: ConnectView<'_>) {
    self.session_id = SessionId::new_v4();
    self.diagnostics_enabled = diagnostics::is_available(message);
    self.highlight_ids = array::from_fn(self::highlight_id);
    self.persistent_data_path = (!self.deterministic_runtime)
      .then(|| message.persistent_data_path().map(PathBuf::from))
      .flatten();
    self.screen_aspect = if message.screen_height() == 0 {
      16.0 / 9.0
    } else {
      f64::from(message.screen_width()) / f64::from(message.screen_height())
    };
    let saved_board = self
      .semantic_fixture
      .is_none()
      .then(|| {
        self
          .persistent_data_path
          .as_deref()
          .and_then(persistence::load)
      })
      .flatten();
    self.started = self.semantic_fixture != Some(VisualState::Title)
      && (self.semantic_fixture.is_some() || saved_board.is_some());
    self.board = saved_board.unwrap_or_else(|| self.starting_board.clone());
    self.visual_state = self.semantic_fixture.unwrap_or(if self.started {
      VisualState::Resumed
    } else {
      VisualState::Title
    });
    self.objects = if self.started {
      self::objects_for_board(&self.board, self.piece_generation)
    } else {
      [None; 64]
    };
    self.cursor = cursor::START;
    self.cursor_visible = false;
    self.selected = None;
    self.pause_open = false;
    self.confirm_new_game = false;
    self.ai_move = None;
    self.ai_poll_deferrals = 0;
    self.music = MusicPlaylist::new();
    self.restart_shortcut.reset();
    if self.started {
      self.music.reset((self.now)());
      if self.board.side_to_move() == Color::Black && self.board.status() == GameStatus::Ongoing {
        self.start_ai();
      }
    }
    info!(
        resumed_game = self.started,
        side_to_move = ?self.board.side_to_move(),
        status = ?self.board.status(),
        "Chess session connected"
    );
  }

  /// Returns the current user-visible state classification.
  pub const fn visual_state(&self) -> VisualState {
    self.visual_state
  }

  pub(crate) fn change_visual_state(&mut self, next: VisualState) -> VisualState {
    let previous = self.visual_state;
    self.visual_state = next;
    info!(from = ?previous, to = ?next, "Chess visual state changed");
    previous
  }

  fn poll_ai_move(&mut self) -> Option<Move> {
    if self.ai_poll_deferrals > 0 {
      self.ai_poll_deferrals -= 1;
      return None;
    }
    let Some(receiver) = &self.ai_move else {
      return None;
    };
    match receiver.try_recv() {
      Ok(mv) => {
        self.ai_move = None;
        info!(
            from = %mv.from,
            to = %mv.to,
            promotion = ?mv.promotion,
            "Computer move selected"
        );
        Some(mv)
      }
      Err(TryRecvError::Empty) => None,
      Err(TryRecvError::Disconnected) => {
        self.ai_move = None;
        tracing::warn!("Computer move search ended without a result");
        None
      }
    }
  }

  fn start_ai(&mut self) {
    info!(
        side_to_move = ?self.board.side_to_move(),
        think_time_ms = self.think_time.as_millis() as u64,
        "Computer move search started"
    );
    let (sender, receiver) = mpsc::channel();
    let board = self.board.clone();
    let think_time = self.think_time;
    let search = move || {
      if let Some(mv) = ai::choose_move(&board, think_time) {
        let _ = sender.send(mv);
      }
    };
    if think_time.is_zero() {
      search();
      self.ai_move = Some(receiver);
      self.ai_poll_deferrals = ZERO_BUDGET_AI_OBSERVATION_POLLS;
      info!(
        observation_polls = ZERO_BUDGET_AI_OBSERVATION_POLLS,
        "Deterministic computer move queued"
      );
      return;
    }
    // The reusable executor keeps browser policy out of the game: Web mobile
    // runs now without nested workers, while Web desktop and native schedule
    // the exact same Rayon search asynchronously on their parallel pools.
    self.thread_pool.execute(search);
    self.ai_move = Some(receiver);
  }
}

fn legal_destinations(board: &Board, from: Square) -> Vec<Square> {
  let mut destinations = Vec::new();
  board.generate_moves_for(from.bitboard(), |moves| {
    destinations.extend(
      moves
        .into_iter()
        .map(|movement| self::visible_destination(board, movement)),
    );
    false
  });
  destinations.sort_unstable();
  destinations.dedup();
  destinations
}

type PendingAi = Receiver<Move>;

fn player_move(board: &Board, from: Square, target: Square) -> Option<Move> {
  if board.side_to_move() != Color::White || board.color_on(from) != Some(Color::White) {
    return None;
  }
  let target = if board.piece_on(from) == Some(Piece::King) {
    match target {
      Square::G1 => Square::H1,
      Square::C1 => Square::A1,
      _ => target,
    }
  } else {
    target
  };
  let promotion = if board.piece_on(from) == Some(Piece::Pawn) && target.rank() == Rank::Eighth {
    Some(Piece::Queen)
  } else {
    None
  };
  let candidate = Move {
    from,
    to: target,
    promotion,
  };
  board.is_legal(candidate).then_some(candidate)
}

fn visible_destination(board: &Board, mv: Move) -> Square {
  let color = board.color_on(mv.from);
  if board.piece_on(mv.from) != Some(Piece::King) || board.color_on(mv.to) != color {
    return mv.to;
  }
  Square::new(
    if mv.to.file() > mv.from.file() {
      File::G
    } else {
      File::C
    },
    mv.from.rank(),
  )
}

fn find_square(objects: &[Option<ObjectId>; 64], object_id: ObjectId) -> Option<Square> {
  objects
    .iter()
    .position(|&candidate| candidate == Some(object_id))
    .map(Square::index)
}

fn find_highlight(highlight_ids: &[ObjectId; 64], object_id: ObjectId) -> Option<Square> {
  highlight_ids
    .iter()
    .position(|&candidate| candidate == object_id)
    .map(Square::index)
}

fn square_at(position: Vector3) -> Square {
  let file = (position.x + 3.5).round().clamp(0.0, 7.0) as usize;
  let rank = (position.z + 3.5).round().clamp(0.0, 7.0) as usize;
  Square::new(File::index(file), Rank::index(rank))
}

fn square_position(square: Square) -> Vector3 {
  self::board_grid().position(square.file() as u32, square.rank() as u32)
}

fn board_grid() -> GridLayout {
  GridLayout::centered(
    Vector3::ZERO,
    8,
    8,
    Vector3::new(1.0, 0.0, 0.0),
    Vector3::new(0.0, 0.0, 1.0),
  )
}

fn objects_for_board(board: &Board, generation: u32) -> [Option<ObjectId>; 64] {
  array::from_fn(|index| {
    board
      .piece_on(Square::index(index))
      .map(|_| self::piece_id(index, generation))
  })
}

fn piece_id(index: usize, generation: u32) -> ObjectId {
  format!("43000000-0000-4000-81{generation:02x}-{index:012x}")
    .parse()
    .expect("generated Chess piece ID is valid")
}

fn highlight_id(index: usize) -> ObjectId {
  format!("43000000-0000-4000-8200-{index:012x}")
    .parse()
    .expect("generated Chess highlight ID is valid")
}

fn promotion_id(square: Square) -> ObjectId {
  format!("43000000-0000-4000-8300-{:012x}", square as usize)
    .parse()
    .expect("generated Chess promotion ID is valid")
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

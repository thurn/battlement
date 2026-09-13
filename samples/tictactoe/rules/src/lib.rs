//! Native rules engine for the standalone Tic-Tac-Toe sample.

use std::time::{Duration, Instant};

use battlement::{ObjectId, SceneId, SessionId, Vector3, object_id, scene_id};
use battlement_native::{
  CoreActionBodyView, CoreClientMessageView, Engine, EngineError, EngineResponse,
  FlatBufferSubmitError, GameObjectOffset, MessageWriter, NativeBatchStart, NativeImageFit,
  NativeObjectPlacement, NativeParentScene, NativePointerEvent, NativePreparedAssetKind,
  NativeTransform, UiEventActionView, UiEventResult,
};
use fastrand::Rng;

const SCENE_ID: SceneId = scene_id!("db931052-2dcc-48c7-8392-246b629e7e68");
const BOARD_CENTER_Y: f64 = -0.7;
const BOARD_SIZE: f64 = 7.2;
const GRID_SIZE: f64 = BOARD_SIZE * 0.8;
const CELL_SIZE: f64 = GRID_SIZE / 3.0;
const MARK_SIZE: f64 = 2.25;
const AI_DELAY: Duration = Duration::from_millis(100);
const PLAYER_TURN: &str = "Your turn — click an empty square";
const THINKING: &str = "Computer thinking…";
const X_MARK_IDS: [ObjectId; 9] = [
  object_id!("603306b7-957f-4a3b-b9b0-b2df0a791975"),
  object_id!("4c2b982e-1d24-4542-b33c-d956b8ad26b9"),
  object_id!("96c06045-daff-49ba-866a-e823bfc857ad"),
  object_id!("2aa4f8c3-ed65-40b8-92b6-cf9a89dc5b40"),
  object_id!("7933afff-c2be-45ac-abbc-3062db3acd1a"),
  object_id!("aaa76dae-e7f5-4999-8079-e387b37d2b5a"),
  object_id!("9ebc719d-4b74-40e3-a1f6-1ce3d9ef3064"),
  object_id!("da780dd9-6869-4c68-b5c6-53e0046a775a"),
  object_id!("db63f03d-b732-4c00-93a6-30369a046eb5"),
];
const O_MARK_IDS: [ObjectId; 9] = [
  object_id!("a2d877c6-cdc1-43c6-bfc6-312a7d06a8f1"),
  object_id!("be6426e2-6ea2-49a1-b031-76ad34419c21"),
  object_id!("2a466fb1-dae6-4cb5-90b8-d1a0fa3a9140"),
  object_id!("181308b6-ad5b-4034-8add-b9548d2c0293"),
  object_id!("5e032b0d-cf9a-48d5-ac51-311df3b338fd"),
  object_id!("a1a90f5c-fcc7-448d-a799-bf151166dd4a"),
  object_id!("32c699d8-b294-44b2-aa64-91342a948832"),
  object_id!("d06c6d93-fa3a-4a9e-8c85-ba92496e4308"),
  object_id!("cda85ac6-6791-46e2-ac7c-03d551d18b62"),
];

/// Address of the sample's content scene.
pub const CONTENT_SCENE: &str = "tictactoe/content";
/// Machine-readable registry consumed by the Ditto coverage checker.
pub const DITTO_VISUAL_STATE_REGISTRY: &str = include_str!("../../ditto-visual-states.toml");
/// Canonical seed used by the sample and its Ditto scenarios.
pub const DITTO_SEED: u64 = 7;
/// Address of the game-board texture.
pub const BOARD_TEXTURE: &str = "tictactoe/board";
/// Address of the player-mark texture.
pub const X_TEXTURE: &str = "tictactoe/x";
/// Address of the computer-mark texture.
pub const O_TEXTURE: &str = "tictactoe/o";
/// Address of the sample's text font.
pub const FONT: &str = "tictactoe/font";
/// Stable identity of the sample's input camera.
pub const CAMERA_ID: ObjectId = object_id!("fa308d92-5ad4-4249-90dc-2d104057bc41");
/// Stable identity of the clickable game board.
pub const BOARD_ID: ObjectId = object_id!("c8c9e10d-585b-45f4-ac19-b76746ed2d25");
/// Stable identity of the visible game-status text.
pub const STATUS_ID: ObjectId = object_id!("9b10a4a0-1367-46a8-9a2c-7c29eef033b1");
/// Stable identity of the visible game title.
pub const TITLE_ID: ObjectId = object_id!("860e3fa1-d047-45ae-869d-3321e9cd3142");

/// Finite user-visible states recognized by the Tic-Tac-Toe engine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VisualState {
  EmptyBoard,
  HumanMove,
  AiResponse,
  PlayerWin,
  ComputerWin,
  Draw,
  RestoredBoard,
}

impl VisualState {
  /// Every visual state in registry order.
  pub const ALL: [Self; 7] = [
    Self::EmptyBoard,
    Self::HumanMove,
    Self::AiResponse,
    Self::PlayerWin,
    Self::ComputerWin,
    Self::Draw,
    Self::RestoredBoard,
  ];

  /// Returns the canonical Ditto registry key.
  pub const fn registry_key(self) -> &'static str {
    match self {
      Self::EmptyBoard => "board.empty",
      Self::HumanMove => "turn.human-move",
      Self::AiResponse => "turn.ai-response",
      Self::PlayerWin => "terminal.player-win",
      Self::ComputerWin => "terminal.computer-win",
      Self::Draw => "terminal.draw",
      Self::RestoredBoard => "board.restored",
    }
  }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mark {
  X,
  O,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Outcome {
  InProgress,
  XWins,
  OWins,
  Draw,
}

/// Native Tic-Tac-Toe rules engine.
pub struct TicTacToeEngine {
  session_id: SessionId,
  round: u32,
  board: [Option<Mark>; 9],
  marker_ids: [Option<ObjectId>; 9],
  outcome: Outcome,
  ai_due: Option<Instant>,
  rng: Rng,
  now: Box<dyn Fn() -> Instant>,
  visual_state: VisualState,
  semantic_fixture: Option<VisualState>,
}

/// Creates the engine used by the native sample.
pub fn create_engine() -> Result<TicTacToeEngine, EngineError> {
  let mut engine = if std::env::var("BATTLEMENT_DITTO_ACTIVE").as_deref() == Ok("1") {
    let epoch = Instant::now();
    create_seeded_engine(DITTO_SEED, move || epoch)
  } else {
    create_seeded_engine(DITTO_SEED, Instant::now)
  };
  engine.semantic_fixture = std::env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE")
    .ok()
    .map(|name| {
      self::semantic_fixture(&name)
        .unwrap_or_else(|| panic!("unknown Tic-Tac-Toe semantic fixture {name:?}"))
    });
  Ok(engine)
}

/// Creates a deterministic engine for simulations.
pub fn create_seeded_engine(seed: u64, now: impl Fn() -> Instant + 'static) -> TicTacToeEngine {
  TicTacToeEngine::with_rng_and_clock(Rng::with_seed(seed), Box::new(now))
}

impl Engine for TicTacToeEngine {
  const WIRE_CONTRACT_DIGEST_C: &'static [u8; 65] = battlement_native::WIRE_CONTRACT_DIGEST_C;

  fn connect(
    &mut self,
    _message: battlement_native::ConnectView<'_>,
  ) -> Result<EngineResponse, EngineError> {
    self.session_id = SessionId::new_v4();
    self.round = 1;
    self.reset_state(self.semantic_fixture.unwrap_or(VisualState::EmptyBoard));
    if self.semantic_fixture == Some(VisualState::HumanMove) {
      self.board[2] = Some(Mark::X);
      self.marker_ids[2] = Some(X_MARK_IDS[2]);
    }
    self::native_snapshot(self.session_id, self.round, &self.board, self.visual_state)
  }

  fn submit(&mut self, bytes: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError> {
    let message = CoreClientMessageView::read(bytes)
      .map_err(|error| FlatBufferSubmitError::invalid_argument(error.to_string()))?;
    let CoreClientMessageView::Action(action) = message else {
      return self::native_empty(self.session_id).map_err(FlatBufferSubmitError::engine);
    };
    if action.session_id() != self::session_bytes(self.session_id) {
      return Err(FlatBufferSubmitError::engine(EngineError::new(
        "Tic-Tac-Toe action session mismatch",
      )));
    }
    let CoreActionBodyView::PointerClick(payload) = action.body() else {
      return self::native_empty(self.session_id).map_err(FlatBufferSubmitError::engine);
    };
    self
      .submit_at(
        action.action_id(),
        payload.object_id(),
        payload.pointer_button(),
        payload.world_hit(),
        (self.now)(),
      )
      .map_err(FlatBufferSubmitError::engine)
  }

  fn submit_ui_event(
    &mut self,
    action: UiEventActionView<'_>,
  ) -> Result<UiEventResult, EngineError> {
    if action.session_id() != self::session_bytes(self.session_id) {
      return Err(EngineError::new("UI event session mismatch"));
    }
    Ok(UiEventResult {
      disposition: if action.default_prevented() {
        battlement::UiEventDisposition::PreventDefault
      } else {
        battlement::UiEventDisposition::Continue
      },
      response: self::native_empty(self.session_id)?,
    })
  }

  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
    let now = (self.now)();
    let Some(due) = self.ai_due else {
      return Ok(None);
    };
    if now < due {
      return Ok(None);
    }
    self.ai_due = None;
    let empty = self::empty_cells(&self.board);
    let index = empty[self.rng.usize(..empty.len())];
    let object_id = self::marker_id(index, Mark::O);
    self.board[index] = Some(Mark::O);
    self.marker_ids[index] = Some(object_id);
    self.outcome = self::outcome(&self.board);
    self.visual_state = self.visual_state_after_ai_move(self.outcome);
    self::native_turn_response(
      self.session_id,
      None,
      Some((object_id, index, Mark::O)),
      self.visual_state,
      Some(true),
    )
    .map(Some)
  }
}

impl TicTacToeEngine {
  /// Returns the current user-visible state classification.
  pub const fn visual_state(&self) -> VisualState {
    self.visual_state
  }

  fn submit_at(
    &mut self,
    action_id: [u8; 16],
    object_id: [u8; 16],
    button: battlement::PointerButton,
    world_hit: [f64; 3],
    now: Instant,
  ) -> Result<EngineResponse, EngineError> {
    if object_id != self::object_bytes(BOARD_ID) || button != battlement::PointerButton::Left {
      return self::native_empty(self.session_id);
    }
    if self.outcome != Outcome::InProgress {
      let marker_ids = self
        .marker_ids
        .iter()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
      self.round += 1;
      self.reset_state(VisualState::RestoredBoard);
      return self::native_reset_response(self.session_id, action_id, &marker_ids, self.round);
    }
    if self.ai_due.is_some() {
      return self::native_empty(self.session_id);
    }
    let Some(index) = self::cell_index(Vector3::new(world_hit[0], world_hit[1], world_hit[2]))
    else {
      return self::native_empty(self.session_id);
    };
    if self.board[index].is_some() {
      return self::native_empty(self.session_id);
    }
    let object_id = self::marker_id(index, Mark::X);
    self.board[index] = Some(Mark::X);
    self.marker_ids[index] = Some(object_id);
    self.outcome = self::outcome(&self.board);
    self.visual_state = self.visual_state_after_player_move(self.outcome);
    let in_progress = self.outcome == Outcome::InProgress;
    if in_progress {
      self.ai_due = Some(now + AI_DELAY);
    }
    self::native_turn_response(
      self.session_id,
      Some(action_id),
      Some((object_id, index, Mark::X)),
      self.visual_state,
      in_progress.then_some(false),
    )
  }

  fn with_rng_and_clock(rng: Rng, now: Box<dyn Fn() -> Instant>) -> Self {
    Self {
      session_id: SessionId::new_v4(),
      round: 1,
      board: [None; 9],
      marker_ids: [None; 9],
      outcome: Outcome::InProgress,
      ai_due: None,
      rng,
      now,
      visual_state: VisualState::EmptyBoard,
      semantic_fixture: None,
    }
  }

  fn reset_state(&mut self, visual_state: VisualState) {
    self.board = [None; 9];
    self.marker_ids = [None; 9];
    self.outcome = Outcome::InProgress;
    self.ai_due = None;
    self.visual_state = visual_state;
  }

  fn visual_state_after_player_move(&self, outcome: Outcome) -> VisualState {
    match outcome {
      Outcome::InProgress => VisualState::HumanMove,
      Outcome::XWins => VisualState::PlayerWin,
      Outcome::OWins => VisualState::ComputerWin,
      Outcome::Draw => VisualState::Draw,
    }
  }

  fn visual_state_after_ai_move(&self, outcome: Outcome) -> VisualState {
    match outcome {
      Outcome::InProgress => VisualState::AiResponse,
      Outcome::XWins => VisualState::PlayerWin,
      Outcome::OWins => VisualState::ComputerWin,
      Outcome::Draw => VisualState::Draw,
    }
  }
}

fn native_turn_response(
  session_id: SessionId,
  action_id: Option<[u8; 16]>,
  marker: Option<(ObjectId, usize, Mark)>,
  visual_state: VisualState,
  input_enabled: Option<bool>,
) -> Result<EngineResponse, EngineError> {
  let session = self::session_bytes(session_id);
  let mut writer = MessageWriter::default();
  let mut commands = Vec::with_capacity(3);
  if let Some((object_id, index, mark)) = marker {
    let marker = self::native_marker(&mut writer, object_id, index, mark)?;
    commands.push(
      writer
        .create_object(self::command_id(), true, marker)
        .map_err(self::writer_error)?,
    );
  }
  commands.push(
    writer
      .text_set_content(
        self::command_id(),
        true,
        self::object_bytes(STATUS_ID),
        self::status_text(visual_state),
      )
      .map_err(self::writer_error)?,
  );
  if let Some(enabled) = input_enabled {
    commands.push(
      writer
        .set_input_enabled(self::command_id(), true, enabled)
        .map_err(self::writer_error)?,
    );
  }
  self::finish_native_commands(writer, session, action_id, &commands)
}

fn native_reset_response(
  session_id: SessionId,
  action_id: [u8; 16],
  marker_ids: &[ObjectId],
  round: u32,
) -> Result<EngineResponse, EngineError> {
  let session = self::session_bytes(session_id);
  let mut writer = MessageWriter::default();
  let mut commands = Vec::with_capacity(marker_ids.len() + 2);
  for object_id in marker_ids {
    commands.push(
      writer
        .destroy_object(self::command_id(), true, self::object_bytes(*object_id))
        .map_err(self::writer_error)?,
    );
  }
  commands.push(
    writer
      .text_set_content(
        self::command_id(),
        true,
        self::object_bytes(TITLE_ID),
        &format!("TIC TAC TOE — ROUND {round}"),
      )
      .map_err(self::writer_error)?,
  );
  commands.push(
    writer
      .text_set_content(
        self::command_id(),
        true,
        self::object_bytes(STATUS_ID),
        self::status_text(VisualState::RestoredBoard),
      )
      .map_err(self::writer_error)?,
  );
  self::finish_native_commands(writer, session, Some(action_id), &commands)
}

fn finish_native_commands(
  mut writer: MessageWriter,
  session: [u8; 16],
  action_id: Option<[u8; 16]>,
  commands: &[battlement_native::CoreCommandOffset],
) -> Result<EngineResponse, EngineError> {
  let group = writer
    .parallel_group(commands)
    .map_err(self::writer_error)?;
  let batch = writer
    .batch(
      *battlement::BatchId::new_v4().as_uuid().as_bytes(),
      session,
      action_id,
      NativeBatchStart::Now,
      &[group],
    )
    .map_err(self::writer_error)?;
  let message = writer
    .finish(session, &[batch])
    .map_err(self::writer_error)?;
  EngineResponse::from_core(session, message)
}

fn native_empty(session_id: SessionId) -> Result<EngineResponse, EngineError> {
  let session = self::session_bytes(session_id);
  let message = MessageWriter::default()
    .finish(session, &[])
    .map_err(self::writer_error)?;
  EngineResponse::from_core(session, message)
}

fn native_snapshot(
  session_id: SessionId,
  round: u32,
  marks: &[Option<Mark>; 9],
  visual_state: VisualState,
) -> Result<EngineResponse, EngineError> {
  let session = self::session_bytes(session_id);
  let mut writer = MessageWriter::with_capacity(16 * 1024);
  let assets = [
    writer.prepared_asset(NativePreparedAssetKind::Scene, CONTENT_SCENE),
    writer.prepared_asset(NativePreparedAssetKind::Texture, BOARD_TEXTURE),
    writer.prepared_asset(NativePreparedAssetKind::Texture, X_TEXTURE),
    writer.prepared_asset(NativePreparedAssetKind::Texture, O_TEXTURE),
    writer.prepared_asset(NativePreparedAssetKind::TextMeshProFont, FONT),
  ];
  let scene = writer
    .scene(self::scene_bytes(SCENE_ID), CONTENT_SCENE)
    .map_err(self::writer_error)?;
  let camera = writer
    .orthographic_camera_object(
      self::object_bytes(CAMERA_ID),
      NativeObjectPlacement {
        parent_scene: NativeParentScene::Persistent,
        transform: NativeTransform {
          position: [0.0, 0.0, -10.0],
          ..Default::default()
        },
        ..Default::default()
      },
      5.6,
      [0.96, 0.93, 0.84, 1.0],
    )
    .map_err(self::writer_error)?;
  let board = writer
    .image_object(
      self::object_bytes(BOARD_ID),
      NativeObjectPlacement {
        transform: NativeTransform {
          position: [0.0, BOARD_CENTER_Y, 0.0],
          ..Default::default()
        },
        pointer_events: &[NativePointerEvent::Click],
        ..Default::default()
      },
      BOARD_TEXTURE,
      BOARD_SIZE,
      BOARD_SIZE,
      NativeImageFit::Stretch,
    )
    .map_err(self::writer_error)?;
  let title = writer
    .text_object_with_color(
      self::object_bytes(TITLE_ID),
      NativeObjectPlacement {
        parent_scene: NativeParentScene::Persistent,
        transform: NativeTransform {
          position: [0.0, 4.7, -0.1],
          ..Default::default()
        },
        ..Default::default()
      },
      &format!("TIC TAC TOE — ROUND {round}"),
      FONT,
      4.0,
      None,
      [0.03, 0.04, 0.08, 1.0],
    )
    .map_err(self::writer_error)?;
  let status = writer
    .text_object_with_color(
      self::object_bytes(STATUS_ID),
      NativeObjectPlacement {
        parent_scene: NativeParentScene::Persistent,
        transform: NativeTransform {
          position: [0.0, 3.75, -0.1],
          ..Default::default()
        },
        ..Default::default()
      },
      self::status_text(visual_state),
      FONT,
      3.2,
      Some(14.0),
      [0.06, 0.08, 0.15, 1.0],
    )
    .map_err(self::writer_error)?;
  let mut objects = vec![camera, board, title, status];
  for (index, mark) in marks.iter().enumerate() {
    if let Some(mark) = mark {
      objects.push(self::native_marker(
        &mut writer,
        self::marker_id(index, *mark),
        index,
        *mark,
      )?);
    }
  }
  let snapshot = writer
    .snapshot(
      session,
      &assets,
      &[scene],
      Some(self::scene_bytes(SCENE_ID)),
      &objects,
      Some(self::object_bytes(CAMERA_ID)),
    )
    .map_err(self::writer_error)?;
  let message = writer
    .finish(session, &[snapshot])
    .map_err(self::writer_error)?;
  EngineResponse::from_core(session, message)
}

fn native_marker(
  writer: &mut MessageWriter,
  object_id: ObjectId,
  index: usize,
  mark: Mark,
) -> Result<GameObjectOffset, EngineError> {
  let position = self::cell_position(index);
  writer
    .image_object(
      self::object_bytes(object_id),
      NativeObjectPlacement {
        transform: NativeTransform {
          position: [position.x, position.y, position.z],
          ..Default::default()
        },
        ..Default::default()
      },
      if mark == Mark::X {
        X_TEXTURE
      } else {
        O_TEXTURE
      },
      MARK_SIZE,
      MARK_SIZE,
      NativeImageFit::Contain,
    )
    .map_err(self::writer_error)
}

fn command_id() -> [u8; 16] {
  *battlement::CommandId::new_v4().as_uuid().as_bytes()
}

fn writer_error(error: impl std::fmt::Display) -> EngineError {
  EngineError::new(error.to_string())
}

fn session_bytes(id: SessionId) -> [u8; 16] {
  *id.as_uuid().as_bytes()
}

fn object_bytes(id: ObjectId) -> [u8; 16] {
  *id.as_uuid().as_bytes()
}

fn scene_bytes(id: SceneId) -> [u8; 16] {
  *id.as_uuid().as_bytes()
}

fn semantic_fixture(name: &str) -> Option<VisualState> {
  match name {
    "human move" => Some(VisualState::HumanMove),
    _ => None,
  }
}

fn marker_id(index: usize, mark: Mark) -> ObjectId {
  match mark {
    Mark::X => X_MARK_IDS[index],
    Mark::O => O_MARK_IDS[index],
  }
}

fn cell_index(world_hit: Vector3) -> Option<usize> {
  let half = GRID_SIZE / 2.0;
  let local_y = world_hit.y - BOARD_CENTER_Y;
  if world_hit.x < -half || world_hit.x >= half {
    return None;
  }
  if local_y < -half || local_y >= half {
    return None;
  }
  let column = ((world_hit.x + half) / CELL_SIZE).floor() as usize;
  let row = ((half - local_y) / CELL_SIZE).floor() as usize;
  Some(row * 3 + column)
}

fn cell_position(index: usize) -> Vector3 {
  let row = index / 3;
  let column = index % 3;
  Vector3::new(
    (column as f64 - 1.0) * CELL_SIZE,
    BOARD_CENTER_Y + (1.0 - row as f64) * CELL_SIZE,
    -0.05,
  )
}

fn outcome(board: &[Option<Mark>; 9]) -> Outcome {
  const LINES: [[usize; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8],
    [0, 4, 8],
    [2, 4, 6],
  ];
  for [first, second, third] in LINES {
    if board[first].is_some() && board[first] == board[second] && board[second] == board[third] {
      return if board[first] == Some(Mark::X) {
        Outcome::XWins
      } else {
        Outcome::OWins
      };
    }
  }
  if board.iter().all(Option::is_some) {
    Outcome::Draw
  } else {
    Outcome::InProgress
  }
}

fn empty_cells(board: &[Option<Mark>; 9]) -> Vec<usize> {
  board
    .iter()
    .enumerate()
    .filter_map(|(index, mark)| mark.is_none().then_some(index))
    .collect()
}

fn status_text(state: VisualState) -> &'static str {
  match state {
    VisualState::EmptyBoard | VisualState::AiResponse | VisualState::RestoredBoard => PLAYER_TURN,
    VisualState::HumanMove => THINKING,
    VisualState::PlayerWin => "You win! Click the board to play again.",
    VisualState::ComputerWin => "Computer wins. Click the board to play again.",
    VisualState::Draw => "Draw! Click the board to play again.",
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

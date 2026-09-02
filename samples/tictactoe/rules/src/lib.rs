//! Native rules engine for the standalone Tic-Tac-Toe sample.

use std::time::{Duration, Instant};

use battlement::{
  ActionBody, ActionId, CameraClearMode, CameraProjection, CameraState, ClientMessage, Color,
  Command, CommandBody, Connect, CoreErrorCode, GameObject, ImageFit, ImageState, ObjectId,
  ParentScene, PointerButton, PointerEvent, PreparedAsset, Response, Scene, SceneId, SessionId,
  Snapshot, TextState, UiEventAction, UiEventResponse, Vector3, object_id, scene_id,
};
use battlement_native::{Engine, EngineError};
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

/// Finite user-visible states exercised by the Tic-Tac-Toe Ditto suite.
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
  let mut engine = create_seeded_engine(DITTO_SEED, Instant::now);
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
  type ActionPayload = ();
  type ErrorCode = CoreErrorCode;
  type Command = Command;

  fn connect(&mut self, _message: Connect) -> Result<Response<Self::Command>, EngineError> {
    self.session_id = SessionId::new_v4();
    self.round = 1;
    self.reset_state(self.semantic_fixture.unwrap_or(VisualState::EmptyBoard));
    if self.semantic_fixture == Some(VisualState::HumanMove) {
      self.board[2] = Some(Mark::X);
      self.marker_ids[2] = Some(X_MARK_IDS[2]);
    }
    Ok(Response::snapshot(self::snapshot(
      self.session_id,
      self.round,
      &self.board,
      self.visual_state,
    )))
  }

  fn submit(
    &mut self,
    message: ClientMessage<Self::ActionPayload, Self::ErrorCode>,
  ) -> Result<Response<Self::Command>, EngineError> {
    Ok(self.submit_at(message, (self.now)()))
  }

  fn submit_ui_event(
    &mut self,
    action: UiEventAction,
  ) -> Result<UiEventResponse<Self::Command>, EngineError> {
    if action.session_id != self.session_id {
      return Err(EngineError::new("UI event session mismatch"));
    }
    Ok(UiEventResponse::from_event(
      &action.event,
      Response::empty(self.session_id),
    ))
  }

  fn poll(&mut self) -> Result<Option<Response<Self::Command>>, EngineError> {
    Ok(self.poll_at((self.now)()))
  }
}

impl TicTacToeEngine {
  /// Returns the current user-visible state classification.
  pub const fn visual_state(&self) -> VisualState {
    self.visual_state
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

  fn submit_at(
    &mut self,
    message: ClientMessage<(), CoreErrorCode>,
    now: Instant,
  ) -> Response<Command> {
    let empty = Response::empty(self.session_id);
    let Some(action) = message.into_action() else {
      return empty;
    };
    let ActionBody::PointerClick(payload) = action.body else {
      return empty;
    };
    if payload.object_id != BOARD_ID || payload.button != PointerButton::Left {
      return empty;
    }
    if self.outcome != Outcome::InProgress {
      return self.reset_round(action.action_id);
    }
    if self.ai_due.is_some() {
      return empty;
    }
    let Some(index) = self::cell_index(payload.world_hit) else {
      return empty;
    };
    if self.board[index].is_some() {
      // An occupied square does not change the game, so there are no commands to send.
      return empty;
    }

    let marker = self.place_mark(index, Mark::X);
    self.outcome = self::outcome(&self.board);
    self.visual_state = self.visual_state_after_player_move(self.outcome);
    let mut commands = vec![CommandBody::object_create(marker)];
    if self.outcome == Outcome::InProgress {
      self.ai_due = Some(now + AI_DELAY);
      commands.push(self::status_command(self.visual_state));
      commands.push(CommandBody::set_input_enabled(false));
    } else {
      commands.push(self::status_command(self.visual_state));
    }
    Response::commands_for_action(self.session_id, action.action_id, commands)
  }

  fn poll_at(&mut self, now: Instant) -> Option<Response<Command>> {
    let due = self.ai_due?;
    if now < due {
      return None;
    }
    self.ai_due = None;
    // An AI turn is scheduled only while the game is in progress, so a cell is available.
    let empty = self::empty_cells(&self.board);
    let index = empty[self.rng.usize(..empty.len())];
    let marker = self.place_mark(index, Mark::O);
    self.outcome = self::outcome(&self.board);
    self.visual_state = self.visual_state_after_ai_move(self.outcome);
    Some(Response::commands(
      self.session_id,
      vec![
        CommandBody::object_create(marker),
        self::status_command(self.visual_state),
        CommandBody::set_input_enabled(true),
      ],
    ))
  }

  fn place_mark(&mut self, index: usize, mark: Mark) -> GameObject {
    let object_id = self::marker_id(index, mark);
    self.board[index] = Some(mark);
    self.marker_ids[index] = Some(object_id);
    self::marker(object_id, index, mark)
  }

  fn reset_round(&mut self, action_id: ActionId) -> Response<Command> {
    let mut commands = self
      .marker_ids
      .iter()
      .flatten()
      .map(|object_id| CommandBody::object_destroy(*object_id))
      .collect::<Vec<_>>();
    self.round += 1;
    self.reset_state(VisualState::RestoredBoard);
    commands.push(CommandBody::set_text(
      TITLE_ID,
      format!("TIC TAC TOE — ROUND {}", self.round),
    ));
    commands.push(self::status_command(self.visual_state));
    Response::commands_for_action(self.session_id, action_id, commands)
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

fn snapshot(
  session_id: SessionId,
  round: u32,
  marks: &[Option<Mark>; 9],
  visual_state: VisualState,
) -> Snapshot {
  let camera = GameObject::new(
    CAMERA_ID,
    CameraState::new()
      .projection(CameraProjection::Orthographic)
      .orthographic_size(5.6)
      .clear_mode(CameraClearMode::SolidColor)
      .clear_color(Color::rgb(0.96, 0.93, 0.84)),
  )
  .parent_scene(ParentScene::Persistent)
  .position(Vector3::new(0.0, 0.0, -10.0));

  let board = GameObject::new(
    BOARD_ID,
    ImageState::new(BOARD_TEXTURE, BOARD_SIZE, BOARD_SIZE),
  )
  .position(Vector3::new(0.0, BOARD_CENTER_Y, 0.0))
  .pointer_events([PointerEvent::Click]);

  let title = GameObject::new(
    TITLE_ID,
    TextState::new(format!("TIC TAC TOE — ROUND {round}"), FONT)
      .size(4.0)
      .color(Color::rgb(0.03, 0.04, 0.08)),
  )
  .parent_scene(ParentScene::Persistent)
  .position(Vector3::new(0.0, 4.7, -0.1));

  let status = GameObject::new(
    STATUS_ID,
    TextState::new(self::status_text(visual_state), FONT)
      .size(3.2)
      .color(Color::rgb(0.06, 0.08, 0.15))
      .wrap_width(14.0),
  )
  .parent_scene(ParentScene::Persistent)
  .position(Vector3::new(0.0, 3.75, -0.1));

  let mut objects = vec![camera, board, title, status];
  objects.extend(marks.iter().enumerate().filter_map(|(index, mark)| {
    mark.map(|mark| self::marker(self::marker_id(index, mark), index, mark))
  }));
  Snapshot::new(
    session_id,
    vec![
      PreparedAsset::scene(CONTENT_SCENE),
      PreparedAsset::texture(BOARD_TEXTURE),
      PreparedAsset::texture(X_TEXTURE),
      PreparedAsset::texture(O_TEXTURE),
      PreparedAsset::text_mesh_pro_font(FONT),
    ],
    vec![Scene::new(SCENE_ID, CONTENT_SCENE)],
    objects,
    CAMERA_ID,
  )
}

fn semantic_fixture(name: &str) -> Option<VisualState> {
  match name {
    "human move" => Some(VisualState::HumanMove),
    _ => None,
  }
}

fn marker(object_id: ObjectId, index: usize, mark: Mark) -> GameObject {
  let texture = if mark == Mark::X {
    X_TEXTURE
  } else {
    O_TEXTURE
  };
  GameObject::new(
    object_id,
    ImageState::new(texture, MARK_SIZE, MARK_SIZE).fit(ImageFit::Contain),
  )
  .position(self::cell_position(index))
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

fn status_command(state: VisualState) -> CommandBody {
  CommandBody::set_text(STATUS_ID, self::status_text(state))
}

battlement_native::export_engine!(create_engine);

//! Reactant rules and world presentation for the Tic-Tac-Toe sample.

mod board;
mod rules;

use std::{cell::RefCell, time::Instant};

use battlement::{ObjectId, PickingMode, Prop, Vector3, object_id};
use battlement_native::{
  Engine, EngineError, EngineResponse, FlatBufferSubmitError, UiEventActionView, UiEventResult,
};
use reactant::{
  GameConsumer, GameHandle,
  app::App,
  prelude::{GameApp, GameRoot},
};

pub use rules::{DITTO_SEED, VisualState};

use crate::{board::Board, rules::TicTacToe};

/// Address of the sample's content scene.
pub const CONTENT_SCENE: &str = "tictactoe/content";
/// Machine-readable registry consumed by the Ditto coverage checker.
pub const DITTO_VISUAL_STATE_REGISTRY: &str = include_str!("../../ditto-visual-states.toml");
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
/// Stable identity of the visible board artwork.
pub const BOARD_ID: ObjectId = object_id!("c8c9e10d-585b-45f4-ac19-b76746ed2d25");
/// Stable identities of the board's row-major input cells.
pub const CELL_IDS: [ObjectId; 9] = [
  object_id!("7113e459-bb65-4bf5-aa12-da28c9b458d1"),
  object_id!("f9e96f44-6187-49fc-9132-af2e45861128"),
  object_id!("2b9c5782-ddcb-48f9-9ff4-b9093e2124c2"),
  object_id!("aa42c289-59fe-4d6c-b0ac-f9c4c9b536ab"),
  object_id!("bdb19541-c287-4973-9887-0ed3126eb0b7"),
  object_id!("9d42b351-554e-4cc1-90d2-e5429931af5f"),
  object_id!("b901e08c-c230-461e-b5bd-3099317a4f6f"),
  object_id!("72aa0618-2cea-4b56-a1df-9c8882a3e0dd"),
  object_id!("99005f11-7e94-4df9-b4d2-1ff582108a89"),
];
/// Stable identities of player marks in row-major board order.
pub const X_MARK_IDS: [ObjectId; 9] = [
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
/// Stable identities of computer marks in row-major board order.
pub const O_MARK_IDS: [ObjectId; 9] = [
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
/// Stable identity of the visible game-status text.
pub const STATUS_ID: ObjectId = object_id!("9b10a4a0-1367-46a8-9a2c-7c29eef033b1");
/// Stable identity of the visible game title.
pub const TITLE_ID: ObjectId = object_id!("860e3fa1-d047-45ae-869d-3321e9cd3142");

const ROOT_ID: ObjectId = object_id!("cbb4c265-6d14-42c2-80ae-ef5669b2132c");

/// Reactant engine used by the standalone sample and public display tests.
pub struct TicTacToeEngine {
  seed: u64,
  fixture: Option<VisualState>,
  app: App<TicTacToeModel>,
}

/// App-owned access to public worker synchronization.
pub struct TicTacToeModel {
  consumer: RefCell<Option<GameConsumer<TicTacToe>>>,
  game: RefCell<Option<GameHandle<TicTacToe>>>,
}

/// Creates the engine used by the native sample.
pub fn create_engine() -> Result<TicTacToeEngine, EngineError> {
  let fixture = std::env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE")
    .ok()
    .map(|name| {
      VisualState::semantic_fixture(&name)
        .unwrap_or_else(|| panic!("unknown Tic-Tac-Toe semantic fixture {name:?}"))
    });
  Ok(TicTacToeEngine::new(DITTO_SEED, fixture))
}

/// Creates a deterministic engine for simulations.
pub fn create_seeded_engine(seed: u64, _now: impl Fn() -> Instant + 'static) -> TicTacToeEngine {
  TicTacToeEngine::new(seed, None)
}

impl TicTacToeEngine {
  fn new(seed: u64, fixture: Option<VisualState>) -> Self {
    Self {
      seed,
      fixture,
      app: self::app(seed, fixture),
    }
  }

  /// Waits for a rules publication without advancing presentation time or frames.
  pub fn wait_for_output(&self, timeout: std::time::Duration) -> bool {
    self.app.model().wait_for_output(timeout)
  }

  /// Waits for current worker cleanup without advancing presentation time or frames.
  pub fn wait_for_worker_stopped(&self, timeout: std::time::Duration) -> bool {
    self.app.model().wait_for_worker_stopped(timeout)
  }

  /// Returns the current public game readiness state.
  pub fn game_status(&self) -> reactant::GameStatus {
    self.app.model().game_status()
  }
}

impl TicTacToeModel {
  fn new() -> Self {
    Self {
      consumer: RefCell::new(None),
      game: RefCell::new(None),
    }
  }

  fn consumer(&self) -> std::cell::Ref<'_, GameConsumer<TicTacToe>> {
    std::cell::Ref::map(self.consumer.borrow(), |consumer| {
      consumer.as_ref().expect("game consumer is initialized")
    })
  }

  fn wait_for_output(&self, timeout: std::time::Duration) -> bool {
    self.consumer().wait_for_output(timeout)
  }

  fn wait_for_worker_stopped(&self, timeout: std::time::Duration) -> bool {
    self.consumer().wait_for_worker_stopped(timeout)
  }

  fn game_status(&self) -> reactant::GameStatus {
    self
      .game
      .borrow()
      .as_ref()
      .expect("game handle is initialized")
      .status()
  }
}

fn app(seed: u64, fixture: Option<VisualState>) -> App<TicTacToeModel> {
  let mut app = App::with_model(CONTENT_SCENE, TicTacToeModel::new());
  let game = app.start_game::<TicTacToe>(rules::State::new(seed, fixture), |connection| {
    rules::Context::new(connection)
  });
  let consumer = app.game_consumer::<TicTacToe>();
  consumer.resume_automatic_submission();
  app.model().consumer.replace(Some(consumer));
  app.model().game.replace(Some(game.clone()));
  app
    .ui(GameRoot::new(Board::new(game)))
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document.element.picking_mode = Prop::Set(PickingMode::Ignore);
      document
    })
    .camera(|_camera| {
      reactant::world::Camera::new()
        .orthographic(5.6)
        .background(battlement::Color::rgb(0.96, 0.93, 0.84))
        .position(Vector3::new(0.0, 0.0, -10.0))
        .into_object(CAMERA_ID)
    })
}

impl Engine for TicTacToeEngine {
  const WIRE_CONTRACT_DIGEST_C: &'static [u8; 65] = battlement_native::WIRE_CONTRACT_DIGEST_C;

  fn connect(
    &mut self,
    message: battlement_native::ConnectView<'_>,
  ) -> Result<EngineResponse, EngineError> {
    self.app = self::app(self.seed, self.fixture);
    Engine::connect(&mut self.app, message)
  }

  fn submit(&mut self, bytes: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError> {
    Engine::submit(&mut self.app, bytes)
  }

  fn submit_ui_event(
    &mut self,
    action: UiEventActionView<'_>,
  ) -> Result<UiEventResult, EngineError> {
    Engine::submit_ui_event(&mut self.app, action)
  }

  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
    Engine::poll(&mut self.app)
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

//! App-owned lifecycle and persistence for the chess sample.

use std::{
  path::PathBuf,
  time::{Duration, Instant},
};

use battlement::ObjectId;
use battlement_native::{
  ConnectView, Engine, EngineError, EngineResponse, FlatBufferSubmitError, UiEventActionView,
  UiEventResult,
};
use cozy_chess::Board;
use reactant::{
  DispatchResult, GameStatus as RulesStatus, app::App, prelude::GameApp, rules::ExecutionMode,
};

use crate::{
  reactant_game::{
    ChessAction, ChessContext, ChessGame, ChessPolicy, ChessState, PersistenceDirective, StartMode,
  },
  reactant_input::ChessModel,
};

const MUSIC_TRACK_DURATION: Duration = Duration::from_secs(120);

/// Complete Reactant chess engine exported by the sample.
pub struct ReactantChessApp {
  app: App<ChessModel>,
  starting_board: Board,
  think_time: Duration,
  seed: Option<u64>,
  now: Box<dyn Fn() -> Instant>,
  initial_state: ChessState,
  load_persistence: bool,
  review_state: Option<crate::visual_state::VisualState>,
  persistent_data_path: Option<PathBuf>,
  persisted_revision: u64,
  persistence_error: Option<String>,
  pending_start: Option<StartMode>,
  music_due: Option<Instant>,
  replacement_generation: u32,
}

impl ReactantChessApp {
  /// Creates the app with production AI settings.
  pub fn new() -> Self {
    Self::with_state(
      ChessState::title(Board::default()),
      Board::default(),
      crate::AI_THINK_TIME,
      None,
      Instant::now,
    )
  }

  /// Creates a deterministic app for public behavior checks.
  pub fn with_think_time(think_time: Duration) -> Self {
    Self::with_think_time_and_clock(think_time, Instant::now)
  }

  /// Creates an app with production AI timing and deterministic presentation randomness.
  pub fn with_seed(seed: u64) -> Self {
    Self::with_state(
      ChessState::title(Board::default()),
      Board::default(),
      crate::AI_THINK_TIME,
      Some(seed),
      Instant::now,
    )
  }

  /// Creates a deterministic app with a caller-owned monotonic clock.
  pub fn with_think_time_and_clock(
    think_time: Duration,
    now: impl Fn() -> Instant + 'static,
  ) -> Self {
    Self::with_state(
      ChessState::title(Board::default()),
      Board::default(),
      think_time,
      Some(43),
      now,
    )
  }

  /// Creates an already-started deterministic position.
  pub fn with_position(fen: &str, think_time: Duration) -> Result<Self, String> {
    let board = fen
      .parse::<Board>()
      .map_err(|error| format!("invalid chess position: {error}"))?;
    Ok(Self::with_state(
      ChessState::resumed(board.clone()),
      board,
      think_time,
      Some(43),
      Instant::now,
    ))
  }

  /// Creates a title-screen app whose next game starts from a supplied position.
  pub fn with_starting_position(fen: &str, think_time: Duration) -> Result<Self, String> {
    let board = fen
      .parse::<Board>()
      .map_err(|error| format!("invalid chess position: {error}"))?;
    Ok(Self::with_state(
      ChessState::title(board.clone()),
      board,
      think_time,
      None,
      Instant::now,
    ))
  }

  pub(crate) fn named_review(name: &str) -> Result<Self, String> {
    let (starting_board, visual_state) = if name == "paused" {
      (Board::default(), crate::visual_state::VisualState::Paused)
    } else {
      let fixture = crate::visual_state::semantic_fixture(name)
        .ok_or_else(|| format!("unknown Reactant Chess app fixture {name:?}"))?;
      (fixture.board, fixture.state)
    };
    let state = if visual_state == crate::visual_state::VisualState::Title {
      ChessState::title(starting_board.clone())
    } else {
      ChessState::review(starting_board.clone(), visual_state)
    };
    let mut app = Self::with_state(
      state,
      starting_board,
      Duration::ZERO,
      Some(43),
      Instant::now,
    );
    app.load_persistence = false;
    app.review_state = Some(visual_state);
    Ok(app)
  }

  fn with_state(
    initial_state: ChessState,
    starting_board: Board,
    think_time: Duration,
    seed: Option<u64>,
    now: impl Fn() -> Instant + 'static,
  ) -> Self {
    let app = crate::reactant_view::app(initial_state.clone(), think_time, seed, false);
    Self {
      app,
      starting_board,
      think_time,
      seed,
      now: Box::new(now),
      initial_state,
      load_persistence: true,
      review_state: None,
      persistent_data_path: None,
      persisted_revision: 0,
      persistence_error: None,
      pending_start: None,
      music_due: None,
      replacement_generation: 0,
    }
  }

  /// Waits for one queued worker publication without advancing presentation time.
  pub fn wait_for_output(&self, timeout: Duration) -> bool {
    self
      .app
      .model()
      .consumer
      .borrow()
      .as_ref()
      .expect("chess consumer")
      .wait_for_output(timeout)
  }

  /// Waits for the active rules worker to stop.
  pub fn wait_for_worker_stopped(&self, timeout: Duration) -> bool {
    self
      .app
      .model()
      .consumer
      .borrow()
      .as_ref()
      .expect("chess consumer")
      .wait_for_worker_stopped(timeout)
  }

  /// Returns current rules readiness.
  pub fn status(&self) -> RulesStatus {
    self.app.model().game().status()
  }

  /// Returns the last accepted chess state.
  pub fn accepted_state(&self) -> ChessState {
    self.app.model().game().accepted_state()
  }

  /// Returns the current user-visible state classification.
  pub fn visual_state(&self) -> crate::visual_state::VisualState {
    let state = self.accepted_state();
    self.app.model().control.visual_state(&state)
  }

  /// Resolves a stable Reactant piece identity to its current native host.
  pub fn native_piece(&self, piece: ObjectId) -> Option<ObjectId> {
    self
      .app
      .presentation(*piece.as_uuid())
      .and_then(|observation| observation.native_objects.first().copied())
  }

  /// Returns the latest non-fatal persistence failure.
  pub fn persistence_error(&self) -> Option<&str> {
    self.persistence_error.as_deref()
  }

  /// Requests a shortcut restart even while the current worker remains active.
  pub fn restart(&self) {
    self.app.model().control.request_restart();
  }

  fn rebuild_for_connect(&mut self, message: ConnectView<'_>) {
    self.persistent_data_path = message.persistent_data_path().map(PathBuf::from);
    let deterministic = std::env::var("BATTLEMENT_DITTO_ACTIVE").as_deref() == Ok("1");
    let state = if deterministic || !self.load_persistence {
      self.initial_state.clone()
    } else {
      self
        .persistent_data_path
        .as_deref()
        .and_then(crate::persistence::load)
        .map(ChessState::resumed)
        .unwrap_or_else(|| self.initial_state.clone())
    };
    self.app = crate::reactant_view::app(
      state,
      self.think_time,
      self.seed,
      crate::diagnostics::is_available(message),
    );
    if let Some(review_state) = self.review_state {
      self.app.model().control.prepare_review(review_state);
    }
    self.persisted_revision = 0;
    self.persistence_error = None;
    self.pending_start = None;
    self.music_due = None;
    self.replacement_generation = 0;
  }

  fn maintain(&mut self) {
    self.apply_replacement();
    let game = self.app.model().game();
    if game.status() == RulesStatus::Ready
      && let Some(mode) = self.pending_start.take()
    {
      let result = game.dispatch(ChessAction::Start(mode));
      assert_eq!(result, DispatchResult::Started);
    }
    let accepted = game.accepted_state();
    if accepted.started() && self.music_due.is_none() {
      self.app.model().control.start_music();
      self.music_due = Some((self.now)() + MUSIC_TRACK_DURATION);
    }
    if self.music_due.is_some_and(|due| (self.now)() >= due) {
      self.app.model().control.next_music();
      self.music_due = Some((self.now)() + MUSIC_TRACK_DURATION);
    }
    if game.status() == RulesStatus::Ready {
      self.persist(&accepted);
      let ai_turn = accepted.board().side_to_move() == cozy_chess::Color::Black
        && accepted.board().status() == cozy_chess::GameStatus::Ongoing;
      if accepted.started() && ai_turn {
        let result = game.dispatch(ChessAction::AiMove);
        assert_eq!(result, DispatchResult::Started);
      }
    }
  }

  fn apply_replacement(&mut self) {
    let Some(request) = self.app.model().control.take_replacement() else {
      return;
    };
    let restart_music = request.mode == StartMode::Restart;
    self
      .app
      .model()
      .control
      .reset_for_replacement(request.cursor_visible);
    self.replacement_generation = self
      .replacement_generation
      .checked_add(1)
      .expect("replacement generation overflow");
    let state =
      ChessState::title_generation(self.starting_board.clone(), self.replacement_generation);
    let game = self.app.start_game::<ChessGame>(state, {
      let think_time = self.think_time;
      let seed = self.seed;
      move |connection| {
        ChessContext::new(
          ExecutionMode::Interactive {
            connection,
            policy: ChessPolicy,
          },
          think_time,
          seed.map_or_else(fastrand::Rng::new, fastrand::Rng::with_seed),
        )
      }
    });
    let consumer = self.app.game_consumer::<ChessGame>();
    consumer.resume_automatic_submission();
    self.app.model().consumer.replace(Some(consumer));
    self.app.model().game.replace(Some(game));
    self.persisted_revision = 0;
    self.persistence_error = None;
    self.pending_start = Some(request.mode);
    if restart_music {
      self.app.model().control.restart_music();
      self.music_due = Some((self.now)() + MUSIC_TRACK_DURATION);
    }
  }

  fn persist(&mut self, state: &ChessState) {
    let revision = state.persistence_revision();
    if revision == 0 || revision == self.persisted_revision {
      return;
    }
    let result = match (&self.persistent_data_path, state.persistence()) {
      (Some(path), PersistenceDirective::Save) => crate::persistence::save(path, state.board()),
      (Some(path), PersistenceDirective::Clear) => crate::persistence::clear(path),
      (None, _) | (_, PersistenceDirective::None) => Ok(()),
    };
    match result {
      Ok(()) => {
        self.persisted_revision = revision;
        self.persistence_error = None;
      }
      Err(error) => {
        tracing::warn!(%error, "Chess accepted state could not be persisted");
        self.persistence_error = Some(error);
      }
    }
  }
}

impl Default for ReactantChessApp {
  fn default() -> Self {
    Self::new()
  }
}

impl Engine for ReactantChessApp {
  const WIRE_CONTRACT_DIGEST_C: &'static [u8; 65] = battlement_native::WIRE_CONTRACT_DIGEST_C;

  fn connect(&mut self, message: ConnectView<'_>) -> Result<EngineResponse, EngineError> {
    self.rebuild_for_connect(message);
    let response = Engine::connect(&mut self.app, message)?;
    self.maintain();
    Ok(response)
  }

  fn submit(&mut self, bytes: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError> {
    let response = Engine::submit(&mut self.app, bytes)?;
    self.maintain();
    Ok(response)
  }

  fn submit_ui_event(
    &mut self,
    action: UiEventActionView<'_>,
  ) -> Result<UiEventResult, EngineError> {
    let response = Engine::submit_ui_event(&mut self.app, action)?;
    self.maintain();
    Ok(response)
  }

  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
    let response = Engine::poll(&mut self.app)?;
    self.maintain();
    Ok(response)
  }
}

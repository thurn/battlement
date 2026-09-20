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
use reactant::{GameStatus as RulesStatus, app::App, prelude::GameApp, rules::ExecutionMode};

use crate::{
  chess_ui_state::{ReplacementRequest, SessionStart},
  reactant_game::{ChessContext, ChessGame, ChessPolicy, ChessState},
  reactant_input::ChessModel,
};

const MUSIC_TRACK_DURATION: Duration = Duration::from_secs(120);

#[derive(Clone)]
struct InitialSession {
  state: ChessState,
  visual_state: crate::visual_state::VisualState,
  origin_saved: bool,
}

/// Complete Reactant chess engine exported by the sample.
pub struct ReactantChessApp {
  app: App<ChessModel>,
  starting_board: Board,
  think_time: Duration,
  seed: Option<u64>,
  now: Box<dyn Fn() -> Instant>,
  initial_session: Option<InitialSession>,
  load_persistence: bool,
  review_state: Option<crate::visual_state::VisualState>,
  persistent_data_path: Option<PathBuf>,
  observed_board: Option<Board>,
  persisted_position: Option<String>,
  persistence_error: Option<String>,
  pending_replacement: Option<ReplacementRequest>,
  music_due: Option<Instant>,
}

impl ReactantChessApp {
  /// Creates the app with production AI settings.
  pub fn new() -> Self {
    Self::with_state(
      None,
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
      None,
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
    Self::with_state(None, Board::default(), think_time, Some(43), now)
  }

  /// Creates an already-started deterministic position.
  pub fn with_position(fen: &str, think_time: Duration) -> Result<Self, String> {
    let board = fen
      .parse::<Board>()
      .map_err(|error| format!("invalid chess position: {error}"))?;
    Ok(Self::with_state(
      Some(InitialSession {
        state: ChessState::new(board.clone()),
        visual_state: crate::visual_state::VisualState::Resumed,
        origin_saved: true,
      }),
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
      None,
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
    let session = if visual_state == crate::visual_state::VisualState::Title {
      None
    } else {
      Some(InitialSession {
        state: ChessState::new(starting_board.clone()),
        visual_state,
        origin_saved: visual_state == crate::visual_state::VisualState::Resumed,
      })
    };
    let mut app = Self::with_state(
      session,
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
    initial_session: Option<InitialSession>,
    starting_board: Board,
    think_time: Duration,
    seed: Option<u64>,
    now: impl Fn() -> Instant + 'static,
  ) -> Self {
    let (state, visual_state, origin_saved) = initial_session
      .as_ref()
      .map(|session| {
        (
          Some(session.state.clone()),
          session.visual_state,
          session.origin_saved,
        )
      })
      .unwrap_or((None, crate::visual_state::VisualState::Title, false));
    let app = crate::reactant_view::app(state, visual_state, origin_saved, think_time, seed, false);
    Self {
      app,
      starting_board,
      think_time,
      seed,
      now: Box::new(now),
      initial_session,
      load_persistence: true,
      review_state: None,
      persistent_data_path: None,
      observed_board: None,
      persisted_position: None,
      persistence_error: None,
      pending_replacement: None,
      music_due: None,
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
      .is_none_or(|consumer| consumer.wait_for_output(timeout))
  }

  /// Waits for the active rules worker to stop.
  pub fn wait_for_worker_stopped(&self, timeout: Duration) -> bool {
    self
      .app
      .model()
      .consumer
      .borrow()
      .as_ref()
      .is_none_or(|consumer| consumer.wait_for_worker_stopped(timeout))
  }

  /// Returns current rules readiness.
  pub fn status(&self) -> RulesStatus {
    self
      .app
      .model()
      .game()
      .map_or(RulesStatus::Ready, |game| game.status())
  }

  /// Returns the last accepted chess state.
  pub fn accepted_state(&self) -> Option<ChessState> {
    self.app.model().game().map(|game| game.accepted_state())
  }

  /// Returns the current user-visible state classification.
  pub fn visual_state(&self) -> crate::visual_state::VisualState {
    self.app.model().control.visual_state()
  }

  /// Resolves a stable Reactant piece identity to its current native host.
  pub fn native_piece(&self, piece: ObjectId) -> Option<ObjectId> {
    self.app.model().control.native_piece(piece)
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
    let session = if deterministic || !self.load_persistence {
      self.initial_session.clone()
    } else {
      self
        .persistent_data_path
        .as_deref()
        .and_then(crate::persistence::load)
        .map(|board| InitialSession {
          state: ChessState::resumed(board),
          visual_state: crate::visual_state::VisualState::Resumed,
          origin_saved: true,
        })
        .or_else(|| self.initial_session.clone())
    };
    let (state, visual_state, origin_saved) = session
      .as_ref()
      .map(|session| {
        (
          Some(session.state.clone()),
          session.visual_state,
          session.origin_saved,
        )
      })
      .unwrap_or((None, crate::visual_state::VisualState::Title, false));
    self.app = crate::reactant_view::app(
      state,
      visual_state,
      origin_saved,
      self.think_time,
      self.seed,
      crate::diagnostics::is_available(message),
    );
    if let Some(review_state) = self.review_state {
      self.app.model().control.prepare_review(review_state);
    }
    self.observed_board = session
      .as_ref()
      .map(|session| session.state.board().clone());
    self.persisted_position = session
      .as_ref()
      .filter(|session| session.origin_saved)
      .map(|session| session.state.board().to_string());
    self.persistence_error = None;
    self.pending_replacement = None;
    self.music_due = None;
  }

  fn maintain(&mut self) {
    self.apply_replacement();
    let Some(game) = self.app.model().game() else {
      return;
    };
    let accepted = game.accepted_state();
    self.observe(&accepted);
    if self.music_due.is_none() {
      self.app.model().control.start_music();
      self.music_due = Some((self.now)() + MUSIC_TRACK_DURATION);
    }
    if self.music_due.is_some_and(|due| (self.now)() >= due) {
      self.app.model().control.next_music();
      self.music_due = Some((self.now)() + MUSIC_TRACK_DURATION);
    }
    self.persist(accepted.board());
  }

  fn apply_replacement(&mut self) {
    let request = if let Some(request) = self.pending_replacement.take() {
      request
    } else {
      let Some(request) = self.app.model().control.take_replacement() else {
        return;
      };
      if let Some(game) = self.app.model().game() {
        game.stop();
        self.app.model().control.cancel_opening();
        self.pending_replacement = Some(request);
        return;
      }
      request
    };
    let restart_music = request.mode == SessionStart::Restart;
    let state = ChessState::with_generation(self.starting_board.clone(), 1);
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
    self
      .app
      .model()
      .control
      .begin_session(request.mode, request.cursor_visible, false);
    self.observed_board = Some(self.starting_board.clone());
    self.persisted_position = None;
    self.persistence_error = None;
    if request.mode == SessionStart::Restart {
      self.clear_persistence();
      self.persisted_position = Some(self.starting_board.to_string());
    }
    if restart_music {
      self.app.model().control.restart_music();
      self.music_due = Some((self.now)() + MUSIC_TRACK_DURATION);
    }
  }

  fn persist(&mut self, board: &Board) {
    let position = board.to_string();
    if self.persisted_position.as_deref() == Some(position.as_str()) {
      return;
    }
    let result = match &self.persistent_data_path {
      Some(path) => crate::persistence::save(path, board),
      None => Ok(()),
    };
    match result {
      Ok(()) => {
        self.persisted_position = Some(position);
        self.persistence_error = None;
      }
      Err(error) => {
        tracing::warn!(%error, "Chess accepted state could not be persisted");
        self.persistence_error = Some(error);
      }
    }
  }

  fn clear_persistence(&mut self) {
    let result = self
      .persistent_data_path
      .as_deref()
      .map_or(Ok(()), crate::persistence::clear);
    match result {
      Ok(()) => self.persistence_error = None,
      Err(error) => {
        tracing::warn!(%error, "Chess saved game could not be cleared");
        self.persistence_error = Some(error);
      }
    }
  }

  fn observe(&mut self, state: &ChessState) {
    let after = state.board();
    if self
      .observed_board
      .as_ref()
      .is_some_and(|before| before.to_string() == after.to_string())
    {
      return;
    }
    if let Some(before) = &self.observed_board
      && let Some(movement) = transition(before, after)
    {
      let mover = before
        .color_on(movement.from)
        .expect("observed legal move has a mover");
      self
        .app
        .model()
        .control
        .observe_visual_state(crate::visual_state::after_move(
          before, after, movement, mover,
        ));
    }
    self.observed_board = Some(after.clone());
  }
}

fn transition(before: &Board, after: &Board) -> Option<cozy_chess::Move> {
  let mut found = None;
  before.generate_moves(|moves| {
    for movement in moves {
      let mut candidate = before.clone();
      candidate.play_unchecked(movement);
      if candidate.to_string() == after.to_string() {
        found = Some(movement);
        return true;
      }
    }
    false
  });
  found
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

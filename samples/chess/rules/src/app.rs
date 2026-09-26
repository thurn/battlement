//! Application assembly and scenario configuration.

use std::sync::Arc;
use std::{
  rc::Rc,
  time::{Duration, Instant},
};

use cozy_chess::Board;
use reactant::Application;

use crate::opponent::Opponent;
use crate::{
  reactant_game::ChessState,
  reactant_view::{self, ChessConfig},
  visual_state::{self, VisualState},
};
use reactant::rules::RulesWorker;
use reactant::{ApplicationEngine, PersistenceBackend};

const AI_THINK_TIME: Duration = Duration::from_secs(2);

/// External services used to construct an opaque chess engine.
pub struct EngineDependencies {
  /// Raw storage for saved progress and local preferences.
  pub persistence: Option<Arc<dyn PersistenceBackend>>,
  /// Logical position mounted directly; None opens the title screen.
  pub position: Option<Board>,
  /// Rules runner; native applications use its default worker.
  pub rules_worker: RulesWorker,
  /// Monotonic time observed by Reactant timers and rules work.
  pub now: Rc<dyn Fn() -> Instant>,
  /// Optional deterministic presentation-randomness seed.
  pub rng_seed: Option<u64>,
  /// Computer turn admission and move selection.
  pub opponent: Opponent,
}

/// Creates the opaque engine used by black-box hosts.
pub fn create_engine(dependencies: EngineDependencies) -> ApplicationEngine {
  let now = dependencies.now.clone();
  ApplicationEngine::with_clock(
    move || self::create_application(&dependencies),
    move || now(),
  )
}

/// Assembles chess for a host that customizes documents before supplying its engine clock.
pub fn create_application(dependencies: &EngineDependencies) -> Application {
  reactant_view::application(ChessConfig {
    starting_board: Board::default(),
    initial_state: dependencies.position.clone().map(ChessState::new),
    visual_state: VisualState::Initial,
    origin_saved: false,
    opponent: dependencies.opponent.clone(),
    seed: dependencies.rng_seed,
    persistence: dependencies.persistence.clone(),
  })
  .rules_worker(dependencies.rules_worker.clone())
}

/// Creates the component-first application used by the native sample.
pub(crate) fn application() -> Application {
  if let Ok(name) = std::env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE") {
    return review_application(&name);
  }
  let think_time = if std::env::var("BATTLEMENT_DITTO_ACTIVE").as_deref() == Ok("1") {
    Duration::ZERO
  } else {
    AI_THINK_TIME
  };
  configured_application(
    Board::default(),
    None,
    VisualState::Title,
    think_time,
    Some(43),
    true,
    Arc::new(reactant::FilePersistenceBackend),
  )
}

fn configured_application(
  starting_board: Board,
  initial_state: Option<ChessState>,
  visual_state: VisualState,
  think_time: Duration,
  seed: Option<u64>,
  load_persistence: bool,
  persistence: Arc<dyn PersistenceBackend>,
) -> Application {
  let origin_saved = visual_state == VisualState::Resumed;
  reactant_view::application(ChessConfig {
    starting_board,
    initial_state,
    visual_state,
    origin_saved,
    opponent: Opponent::search(think_time),
    seed,
    persistence: load_persistence.then_some(persistence),
  })
}

fn review_application(name: &str) -> Application {
  let (board, state) = if name == "paused" {
    (Board::default(), VisualState::Paused)
  } else {
    let fixture = visual_state::semantic_fixture(name)
      .unwrap_or_else(|| panic!("unknown Reactant Chess app fixture {name:?}"));
    (fixture.board, fixture.state)
  };
  let initial = (state != VisualState::Title).then(|| ChessState::new(board.clone()));
  configured_application(
    board,
    initial,
    state,
    Duration::ZERO,
    Some(43),
    false,
    Arc::new(reactant::FilePersistenceBackend),
  )
}

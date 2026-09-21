//! Application assembly and scenario configuration.

use std::{
  rc::Rc,
  time::{Duration, Instant},
};

use cozy_chess::Board;
use reactant::Application;

use crate::{
  reactant_game::ChessState,
  reactant_view::{self, ChessConfig},
  visual_state::{self, VisualState},
};
use reactant::{ApplicationEngine, Engine as RulesEngine, PersistenceBackend};

const AI_THINK_TIME: Duration = Duration::from_secs(2);

/// External services used to construct an opaque chess engine.
pub struct EngineDependencies {
  /// Raw saved-game storage.
  pub persistence: Rc<dyn PersistenceBackend>,
  /// Monotonic time observed by Reactant timers and rules work.
  pub now: Rc<dyn Fn() -> Instant>,
  /// Optional deterministic presentation-randomness seed.
  pub rng_seed: Option<u64>,
  /// Maximum computer search duration.
  pub think_time: Duration,
}

/// Creates the opaque engine used by black-box hosts.
pub fn create_engine(dependencies: EngineDependencies) -> impl RulesEngine {
  let now = dependencies.now.clone();
  let persistence = dependencies.persistence.clone();
  let think_time = dependencies.think_time;
  let seed = dependencies.rng_seed;
  ApplicationEngine::with_clock(
    move || {
      configured_application(
        Board::default(),
        None,
        VisualState::Title,
        think_time,
        seed,
        true,
        persistence.clone(),
      )
    },
    move || now(),
  )
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
    Rc::new(reactant::FilePersistenceBackend),
  )
}

fn configured_application(
  starting_board: Board,
  initial_state: Option<ChessState>,
  visual_state: VisualState,
  think_time: Duration,
  seed: Option<u64>,
  load_persistence: bool,
  persistence: Rc<dyn PersistenceBackend>,
) -> Application {
  let origin_saved = visual_state == VisualState::Resumed;
  reactant_view::application(ChessConfig {
    starting_board,
    initial_state,
    visual_state,
    origin_saved,
    think_time,
    seed,
    load_persistence,
    persistence,
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
    Rc::new(reactant::FilePersistenceBackend),
  )
}

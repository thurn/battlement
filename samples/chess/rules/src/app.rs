//! Application assembly and scenario configuration.

use std::time::Duration;

use cozy_chess::Board;

use crate::{
  reactant_game::ChessState,
  reactant_view::{self, ChessConfig},
  visual_state::{self, VisualState},
};
use reactant::Application;

const AI_THINK_TIME: Duration = Duration::from_secs(2);

/// Creates the component-first application used by the native sample.
pub fn application() -> Application {
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
    false,
    think_time,
    Some(43),
    true,
  )
}

#[cfg(test)]
pub(super) fn application_with_think_time(think_time: Duration) -> Application {
  configured_application(
    Board::default(),
    None,
    VisualState::Title,
    false,
    think_time,
    Some(43),
    true,
  )
}

#[cfg(test)]
pub(super) fn application_with_position(fen: &str, think_time: Duration) -> Application {
  configured_application(
    fen
      .parse::<Board>()
      .unwrap_or_else(|error| panic!("invalid chess position: {error}")),
    None,
    VisualState::Title,
    false,
    think_time,
    Some(43),
    false,
  )
}

#[cfg(test)]
pub(super) fn application_at_position(fen: &str, think_time: Duration) -> Application {
  let board = fen
    .parse::<Board>()
    .unwrap_or_else(|error| panic!("invalid chess position: {error}"));
  configured_application(
    board.clone(),
    Some(ChessState::new(board)),
    VisualState::Resumed,
    true,
    think_time,
    Some(43),
    false,
  )
}

fn configured_application(
  starting_board: Board,
  initial_state: Option<ChessState>,
  visual_state: VisualState,
  origin_saved: bool,
  think_time: Duration,
  seed: Option<u64>,
  load_persistence: bool,
) -> Application {
  reactant_view::application(ChessConfig {
    starting_board,
    initial_state,
    visual_state,
    origin_saved,
    think_time,
    seed,
    load_persistence,
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
    state == VisualState::Resumed,
    Duration::ZERO,
    Some(43),
    false,
  )
}

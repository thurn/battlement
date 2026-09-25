//! Session-owned computer admission after visible player-move completion.

use std::time::Duration;

use cozy_chess::{Color, GameStatus};
use reactant::{
  DispatchResult, GameHandle, GameStatus as RulesStatus, application, hooks, prelude::*,
};

use crate::{
  chess_ui_state::{AppScreen, ChessUiController},
  opponent::Opponent,
  reactant_game::{ChessAction, ChessGame},
  settings,
};

pub(crate) struct TurnCoordinator {
  pub opponent: Opponent,
  pub game: GameHandle<ChessGame>,
  pub control: ChessUiController,
}

struct WaitingTurn {
  opponent: Opponent,
  game: GameHandle<ChessGame>,
  control: ChessUiController,
}

impl Component for TurnCoordinator {
  fn render(&self) -> impl Render {
    let state = reactant::use_game_state::<ChessGame>();
    let status = reactant::use_game_status::<ChessGame>();
    let motion_ready = reactant::use_game_motion_ready::<ChessGame>();
    let local = self.control.snapshot();
    let computer_turn =
      state.board().side_to_move() == Color::Black && state.board().status() == GameStatus::Ongoing;
    let ready = status == RulesStatus::Ready && motion_ready && !local.spawning;
    (computer_turn && ready).then(|| {
      WaitingTurn {
        opponent: self.opponent.clone(),
        game: self.game.clone(),
        control: self.control.clone(),
      }
      .key((local.opening_generation, state.board().hash()))
    })
  }
}

impl Component for WaitingTurn {
  fn render(&self) -> impl Render {
    let local = self.control.snapshot();
    let application = application::use_application_state();
    let active = local.screen == AppScreen::Game && !local.pause_open() && application.is_active();
    let delayed = settings::use_settings().desired.increase_move_duration;
    let (elapsed, set_elapsed) = hooks::use_state(false);
    reactant::use_pausable_timeout(Duration::from_secs(2), !active, move || {
      set_elapsed.set(true);
    });
    let dispatched = hooks::use_ref(false);
    let game = self.game.clone();
    let opponent = self.opponent.clone();
    let control = self.control.clone();
    hooks::use_effect(
      move || {
        if !active || (delayed && !elapsed) {
          return;
        }
        let current = control.current();
        if current.screen != AppScreen::Game || current.pause_open() {
          return;
        }
        if dispatched.get() || !opponent.permitted() {
          return;
        }
        dispatched.replace(true);
        assert_eq!(
          game.dispatch(ChessAction::ComputerMove),
          DispatchResult::Started
        );
      },
      (active, delayed, elapsed, self.opponent.revision()),
    );
  }
}

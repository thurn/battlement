//! Global input normalization into the shared chess UI action vocabulary.

use std::collections::HashSet;

use battlement::{ControllerButton, ControllerInputSettings, DebugUiSurface, PhysicalKey};
use reactant::{Application, GameHandle, GlobalInput};

use crate::{
  chess_ui_state::{ChessUiController, UiAction},
  reactant_game::ChessGame,
};

const GLOBAL_KEYS: [PhysicalKey; 18] = [
  PhysicalKey::ArrowLeft,
  PhysicalKey::ArrowRight,
  PhysicalKey::ArrowUp,
  PhysicalKey::ArrowDown,
  PhysicalKey::Enter,
  PhysicalKey::NumpadEnter,
  PhysicalKey::Space,
  PhysicalKey::Escape,
  PhysicalKey::Minus,
  PhysicalKey::Equal,
  PhysicalKey::KeyR,
  PhysicalKey::ShiftLeft,
  PhysicalKey::ShiftRight,
  PhysicalKey::ControlLeft,
  PhysicalKey::ControlRight,
  PhysicalKey::MetaLeft,
  PhysicalKey::MetaRight,
  PhysicalKey::KeyL,
];

const CONTROLLER_BUTTONS: [ControllerButton; 5] = [
  ControllerButton::South,
  ControllerButton::East,
  ControllerButton::LeftShoulder,
  ControllerButton::RightShoulder,
  ControllerButton::Start,
];

pub(crate) fn configure_application(application: Application) -> Application {
  application.global_keys(GLOBAL_KEYS).controller_input(
    ControllerInputSettings::new()
      .buttons(CONTROLLER_BUTTONS)
      .stick_dead_zone(0.35)
      .repeat_timing_ms(275, 125),
  )
}

pub(crate) fn use_chess_input(control: ChessUiController, game: Option<GameHandle<ChessGame>>) {
  reactant::use_global_input(move |input| match input {
    GlobalInput::KeyDown(key) => key_down(&control, game.as_ref(), key),
    GlobalInput::KeyUp(key) => control.dispatch(game.as_ref(), UiAction::KeyUp(key)),
    GlobalInput::ControllerButtonDown(button) => controller_button(&control, game.as_ref(), button),
    GlobalInput::ControllerNavigate(direction) => {
      if game.is_some() {
        control.dispatch(
          game.as_ref(),
          UiAction::MoveCursor(crate::cursor::moved_in_direction(
            control.current().cursor,
            direction,
          )),
        );
      }
    }
    GlobalInput::ControllerButtonUp(_) => {}
  });
}

fn key_down(control: &ChessUiController, game: Option<&GameHandle<ChessGame>>, key: PhysicalKey) {
  let mut held = control.current().held;
  held.insert(key);
  if restart_shortcut(&held) {
    control.request_restart();
    return;
  }
  control.dispatch(game, UiAction::KeyDown(key));
  match key {
    PhysicalKey::KeyL => control.dispatch(game, UiAction::ShowDebug(DebugUiSurface::LogViewer)),
    PhysicalKey::ArrowLeft
    | PhysicalKey::ArrowRight
    | PhysicalKey::ArrowUp
    | PhysicalKey::ArrowDown
      if game.is_some() =>
    {
      control.dispatch(
        game,
        UiAction::MoveCursor(crate::cursor::moved(control.current().cursor, key)),
      );
    }
    PhysicalKey::Escape if control.current().selected.is_some() => {
      control.dispatch(game, UiAction::CancelSelection);
    }
    PhysicalKey::Escape if game.is_some() => control.dispatch(game, UiAction::TogglePause),
    PhysicalKey::Equal => adjust_volume(control, game, 0.1),
    PhysicalKey::Minus => adjust_volume(control, game, -0.1),
    PhysicalKey::Enter | PhysicalKey::NumpadEnter | PhysicalKey::Space if game.is_none() => {
      control.request_start(true);
    }
    PhysicalKey::Enter | PhysicalKey::NumpadEnter | PhysicalKey::Space => {
      activate_cursor(control, game.expect("active session checked above"));
    }
    _ => {}
  }
}

fn controller_button(
  control: &ChessUiController,
  game: Option<&GameHandle<ChessGame>>,
  button: ControllerButton,
) {
  let local = control.current();
  match button {
    ControllerButton::Start if game.is_some() => control.dispatch(game, UiAction::TogglePause),
    ControllerButton::South if local.pause_open() => control.dispatch(
      game,
      UiAction::RequestNewGame {
        cursor_visible: true,
      },
    ),
    ControllerButton::East if local.confirm_new_game() => {
      control.dispatch(game, UiAction::DismissNewGameConfirmation);
    }
    ControllerButton::East if local.pause_open() => control.dispatch(game, UiAction::TogglePause),
    ControllerButton::LeftShoulder if local.pause_open() => adjust_volume(control, game, -0.1),
    ControllerButton::RightShoulder if local.pause_open() => adjust_volume(control, game, 0.1),
    ControllerButton::LeftShoulder if game.is_some() => {
      control.dispatch(game, UiAction::CycleCursor(false));
    }
    ControllerButton::RightShoulder if game.is_some() => {
      control.dispatch(game, UiAction::CycleCursor(true));
    }
    ControllerButton::South if game.is_some() => {
      activate_cursor(control, game.expect("active game"));
    }
    ControllerButton::East if game.is_some() => control.dispatch(game, UiAction::CancelSelection),
    ControllerButton::South => control.request_start(true),
    _ => {}
  }
}

fn activate_cursor(control: &ChessUiController, game: &GameHandle<ChessGame>) {
  let local = control.current();
  control.dispatch(
    Some(game),
    if local.selected == Some(local.cursor) {
      UiAction::CancelSelection
    } else {
      UiAction::Activate(local.cursor)
    },
  );
}

fn adjust_volume(control: &ChessUiController, game: Option<&GameHandle<ChessGame>>, delta: f64) {
  control.dispatch(
    game,
    UiAction::SetVolume((control.current().volume + delta).clamp(0.0, 1.0)),
  );
}

fn restart_shortcut(held: &HashSet<PhysicalKey>) -> bool {
  held.contains(&PhysicalKey::KeyR)
    && held
      .iter()
      .any(|key| matches!(key, PhysicalKey::ShiftLeft | PhysicalKey::ShiftRight))
    && held.iter().any(|key| {
      matches!(
        key,
        PhysicalKey::ControlLeft
          | PhysicalKey::ControlRight
          | PhysicalKey::MetaLeft
          | PhysicalKey::MetaRight
      )
    })
}

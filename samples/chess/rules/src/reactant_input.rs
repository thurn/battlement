//! Global input normalization into the shared chess UI action vocabulary.

use std::collections::HashSet;

use battlement::{ControllerButton, ControllerInputSettings, DebugUiSurface, PhysicalKey};
use reactant::{Application, GameHandle, GlobalInput};

use crate::cursor;
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

/// Declares the global keys and controller controls the host should forward.
///
/// Reactant opts into global input explicitly. Keeping the allowlist beside its
/// mapping makes host subscription and action normalization easy to review.
pub fn configure_application(application: Application) -> Application {
  application.global_keys(GLOBAL_KEYS).controller_input(
    ControllerInputSettings::new()
      .buttons(CONTROLLER_BUTTONS)
      .stick_dead_zone(0.35)
      .repeat_timing_ms(275, 125),
  )
}

/// Installs one global-input hook and normalizes events into [`UiAction`] values.
///
/// The title screen passes no game handle, while an active session passes one.
/// Sharing this hook ensures keyboard and controller input use the same reducer
/// and legality paths as clicks and drags.
pub fn use_chess_input(control: ChessUiController, game: Option<GameHandle<ChessGame>>) {
  reactant::use_global_input(move |input| match input {
    GlobalInput::Reset => control.dispatch(game.as_ref(), UiAction::ClearHeldKeys),
    GlobalInput::KeyDown(input) => key_down(&control, game.as_ref(), input.key),
    GlobalInput::KeyUp(input) => control.dispatch(game.as_ref(), UiAction::KeyUp(input.key)),
    GlobalInput::ControllerButtonDown(input) => {
      controller_button(&control, game.as_ref(), input.button)
    }
    GlobalInput::ControllerNavigate(input) => {
      if game.is_some() {
        control.dispatch(
          game.as_ref(),
          UiAction::MoveCursor(cursor::moved_in_direction(
            control.current().cursor,
            input.direction,
          )),
        );
      }
    }
    GlobalInput::ControllerButtonUp(_) => {}
  });
}

/// Applies chord detection, then maps one keyboard press to app intent.
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
        UiAction::MoveCursor(cursor::moved(control.current().cursor, key)),
      );
    }
    PhysicalKey::Escape if control.current().selected.is_some() => {
      control.dispatch(game, UiAction::CancelSelection);
    }
    PhysicalKey::Escape if game.is_some() => control.dispatch(game, UiAction::TogglePause),
    PhysicalKey::Equal => adjust_volume(control, game, 0.1),
    PhysicalKey::Minus => adjust_volume(control, game, -0.1),
    PhysicalKey::Enter | PhysicalKey::NumpadEnter | PhysicalKey::Space => {
      if let Some(game) = game {
        activate_cursor(control, game);
      }
    }
    _ => {}
  }
}

/// Maps controller buttons according to the current screen and overlay state.
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
    ControllerButton::South => {}
    _ => {}
  }
}

/// Selects, activates, or cancels the square addressed by the shared cursor.
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

/// Converts a relative input into the reducer's clamped absolute volume action.
fn adjust_volume(control: &ChessUiController, game: Option<&GameHandle<ChessGame>>, delta: f64) {
  control.dispatch(
    game,
    UiAction::SetVolume((control.current().volume + delta).clamp(0.0, 1.0)),
  );
}

/// Recognizes Control/Command + Shift + R independent of left/right modifiers.
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

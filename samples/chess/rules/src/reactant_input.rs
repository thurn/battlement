//! Saved gameplay maps with independent standard menu cancellation.

use battlement::{
  ControllerButton, ControllerInputSettings, ControllerNavigationSource, DebugUiSurface,
  PhysicalKey,
};
use reactant::{Application, GameHandle, GlobalInput, InputSubscription};

use crate::{
  chess_ui_state::{ChessUiController, UiAction},
  cursor,
  reactant_game::ChessGame,
  settings::{
    self, ChessSettings,
    bindings::{self, ControllerBinding, GameplayAction},
  },
};

const MENU_KEYS: [PhysicalKey; 15] = [
  PhysicalKey::Escape,
  PhysicalKey::Minus,
  PhysicalKey::Equal,
  PhysicalKey::KeyR,
  PhysicalKey::ShiftLeft,
  PhysicalKey::ShiftRight,
  PhysicalKey::ControlLeft,
  PhysicalKey::ControlRight,
  PhysicalKey::AltLeft,
  PhysicalKey::AltRight,
  PhysicalKey::MetaLeft,
  PhysicalKey::MetaRight,
  PhysicalKey::KeyL,
  PhysicalKey::Enter,
  PhysicalKey::NumpadEnter,
];

/// Keeps standard menu recovery and developer modifiers independent of gameplay maps.
pub fn configure_application(application: Application) -> Application {
  application.global_keys(MENU_KEYS).controller_input(
    ControllerInputSettings::new()
      .buttons([
        ControllerButton::East,
        ControllerButton::South,
        ControllerButton::LeftShoulder,
        ControllerButton::RightShoulder,
      ])
      .stick_dead_zone(0.35)
      .repeat_timing_ms(275, 125),
  )
}

/// Subscribes to the current saved maps and routes unclaimed physical input.
pub fn use_chess_input(control: ChessUiController, game: Option<GameHandle<ChessGame>>) {
  let settings = settings::use_settings().desired;
  reactant::use_input_subscription(InputSubscription {
    keys: settings.keyboard.values().to_vec(),
    buttons: settings
      .controller
      .values()
      .into_iter()
      .filter_map(ControllerBinding::button)
      .collect(),
    navigation: true,
  });
  reactant::use_global_input(move |input| match input {
    GlobalInput::Reset => control.dispatch(game.as_ref(), UiAction::ClearHeldKeys),
    GlobalInput::KeyDown(input) => self::key_down(&control, game.as_ref(), settings, input.key),
    GlobalInput::KeyUp(input) => control.dispatch(game.as_ref(), UiAction::KeyUp(input.key)),
    GlobalInput::ControllerButtonDown(input) => {
      if input.button == ControllerButton::East {
        self::cancel(&control, game.as_ref());
      } else if let Some(binding) = ControllerBinding::from_button(input.button) {
        if let Some(action) = settings.controller.action(binding) {
          self::perform(&control, game.as_ref(), action);
        } else if control.current().pause_open() {
          match input.button {
            ControllerButton::LeftShoulder => self::adjust_volume(&control, game.as_ref(), -0.1),
            ControllerButton::RightShoulder => self::adjust_volume(&control, game.as_ref(), 0.1),
            _ => {}
          }
        }
      }
    }
    GlobalInput::ControllerNavigate(input) => {
      let action = if input.source == ControllerNavigationSource::LeftStick {
        Some(GameplayAction::from_direction(input.direction))
      } else {
        settings
          .controller
          .action(ControllerBinding::from_direction(input.direction))
      };
      if let Some(action) = action
        && (!input.repeat || action.direction().is_some())
      {
        self::perform(&control, game.as_ref(), action);
      }
    }
    GlobalInput::ControllerButtonUp(_) => {}
  });
}

fn key_down(
  control: &ChessUiController,
  game: Option<&GameHandle<ChessGame>>,
  settings: ChessSettings,
  key: PhysicalKey,
) {
  if control.current().held.contains(&key) {
    return;
  }
  control.dispatch(game, UiAction::KeyDown(key));
  let held = control.current().held;
  if held.iter().any(|key| bindings::is_modifier(*key)) {
    let shift = held
      .iter()
      .any(|key| matches!(key, PhysicalKey::ShiftLeft | PhysicalKey::ShiftRight));
    let command = held.iter().any(|key| {
      matches!(
        key,
        PhysicalKey::ControlLeft
          | PhysicalKey::ControlRight
          | PhysicalKey::MetaLeft
          | PhysicalKey::MetaRight
      )
    });
    if key == PhysicalKey::KeyR && shift && command {
      control.request_restart();
    }
    return;
  }
  if key == PhysicalKey::Escape {
    let local = control.current();
    if local.selected.is_some() || local.pause_open() {
      self::cancel(control, game);
      return;
    }
  }
  if let Some(action) = settings.keyboard.action(key) {
    self::perform(control, game, action);
    return;
  }
  match key {
    PhysicalKey::KeyL => control.dispatch(game, UiAction::ShowDebug(DebugUiSurface::LogViewer)),
    PhysicalKey::Equal => self::adjust_volume(control, game, 0.1),
    PhysicalKey::Minus => self::adjust_volume(control, game, -0.1),
    _ => {}
  }
}

fn cancel(control: &ChessUiController, game: Option<&GameHandle<ChessGame>>) {
  let local = control.current();
  if local.confirm_new_game() {
    control.dispatch(game, UiAction::DismissNewGameConfirmation);
  } else if local.pause_open() {
    control.dispatch(game, UiAction::TogglePause);
  } else if local.selected.is_some() {
    control.dispatch(game, UiAction::CancelSelection);
  }
}

fn perform(
  control: &ChessUiController,
  game: Option<&GameHandle<ChessGame>>,
  action: GameplayAction,
) {
  let Some(game) = game else {
    return;
  };
  if action == GameplayAction::Pause {
    control.dispatch(Some(game), UiAction::TogglePause);
    return;
  }
  let local = control.current();
  if local.pause_open() || local.spawning {
    return;
  }
  if let Some(direction) = action.direction() {
    control.dispatch(
      Some(game),
      UiAction::MoveCursor(cursor::moved_in_direction(local.cursor, direction)),
    );
  } else {
    match action {
      GameplayAction::MovePiece => control.dispatch(
        Some(game),
        if local.selected == Some(local.cursor) {
          UiAction::CancelSelection
        } else {
          UiAction::Activate(local.cursor)
        },
      ),
      GameplayAction::Restart => control.request_restart(),
      _ => unreachable!("directional and pause actions handled above"),
    }
  }
}

fn adjust_volume(control: &ChessUiController, game: Option<&GameHandle<ChessGame>>, delta: f64) {
  control.dispatch(
    game,
    UiAction::SetVolume((control.current().volume + delta).clamp(0.0, 1.0)),
  );
}

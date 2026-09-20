//! Native input normalization into the shared chess UI action vocabulary.

use std::{cell::RefCell, collections::HashSet};

use battlement::{
  ControllerButton, ControllerInputSettings, DebugUiSurface, ObjectId, PhysicalKey, Vector3,
};
use battlement_native::CoreActionBodyView;
use reactant::{GameConsumer, GameHandle, app::App};

use crate::{
  chess_ui_state::{AppControl, ChessUiState, UiAction},
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

/// App-owned handles consumed by the rendered component tree and native input.
pub(crate) struct ChessModel {
  pub(crate) control: AppControl,
  pub(crate) consumer: RefCell<Option<GameConsumer<ChessGame>>>,
  pub(crate) game: RefCell<Option<GameHandle<ChessGame>>>,
}

impl ChessModel {
  pub(crate) fn new(local: ChessUiState) -> Self {
    Self {
      control: AppControl::new(local),
      consumer: RefCell::new(None),
      game: RefCell::new(None),
    }
  }

  pub(crate) fn game(&self) -> Option<GameHandle<ChessGame>> {
    self.game.borrow().clone()
  }

  fn handle_core(&mut self, body: CoreActionBodyView<'_>) {
    let game = self.game();
    match body {
      CoreActionBodyView::KeyDown(value) => self.key_down(game.as_ref(), value.physical_key()),
      CoreActionBodyView::KeyUp(value) => {
        self
          .control
          .dispatch(game.as_ref(), UiAction::KeyUp(value.physical_key()));
      }
      CoreActionBodyView::ControllerButtonDown(value) => {
        self.controller_button(game.as_ref(), value.controller_button());
      }
      CoreActionBodyView::ControllerNavigate(value) if game.is_some() => {
        self.control.dispatch(
          game.as_ref(),
          UiAction::MoveCursor(crate::cursor::moved_in_direction(
            self.control.snapshot().cursor,
            value.controller_direction(),
          )),
        );
      }
      CoreActionBodyView::DragStart(value) if game.is_some() => {
        let piece = ObjectId::from_bytes(value.object_id()).expect("validated drag object");
        self
          .control
          .dispatch(game.as_ref(), UiAction::BeginDrag(piece));
      }
      CoreActionBodyView::DragEnd(value) if game.is_some() => {
        let piece = ObjectId::from_bytes(value.object_id()).expect("validated drag object");
        let [x, y, z] = value.world_position();
        self.control.dispatch(
          game.as_ref(),
          UiAction::EndDrag(piece, crate::square_at(Vector3::new(x, y, z))),
        );
      }
      _ => {}
    }
  }

  fn key_down(&self, game: Option<&GameHandle<ChessGame>>, key: PhysicalKey) {
    self.control.dispatch(game, UiAction::KeyDown(key));
    let held = self.control.snapshot().held;
    if restart_shortcut(&held) {
      self.control.request_restart();
      return;
    }
    match key {
      PhysicalKey::KeyL => self
        .control
        .dispatch(game, UiAction::ShowDebug(DebugUiSurface::LogViewer)),
      PhysicalKey::ArrowLeft
      | PhysicalKey::ArrowRight
      | PhysicalKey::ArrowUp
      | PhysicalKey::ArrowDown
        if game.is_some() =>
      {
        self.control.dispatch(
          game,
          UiAction::MoveCursor(crate::cursor::moved(self.control.snapshot().cursor, key)),
        );
      }
      PhysicalKey::Escape if self.control.snapshot().selected.is_some() => {
        self.control.dispatch(game, UiAction::CancelSelection);
      }
      PhysicalKey::Escape if game.is_some() => {
        self.control.dispatch(game, UiAction::TogglePause);
      }
      PhysicalKey::Equal => self.adjust_volume(game, 0.1),
      PhysicalKey::Minus => self.adjust_volume(game, -0.1),
      PhysicalKey::Enter | PhysicalKey::NumpadEnter | PhysicalKey::Space if game.is_none() => {
        self.control.request_start(true);
      }
      PhysicalKey::Enter | PhysicalKey::NumpadEnter | PhysicalKey::Space => {
        self.activate_cursor(game.expect("active session checked above"));
      }
      _ => {}
    }
  }

  fn controller_button(&self, game: Option<&GameHandle<ChessGame>>, button: ControllerButton) {
    let local = self.control.snapshot();
    match button {
      ControllerButton::Start if game.is_some() => {
        self.control.dispatch(game, UiAction::TogglePause);
      }
      ControllerButton::South if local.pause_open() => self.control.dispatch(
        game,
        UiAction::RequestNewGame {
          cursor_visible: true,
        },
      ),
      ControllerButton::East if local.confirm_new_game() => self
        .control
        .dispatch(game, UiAction::DismissNewGameConfirmation),
      ControllerButton::East if local.pause_open() => {
        self.control.dispatch(game, UiAction::TogglePause);
      }
      ControllerButton::LeftShoulder if local.pause_open() => self.adjust_volume(game, -0.1),
      ControllerButton::RightShoulder if local.pause_open() => self.adjust_volume(game, 0.1),
      ControllerButton::LeftShoulder if game.is_some() => {
        self.control.dispatch(game, UiAction::CycleCursor(false));
      }
      ControllerButton::RightShoulder if game.is_some() => {
        self.control.dispatch(game, UiAction::CycleCursor(true));
      }
      ControllerButton::South if game.is_some() => self.activate_cursor(game.unwrap()),
      ControllerButton::East if game.is_some() => {
        self.control.dispatch(game, UiAction::CancelSelection);
      }
      ControllerButton::South => {
        self.control.request_start(true);
      }
      _ => {}
    }
  }

  fn activate_cursor(&self, game: &GameHandle<ChessGame>) {
    let local = self.control.snapshot();
    self.control.dispatch(
      Some(game),
      if local.selected == Some(local.cursor) {
        UiAction::CancelSelection
      } else {
        UiAction::Activate(local.cursor)
      },
    );
  }

  fn adjust_volume(&self, game: Option<&GameHandle<ChessGame>>, delta: f64) {
    self.control.dispatch(
      game,
      UiAction::SetVolume((self.control.snapshot().volume + delta).clamp(0.0, 1.0)),
    );
  }
}

pub(crate) fn configure_app(app: App<ChessModel>) -> App<ChessModel> {
  app
    .global_keys(GLOBAL_KEYS)
    .controller_input(
      ControllerInputSettings::new()
        .buttons(CONTROLLER_BUTTONS)
        .stick_dead_zone(0.35)
        .repeat_timing_ms(275, 125),
    )
    .on_core_action(|model, body| model.handle_core(body))
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

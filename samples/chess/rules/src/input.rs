use std::collections::HashSet;

use battlement::{
  ActionBody, ClientMessage, Command, CommandBody, ControllerButton, CoreErrorCode, DebugUiPayload,
  DebugUiSurface, PhysicalKey, PointerButton, Response,
};
use battlement_native::EngineError;
use tracing::info;

use crate::{
  ChessEngine, MUSIC_VOLUME_STEP, PLAY_BUTTON_ID, REFRESH_BUTTON_ID, audio,
  visual_state::VisualState,
};

pub(crate) struct RestartShortcut {
  held: HashSet<PhysicalKey>,
}

impl RestartShortcut {
  pub(crate) fn new() -> Self {
    Self {
      held: HashSet::new(),
    }
  }

  pub(crate) fn observe(&mut self, body: &ActionBody) -> bool {
    match body {
      ActionBody::KeyDown(payload) => {
        self.held.insert(payload.key);
        self.held.contains(&PhysicalKey::KeyR) && self.shift_held() && self.primary_modifier_held()
      }
      ActionBody::KeyUp(payload) => {
        self.held.remove(&payload.key);
        false
      }
      _ => false,
    }
  }

  pub(crate) fn reset(&mut self) {
    self.held.clear();
  }

  fn shift_held(&self) -> bool {
    self
      .held
      .iter()
      .any(|key| matches!(key, PhysicalKey::ShiftLeft | PhysicalKey::ShiftRight))
  }

  fn primary_modifier_held(&self) -> bool {
    self.held.iter().any(|key| {
      matches!(
        key,
        PhysicalKey::ControlLeft
          | PhysicalKey::ControlRight
          | PhysicalKey::MetaLeft
          | PhysicalKey::MetaRight
      )
    })
  }
}

impl ChessEngine {
  pub(crate) fn submit_message(
    &mut self,
    message: ClientMessage<(), CoreErrorCode>,
  ) -> Result<Response<Command>, EngineError> {
    let empty = Response::empty(self.session_id);
    let Some(action) = message.into_action() else {
      return Ok(empty);
    };
    if self.restart_shortcut.observe(&action.body) {
      self.restart_shortcut.reset();
      return self.restart_game(action.action_id, true);
    }
    match action.body {
      ActionBody::KeyDown(payload) if payload.key == PhysicalKey::KeyL => {
        info!("Chess log viewer opened");
        Ok(audio::response_for_action(
          self.session_id,
          action.action_id,
          [CommandBody::DebugUi(DebugUiPayload {
            surface: DebugUiSurface::LogViewer,
            visible: true,
          })],
        ))
      }
      ActionBody::PointerClick(payload)
        if payload.object_id == PLAY_BUTTON_ID && payload.button == PointerButton::Left =>
      {
        self.start_game(action.action_id, false)
      }
      ActionBody::PointerClick(payload)
        if payload.object_id == REFRESH_BUTTON_ID
          && payload.button == PointerButton::Left
          && self.pause_open =>
      {
        self.confirm_or_start_new_game(action.action_id, false)
      }
      ActionBody::PointerClick(payload) if payload.button == PointerButton::Left => {
        self.submit_click(action.action_id, payload.object_id)
      }
      ActionBody::DragEnd(payload) => {
        self.submit_drag(action.action_id, payload.object_id, payload.world_position)
      }
      ActionBody::DragStart(payload) => {
        let Some(square) = crate::find_square(&self.objects, payload.object_id) else {
          return Ok(empty);
        };
        self.cursor = square;
        self.selected = None;
        let state_commands = self.set_visual_state(VisualState::Selected);
        let commands = self
          .hide_highlight_commands()
          .into_iter()
          .chain(self.cursor_commands(square, false))
          .chain(self.highlight_commands(payload.object_id))
          .chain(state_commands)
          .collect::<Vec<_>>();
        Ok(audio::response_for_action(
          self.session_id,
          action.action_id,
          commands,
        ))
      }
      ActionBody::KeyDown(payload)
        if !self.started
          && matches!(
            payload.key,
            PhysicalKey::Enter | PhysicalKey::NumpadEnter | PhysicalKey::Space
          ) =>
      {
        self.start_game(action.action_id, true)
      }
      ActionBody::KeyDown(payload)
        if self.started
          && matches!(
            payload.key,
            PhysicalKey::ArrowLeft
              | PhysicalKey::ArrowRight
              | PhysicalKey::ArrowUp
              | PhysicalKey::ArrowDown
          ) =>
      {
        self.move_cursor(action.action_id, payload.key)
      }
      ActionBody::KeyDown(payload)
        if self.started
          && matches!(
            payload.key,
            PhysicalKey::Enter | PhysicalKey::NumpadEnter | PhysicalKey::Space
          ) =>
      {
        self.activate_cursor(action.action_id)
      }
      ActionBody::KeyDown(payload)
        if self.started && payload.key == PhysicalKey::Escape && self.selected.is_some() =>
      {
        self.cancel_selection(action.action_id)
      }
      ActionBody::KeyDown(payload) if self.started && payload.key == PhysicalKey::Escape => {
        self.toggle_pause(action.action_id)
      }
      ActionBody::KeyDown(payload) if payload.key == PhysicalKey::Equal => {
        self.adjust_music(action.action_id, MUSIC_VOLUME_STEP)
      }
      ActionBody::KeyDown(payload) if payload.key == PhysicalKey::Minus => {
        self.adjust_music(action.action_id, -MUSIC_VOLUME_STEP)
      }
      ActionBody::ControllerButtonDown(payload)
        if payload.button == ControllerButton::South && !self.started =>
      {
        self.start_game(action.action_id, true)
      }
      ActionBody::ControllerButtonDown(payload)
        if payload.button == ControllerButton::Start && self.started =>
      {
        self.toggle_pause(action.action_id)
      }
      ActionBody::ControllerButtonDown(payload) if self.pause_open => {
        self.handle_pause_button(action.action_id, payload.button)
      }
      ActionBody::ControllerButtonDown(payload)
        if payload.button == ControllerButton::South && self.started =>
      {
        self.activate_cursor(action.action_id)
      }
      ActionBody::ControllerButtonDown(payload)
        if payload.button == ControllerButton::East && self.started =>
      {
        self.cancel_selection(action.action_id)
      }
      ActionBody::ControllerButtonDown(payload)
        if payload.button == ControllerButton::LeftShoulder && self.started =>
      {
        self.cycle_cursor(action.action_id, false)
      }
      ActionBody::ControllerButtonDown(payload)
        if payload.button == ControllerButton::RightShoulder && self.started =>
      {
        self.cycle_cursor(action.action_id, true)
      }
      ActionBody::ControllerNavigate(payload) if self.started && !self.pause_open => {
        self.move_cursor_direction(action.action_id, payload.direction)
      }
      _ => Ok(empty),
    }
  }
}

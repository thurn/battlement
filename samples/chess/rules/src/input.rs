use std::collections::HashSet;

use battlement::{
  ActionBody, ActionId, ClientMessage, Command, CommandBody, ControllerButton, ControllerDirection,
  CoreErrorCode, DebugUiPayload, DebugUiSurface, ObjectId, PhysicalKey, PointerButton, Response,
  Vector3,
};
use battlement_native::{CoreActionBodyView, EngineError, NativeResponse};
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

  fn observe(&mut self, input: &ChessInput) -> bool {
    match input {
      ChessInput::KeyDown(key) => {
        self.held.insert(*key);
        self.held.contains(&PhysicalKey::KeyR) && self.shift_held() && self.primary_modifier_held()
      }
      ChessInput::KeyUp(key) => {
        self.held.remove(key);
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

enum ChessInput {
  KeyDown(PhysicalKey),
  KeyUp(PhysicalKey),
  PointerClick(ObjectId, PointerButton),
  DragStart(ObjectId),
  DragEnd(ObjectId, Vector3),
  ControllerButtonDown(ControllerButton),
  ControllerNavigate(ControllerDirection),
  Other,
}

impl ChessInput {
  fn from_owned(body: ActionBody) -> Self {
    match body {
      ActionBody::KeyDown(value) => Self::KeyDown(value.key),
      ActionBody::KeyUp(value) => Self::KeyUp(value.key),
      ActionBody::PointerClick(value) => Self::PointerClick(value.object_id, value.button),
      ActionBody::DragStart(value) => Self::DragStart(value.object_id),
      ActionBody::DragEnd(value) => Self::DragEnd(value.object_id, value.world_position),
      ActionBody::ControllerButtonDown(value) => Self::ControllerButtonDown(value.button),
      ActionBody::ControllerNavigate(value) => Self::ControllerNavigate(value.direction),
      _ => Self::Other,
    }
  }

  fn from_view(body: CoreActionBodyView<'_>) -> Self {
    match body {
      CoreActionBodyView::KeyDown(value) => Self::KeyDown(value.physical_key()),
      CoreActionBodyView::KeyUp(value) => Self::KeyUp(value.physical_key()),
      CoreActionBodyView::PointerClick(value) => Self::PointerClick(
        ObjectId::from_bytes(value.object_id()).expect("validated object UUID"),
        value.pointer_button(),
      ),
      CoreActionBodyView::DragStart(value) => {
        Self::DragStart(ObjectId::from_bytes(value.object_id()).expect("validated object UUID"))
      }
      CoreActionBodyView::DragEnd(value) => {
        let [x, y, z] = value.world_position();
        Self::DragEnd(
          ObjectId::from_bytes(value.object_id()).expect("validated object UUID"),
          Vector3::new(x, y, z),
        )
      }
      CoreActionBodyView::ControllerButtonDown(value) => {
        Self::ControllerButtonDown(value.controller_button())
      }
      CoreActionBodyView::ControllerNavigate(value) => {
        Self::ControllerNavigate(value.controller_direction())
      }
      _ => Self::Other,
    }
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
    self.submit_input(action.action_id, ChessInput::from_owned(action.body))
  }

  pub(crate) fn submit_action_view(
    &mut self,
    action_id: ActionId,
    body: CoreActionBodyView<'_>,
  ) -> Result<Response<Command>, EngineError> {
    self.submit_input(action_id, ChessInput::from_view(body))
  }

  pub(crate) fn submit_action_view_native(
    &mut self,
    action_id: ActionId,
    body: CoreActionBodyView<'_>,
  ) -> Result<NativeResponse, EngineError> {
    let input = ChessInput::from_view(body);
    if self.restart_shortcut.observe(&input) {
      self.restart_shortcut.reset();
      return crate::native::restart_game(self, action_id, true);
    }
    match input {
      ChessInput::KeyDown(PhysicalKey::KeyL) => {
        info!("Chess log viewer opened");
        crate::native::show_log_viewer(self.session_id, action_id)
      }
      ChessInput::KeyDown(key)
        if self.started
          && matches!(
            key,
            PhysicalKey::ArrowLeft
              | PhysicalKey::ArrowRight
              | PhysicalKey::ArrowUp
              | PhysicalKey::ArrowDown
          ) =>
      {
        self.move_cursor_native(action_id, crate::cursor::moved(self.cursor, key))
      }
      ChessInput::KeyDown(PhysicalKey::Escape) if self.started && self.selected.is_some() => {
        self.cancel_selection_native(action_id)
      }
      ChessInput::KeyDown(PhysicalKey::Escape) if self.started => {
        self.toggle_pause_native(action_id)
      }
      ChessInput::KeyDown(PhysicalKey::Equal) => {
        self.adjust_music_native(action_id, MUSIC_VOLUME_STEP)
      }
      ChessInput::KeyDown(PhysicalKey::Minus) => {
        self.adjust_music_native(action_id, -MUSIC_VOLUME_STEP)
      }
      ChessInput::ControllerButtonDown(ControllerButton::Start) if self.started => {
        self.toggle_pause_native(action_id)
      }
      ChessInput::ControllerButtonDown(ControllerButton::South) if self.pause_open => {
        self.confirm_or_start_new_game_native(action_id, true)
      }
      ChessInput::ControllerButtonDown(ControllerButton::East)
        if self.pause_open && self.confirm_new_game =>
      {
        self.confirm_new_game = false;
        NativeResponse::empty(*self.session_id.as_uuid().as_bytes())
      }
      ChessInput::ControllerButtonDown(ControllerButton::East) if self.pause_open => {
        self.toggle_pause_native(action_id)
      }
      ChessInput::ControllerButtonDown(ControllerButton::LeftShoulder) if self.pause_open => {
        self.adjust_music_native(action_id, -MUSIC_VOLUME_STEP)
      }
      ChessInput::ControllerButtonDown(ControllerButton::RightShoulder) if self.pause_open => {
        self.adjust_music_native(action_id, MUSIC_VOLUME_STEP)
      }
      ChessInput::ControllerButtonDown(ControllerButton::LeftShoulder)
        if self.started && !self.pause_open =>
      {
        self.cycle_cursor_native(action_id, false)
      }
      ChessInput::ControllerButtonDown(ControllerButton::RightShoulder)
        if self.started && !self.pause_open =>
      {
        self.cycle_cursor_native(action_id, true)
      }
      ChessInput::ControllerNavigate(direction) if self.started && !self.pause_open => self
        .move_cursor_native(
          action_id,
          crate::cursor::moved_in_direction(self.cursor, direction),
        ),
      ChessInput::PointerClick(object_id, PointerButton::Left)
        if object_id != PLAY_BUTTON_ID && object_id != REFRESH_BUTTON_ID =>
      {
        self.submit_click_native(action_id, object_id)
      }
      ChessInput::DragStart(object_id) => self.drag_start_native(action_id, object_id),
      ChessInput::DragEnd(object_id, world_position) => {
        self.submit_drag_native(action_id, object_id, world_position)
      }
      ChessInput::ControllerButtonDown(ControllerButton::South) if self.started => {
        self.activate_cursor_native(action_id)
      }
      ChessInput::ControllerButtonDown(ControllerButton::East) if self.started => {
        self.cancel_selection_native(action_id)
      }
      ChessInput::PointerClick(object_id, PointerButton::Left) if object_id == PLAY_BUTTON_ID => {
        crate::native::start_game(self, action_id, false)
      }
      ChessInput::PointerClick(object_id, PointerButton::Left)
        if object_id == REFRESH_BUTTON_ID && self.pause_open =>
      {
        self.confirm_or_start_new_game_native(action_id, false)
      }
      ChessInput::KeyDown(key)
        if !self.started
          && matches!(
            key,
            PhysicalKey::Enter | PhysicalKey::NumpadEnter | PhysicalKey::Space
          ) =>
      {
        crate::native::start_game(self, action_id, true)
      }
      ChessInput::ControllerButtonDown(ControllerButton::South) if !self.started => {
        crate::native::start_game(self, action_id, true)
      }
      ChessInput::KeyDown(key)
        if self.started
          && matches!(
            key,
            PhysicalKey::Enter | PhysicalKey::NumpadEnter | PhysicalKey::Space
          ) =>
      {
        self.activate_cursor_native(action_id)
      }
      ChessInput::KeyUp(_) | ChessInput::Other => {
        NativeResponse::empty(*self.session_id.as_uuid().as_bytes())
      }
      _ => NativeResponse::empty(*self.session_id.as_uuid().as_bytes()),
    }
  }

  fn submit_input(
    &mut self,
    action_id: ActionId,
    input: ChessInput,
  ) -> Result<Response<Command>, EngineError> {
    if self.restart_shortcut.observe(&input) {
      self.restart_shortcut.reset();
      return self.restart_game(action_id, true);
    }
    self.submit_observed_input(action_id, input)
  }

  fn submit_observed_input(
    &mut self,
    action_id: ActionId,
    input: ChessInput,
  ) -> Result<Response<Command>, EngineError> {
    let empty = Response::empty(self.session_id);
    match input {
      ChessInput::KeyDown(PhysicalKey::KeyL) => {
        info!("Chess log viewer opened");
        Ok(audio::response_for_action(
          self.session_id,
          action_id,
          [CommandBody::DebugUi(DebugUiPayload {
            surface: DebugUiSurface::LogViewer,
            visible: true,
          })],
        ))
      }
      ChessInput::PointerClick(object_id, button)
        if object_id == PLAY_BUTTON_ID && button == PointerButton::Left =>
      {
        self.start_game(action_id, false)
      }
      ChessInput::PointerClick(object_id, button)
        if object_id == REFRESH_BUTTON_ID && button == PointerButton::Left && self.pause_open =>
      {
        self.confirm_or_start_new_game(action_id, false)
      }
      ChessInput::PointerClick(object_id, PointerButton::Left) => {
        self.submit_click(action_id, object_id)
      }
      ChessInput::DragEnd(object_id, world_position) => {
        self.submit_drag(action_id, object_id, world_position)
      }
      ChessInput::DragStart(object_id) => {
        let Some(square) = crate::find_square(&self.objects, object_id) else {
          return Ok(empty);
        };
        self.cursor = square;
        self.selected = None;
        let state_commands = self.set_visual_state(VisualState::Selected);
        let commands = self
          .hide_highlight_commands()
          .into_iter()
          .chain(self.cursor_commands(square, false))
          .chain(self.highlight_commands(object_id))
          .chain(state_commands)
          .collect::<Vec<_>>();
        Ok(audio::response_for_action(
          self.session_id,
          action_id,
          commands,
        ))
      }
      ChessInput::KeyDown(key)
        if !self.started
          && matches!(
            key,
            PhysicalKey::Enter | PhysicalKey::NumpadEnter | PhysicalKey::Space
          ) =>
      {
        self.start_game(action_id, true)
      }
      ChessInput::KeyDown(key)
        if self.started
          && matches!(
            key,
            PhysicalKey::ArrowLeft
              | PhysicalKey::ArrowRight
              | PhysicalKey::ArrowUp
              | PhysicalKey::ArrowDown
          ) =>
      {
        self.move_cursor(action_id, key)
      }
      ChessInput::KeyDown(key)
        if self.started
          && matches!(
            key,
            PhysicalKey::Enter | PhysicalKey::NumpadEnter | PhysicalKey::Space
          ) =>
      {
        self.activate_cursor(action_id)
      }
      ChessInput::KeyDown(key)
        if self.started && key == PhysicalKey::Escape && self.selected.is_some() =>
      {
        self.cancel_selection(action_id)
      }
      ChessInput::KeyDown(key) if self.started && key == PhysicalKey::Escape => {
        self.toggle_pause(action_id)
      }
      ChessInput::KeyDown(PhysicalKey::Equal) => self.adjust_music(action_id, MUSIC_VOLUME_STEP),
      ChessInput::KeyDown(PhysicalKey::Minus) => self.adjust_music(action_id, -MUSIC_VOLUME_STEP),
      ChessInput::ControllerButtonDown(button)
        if button == ControllerButton::South && !self.started =>
      {
        self.start_game(action_id, true)
      }
      ChessInput::ControllerButtonDown(button)
        if button == ControllerButton::Start && self.started =>
      {
        self.toggle_pause(action_id)
      }
      ChessInput::ControllerButtonDown(button) if self.pause_open => {
        self.handle_pause_button(action_id, button)
      }
      ChessInput::ControllerButtonDown(button)
        if button == ControllerButton::South && self.started =>
      {
        self.activate_cursor(action_id)
      }
      ChessInput::ControllerButtonDown(button)
        if button == ControllerButton::East && self.started =>
      {
        self.cancel_selection(action_id)
      }
      ChessInput::ControllerButtonDown(button)
        if button == ControllerButton::LeftShoulder && self.started =>
      {
        self.cycle_cursor(action_id, false)
      }
      ChessInput::ControllerButtonDown(button)
        if button == ControllerButton::RightShoulder && self.started =>
      {
        self.cycle_cursor(action_id, true)
      }
      ChessInput::ControllerNavigate(direction) if self.started && !self.pause_open => {
        self.move_cursor_direction(action_id, direction)
      }
      _ => Ok(empty),
    }
  }

  fn move_cursor_native(
    &mut self,
    action_id: ActionId,
    square: cozy_chess::Square,
  ) -> Result<NativeResponse, EngineError> {
    self.cursor_visible = true;
    self.cursor = square;
    crate::native::action_response(self.session_id, action_id, |message| {
      crate::native::write_cursor(message, square, false, true)
    })
  }

  fn cycle_cursor_native(
    &mut self,
    action_id: ActionId,
    forward: bool,
  ) -> Result<NativeResponse, EngineError> {
    let candidates = self.controller_cycle_squares();
    if candidates.is_empty() {
      return NativeResponse::empty(*self.session_id.as_uuid().as_bytes());
    }
    let current = candidates.iter().position(|&square| square == self.cursor);
    let index = match (current, forward) {
      (Some(index), true) => (index + 1) % candidates.len(),
      (Some(0), false) | (None, false) => candidates.len() - 1,
      (Some(index), false) => index - 1,
      (None, true) => 0,
    };
    self.move_cursor_native(action_id, candidates[index])
  }

  fn cancel_selection_native(
    &mut self,
    action_id: ActionId,
  ) -> Result<NativeResponse, EngineError> {
    self.cursor_visible = true;
    if let Some(selected) = self.selected.take() {
      self.cursor = selected;
    }
    let cursor = self.cursor;
    let highlight_ids = self.highlight_ids;
    let previous = self.change_visual_state(VisualState::Initial);
    crate::native::action_response(self.session_id, action_id, move |message| {
      let mut commands = crate::native::write_hidden_highlights(message, &highlight_ids)?;
      commands.extend(crate::native::write_cursor(message, cursor, false, true)?);
      commands.extend(crate::visual_state::write_transition(
        message,
        previous,
        VisualState::Initial,
      )?);
      Ok(commands)
    })
  }

  fn toggle_pause_native(&mut self, action_id: ActionId) -> Result<NativeResponse, EngineError> {
    self.pause_open = !self.pause_open;
    self.confirm_new_game = false;
    let next = if self.pause_open {
      VisualState::Paused
    } else {
      VisualState::Initial
    };
    let previous = self.change_visual_state(next);
    let pause_open = self.pause_open;
    info!(open = pause_open, "Chess pause menu changed");
    crate::native::action_response(self.session_id, action_id, move |message| {
      let mut commands = vec![message.set_object_active(
        *battlement::CommandId::new_v4().as_uuid().as_bytes(),
        true,
        *REFRESH_BUTTON_ID.as_uuid().as_bytes(),
        pause_open,
      )?];
      commands.extend(crate::visual_state::write_transition(
        message, previous, next,
      )?);
      Ok(commands)
    })
  }

  fn adjust_music_native(
    &mut self,
    action_id: ActionId,
    delta: f64,
  ) -> Result<NativeResponse, EngineError> {
    let previous_volume = self.music.volume();
    let target = self.music.set_volume_target(previous_volume + delta);
    let volume = self.music.volume();
    info!(previous_volume, volume, "Chess music volume changed");
    let sound = if delta > 0.0 {
      crate::VOLUME_UP_SOUND
    } else {
      crate::VOLUME_DOWN_SOUND
    };
    crate::native::action_response(self.session_id, action_id, move |message| {
      let mut commands = Vec::with_capacity(2);
      if let Some((audio_command_id, volume)) = target {
        commands.push(message.set_audio_volume(
          *battlement::CommandId::new_v4().as_uuid().as_bytes(),
          true,
          *audio_command_id.as_uuid().as_bytes(),
          volume,
        )?);
      }
      commands.push(crate::native::write_sound(message, sound.as_str())?);
      Ok(commands)
    })
  }

  fn confirm_or_start_new_game_native(
    &mut self,
    action_id: ActionId,
    cursor_visible: bool,
  ) -> Result<NativeResponse, EngineError> {
    if self.confirm_new_game {
      return crate::native::new_game(self, action_id, cursor_visible);
    }
    self.confirm_new_game = true;
    info!("New chess game confirmation requested");
    crate::native::action_response(self.session_id, action_id, |message| {
      Ok(vec![crate::native::write_sound(
        message,
        crate::INVALID_DROP_SOUND.as_str(),
      )?])
    })
  }

  fn drag_start_native(
    &mut self,
    action_id: ActionId,
    object_id: ObjectId,
  ) -> Result<NativeResponse, EngineError> {
    let Some(square) = crate::find_square(&self.objects, object_id) else {
      return NativeResponse::empty(*self.session_id.as_uuid().as_bytes());
    };
    self.cursor = square;
    self.selected = None;
    self.select_piece_native(action_id, square, object_id)
  }

  fn submit_drag_native(
    &mut self,
    action_id: ActionId,
    object_id: ObjectId,
    world_position: Vector3,
  ) -> Result<NativeResponse, EngineError> {
    let Some(from) = crate::find_square(&self.objects, object_id) else {
      let highlight_ids = self.highlight_ids;
      return crate::native::action_response(self.session_id, action_id, move |message| {
        let mut commands = crate::native::write_hidden_highlights(message, &highlight_ids)?;
        commands.push(crate::native::write_sound(
          message,
          crate::INVALID_DROP_SOUND.as_str(),
        )?);
        Ok(commands)
      });
    };
    let target = crate::square_at(world_position);
    if self.board.side_to_move() != cozy_chess::Color::White
      || self.board.status() != cozy_chess::GameStatus::Ongoing
    {
      self.cursor = from;
      self.selected = None;
      let highlight_ids = self.highlight_ids;
      let visible = self.cursor_visible;
      return crate::native::action_response(self.session_id, action_id, move |message| {
        let mut commands = vec![crate::native::write_move(message, object_id, from, false)?];
        commands.extend(crate::native::write_hidden_highlights(
          message,
          &highlight_ids,
        )?);
        commands.extend(crate::native::write_cursor(message, from, false, visible)?);
        Ok(commands)
      });
    }
    if target == from {
      self.cursor = from;
      self.selected = Some(from);
      let visible = self.cursor_visible;
      let previous = self.change_visual_state(VisualState::Selected);
      return crate::native::action_response(self.session_id, action_id, move |message| {
        let mut commands = vec![crate::native::write_move(message, object_id, from, false)?];
        commands.extend(crate::native::write_cursor(message, from, true, visible)?);
        commands.extend(crate::visual_state::write_transition(
          message,
          previous,
          VisualState::Selected,
        )?);
        Ok(commands)
      });
    }
    self.selected = None;
    let highlight_ids = self.highlight_ids;
    let Some(movement) = crate::player_move(&self.board, from, target) else {
      self.cursor = from;
      let visible = self.cursor_visible;
      return crate::native::action_response(self.session_id, action_id, move |message| {
        let mut commands = crate::native::write_hidden_highlights(message, &highlight_ids)?;
        commands.push(crate::native::write_move(message, object_id, from, false)?);
        commands.push(crate::native::write_sound(
          message,
          crate::INVALID_DROP_SOUND.as_str(),
        )?);
        commands.extend(crate::native::write_cursor(message, from, false, visible)?);
        Ok(commands)
      });
    };

    self.cursor = target;
    let mut board_after = self.board.clone();
    board_after.play_unchecked(movement);
    let metadata = if self.diagnostics_enabled {
      match board_after.status() {
        cozy_chess::GameStatus::Won => vec![("chess.game_status", "won")],
        cozy_chess::GameStatus::Drawn => vec![("chess.game_status", "drawn")],
        cozy_chess::GameStatus::Ongoing => Vec::new(),
      }
    } else {
      Vec::new()
    };
    let visible = self.cursor_visible;
    let session_id = self.session_id;
    let response = crate::native::batch_response_with_metadata(
      session_id,
      Some(action_id),
      battlement_native::NativeBatchStart::Now,
      &metadata,
      |message| {
        let groups = crate::native::apply_move(self, message, movement, false)?;
        let mut commands = crate::native::write_hidden_highlights(message, &highlight_ids)
          .map_err(crate::native::protocol)?;
        commands.extend(groups.into_iter().flatten());
        commands.extend(
          crate::native::write_cursor(message, target, false, visible)
            .map_err(crate::native::protocol)?,
        );
        if self.board.status() == cozy_chess::GameStatus::Ongoing {
          commands.push(
            message
              .set_local_scale(
                *battlement::CommandId::new_v4().as_uuid().as_bytes(),
                true,
                *crate::cursor::EFFECT_ID.as_uuid().as_bytes(),
                [0.55, 0.55, 0.55],
              )
              .map_err(crate::native::protocol)?,
          );
          commands.push(
            message
              .set_input_enabled(
                *battlement::CommandId::new_v4().as_uuid().as_bytes(),
                true,
                false,
              )
              .map_err(crate::native::protocol)?,
          );
        }
        Ok(vec![commands])
      },
    )?;
    if self.board.status() == cozy_chess::GameStatus::Ongoing {
      self.start_ai();
    }
    Ok(response)
  }

  fn submit_click_native(
    &mut self,
    action_id: ActionId,
    object_id: ObjectId,
  ) -> Result<NativeResponse, EngineError> {
    if let Some(target) = crate::find_highlight(&self.highlight_ids, object_id) {
      self.cursor = target;
      return self.submit_selected_move_native(action_id, target);
    }
    let Some(square) = crate::find_square(&self.objects, object_id) else {
      return NativeResponse::empty(*self.session_id.as_uuid().as_bytes());
    };
    self.cursor = square;
    if self.board.side_to_move() != cozy_chess::Color::White
      || self.board.status() != cozy_chess::GameStatus::Ongoing
    {
      self.selected = None;
      let highlight_ids = self.highlight_ids;
      let visible = self.cursor_visible;
      return crate::native::action_response(self.session_id, action_id, move |message| {
        let mut commands = crate::native::write_hidden_highlights(message, &highlight_ids)?;
        commands.extend(crate::native::write_cursor(
          message, square, false, visible,
        )?);
        Ok(commands)
      });
    }
    if self.board.color_on(square) == Some(cozy_chess::Color::White) {
      return self.select_piece_native(action_id, square, object_id);
    }
    self.submit_selected_move_native(action_id, square)
  }

  fn activate_cursor_native(&mut self, action_id: ActionId) -> Result<NativeResponse, EngineError> {
    self.cursor_visible = true;
    if self.board.side_to_move() != cozy_chess::Color::White
      || self.board.status() != cozy_chess::GameStatus::Ongoing
    {
      let cursor = self.cursor;
      return crate::native::action_response(self.session_id, action_id, move |message| {
        crate::native::write_cursor(message, cursor, false, true)
      });
    }
    if self.selected == Some(self.cursor) {
      return self.cancel_selection_native(action_id);
    }
    if self.board.color_on(self.cursor) == Some(cozy_chess::Color::White) {
      let object_id =
        self.objects[self.cursor as usize].expect("occupied cursor square should have an object");
      return self.select_piece_native(action_id, self.cursor, object_id);
    }
    self.submit_selected_move_native(action_id, self.cursor)
  }

  fn select_piece_native(
    &mut self,
    action_id: ActionId,
    square: cozy_chess::Square,
    _object_id: ObjectId,
  ) -> Result<NativeResponse, EngineError> {
    self.selected = Some(square);
    let mut targets = [false; 64];
    self.board.generate_moves_for(square.bitboard(), |moves| {
      for movement in moves {
        targets[crate::visible_destination(&self.board, movement) as usize] = true;
      }
      false
    });
    let active = self
      .highlight_ids
      .iter()
      .zip(targets)
      .filter_map(|(object_id, active)| active.then_some(*object_id))
      .collect::<Vec<_>>();
    let sounds = audio::PICKUP_SOUNDS;
    let sound_index = self.rng.usize(..sounds.len());
    let highlight_ids = self.highlight_ids;
    let visible = self.cursor_visible;
    let previous = self.change_visual_state(VisualState::Selected);
    crate::native::action_response(self.session_id, action_id, move |message| {
      let mut commands = crate::native::write_hidden_highlights(message, &highlight_ids)?;
      commands.extend(crate::native::write_cursor(message, square, true, visible)?);
      for object_id in active {
        commands.push(message.set_object_active(
          *battlement::CommandId::new_v4().as_uuid().as_bytes(),
          true,
          *object_id.as_uuid().as_bytes(),
          true,
        )?);
      }
      commands.push(crate::native::write_sound(
        message,
        sounds[sound_index].as_str(),
      )?);
      commands.extend(crate::visual_state::write_transition(
        message,
        previous,
        VisualState::Selected,
      )?);
      Ok(commands)
    })
  }

  fn submit_selected_move_native(
    &mut self,
    action_id: ActionId,
    target: cozy_chess::Square,
  ) -> Result<NativeResponse, EngineError> {
    let Some(from) = self.selected else {
      let visible = self.cursor_visible;
      return crate::native::action_response(self.session_id, action_id, move |message| {
        crate::native::write_cursor(message, target, false, visible)
      });
    };
    let Some(movement) = crate::player_move(&self.board, from, target) else {
      let visible = self.cursor_visible;
      return crate::native::action_response(self.session_id, action_id, move |message| {
        let mut commands = crate::native::write_cursor(message, target, false, visible)?;
        commands.push(crate::native::write_sound(
          message,
          crate::INVALID_DROP_SOUND.as_str(),
        )?);
        commands.push(message.vibrate_controller(
          *battlement::CommandId::new_v4().as_uuid().as_bytes(),
          true,
          0.2,
          0.25,
          90,
        )?);
        Ok(commands)
      });
    };
    let mut board_after = self.board.clone();
    board_after.play_unchecked(movement);
    let metadata = if self.diagnostics_enabled {
      match board_after.status() {
        cozy_chess::GameStatus::Won => vec![("chess.game_status", "won")],
        cozy_chess::GameStatus::Drawn => vec![("chess.game_status", "drawn")],
        cozy_chess::GameStatus::Ongoing => Vec::new(),
      }
    } else {
      Vec::new()
    };
    self.selected = None;
    let session_id = self.session_id;
    let highlight_ids = self.highlight_ids;
    let visible = self.cursor_visible;
    let response = crate::native::batch_response_with_metadata(
      session_id,
      Some(action_id),
      battlement_native::NativeBatchStart::Now,
      &metadata,
      |message| {
        let mut groups = crate::native::apply_move(self, message, movement, true)?;
        let mut prefix = crate::native::write_hidden_highlights(message, &highlight_ids)
          .map_err(crate::native::protocol)?;
        prefix.extend(
          crate::native::write_cursor(message, target, false, visible)
            .map_err(crate::native::protocol)?,
        );
        groups[0].splice(0..0, prefix);
        if self.board.status() == cozy_chess::GameStatus::Ongoing {
          groups[0].push(
            message
              .set_local_scale(
                *battlement::CommandId::new_v4().as_uuid().as_bytes(),
                true,
                *crate::cursor::EFFECT_ID.as_uuid().as_bytes(),
                [0.55, 0.55, 0.55],
              )
              .map_err(crate::native::protocol)?,
          );
          groups[0].push(
            message
              .set_input_enabled(
                *battlement::CommandId::new_v4().as_uuid().as_bytes(),
                true,
                false,
              )
              .map_err(crate::native::protocol)?,
          );
        }
        Ok(groups)
      },
    )?;
    if self.board.status() == cozy_chess::GameStatus::Ongoing {
      self.start_ai();
    }
    Ok(response)
  }
}

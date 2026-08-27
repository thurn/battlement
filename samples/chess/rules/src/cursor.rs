use battlement::{
  ActionId, Command, CommandBody, ControllerDirection, ControllerVibrationPayload, GameObject,
  GameObjectKind, ObjectId, ObjectSetActivePayload, ParticlePlayPayload, PhysicalKey,
  PositionPayload, PropertyCommand, Response, ScalePayload, Vector3, object_id,
};
use battlement_native::EngineError;
use cozy_chess::{Color, GameStatus, Square};

use crate::{ChessEngine, assets::effects, audio, movement};

const EFFECT_ID: ObjectId = object_id!("349022dd-0f5f-4d47-bfc8-7caf62419455");

/// Initial cursor square after starting or resetting a game.
pub const START: Square = Square::E2;

pub fn moved(square: Square, key: PhysicalKey) -> Square {
  let direction = match key {
    PhysicalKey::ArrowLeft => ControllerDirection::Left,
    PhysicalKey::ArrowRight => ControllerDirection::Right,
    PhysicalKey::ArrowUp => ControllerDirection::Up,
    PhysicalKey::ArrowDown => ControllerDirection::Down,
    _ => return square,
  };
  moved_in_direction(square, direction)
}

pub fn moved_in_direction(square: Square, direction: ControllerDirection) -> Square {
  let offset = match direction {
    ControllerDirection::Left => (-1, 0),
    ControllerDirection::Right => (1, 0),
    ControllerDirection::Up => (0, 1),
    ControllerDirection::Down => (0, -1),
  };
  square.try_offset(offset.0, offset.1).unwrap_or(square)
}

pub fn dim_command(dimmed: bool) -> CommandBody {
  CommandBody::TransformSetLocalScale(PropertyCommand::canceling(ScalePayload {
    object_id: EFFECT_ID,
    scale: if dimmed {
      Vector3::new(0.55, 0.55, 0.55)
    } else {
      Vector3::ONE
    },
  }))
}

pub fn commands(square: Square, restart: bool) -> [CommandBody; 3] {
  [
    CommandBody::TransformSetWorldPosition(PropertyCommand::canceling(PositionPayload {
      object_id: EFFECT_ID,
      position: crate::square_position(square),
    })),
    CommandBody::ObjectSetActive(ObjectSetActivePayload {
      object_id: EFFECT_ID,
      active: true,
    }),
    CommandBody::ParticlePlay(ParticlePlayPayload {
      object_id: EFFECT_ID,
      restart,
    }),
  ]
}

pub fn state_commands(square: Square, restart: bool, visible: bool) -> Vec<CommandBody> {
  if visible {
    commands(square, restart).into()
  } else {
    vec![CommandBody::ObjectSetActive(ObjectSetActivePayload {
      object_id: EFFECT_ID,
      active: false,
    })]
  }
}

pub fn object(square: Square, active: bool) -> GameObject {
  GameObject::new(EFFECT_ID, GameObjectKind::prefab(effects::PIECE_SELECTED))
    .active(active)
    .position(crate::square_position(square))
}

impl ChessEngine {
  pub(crate) fn cursor_commands(&self, square: Square, restart: bool) -> Vec<CommandBody> {
    if self.cursor_visible {
      self::commands(square, restart).into()
    } else {
      Vec::new()
    }
  }

  pub(crate) fn move_cursor(
    &mut self,
    action_id: ActionId,
    key: PhysicalKey,
  ) -> Result<Response<Command>, EngineError> {
    self.cursor_visible = true;
    self.cursor = self::moved(self.cursor, key);
    Ok(audio::response_for_action(
      self.session_id,
      action_id,
      self::commands(self.cursor, false),
    ))
  }

  pub(crate) fn move_cursor_direction(
    &mut self,
    action_id: ActionId,
    direction: ControllerDirection,
  ) -> Result<Response<Command>, EngineError> {
    self.cursor_visible = true;
    self.cursor = self::moved_in_direction(self.cursor, direction);
    Ok(audio::response_for_action(
      self.session_id,
      action_id,
      self::commands(self.cursor, false),
    ))
  }

  pub(crate) fn cycle_cursor(
    &mut self,
    action_id: ActionId,
    forward: bool,
  ) -> Result<Response<Command>, EngineError> {
    let candidates = self.controller_cycle_squares();
    if candidates.is_empty() {
      return Ok(Response::empty(self.session_id));
    }
    let current = candidates.iter().position(|&square| square == self.cursor);
    let index = match (current, forward) {
      (Some(index), true) => (index + 1) % candidates.len(),
      (Some(0), false) | (None, false) => candidates.len() - 1,
      (Some(index), false) => index - 1,
      (None, true) => 0,
    };
    self.cursor_visible = true;
    self.cursor = candidates[index];
    Ok(audio::response_for_action(
      self.session_id,
      action_id,
      self::commands(self.cursor, false),
    ))
  }

  pub(crate) fn activate_cursor(
    &mut self,
    action_id: ActionId,
  ) -> Result<Response<Command>, EngineError> {
    self.cursor_visible = true;
    if self.board.side_to_move() != Color::White || self.board.status() != GameStatus::Ongoing {
      return Ok(audio::response_for_action(
        self.session_id,
        action_id,
        self::commands(self.cursor, false),
      ));
    }
    if self.selected == Some(self.cursor) {
      return self.cancel_selection(action_id);
    }
    if self.board.color_on(self.cursor) == Some(Color::White) {
      let object_id =
        self.objects[self.cursor as usize].expect("occupied cursor square should have an object");
      return self.select_piece(action_id, self.cursor, object_id);
    }
    self.submit_selected_move(action_id, self.cursor)
  }

  pub(crate) fn cancel_selection(
    &mut self,
    action_id: ActionId,
  ) -> Result<Response<Command>, EngineError> {
    self.cursor_visible = true;
    if let Some(selected) = self.selected.take() {
      self.cursor = selected;
    }
    Ok(audio::response_for_action(
      self.session_id,
      action_id,
      self
        .hide_highlight_commands()
        .into_iter()
        .chain(self::commands(self.cursor, false)),
    ))
  }

  pub(crate) fn submit_click(
    &mut self,
    action_id: ActionId,
    object_id: ObjectId,
  ) -> Result<Response<Command>, EngineError> {
    if let Some(target) = crate::find_highlight(&self.highlight_ids, object_id) {
      self.cursor = target;
      return self.submit_selected_move(action_id, target);
    }
    let Some(square) = crate::find_square(&self.objects, object_id) else {
      return Ok(Response::empty(self.session_id));
    };
    self.cursor = square;
    if self.board.side_to_move() != Color::White || self.board.status() != GameStatus::Ongoing {
      self.selected = None;
      return Ok(audio::response_for_action(
        self.session_id,
        action_id,
        self
          .hide_highlight_commands()
          .into_iter()
          .chain(self.cursor_commands(square, false)),
      ));
    }
    if self.board.color_on(square) == Some(Color::White) {
      return self.select_piece(action_id, square, object_id);
    }
    self.submit_selected_move(action_id, square)
  }

  pub(crate) fn submit_selected_move(
    &mut self,
    action_id: ActionId,
    target: Square,
  ) -> Result<Response<Command>, EngineError> {
    let Some(from) = self.selected else {
      return Ok(audio::response_for_action(
        self.session_id,
        action_id,
        self.cursor_commands(target, false),
      ));
    };
    let Some(mv) = crate::player_move(&self.board, from, target) else {
      return Ok(audio::response_for_action(
        self.session_id,
        action_id,
        self.cursor_commands(target, false).into_iter().chain([
          audio::play_sound(crate::INVALID_DROP_SOUND),
          CommandBody::ControllerVibrate(ControllerVibrationPayload {
            low_frequency: 0.2,
            high_frequency: 0.25,
            duration_ms: 90,
          }),
        ]),
      ));
    };

    self.selected = None;
    let mut groups = self.apply_move(mv, true)?;
    groups[0].splice(
      0..0,
      self
        .hide_highlight_commands()
        .into_iter()
        .chain(self.cursor_commands(target, false)),
    );
    if self.board.status() == GameStatus::Ongoing {
      groups[0].extend([
        self::dim_command(true),
        CommandBody::set_input_enabled(false),
      ]);
      self.start_ai();
    }
    Ok(movement::response_for_groups(
      self.session_id,
      action_id,
      groups,
    ))
  }

  fn select_piece(
    &mut self,
    action_id: ActionId,
    square: Square,
    object_id: ObjectId,
  ) -> Result<Response<Command>, EngineError> {
    self.selected = Some(square);
    Ok(audio::response_for_action(
      self.session_id,
      action_id,
      self
        .hide_highlight_commands()
        .into_iter()
        .chain(self.cursor_commands(square, true))
        .chain(self.highlight_commands(object_id)),
    ))
  }

  fn controller_cycle_squares(&self) -> Vec<Square> {
    if let Some(selected) = self.selected {
      return crate::legal_destinations(&self.board, selected);
    }
    Square::ALL
      .into_iter()
      .filter(|&square| {
        self.board.color_on(square) == Some(Color::White)
          && !crate::legal_destinations(&self.board, square).is_empty()
      })
      .collect()
  }
}

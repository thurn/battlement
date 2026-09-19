use battlement::{ControllerDirection, ObjectId, PhysicalKey, object_id};
use cozy_chess::Square;

pub(crate) const EFFECT_ID: ObjectId = object_id!("349022dd-0f5f-4d47-bfc8-7caf62419455");

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

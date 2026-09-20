//! Keyboard and controller navigation over the finite chessboard grid.

use battlement::{ControllerDirection, ObjectId, PhysicalKey, object_id};
use cozy_chess::Square;

/// Stable host identity for the world-space cursor effect.
pub const EFFECT_ID: ObjectId = object_id!("349022dd-0f5f-4d47-bfc8-7caf62419455");

/// Initial cursor square after starting or resetting a game.
pub const START: Square = Square::E2;

/// Moves the cursor for an arrow key, leaving it in place for other keys or board edges.
///
/// Input translation stays separate from rendering so keyboard and controller
/// paths can share the same grid behavior and be tested without a host scene.
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

/// Moves one square in a controller direction without wrapping at board edges.
pub fn moved_in_direction(square: Square, direction: ControllerDirection) -> Square {
  let offset = match direction {
    ControllerDirection::Left => (-1, 0),
    ControllerDirection::Right => (1, 0),
    ControllerDirection::Up => (0, 1),
    ControllerDirection::Down => (0, -1),
  };
  square.try_offset(offset.0, offset.1).unwrap_or(square)
}

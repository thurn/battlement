//! Shared Motion sequences for chess-piece movement.

use std::time::Duration;

use battlement::{ObjectId, Vector3};
use cozy_chess::Square;
use reactant::{
  animation_controls::{AnimationSequence, MotionSelector, SequencePosition},
  prelude::{Easing, StyleTarget, Transition},
};

use crate::position::Movement;

const MOVE_DURATION: Duration = Duration::from_millis(300);
const KNIGHT_FIRST_LEG: Duration = Duration::from_millis(200);
const KNIGHT_SECOND_LEG: Duration = Duration::from_millis(120);
const CAPTURE_EFFECT_LIFETIME: Duration = Duration::from_millis(2_000);

pub(crate) const fn arrival_duration(movement: &Movement) -> Duration {
  match movement {
    Movement::Knight { .. }
    | Movement::Capture {
      knight_corner: Some(_),
      ..
    } => KNIGHT_FIRST_LEG.saturating_add(KNIGHT_SECOND_LEG),
    Movement::Move { .. }
    | Movement::Capture { .. }
    | Movement::Castle { .. }
    | Movement::Promotion { .. } => MOVE_DURATION,
  }
}

pub(crate) fn sequence(
  movement: &Movement,
  references: &[reactant::prelude::ObjectRef; 64],
) -> AnimationSequence {
  match *movement {
    Movement::Move { piece, to } => move_step(references, piece, to, MOVE_DURATION),
    Movement::Knight { piece, corner, to } => knight_steps(references, piece, corner, to),
    Movement::Capture {
      piece,
      captured,
      capture_at: _,
      to,
      knight_corner,
    } => {
      let sequence = knight_corner.map_or_else(
        || move_step(references, piece, to, MOVE_DURATION),
        |corner| knight_steps(references, piece, corner, to),
      );
      sequence.particle_for(
        crate::assets::effects::CAPTURE,
        piece_reference(references, captured)
          .local_point(Vector3::ZERO)
          .capture_at_start(),
        CAPTURE_EFFECT_LIFETIME,
      )
    }
    Movement::Castle {
      king,
      king_to,
      rook,
      rook_to,
    } => move_step(references, king, king_to, MOVE_DURATION)
      .then(
        MotionSelector::object(piece_reference(references, rook)),
        position_target(rook_to),
        movement_transition(MOVE_DURATION),
      )
      .at(SequencePosition::WithPrevious(0.0)),
    Movement::Promotion {
      piece,
      captured,
      to,
    } => {
      let mut sequence = move_step(references, piece, to, MOVE_DURATION);
      if let Some(captured) = captured {
        sequence = sequence.particle_for(
          crate::assets::effects::CAPTURE,
          piece_reference(references, captured)
            .local_point(Vector3::ZERO)
            .capture_at_start(),
          CAPTURE_EFFECT_LIFETIME,
        );
      }
      sequence
    }
  }
}

fn move_step(
  references: &[reactant::prelude::ObjectRef; 64],
  piece: ObjectId,
  to: Square,
  duration: Duration,
) -> AnimationSequence {
  AnimationSequence::new().animate(
    MotionSelector::object(piece_reference(references, piece)),
    position_target(to),
    movement_transition(duration),
  )
}

fn knight_steps(
  references: &[reactant::prelude::ObjectRef; 64],
  piece: ObjectId,
  corner: Square,
  to: Square,
) -> AnimationSequence {
  move_step(references, piece, corner, KNIGHT_FIRST_LEG).then(
    MotionSelector::object(piece_reference(references, piece)),
    position_target(to),
    movement_transition(KNIGHT_SECOND_LEG),
  )
}

fn piece_reference(
  references: &[reactant::prelude::ObjectRef; 64],
  piece: ObjectId,
) -> reactant::prelude::ObjectRef {
  let index = piece
    .as_uuid()
    .as_bytes()
    .iter()
    .skip(10)
    .fold(0_usize, |value, byte| (value << 8) | usize::from(*byte));
  references
    .get(index)
    .unwrap_or_else(|| panic!("piece identity is outside the stable board map: {piece}"))
    .clone()
}

pub(crate) fn position_target(square: Square) -> StyleTarget {
  let position = crate::square_position(square);
  StyleTarget::new()
    .local_position_x(position.x as f32)
    .local_position_y(position.y as f32)
    .local_position_z(position.z as f32)
}

fn movement_transition(duration: Duration) -> Transition {
  Transition::tween()
    .duration_secs(duration.as_secs_f64())
    .ease(Easing::InOutSine)
}

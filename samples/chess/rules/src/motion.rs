//! Shared Motion sequences for chess-piece movement.

use std::time::Duration;

use battlement::Vector3;
use cozy_chess::Square;
use reactant::{
  animation_controls::{AnimationSequence, MotionSelector, SequencePosition},
  prelude::{Easing, ObjectRef, StyleTarget, Transition},
};

use crate::chess_board;
use crate::position::{Movement, PieceIdentity};

const REDUCED_DURATION: Duration = Duration::from_millis(120);
const MOVE_DURATION: Duration = Duration::from_millis(300);
const KNIGHT_FIRST_LEG: Duration = Duration::from_millis(200);
const KNIGHT_SECOND_LEG: Duration = Duration::from_millis(120);
const CAPTURE_EFFECT_LIFETIME: Duration = Duration::from_millis(2_000);

/// Returns when a movement reaches its destination for synchronized audio.
///
/// The board component uses the same timing as the sequence builder rather
/// than duplicating animation constants, keeping sound and motion aligned.
pub const fn arrival_duration(movement: &Movement, reduced: bool) -> Duration {
  if reduced {
    return REDUCED_DURATION;
  }
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

/// Converts a semantic chess movement into one host-independent Motion sequence.
///
/// Rules code describes *what* moved through [`Movement`]; this presentation
/// module decides *how* that event is animated. That separation is idiomatic in
/// Reactant because the rules worker remains deterministic while the component
/// tree owns effects and host object references.
pub fn sequence(
  movement: &Movement,
  references: &[ObjectRef; 64],
  reduced: bool,
) -> AnimationSequence {
  let duration = if reduced {
    REDUCED_DURATION
  } else {
    MOVE_DURATION
  };
  match *movement {
    Movement::Move { piece, to } => move_step(references, piece, to, duration),
    Movement::Knight { piece, corner, to } => {
      if reduced {
        move_step(references, piece, to, duration)
      } else {
        knight_steps(references, piece, corner, to)
      }
    }
    Movement::Capture {
      piece,
      captured,
      capture_at: _,
      to,
      knight_corner,
    } => {
      let sequence = knight_corner.filter(|_| !reduced).map_or_else(
        || move_step(references, piece, to, duration),
        |corner| knight_steps(references, piece, corner, to),
      );
      if reduced {
        return sequence;
      }
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
    } => move_step(references, king, king_to, duration)
      .then(
        MotionSelector::object(piece_reference(references, rook)),
        position_target(rook_to),
        movement_transition(duration),
      )
      .at(SequencePosition::WithPrevious(0.0)),
    Movement::Promotion {
      piece,
      captured,
      to,
    } => {
      let mut sequence = move_step(references, piece, to, duration);
      if let Some(captured) = captured.filter(|_| !reduced) {
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

/// Creates the Motion target corresponding to the center of a chess square.
pub fn position_target(square: Square) -> StyleTarget {
  let position = chess_board::square_position(square);
  StyleTarget::new()
    .local_position_x(position.x as f32)
    .local_position_y(position.y as f32)
    .local_position_z(position.z as f32)
}

/// Builds the common single-segment translation used by most pieces.
fn move_step(
  references: &[ObjectRef; 64],
  piece: PieceIdentity,
  to: Square,
  duration: Duration,
) -> AnimationSequence {
  AnimationSequence::new().animate(
    MotionSelector::object(piece_reference(references, piece)),
    position_target(to),
    movement_transition(duration),
  )
}

/// Animates a knight through an L-shaped corner so its path reads clearly in 3D.
fn knight_steps(
  references: &[ObjectRef; 64],
  piece: PieceIdentity,
  corner: Square,
  to: Square,
) -> AnimationSequence {
  move_step(references, piece, corner, KNIGHT_FIRST_LEG).then(
    MotionSelector::object(piece_reference(references, piece)),
    position_target(to),
    movement_transition(KNIGHT_SECOND_LEG),
  )
}

/// Resolves a piece's explicit reference slot into the object reference captured by the board.
fn piece_reference(references: &[ObjectRef; 64], piece: PieceIdentity) -> ObjectRef {
  references
    .get(piece.reference_slot)
    .unwrap_or_else(|| {
      panic!(
        "piece reference slot is outside the stable board map: {}",
        piece.reference_slot
      )
    })
    .clone()
}

/// Uses one easing curve for every board translation so composed moves feel coherent.
fn movement_transition(duration: Duration) -> Transition {
  Transition::tween()
    .duration_secs(duration.as_secs_f64())
    .ease(Easing::InOutSine)
}

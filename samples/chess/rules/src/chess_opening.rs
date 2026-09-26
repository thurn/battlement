//! The initial piece reveal and its reduced-motion equivalent.

use crate::{
  chess_ui_state::SessionStart,
  position::{ChessPiece, PieceIdentity},
};
use battlement::Vector3;
use cozy_chess::Color;
use reactant::{
  animation_controls::{AnimationSequence, MotionSelector, SequencePosition},
  prelude::{Easing, ObjectRef, StyleTarget, Transition},
};
use std::time::Duration;

const PIECE_SPAWN_SEQUENCE_DURATION_MS: u64 = 5_070;
const CRITICAL_BEAT_INTERVAL_MS: u64 = 570;
const CRITICAL_FIRST_BEAT_OFFSET_MS: u64 = 80;
const PIECE_SPAWN_BEAT_COUNT: usize = 8;
const PIECE_SPAWN_EFFECT_LIFETIME_MS: u64 = 1_000;

/// Builds the staged piece reveal used when mounting a fresh or restarted session.
///
/// White and black pieces enter in parallel groups aligned to the music beats.
/// A refresh skips the reveal because its existing board is already visible.
pub fn sequence(
  mode: SessionStart,
  pieces: &[ChessPiece],
  references: &[ObjectRef; 64],
  reduced: bool,
) -> AnimationSequence {
  let sound = if mode == SessionStart::Fresh {
    crate::audio::START_SOUND
  } else {
    crate::audio::RESET_SOUND
  };
  let mut sequence = AnimationSequence::new().play_sound(sound);
  if reduced || mode == SessionStart::Refresh {
    return sequence;
  }

  let white = pieces
    .iter()
    .filter(|piece| piece.color == Color::White)
    .map(|piece| piece.identity)
    .collect::<Vec<_>>();
  let black = pieces
    .iter()
    .filter(|piece| piece.color == Color::Black)
    .map(|piece| piece.identity)
    .collect::<Vec<_>>();
  let maximum = white.len().max(black.len());
  let per_beat = maximum.div_ceil(PIECE_SPAWN_BEAT_COUNT).max(1);
  let stages = maximum.div_ceil(per_beat);
  for beat in 0..stages {
    let start = beat * per_beat;
    let end = start + per_beat;
    let at = Duration::from_millis(
      CRITICAL_FIRST_BEAT_OFFSET_MS + beat as u64 * CRITICAL_BEAT_INTERVAL_MS,
    );
    for piece in white
      .get(start..end.min(white.len()))
      .into_iter()
      .flatten()
      .chain(black.get(start..end.min(black.len())).into_iter().flatten())
    {
      let reference = piece_reference(references, *piece);
      sequence = sequence
        .animate(
          MotionSelector::object(reference.clone()),
          StyleTarget::new()
            .local_scale_x(1.0)
            .local_scale_y(1.0)
            .local_scale_z(1.0),
          Transition::tween().duration_secs(0.2).ease(Easing::EaseOut),
        )
        .at(SequencePosition::Absolute(at))
        .particle_for(
          crate::assets::effects::PIECE_SPAWN,
          reference.local_point(Vector3::ZERO).capture_at_start(),
          Duration::from_millis(PIECE_SPAWN_EFFECT_LIFETIME_MS),
        )
        .at(SequencePosition::Absolute(at));
    }
  }
  sequence
    .animate(
      MotionSelector::ScopeRoot,
      StyleTarget::new().local_scale_factor_x(1.0),
      Transition::tween()
        .duration_secs(PIECE_SPAWN_SEQUENCE_DURATION_MS as f64 / 1_000.0)
        .ease(Easing::Linear),
    )
    .at(SequencePosition::Absolute(Duration::ZERO))
}

fn piece_reference(references: &[ObjectRef; 64], piece: PieceIdentity) -> ObjectRef {
  references[piece.reference_slot].clone()
}

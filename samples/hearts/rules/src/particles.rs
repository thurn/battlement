use std::time::Duration;

use battlement::Vector3;
use reactant::{
  SnapshotAnimation,
  animation_controls::{self, AnimationSequence, SequenceParticleOptions},
  motion_config, native_host,
  native_host::ObjectRef,
  prelude::*,
  world,
};
use reactant_rules::ReducerGame;

use crate::{assets, domain::Event, reducer::HeartsReducer};

pub(crate) struct GameParticles {
  pub aspect: f64,
}

impl Component for GameParticles {
  fn render(&self) -> impl Render {
    let scope = animation_controls::use_animation_scope();
    let anchor = native_host::use_object_ref();
    let reduced = motion_config::use_reduced_motion();
    let position = anchor.clone();
    let animation_scope = scope.clone();
    reactant::use_animate::<ReducerGame<HeartsReducer>>(move |event| {
      if reduced {
        return None;
      }
      let seed = match event {
        Event::CardPlayed {
          broke_hearts: true, ..
        } => 71,
        Event::HandScored { .. } => 83,
        _ => return None,
      };
      Some(SnapshotAnimation::sequence(animation_scope, self::burst(position, seed)).nonblocking())
    });
    (
      self::ambient(self.aspect, 43),
      self::anchor(self.aspect)
        .reference(anchor)
        .motion(MotionProps::new().animation_scope(scope)),
    )
  }
}

pub(crate) fn ambient(aspect: f64, seed: u32) -> impl Render {
  [-1.0, 1.0]
    .into_iter()
    .enumerate()
    .map(|(index, side)| {
      world::Group::new()
        .position(Vector3::new(side * 5.7 * aspect, 0.6, 0.0))
        .child(world::ParticleEmitter::new(
          assets::hearts::particles::MOTES,
          seed + index as u32,
        ))
    })
    .collect::<Vec<_>>()
}

pub(crate) fn anchor(aspect: f64) -> world::Group {
  world::Group::new().position(Vector3::new(5.7 * aspect - 0.9, 3.2, 0.0))
}

pub(crate) fn burst(position: ObjectRef, seed: u32) -> AnimationSequence {
  AnimationSequence::new().particle_with(
    assets::hearts::particles::ACCENT,
    position.local_point(Vector3::ZERO).capture_at_start(),
    SequenceParticleOptions {
      lifetime: Duration::from_millis(750),
      seed,
    },
  )
}

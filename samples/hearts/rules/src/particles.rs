use std::time::Duration;

use battlement::Vector3;
use reactant::{
  animation_controls::{AnimationSequence, SequenceParticleOptions},
  native_host::ObjectRef,
  prelude::*,
  world,
};

use crate::assets;

pub(crate) struct GameParticles {
  pub aspect: f64,
}

impl Component for GameParticles {
  fn render(&self) -> impl Render {
    self::ambient(self.aspect, 43)
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

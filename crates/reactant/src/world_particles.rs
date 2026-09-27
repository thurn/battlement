//! Particle prefabs follow their component and inherited presentation scope.

use battlement::{Command, CommandBody, ParticlePlayPayload, PrefabAddress};
use reactant_core::{app_context, hooks, motion_config, native_host, prelude::*};

use crate::world;

/// A seeded decorative emitter. Disabling or reduced motion removes its native
/// subtree; the inherited game scope owns pause, resume and session cancellation.
#[derive(Clone)]
pub struct ParticleEmitter {
  address: PrefabAddress,
  seed: u32,
  enabled: bool,
}

impl ParticleEmitter {
  /// Creates an emitter with an explicit reproducible seed.
  pub fn new(address: impl Into<PrefabAddress>, seed: u32) -> Self {
    assert!(seed != 0, "particle seed must be nonzero");
    Self {
      address: address.into(),
      seed,
      enabled: true,
    }
  }

  /// Removes native particles while disabled.
  pub fn enabled(mut self, enabled: bool) -> Self {
    self.enabled = enabled;
    self
  }
}

impl Component for ParticleEmitter {
  fn render(&self) -> impl Render {
    let reduced = motion_config::use_reduced_motion();
    let enabled = self.enabled && !reduced;
    enabled.then(|| MountedEmitter(self.clone()))
  }
}

struct MountedEmitter(ParticleEmitter);

impl Component for MountedEmitter {
  fn render(&self) -> impl Render {
    let reference = native_host::use_object_ref();
    let app = app_context::use_app();
    let seed = self.0.seed;
    hooks::use_effect(
      {
        let reference = reference.clone();
        move || {
          let object_id = reference.object_id().expect("particle prefab is attached");
          app.send(
            Command::new_v4(CommandBody::ParticlePlay(ParticlePlayPayload {
              object_id,
              restart: true,
              seed,
            }))
            .nonblocking(),
          );
        }
      },
      (self.0.address.clone(), seed),
    );
    world::Prefab::at(self.0.address.clone()).reference(reference)
  }
}

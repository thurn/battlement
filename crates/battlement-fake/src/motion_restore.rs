//! Released world gestures follow the current underlying transform channels.

use std::collections::HashMap;

use battlement::{
  MotionDescriptor, MotionGeneration, MotionLayer, MotionProperty, MotionRepeat, MotionSlotId,
  MotionValue, TransitionDefinition,
};
use battlement_ui_fake::UiWorld;

use crate::{motion_scalar, motion_slot::Slot, motion_target::Target, world::FakeWorld};

#[derive(Default)]
pub(crate) struct LayerRestore {
  baseline: HashMap<MotionProperty, MotionValue>,
  releases: HashMap<MotionSlotId, Release>,
}

struct Release {
  anchor: u64,
  generation: MotionGeneration,
  tracks: Vec<ReleaseTrack>,
}
struct ReleaseTrack {
  property: MotionProperty,
  origin: f64,
  velocity: f64,
  lower: f64,
  transition: TransitionDefinition,
}

impl LayerRestore {
  pub(crate) fn install(
    definition: &MotionDescriptor,
    target: &mut Target,
    world: &FakeWorld,
    ui: &UiWorld,
    mut previous: Self,
  ) -> Self {
    if !target.is_world() {
      return Self::default();
    }
    let mut baseline = HashMap::new();
    previous.releases.retain(|id, release| {
      definition
        .slots
        .iter()
        .any(|slot| slot.slot == *id && slot.generation == release.generation)
    });
    for slot in &definition.slots {
      if matches!(slot.layer, MotionLayer::Animate | MotionLayer::Exit) {
        continue;
      }
      for track in &slot.target.tracks {
        if definition
          .value_bindings
          .iter()
          .any(|binding| binding.property == track.property)
        {
          continue;
        }
        baseline.entry(track.property).or_insert_with(|| {
          previous
            .baseline
            .get(&track.property)
            .cloned()
            .unwrap_or_else(|| target.read(track.property, world, ui))
        });
      }
    }
    Self {
      baseline,
      releases: previous.releases,
    }
  }

  pub(crate) fn begin(&self, target: &mut Target, world: &mut FakeWorld, ui: &mut UiWorld) {
    for (property, value) in &self.baseline {
      target.write(*property, value.clone(), world, ui);
    }
  }

  pub(crate) fn activate(
    &mut self,
    slot: &Slot,
    active: bool,
    target: &mut Target,
    world: &FakeWorld,
    ui: &UiWorld,
    now: u64,
  ) {
    if active {
      self.releases.remove(&slot.definition.slot);
      return;
    }
    if !slot.active || !target.is_world() {
      return;
    }
    let tracks = slot
      .tracks
      .iter()
      .map(|track| {
        let MotionValue::Scalar(origin) = target.read(track.definition.property, world, ui) else {
          panic!("world gesture requires a scalar channel");
        };
        let mut transition = track.definition.transition.clone();
        transition.repeat = MotionRepeat::None;
        ReleaseTrack {
          property: track.definition.property,
          origin: f64::from(origin),
          velocity: track.velocity,
          lower: f64::from(origin),
          transition,
        }
      })
      .collect();
    self.releases.insert(
      slot.definition.slot,
      Release {
        anchor: now,
        generation: slot.definition.generation,
        tracks,
      },
    );
  }

  pub(crate) fn sample(
    &mut self,
    slot: &Slot,
    target: &mut Target,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
    now: u64,
    reduced: bool,
  ) {
    let Some(release) = self.releases.get_mut(&slot.definition.slot) else {
      return;
    };
    let mut done = true;
    for track in &mut release.tracks {
      let MotionValue::Scalar(lower) = target.read(track.property, world, ui) else {
        panic!("world gesture requires a scalar channel");
      };
      track.lower = f64::from(lower);
      if reduced {
        continue;
      }
      let sample = motion_scalar::sample(
        track.origin,
        f64::from(lower),
        track.velocity,
        &track.transition,
        now.saturating_sub(release.anchor),
      );
      target.write(
        track.property,
        MotionValue::Scalar(sample.value as f32),
        world,
        ui,
      );
      done &= sample.done;
    }
    if done {
      self.releases.remove(&slot.definition.slot);
    }
  }

  pub(crate) fn deadline(&self, now: u64) -> Option<u64> {
    self
      .releases
      .values()
      .flat_map(|release| {
        release.tracks.iter().filter_map(|track| {
          motion_scalar::duration(track.origin, track.lower, track.velocity, &track.transition).map(
            |duration| {
              release
                .anchor
                .saturating_add(duration)
                .max(now.saturating_add(1))
            },
          )
        })
      })
      .min()
  }

  pub(crate) fn rebase_clock(&mut self, now: u64) {
    // Rebase retained release origins without advancing presentation time.
    for release in self.releases.values_mut() {
      for track in &mut release.tracks {
        track.transition.delay_micros -= i64::try_from(now.saturating_sub(release.anchor))
          .expect("Motion release elapsed time overflow");
      }
      release.anchor = 0;
    }
  }
}

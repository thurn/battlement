//! Imperative targets reuse descriptor tracks and command-operation status.

use battlement::{
  MotionCallbackSubscriptions, MotionControlCommand, MotionControlOperation, MotionControlTarget,
  MotionGeneration, MotionLayer, MotionPlaybackCommand, MotionPlaybackOperation,
  MotionPlaybackOutcome, MotionRepeat, MotionSlotDescriptor, MotionSlotId, MotionTargetDescriptor,
  MotionValuePlaybackOperation, ObjectId,
};
use battlement_ui_fake::UiWorld;

use crate::{
  motion::{Descriptor, MotionWorld},
  motion_ownership,
  motion_playbacks::{Address, RunningMotion},
  motion_slot::Slot,
  motion_track::Track,
  world::FakeWorld,
};

pub(crate) const IMPERATIVE_SLOT: MotionSlotId = MotionSlotId(u64::MAX - 1024);

#[derive(Clone)]
pub(crate) struct ActiveControl {
  playback: ObjectId,
  generation: u32,
  target: MotionControlTarget,
  blocking: bool,
}

impl MotionWorld {
  pub(crate) fn control(
    &mut self,
    operation: &MotionControlOperation,
    blocking: bool,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
    now: u64,
  ) -> Option<RunningMotion> {
    let bindings = self
      .entries
      .iter()
      .filter(|(_, entry)| entry.definition.control_id == Some(operation.control_id))
      .map(|(id, _)| *id)
      .collect::<Vec<_>>();
    if blocking && matches!(operation.command, MotionControlCommand::Start { .. }) {
      assert!(
        !bindings.is_empty(),
        "blocking Motion controls require a mounted target"
      );
    }
    let target = match &operation.command {
      MotionControlCommand::Start { target, .. } | MotionControlCommand::Set(target) => {
        Some(target)
      }
      _ => None,
    };
    let targets = target
      .map(|target| {
        bindings
          .iter()
          .map(|id| {
            let entry = &self.entries[id];
            let resolved = resolve(entry, target);
            validate(entry, &resolved, blocking);
            (*id, resolved)
          })
          .collect::<Vec<_>>()
      })
      .unwrap_or_default();
    if let Some(previous) = self.controls.remove(&operation.control_id) {
      self.playbacks.finish(
        previous.playback,
        if matches!(operation.command, MotionControlCommand::Stop) {
          MotionPlaybackOutcome::Stopped
        } else {
          MotionPlaybackOutcome::Cancelled
        },
      );
    }
    match &operation.command {
      MotionControlCommand::Start {
        playback_id,
        generation,
        target,
      } => {
        let infinite = targets.iter().any(|(_, target)| {
          target
            .tracks
            .iter()
            .any(|track| track.transition.repeat == MotionRepeat::Forever)
        });
        assert!(
          !blocking || !infinite,
          "infinite Motion cannot block a command batch"
        );
        let running = self.playbacks.register(*playback_id, *generation, infinite);
        self.controls.insert(
          operation.control_id,
          ActiveControl {
            playback: *playback_id,
            generation: *generation,
            target: target.clone(),
            blocking,
          },
        );
        for (id, target) in targets {
          let address = self.install_imperative(id, target, *generation, world, ui, now);
          self.playbacks.attach(*playback_id, address);
        }
        Some(running)
      }
      MotionControlCommand::Set(_) => {
        for (id, target) in targets {
          let entry = self.entries.get_mut(&id).unwrap();
          entry
            .slots
            .retain(|slot| slot.definition.slot.0 < u64::MAX - 2048);
          entry.sampled = None;
          for track in &target.tracks {
            if let Some(value) = track.values.last() {
              entry.target.write(track.property, value.clone(), world, ui);
            }
          }
          for value in &target.transition_end {
            entry
              .target
              .write(value.property, value.value.clone(), world, ui);
          }
        }
        None
      }
      MotionControlCommand::Stop => {
        for id in bindings {
          let clock = self.clock(self.entries[&id].definition.clock, now);
          let entry = self.entries.get_mut(&id).unwrap();
          for slot in &mut entry.slots {
            if slot.definition.slot.0 >= u64::MAX - 2048 {
              slot.apply(MotionPlaybackCommand::Stop, clock);
            }
          }
          entry.sampled = None;
        }
        None
      }
      MotionControlCommand::Clear => {
        for id in bindings {
          let entry = self.entries.get_mut(&id).unwrap();
          entry
            .slots
            .retain(|slot| slot.definition.slot.0 < u64::MAX - 2048);
          entry.sampled = None;
        }
        None
      }
    }
  }

  pub(crate) fn imperative_playback(
    &mut self,
    operation: MotionValuePlaybackOperation,
    now: u64,
  ) -> bool {
    let Some(playback) = self.playbacks.get(operation.playback_id) else {
      return false;
    };
    assert_eq!(
      playback.generation, operation.generation,
      "Motion playback generation is stale"
    );
    let addresses = playback.addresses.clone();
    self.sequence_playback(
      operation.playback_id,
      operation.generation,
      operation.command,
      now,
    );
    for address in addresses {
      self.playback(
        MotionPlaybackOperation {
          descriptor_id: address.descriptor,
          slot: address.slot,
          generation: address.generation,
          command: operation.command,
        },
        now,
      );
    }
    true
  }

  pub(crate) fn attach_control(
    &mut self,
    id: ObjectId,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
    now: u64,
  ) {
    let entry = &self.entries[&id];
    let Some(control) = entry
      .definition
      .control_id
      .and_then(|id| self.controls.get(&id))
      .cloned()
    else {
      return;
    };
    if entry
      .slots
      .iter()
      .any(|slot| slot.definition.slot == IMPERATIVE_SLOT)
    {
      return;
    }
    let target = resolve(entry, &control.target);
    validate(entry, &target, control.blocking);
    let address = self.install_imperative(id, target, control.generation, world, ui, now);
    self.playbacks.attach(control.playback, address);
  }

  pub(crate) fn install_imperative(
    &mut self,
    id: ObjectId,
    target: MotionTargetDescriptor,
    generation: u32,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
    now: u64,
  ) -> Address {
    self
      .install_imperatives(id, vec![(target, 0)], generation, world, ui, now)
      .remove(0)
  }

  pub(crate) fn install_imperatives(
    &mut self,
    id: ObjectId,
    targets: Vec<(MotionTargetDescriptor, u64)>,
    generation: u32,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
    now: u64,
  ) -> Vec<Address> {
    self
      .install_imperatives_with_remaps(id, targets, generation, world, ui, now)
      .0
  }

  pub(crate) fn install_imperatives_with_remaps(
    &mut self,
    id: ObjectId,
    targets: Vec<(MotionTargetDescriptor, u64)>,
    generation: u32,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
    now: u64,
  ) -> (Vec<Address>, Vec<(Address, Address)>) {
    let clock = self.clock(self.entries[&id].definition.clock, now);
    let entry = self.entries.get_mut(&id).unwrap();
    let mut prepared = Vec::new();
    for (target, offset) in targets {
      assert!(
        offset <= 1024,
        "Motion sequence exceeds imperative slot capacity"
      );
      let slot_id = MotionSlotId(IMPERATIVE_SLOT.0 - offset);
      let generation = entry
        .slots
        .iter()
        .find(|slot| slot.definition.slot == slot_id)
        .map_or(generation, |slot| {
          generation.max(
            slot
              .definition
              .generation
              .0
              .checked_add(1)
              .expect("Motion generation overflow"),
          )
        });
      let tracks = target
        .tracks
        .iter()
        .map(|track| {
          let velocity = entry
            .slots
            .iter()
            .rev()
            .flat_map(|slot| &slot.tracks)
            .find(|old| old.definition.property == track.property)
            .map_or(0.0, |track| track.velocity);
          Track::new(
            track.clone(),
            entry.target.read(track.property, world, ui),
            velocity,
          )
        })
        .collect();
      prepared.push(Slot::new(
        MotionSlotDescriptor {
          slot: slot_id,
          generation: MotionGeneration(generation),
          layer: MotionLayer::Animate,
          target,
          callbacks: MotionCallbackSubscriptions::default(),
        },
        tracks,
        clock,
      ));
    }
    let remaps = motion_ownership::retain_disjoint(id, &mut entry.slots, &prepared);
    let addresses = prepared
      .iter()
      .map(|slot| Address {
        descriptor: id,
        slot: slot.definition.slot,
        generation: slot.definition.generation,
      })
      .collect();
    entry.slots.extend(prepared);
    entry
      .slots
      .sort_by_key(|slot| (slot.definition.layer, slot.definition.slot));
    entry.sampled = None;
    self.playbacks.remap(&remaps);
    (addresses, remaps)
  }

  pub(crate) fn sample_playbacks(&mut self, now: u64) {
    for (id, addresses) in self.playbacks.cancellations() {
      self.sequences.remove(&id);
      for address in addresses {
        if let Some(entry) = self.entries.get_mut(&address.descriptor) {
          let clock = crate::motion::clock(&self.clocks, entry.definition.clock, now);
          if let Some(slot) = entry
            .slots
            .iter_mut()
            .find(|slot| slot.definition.slot == address.slot)
            && slot.definition.generation == address.generation
          {
            slot.apply(MotionPlaybackCommand::Cancel, clock);
            entry.sampled = None;
          }
        }
      }
      self.playbacks.finish(id, MotionPlaybackOutcome::Cancelled);
    }
    let deferred = self.sequences.keys().copied().collect();
    self.playbacks.sample(&deferred, |address| {
      let Some(entry) = self.entries.get(&address.descriptor) else {
        return Some(MotionPlaybackOutcome::Cancelled);
      };
      let Some(slot) = entry
        .slots
        .iter()
        .find(|slot| slot.definition.slot == address.slot)
      else {
        return Some(MotionPlaybackOutcome::Cancelled);
      };
      if slot.definition.generation != address.generation {
        return Some(MotionPlaybackOutcome::Cancelled);
      }
      slot.outcome
    });
    self
      .controls
      .retain(|_, control| self.playbacks.get(control.playback).is_some());
  }
}

fn resolve(entry: &Descriptor, target: &MotionControlTarget) -> MotionTargetDescriptor {
  match target {
    MotionControlTarget::Target(target) => target.clone(),
    MotionControlTarget::Variant(name) => entry
      .definition
      .named_targets
      .iter()
      .find(|target| target.name == *name)
      .expect("controlled Motion variant is absent")
      .target
      .clone(),
  }
}

pub(crate) fn validate(entry: &Descriptor, target: &MotionTargetDescriptor, blocking: bool) {
  target.validate().expect("invalid imperative Motion target");
  if blocking {
    assert!(
      target
        .tracks
        .iter()
        .all(|track| track.transition.repeat != MotionRepeat::Forever),
      "infinite Motion cannot block a command batch"
    );
  }
  let properties = target
    .tracks
    .iter()
    .map(|track| track.property)
    .chain(target.transition_end.iter().map(|value| value.property));
  assert!(
    properties
      .into_iter()
      .all(|property| property.is_world_transform() == entry.target.is_world()),
    "imperative Motion target is incompatible with its host"
  );
}

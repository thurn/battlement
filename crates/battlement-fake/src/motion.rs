//! Shared Motion sampling driven by the existing presentation clock.

use std::collections::HashMap;

use battlement::{
  MotionClockSource, MotionControlledClockCommand, MotionControlledClockOperation,
  MotionDescriptor, MotionEventBatch, MotionEventKind, MotionLifecycleEvent, MotionPlaybackCommand,
  MotionPlaybackOperation, MotionPresentationSample, MotionPropertyValue, MotionSequence, ObjectId,
  Prop, UiNode, UiVisualElementProperties,
};
use battlement_ui_fake::UiWorld;

use crate::{
  motion_gestures::Gestures,
  motion_graph::Graph,
  motion_graph_bindings,
  motion_playbacks::Playbacks,
  motion_restore::LayerRestore,
  motion_slot::Slot,
  motion_target::{self, Target},
  motion_track::Track,
  world::FakeWorld,
};

#[path = "motion_controls.rs"]
mod controls;
#[path = "motion_scopes.rs"]
mod scopes;

#[derive(Default)]
pub(crate) struct MotionWorld {
  entries: HashMap<ObjectId, Descriptor>,
  hosts: HashMap<ObjectId, ObjectId>,
  clocks: HashMap<ObjectId, u64>,
  sequence: u64,
  gestures: Gestures,
  playbacks: Playbacks,
  graph: Graph,
  controls: HashMap<ObjectId, controls::ActiveControl>,
  events: Vec<MotionLifecycleEvent>,
  samples: HashMap<(ObjectId, battlement::MotionSlotId), MotionPresentationSample>,
}

struct Descriptor {
  definition: MotionDescriptor,
  target: Target,
  slots: Vec<Slot>,
  sampled: Option<(u64, u64)>,
  restore: LayerRestore,
}

impl MotionWorld {
  pub(crate) fn install(
    &mut self,
    host: ObjectId,
    definition: Option<MotionDescriptor>,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
    now: u64,
  ) {
    self.install_inner(host, definition, world, ui, now, false);
  }

  fn install_inner(
    &mut self,
    host: ObjectId,
    definition: Option<MotionDescriptor>,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
    now: u64,
    reconnect: bool,
  ) {
    let Some(definition) = definition else {
      if let Some(id) = self.hosts.get(&host) {
        self
          .entries
          .get_mut(id)
          .unwrap()
          .target
          .clear_contribution(world, ui);
      }
      self.remove(host);
      return;
    };
    definition.validate().expect("invalid Motion descriptor");
    assert_eq!(definition.host_id, host, "Motion host mismatch");
    if let Some(id) = self.hosts.get(&host) {
      assert_eq!(
        *id, definition.descriptor_id,
        "a host cannot own two Motion descriptors"
      );
    }
    let id = definition.descriptor_id;
    let previous = self.entries.remove(&id);
    if let Some(previous) = &previous {
      assert!(
        definition.generation > previous.definition.generation
          || (reconnect && definition.generation == previous.definition.generation),
        "Motion descriptor generation must advance"
      );
    }
    let mut target = previous
      .as_ref()
      .map_or_else(|| Target::new(host, world, ui), |old| old.target.clone());
    if reconnect && previous.is_some() {
      target.reconnect(world, ui);
    }
    if !definition
      .value_bindings
      .iter()
      .any(|binding| binding.composition == battlement::MotionBindingComposition::Compose)
    {
      target.clear_contribution(world, ui);
    }
    let clock = self.clock(definition.clock, now);
    let mut slots = Vec::new();
    for slot in &definition.slots {
      let old = previous.as_ref().and_then(|old| {
        old
          .slots
          .iter()
          .find(|old| old.definition.slot == slot.slot)
      });
      if let Some(old) = old {
        if reconnect && slot.generation == old.definition.generation {
          slots.push(old.clone());
          continue;
        }
        assert!(
          slot.generation > old.definition.generation,
          "Motion slot generation must advance"
        );
      }
      let tracks = slot
        .target
        .tracks
        .iter()
        .map(|track| {
          let displayed = target.read(track.property, world, ui);
          let initial = if previous.is_none() && !definition.initial_disabled {
            definition
              .initial
              .as_ref()
              .and_then(|initial| {
                initial
                  .tracks
                  .iter()
                  .find(|value| value.property == track.property)
              })
              .and_then(|track| track.values.last())
              .cloned()
          } else {
            None
          };
          let origin = initial.unwrap_or_else(|| {
            if track.values.len() > 1 {
              track.values[0].clone()
            } else {
              displayed
            }
          });
          let velocity = previous
            .as_ref()
            .and_then(|old| {
              old
                .slots
                .iter()
                .rev()
                .flat_map(|slot| &slot.tracks)
                .find(|old| old.definition.property == track.property)
            })
            .map_or(0.0, |old| old.velocity);
          Track::new(track.clone(), origin, velocity)
        })
        .collect();
      slots.push(Slot::new(slot.clone(), tracks, clock));
    }
    if reconnect && let Some(previous) = &previous {
      slots.extend(
        previous
          .slots
          .iter()
          .filter(|slot| slot.definition.slot.0 >= u64::MAX - 2048)
          .cloned(),
      );
    }
    slots.sort_by_key(|slot| (slot.definition.layer, slot.definition.slot));
    for slot in &slots {
      if slot.active && (!reconnect || previous.is_none()) {
        for track in &slot.tracks {
          let reduced = definition.reduced_motion == battlement::ReducedMotionPolicy::Always
            && motion_target::spatial(track.definition.property);
          target.write(
            track.definition.property,
            if reduced {
              track.target().clone()
            } else {
              track.origin.clone()
            },
            world,
            ui,
          );
        }
      }
      if !reconnect || previous.is_none() {
        self.emit(id, slot, MotionEventKind::Activated, 0);
      }
    }
    let mut restore =
      LayerRestore::install(&definition, &mut target, world, ui, LayerRestore::default());
    if let Some(previous) = previous {
      restore = LayerRestore::install(&definition, &mut target, world, ui, previous.restore);
      for old in &previous.slots {
        if !reconnect && old.outcome.is_none() && old.definition.callbacks.cancel {
          self.emit(
            id,
            old,
            MotionEventKind::Cancelled,
            old.elapsed(self.clock(previous.definition.clock, now)),
          );
        }
      }
    }
    self.hosts.insert(host, id);
    self.entries.insert(
      id,
      Descriptor {
        definition,
        target,
        slots,
        sampled: None,
        restore,
      },
    );
    self
      .graph
      .rebuild(self.entries.values().map(|entry| &entry.definition));
    self.attach_control(id, world, ui, now);
  }

  pub(crate) fn deadline(&self, now: u64) -> Option<u64> {
    self
      .entries
      .values()
      .filter(|entry| {
        matches!(
          entry.definition.clock,
          MotionClockSource::Unscaled | MotionClockSource::Scaled
        )
      })
      .flat_map(|entry| {
        entry
          .slots
          .iter()
          .filter_map(|slot| slot.deadline(now))
          .chain(entry.restore.deadline(now))
      })
      .chain(self.graph.deadline(now))
      .min()
  }

  pub(crate) fn rebase_clock(&mut self, now: u64) {
    self.graph.rebase(now);
    for entry in self.entries.values_mut() {
      if matches!(
        entry.definition.clock,
        MotionClockSource::Unscaled | MotionClockSource::Scaled
      ) {
        for slot in &mut entry.slots {
          slot.held = slot.elapsed(now);
          slot.anchor = 0;
        }
      }
      entry.restore.rebase_clock(now);
      entry.sampled = None;
    }
  }

  pub(crate) fn capture(&mut self, world: &FakeWorld, ui: &UiWorld) {
    for entry in self.entries.values_mut() {
      if !entry.target.exists(world, ui) {
        continue;
      }
      for slot in &entry.slots {
        for track in &slot.tracks {
          entry.target.capture(track.definition.property, world, ui);
        }
      }
    }
  }

  pub(crate) fn restore(
    &mut self,
    definitions: Vec<MotionDescriptor>,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
    now: u64,
  ) {
    let retained = definitions
      .iter()
      .map(|value| value.host_id)
      .collect::<std::collections::HashSet<_>>();
    let removed = self
      .hosts
      .keys()
      .filter(|id| !retained.contains(id))
      .copied()
      .collect::<Vec<_>>();
    for host in removed {
      self.remove(host);
    }
    for definition in definitions {
      self.install_inner(definition.host_id, Some(definition), world, ui, now, true);
    }
  }

  pub(crate) fn install_tree(
    &mut self,
    node: &UiNode,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
    now: u64,
  ) {
    self.apply_property(
      node.object_id,
      &node.element.visual_element().motion,
      world,
      ui,
      now,
    );
    for child in &node.children {
      self.install_tree(child, world, ui, now);
    }
  }

  pub(crate) fn apply_property(
    &mut self,
    host: ObjectId,
    motion: &Prop<MotionDescriptor>,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
    now: u64,
  ) {
    match motion {
      Prop::Unset => {}
      Prop::Reset => self.remove(host),
      Prop::Set(value) => self.install(host, Some(value.clone()), world, ui, now),
    }
  }

  pub(crate) fn remove(&mut self, host: ObjectId) {
    self.gestures.remove(host);
    if let Some(id) = self.hosts.remove(&host) {
      self.entries.remove(&id);
      self.samples.retain(|(owner, _), _| *owner != id);
      self
        .graph
        .rebuild(self.entries.values().map(|entry| &entry.definition));
    }
  }

  pub(crate) fn value(
    &mut self,
    operation: &battlement::MotionValueOperation,
    blocking: bool,
    now: u64,
  ) -> Option<crate::motion_playbacks::RunningMotion> {
    self.graph.apply(operation, blocking, now)
  }

  pub(crate) fn value_playback(
    &mut self,
    operation: battlement::MotionValuePlaybackOperation,
    now: u64,
  ) {
    if !self.imperative_playback(operation, now) {
      self.graph.playback(operation, now);
    }
  }

  pub(crate) fn controlled(&mut self, operation: MotionControlledClockOperation) {
    self.graph.invalidate();
    let current = self.clocks.entry(operation.clock_id).or_default();
    match operation.command {
      MotionControlledClockCommand::Set { elapsed_micros } => {
        assert!(
          elapsed_micros >= *current,
          "controlled Motion time cannot move backward"
        );
        *current = elapsed_micros;
      }
      MotionControlledClockCommand::Advance { delta_micros } => {
        *current = current
          .checked_add(delta_micros)
          .expect("Motion clock overflow")
      }
    }
  }

  pub(crate) fn playback(&mut self, operation: MotionPlaybackOperation, now: u64) {
    let definition = &self
      .entries
      .get(&operation.descriptor_id)
      .expect("Motion descriptor is absent")
      .definition;
    let clock = self.clock(definition.clock, now);
    let entry = self.entries.get_mut(&operation.descriptor_id).unwrap();
    entry.sampled = None;
    let slot = entry
      .slots
      .iter_mut()
      .find(|slot| slot.definition.slot == operation.slot)
      .expect("Motion slot is absent");
    assert_eq!(
      slot.definition.generation, operation.generation,
      "Motion slot generation is stale"
    );
    let active = slot.outcome.is_none();
    slot.apply(operation.command, clock);
    let kind = match operation.command {
      MotionPlaybackCommand::Stop if slot.definition.callbacks.stop => {
        Some(MotionEventKind::Stopped)
      }
      MotionPlaybackCommand::Cancel if slot.definition.callbacks.cancel => {
        Some(MotionEventKind::Cancelled)
      }
      MotionPlaybackCommand::Complete if slot.definition.callbacks.complete => {
        Some(MotionEventKind::Completed)
      }
      _ => None,
    };
    if active && let Some(kind) = kind {
      self.sequence += 1;
      self.events.push(MotionLifecycleEvent {
        sequence: MotionSequence(self.sequence),
        descriptor_id: operation.descriptor_id,
        slot: operation.slot,
        generation: operation.generation,
        elapsed_micros: slot.elapsed(clock),
        kind,
      });
    }
  }

  pub(crate) fn sample(&mut self, world: &mut FakeWorld, ui: &mut UiWorld, now: u64, frame: u64) {
    let missing = self
      .entries
      .values()
      .filter(|entry| !entry.target.exists(world, ui))
      .map(|entry| entry.definition.host_id)
      .collect::<Vec<_>>();
    for host in missing {
      self.remove(host);
    }
    self.sample_playbacks(now);
    let graph_sampled = self
      .graph
      .sample(now, frame, |source| clock(&self.clocks, source, now));
    if graph_sampled {
      for entry in self.entries.values_mut() {
        for binding in &entry.definition.value_bindings {
          let reduced = entry.definition.reduced_motion == battlement::ReducedMotionPolicy::Always
            && motion_target::spatial(binding.property);
          let value = if reduced {
            motion_graph_bindings::reduced(
              binding.property,
              self.graph.destination(binding.value_id),
            )
          } else {
            motion_graph_bindings::adapt(binding.property, self.graph.value(binding.value_id))
          };
          if binding.composition == battlement::MotionBindingComposition::Compose {
            entry.target.set_contribution(value, world, ui);
          } else {
            entry.target.write(binding.property, value, world, ui);
          }
          if let Some(message) = entry.target.failure.take() {
            assert!(self.graph.fail(binding.value_id, &message), "{message}");
          }
        }
      }
    }
    let mut failed = Vec::new();
    for (id, entry) in &mut self.entries {
      let clock = clock(&self.clocks, entry.definition.clock, now);
      if entry.sampled == Some((clock, frame)) {
        continue;
      }
      entry.sampled = Some((clock, frame));
      entry.restore.begin(&mut entry.target, world, ui);
      for slot in &mut entry.slots {
        let elapsed = slot.elapsed(clock);
        for kind in slot.sample(
          &mut entry.target,
          world,
          ui,
          clock,
          entry.definition.reduced_motion,
        ) {
          self.sequence += 1;
          self.events.push(MotionLifecycleEvent {
            sequence: MotionSequence(self.sequence),
            descriptor_id: *id,
            slot: slot.definition.slot,
            generation: slot.definition.generation,
            elapsed_micros: elapsed,
            kind,
          });
        }
        if !slot.active {
          entry.restore.sample(
            slot,
            &mut entry.target,
            world,
            ui,
            clock,
            entry.definition.reduced_motion == battlement::ReducedMotionPolicy::Always,
          );
        }
        if slot.active && slot.definition.callbacks.update {
          self.samples.insert(
            (*id, slot.definition.slot),
            MotionPresentationSample {
              descriptor_id: *id,
              slot: slot.definition.slot,
              generation: slot.definition.generation,
              elapsed_micros: elapsed,
              values: slot
                .presentation
                .iter()
                .map(|(property, value)| MotionPropertyValue {
                  property: *property,
                  value: value.clone(),
                })
                .collect(),
            },
          );
        }
        if let Some(message) = entry.target.failure.take() {
          assert!(self.playbacks.owns_descriptor(*id), "{message}");
          failed.push((entry.definition.host_id, *id, message));
          self.samples.retain(|(descriptor, _), _| descriptor != id);
          break;
        }
      }
    }
    for (host, id, message) in failed {
      for address in self.playbacks.fail_descriptor(id, &message) {
        if let Some(entry) = self.entries.get_mut(&address.descriptor) {
          let clock = clock(&self.clocks, entry.definition.clock, now);
          if let Some(slot) = entry.slots.iter_mut().find(|slot| {
            slot.definition.slot == address.slot && slot.definition.generation == address.generation
          }) {
            slot.apply(MotionPlaybackCommand::Cancel, clock);
            entry.sampled = None;
          }
        }
      }
      self.remove(host);
    }
    self.sample_playbacks(now);
    if graph_sampled {
      self.graph.capture(
        self
          .entries
          .values()
          .flat_map(|entry| &entry.definition.value_subscriptions),
        frame,
      );
    }
  }

  pub(crate) fn handle(
    &mut self,
    event: &battlement::UiEvent,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
    now: u64,
  ) {
    self.gestures.handle(event, world, ui);
    self.sync_gestures(world, ui, now);
  }

  pub(crate) fn clear_gestures(&mut self, world: &mut FakeWorld, ui: &mut UiWorld, now: u64) {
    self.gestures = Gestures::default();
    self.sync_gestures(world, ui, now);
  }

  fn sync_gestures(&mut self, world: &mut FakeWorld, ui: &mut UiWorld, now: u64) {
    for entry in self.entries.values_mut() {
      if !entry.target.exists(world, ui) {
        continue;
      }
      let clock = clock(&self.clocks, entry.definition.clock, now);
      for slot in &mut entry.slots {
        if matches!(
          slot.definition.layer,
          battlement::MotionLayer::Animate | battlement::MotionLayer::Exit
        ) {
          continue;
        }
        let active = self
          .gestures
          .active(entry.definition.host_id, slot.definition.layer);
        if active == slot.active {
          continue;
        }
        entry
          .restore
          .activate(slot, active, &mut entry.target, world, ui, clock);
        slot.activate(active, &mut entry.target, world, ui, clock);
        entry.sampled = None;
      }
    }
  }

  pub(crate) fn drain(&mut self) -> Option<MotionEventBatch> {
    let has_samples = !self.events.is_empty() || !self.samples.is_empty();
    let has_graph = self.graph.has_events() || self.playbacks.has_events();
    if !has_samples && !has_graph {
      return None;
    }
    let events = std::mem::take(&mut self.events);
    Some(MotionEventBatch {
      first_sequence: events
        .first()
        .map_or(MotionSequence(self.sequence), |event| event.sequence),
      last_sequence: events
        .last()
        .map_or(MotionSequence(self.sequence), |event| event.sequence),
      events,
      samples: self.samples.drain().map(|(_, sample)| sample).collect(),
      value_samples: self.graph.drain_samples(),
      playback_events: self
        .playbacks
        .drain()
        .into_iter()
        .chain(self.graph.drain_events())
        .collect(),
      gesture_events: Vec::new(),
    })
  }

  fn clock(&self, source: MotionClockSource, now: u64) -> u64 {
    clock(&self.clocks, source, now)
  }
  fn emit(&mut self, id: ObjectId, slot: &Slot, kind: MotionEventKind, elapsed: u64) {
    self.sequence += 1;
    self.events.push(MotionLifecycleEvent {
      sequence: MotionSequence(self.sequence),
      descriptor_id: id,
      slot: slot.definition.slot,
      generation: slot.definition.generation,
      elapsed_micros: elapsed,
      kind,
    });
  }
}

fn clock(clocks: &HashMap<ObjectId, u64>, source: MotionClockSource, now: u64) -> u64 {
  match source {
    MotionClockSource::Controlled(id) => clocks.get(&id).copied().unwrap_or(0),
    MotionClockSource::Unscaled | MotionClockSource::Scaled => now,
    MotionClockSource::Audio(_) => 0,
  }
}

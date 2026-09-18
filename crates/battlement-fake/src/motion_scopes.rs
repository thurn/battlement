//! Scoped dependency graphs executed by the shared fake Motion sampler.

use std::collections::{HashMap, HashSet};

use battlement::{
  MotionClockSource, MotionPlaybackCommand, MotionPlaybackOutcome, MotionPositionReference,
  MotionProperty, MotionPropertyTarget, MotionPropertyTrack, MotionPropertyValue,
  MotionReferenceResolution, MotionRepeat, MotionScopeCommand, MotionScopeOperation,
  MotionSelector, MotionSequenceConflict, MotionSequenceEntry, MotionSequenceLabelEvent,
  MotionSequenceSchedule, MotionTargetDescriptor, MotionValue, ObjectId, validate_motion_sequence,
};
use battlement_ui_fake::UiWorld;

use crate::{
  motion::{MotionWorld, controls},
  motion_playbacks::{Address, RunningMotion},
  transform,
  world::FakeWorld,
};

#[derive(Clone)]
pub(crate) struct Sequence {
  playback_id: ObjectId,
  generation: u32,
  clock: MotionClockSource,
  anchor: u64,
  held: u64,
  speed: f64,
  paused: bool,
  entries: Vec<Entry>,
  labels: HashMap<String, usize>,
}

#[derive(Clone)]
struct Entry {
  definition: MotionSequenceEntry,
  targets: Vec<ObjectId>,
  captured: HashMap<ObjectId, Vec<MotionPropertyValue>>,
  captured_particle: Option<battlement::Vector3>,
  addresses: Vec<Address>,
  started: Option<u64>,
  completed: Option<u64>,
}

impl MotionWorld {
  pub(crate) fn scope(
    &mut self,
    operation: &MotionScopeOperation,
    blocking: bool,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
    now: u64,
  ) -> Option<RunningMotion> {
    let root = self
      .entries
      .values()
      .find(|entry| {
        entry.definition.scope_root && entry.definition.scope_id == Some(operation.scope_id)
      })?
      .definition
      .host_id;
    match &operation.command {
      MotionScopeCommand::Start {
        playback_id,
        generation,
        entries,
      } => {
        validate_motion_sequence(entries).expect("invalid Motion sequence graph");
        let mut prepared = Vec::with_capacity(entries.len());
        for definition in entries {
          if let MotionSequenceEntry::Particle { particle, .. } = definition {
            let mut entry = Entry::new(definition.clone(), Vec::new());
            self.validate_effect_position(&particle.position, world);
            if particle.position.resolution == MotionReferenceResolution::CaptureAtStart {
              entry.captured_particle = Some(self.effect_position(&particle.position, world));
            }
            prepared.push(entry);
            continue;
          }
          let MotionSequenceEntry::Animate {
            selector,
            target,
            position,
            position_transition,
            ..
          } = definition
          else {
            prepared.push(Entry::new(definition.clone(), Vec::new()));
            continue;
          };
          let targets = self.select(root, selector, world, ui);
          assert!(!targets.is_empty(), "Motion sequence target is absent");
          let mut entry = Entry::new(definition.clone(), targets);
          for id in &entry.targets {
            let resolved_position = position
              .as_ref()
              .map(|reference| self.resolve_position(*id, reference, world, ui));
            if let (Some(reference), Some(values)) = (position, &resolved_position)
              && reference.resolution == MotionReferenceResolution::CaptureAtStart
            {
              entry.captured.insert(*id, values.clone());
            }
            let resolved =
              target_with_position(target, position_transition, resolved_position.as_deref());
            controls::validate(&self.entries[id], &resolved, blocking);
          }
          prepared.push(entry);
        }
        validate_conflicts(entries, &prepared);
        let infinite = entries.iter().any(infinite);
        let running = self.playbacks.register(*playback_id, *generation, infinite);
        let descriptor_id = self.hosts[&root];
        let clock_source = self.entries[&descriptor_id].definition.clock;
        let clock = self.clock(clock_source, now);
        let labels = prepared
          .iter()
          .enumerate()
          .filter_map(|(index, entry)| match &entry.definition {
            MotionSequenceEntry::Label { name, .. } => Some((name.clone(), index)),
            _ => None,
          })
          .collect();
        self.sequences.insert(
          *playback_id,
          Sequence {
            playback_id: *playback_id,
            generation: *generation,
            clock: clock_source,
            anchor: clock,
            held: 0,
            speed: 1.0,
            paused: false,
            entries: prepared,
            labels,
          },
        );
        self.progress_sequence(*playback_id, world, ui, now);
        Some(running)
      }
      MotionScopeCommand::Set { selector, target } => {
        let selected = self.select(root, selector, world, ui);
        for id in &selected {
          controls::validate(&self.entries[id], target, false);
        }
        for id in selected {
          let entry = self.entries.get_mut(&id).unwrap();
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
      MotionScopeCommand::Stop(selector) => {
        for id in self.select(root, selector, world, ui) {
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
    }
  }

  pub(crate) fn progress_sequences(&mut self, world: &mut FakeWorld, ui: &mut UiWorld, now: u64) {
    let ids = self.sequences.keys().copied().collect::<Vec<_>>();
    for id in ids {
      self.progress_sequence(id, world, ui, now);
    }
  }

  fn progress_sequence(&mut self, id: ObjectId, world: &mut FakeWorld, ui: &mut UiWorld, now: u64) {
    let Some(mut sequence) = self.sequences.remove(&id) else {
      return;
    };
    if let Some(outcome) = self.sequence_terminal_outcome(&sequence) {
      self.clear_sequence_imperatives(&sequence, world, ui, now);
      self.playbacks.finish(sequence.playback_id, outcome);
      return;
    }
    let clock = self.clock(sequence.clock, now);
    self.follow_positions(&sequence, world, ui, clock);
    let elapsed = sequence.elapsed(clock);
    if !sequence.paused {
      let mut changed = true;
      while changed {
        changed = false;
        for index in 0..sequence.entries.len() {
          if sequence.entries[index].started.is_none()
            && sequence
              .eligible(index)
              .is_some_and(|value| value <= elapsed)
          {
            sequence.entries[index].started = Some(elapsed);
            match sequence.entries[index].definition.clone() {
              MotionSequenceEntry::Label { name, .. } => {
                self.label_events.push(MotionSequenceLabelEvent {
                  playback_id: sequence.playback_id,
                  generation: sequence.generation,
                  label: name,
                });
                sequence.entries[index].completed = Some(elapsed);
                changed = true;
              }
              MotionSequenceEntry::Animate {
                target,
                position,
                position_transition,
                ..
              } => {
                let targets = sequence.entries[index].targets.clone();
                for target_id in targets {
                  assert!(
                    self.entries.contains_key(&target_id),
                    "Motion sequence target was removed before it started"
                  );
                  let position_values = sequence.entries[index]
                    .captured
                    .get(&target_id)
                    .cloned()
                    .or_else(|| {
                      position
                        .as_ref()
                        .map(|reference| self.resolve_position(target_id, reference, world, ui))
                    });
                  let resolved =
                    target_with_position(&target, &position_transition, position_values.as_deref());
                  let (mut addresses, remaps) = self.install_imperatives_with_remaps(
                    target_id,
                    vec![(resolved, index as u64)],
                    sequence.generation,
                    world,
                    ui,
                    now,
                  );
                  for (old, replacement) in remaps {
                    for entry in &mut sequence.entries {
                      for address in &mut entry.addresses {
                        if *address == old {
                          *address = replacement;
                        }
                      }
                    }
                  }
                  let address = addresses.remove(0);
                  self.playbacks.attach(sequence.playback_id, address);
                  let valid = self
                    .playbacks
                    .get(sequence.playback_id)
                    .unwrap()
                    .addresses
                    .iter()
                    .copied()
                    .filter(|address| self.address_exists(*address))
                    .collect::<Vec<_>>();
                  self
                    .playbacks
                    .retain_addresses(sequence.playback_id, &valid);
                  sequence.entries[index].addresses.push(address);
                }
                if sequence.entries[index].addresses.is_empty() {
                  sequence.entries[index].completed = Some(elapsed);
                  changed = true;
                }
              }
              MotionSequenceEntry::Sound { sound, .. } => {
                self
                  .audio_occurrences
                  .push(crate::effects::AudioOccurrence {
                    command_id: occurrence_id(sequence.playback_id, index),
                    address: battlement::AudioClipAddress::from(sound.address),
                    volume: sound.volume,
                    pitch: sound.pitch,
                    looping: sound.looping,
                  });
                sequence.entries[index].completed = Some(elapsed);
                changed = true;
              }
              MotionSequenceEntry::Particle { particle, .. } => {
                let position = sequence.entries[index]
                  .captured_particle
                  .unwrap_or_else(|| self.effect_position(&particle.position, world));
                self
                  .particle_occurrences
                  .push(crate::effects::ParticleOccurrence {
                    command_id: occurrence_id(sequence.playback_id, index),
                    address: battlement::PrefabAddress::from(particle.address),
                    location: battlement::ParticleSpawnLocation::WorldPosition(position),
                    lifetime_ms: particle.lifetime_ms,
                  });
                sequence.entries[index].completed = Some(elapsed);
                changed = true;
              }
            }
          }
          if sequence.entries[index].started.is_some()
            && sequence.entries[index].completed.is_none()
            && self.sequence_entry_terminal(&sequence.entries[index])
          {
            sequence.entries[index].completed = Some(elapsed);
            changed = true;
          }
        }
      }
    }
    if sequence
      .entries
      .iter()
      .all(|entry| entry.completed.is_some())
    {
      self.clear_sequence_imperatives(&sequence, world, ui, now);
      self
        .playbacks
        .finish(sequence.playback_id, MotionPlaybackOutcome::Completed);
    } else {
      self.sequences.insert(id, sequence);
    }
  }

  fn clear_sequence_imperatives(
    &mut self,
    sequence: &Sequence,
    world: &FakeWorld,
    ui: &UiWorld,
    now: u64,
  ) {
    let addresses = sequence
      .entries
      .iter()
      .flat_map(|entry| &entry.addresses)
      .copied()
      .collect::<Vec<_>>();
    let clocks = &self.clocks;
    for descriptor in self.entries.values_mut() {
      let descriptor_id = descriptor.definition.descriptor_id;
      let released = descriptor
        .slots
        .iter()
        .filter(|slot| {
          addresses.contains(&Address {
            descriptor: descriptor_id,
            slot: slot.definition.slot,
            generation: slot.definition.generation,
          })
        })
        .flat_map(|slot| {
          slot
            .definition
            .target
            .tracks
            .iter()
            .map(|track| track.property)
            .chain(
              slot
                .definition
                .target
                .transition_end
                .iter()
                .map(|value| value.property),
            )
        })
        .collect::<HashSet<_>>();
      let clock = crate::motion::clock(clocks, descriptor.definition.clock, now);
      let target = &mut descriptor.target;
      for slot in descriptor.slots.iter_mut().filter(|slot| {
        slot.definition.slot.0 < u64::MAX - 2048
          && slot
            .definition
            .target
            .tracks
            .iter()
            .any(|track| released.contains(&track.property))
      }) {
        slot.retarget_from_presentation(target, world, ui, clock);
      }
      descriptor.slots.retain(|slot| {
        !addresses.contains(&Address {
          descriptor: descriptor_id,
          slot: slot.definition.slot,
          generation: slot.definition.generation,
        })
      });
      descriptor.sampled = None;
    }
  }

  fn follow_positions(
    &mut self,
    sequence: &Sequence,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
    now: u64,
  ) {
    for entry in &sequence.entries {
      let MotionSequenceEntry::Animate {
        position: Some(reference),
        ..
      } = &entry.definition
      else {
        continue;
      };
      if reference.resolution != MotionReferenceResolution::Follow
        || entry.started.is_none()
        || entry.completed.is_some()
      {
        continue;
      }
      for address in &entry.addresses {
        let values = self.resolve_position(address.descriptor, reference, world, ui);
        if let Some(descriptor) = self.entries.get_mut(&address.descriptor) {
          let clock = crate::motion::clock(&self.clocks, descriptor.definition.clock, now);
          if let Some(slot) = descriptor.slots.iter_mut().find(|slot| {
            slot.definition.slot == address.slot && slot.definition.generation == address.generation
          }) {
            slot.retarget_position(&values, &mut descriptor.target, world, ui, clock);
            descriptor.sampled = None;
          }
        }
      }
    }
  }

  fn sequence_entry_terminal(&self, entry: &Entry) -> bool {
    !entry.addresses.is_empty()
      && entry.addresses.iter().all(|address| {
        let descriptor = &self.entries[&address.descriptor];
        descriptor
          .slots
          .iter()
          .find(|slot| {
            slot.definition.slot == address.slot && slot.definition.generation == address.generation
          })
          .is_none_or(|slot| slot.outcome == Some(MotionPlaybackOutcome::Completed))
      })
  }

  fn address_exists(&self, address: Address) -> bool {
    self
      .entries
      .get(&address.descriptor)
      .is_some_and(|descriptor| {
        descriptor.slots.iter().any(|slot| {
          slot.definition.slot == address.slot && slot.definition.generation == address.generation
        })
      })
  }

  fn sequence_terminal_outcome(&self, sequence: &Sequence) -> Option<MotionPlaybackOutcome> {
    sequence
      .entries
      .iter()
      .flat_map(|entry| &entry.addresses)
      .filter_map(|address| {
        self
          .entries
          .get(&address.descriptor)
          .and_then(|descriptor| {
            descriptor
              .slots
              .iter()
              .find(|slot| {
                slot.definition.slot == address.slot
                  && slot.definition.generation == address.generation
              })
              .and_then(|slot| slot.outcome)
          })
      })
      .find(|outcome| *outcome != MotionPlaybackOutcome::Completed)
  }

  pub(crate) fn sequence_playback(
    &mut self,
    id: ObjectId,
    generation: u32,
    command: MotionPlaybackCommand,
    now: u64,
  ) {
    let Some(sequence) = self.sequences.get_mut(&id) else {
      return;
    };
    assert_eq!(
      sequence.generation, generation,
      "Motion sequence generation is stale"
    );
    let clock = crate::motion::clock(&self.clocks, sequence.clock, now);
    let elapsed = sequence.elapsed(clock);
    match command {
      MotionPlaybackCommand::Play => {
        if sequence.paused {
          sequence.anchor = clock;
          sequence.paused = false;
        }
      }
      MotionPlaybackCommand::Pause => {
        sequence.held = elapsed;
        sequence.paused = true;
      }
      MotionPlaybackCommand::SetSpeed { value } => {
        assert!(
          value.is_finite() && value >= 0.0,
          "invalid Motion playback speed"
        );
        sequence.held = elapsed;
        sequence.anchor = clock;
        sequence.speed = value;
        sequence.paused = value == 0.0;
      }
      MotionPlaybackCommand::Stop
      | MotionPlaybackCommand::Cancel
      | MotionPlaybackCommand::Complete => {
        self.sequences.remove(&id);
      }
      _ => {}
    }
  }

  fn resolve_position(
    &self,
    target: ObjectId,
    reference: &MotionPositionReference,
    world: &FakeWorld,
    ui: &UiWorld,
  ) -> Vec<MotionPropertyValue> {
    let reference_descriptor = self
      .hosts
      .get(&reference.object_id)
      .and_then(|id| self.entries.get(id))
      .expect("Motion sequence position reference is absent");
    let target_descriptor = &self.entries[&target];
    assert_eq!(
      target_descriptor.target.is_world(),
      reference_descriptor.target.is_world(),
      "Motion sequence positions cannot cross UI and world hosts"
    );
    if target_descriptor.target.is_world() {
      assert!(
        reference.anchor.is_none(),
        "fake Motion cannot resolve prepared world anchor metadata"
      );
      let desired = world.world_transform(reference.object_id);
      let parent = world
        .object(target_descriptor.definition.host_id)
        .and_then(|object| object.parent_id())
        .map(|id| world.world_transform(id));
      let local = transform::relative(parent, desired).position;
      return vec![
        property(MotionProperty::LocalPositionX, local.x),
        property(MotionProperty::LocalPositionY, local.y),
        property(MotionProperty::LocalPositionZ, local.z),
      ];
    }
    assert!(
      reference.anchor.is_none(),
      "UI Motion positions do not support named anchors"
    );
    [MotionProperty::X, MotionProperty::Y, MotionProperty::Z]
      .into_iter()
      .map(|property| MotionPropertyValue {
        property,
        value: ui
          .motion_value(reference.object_id, property)
          .unwrap_or(MotionValue::Length(battlement::Length::Px(0.0))),
      })
      .collect()
  }

  fn validate_effect_position(&self, reference: &MotionPositionReference, world: &FakeWorld) {
    assert!(
      world.object(reference.object_id).is_some(),
      "Motion particle position reference is absent"
    );
    assert!(
      reference.anchor.is_none(),
      "fake Motion cannot resolve prepared world anchor metadata"
    );
  }

  fn effect_position(
    &self,
    reference: &MotionPositionReference,
    world: &FakeWorld,
  ) -> battlement::Vector3 {
    self.validate_effect_position(reference, world);
    world.world_transform(reference.object_id).position
  }

  fn select(
    &self,
    root: ObjectId,
    selector: &MotionSelector,
    world: &FakeWorld,
    ui: &UiWorld,
  ) -> Vec<ObjectId> {
    self
      .entries
      .iter()
      .filter(|(_, entry)| {
        let host = entry.definition.host_id;
        match selector {
          MotionSelector::Element(id) => host == *id,
          MotionSelector::ScopeRoot => host == root,
          MotionSelector::Name(name) => {
            entry.definition.motion_name.as_ref() == Some(name) && contains(root, host, world, ui)
          }
          MotionSelector::Children => parent(host, world, ui) == Some(root),
          MotionSelector::Descendants => host != root && contains(root, host, world, ui),
        }
      })
      .map(|(id, _)| *id)
      .collect()
  }
}

impl Entry {
  fn new(definition: MotionSequenceEntry, targets: Vec<ObjectId>) -> Self {
    Self {
      definition,
      targets,
      captured: HashMap::new(),
      captured_particle: None,
      addresses: Vec::new(),
      started: None,
      completed: None,
    }
  }
}

impl Sequence {
  fn elapsed(&self, now: u64) -> u64 {
    self.held.saturating_add(if self.paused {
      0
    } else {
      ((now.saturating_sub(self.anchor) as f64) * self.speed).round_ties_even() as u64
    })
  }

  fn eligible(&self, index: usize) -> Option<u64> {
    match schedule(&self.entries[index].definition) {
      MotionSequenceSchedule::Absolute(value) => Some(*value),
      MotionSequenceSchedule::RelativeStart {
        entry,
        offset_micros,
      } => add(self.entries[*entry as usize].started?, *offset_micros),
      MotionSequenceSchedule::AfterCompletion {
        entry,
        offset_micros,
      } => add(self.entries[*entry as usize].completed?, *offset_micros),
      MotionSequenceSchedule::Label {
        name,
        offset_micros,
      } => add(self.entries[self.labels[name]].started?, *offset_micros),
    }
  }
}

fn target_with_position(
  target: &MotionTargetDescriptor,
  transition: &battlement::TransitionDefinition,
  values: Option<&[MotionPropertyValue]>,
) -> MotionTargetDescriptor {
  let mut resolved = target.clone();
  if let Some(values) = values {
    resolved
      .tracks
      .extend(values.iter().map(|value| MotionPropertyTrack {
        property: value.property,
        target: MotionPropertyTarget::Host,
        values: vec![value.value.clone()],
        transition: transition.clone(),
        times: None,
      }));
  }
  resolved
}

fn validate_conflicts(definitions: &[MotionSequenceEntry], entries: &[Entry]) {
  for later in 0..definitions.len() {
    let MotionSequenceEntry::Animate {
      target: right,
      position: right_position,
      conflict,
      ..
    } = &definitions[later]
    else {
      continue;
    };
    for earlier in 0..later {
      let MotionSequenceEntry::Animate {
        target: left,
        position: left_position,
        ..
      } = &definitions[earlier]
      else {
        continue;
      };
      if !entries[later]
        .targets
        .iter()
        .any(|target| entries[earlier].targets.contains(target))
      {
        continue;
      }
      let left_properties = properties(left, left_position.is_some());
      let right_properties = properties(right, right_position.is_some());
      let overlaps = !left_properties.is_disjoint(&right_properties);
      assert!(
        !overlaps
          || *conflict == MotionSequenceConflict::Replace
          || depends_on_completion(definitions, later, earlier, &mut HashSet::new()),
        "overlapping Motion sequence property writes require replacement"
      );
    }
  }
}

fn properties(
  target: &MotionTargetDescriptor,
  position: bool,
) -> HashSet<(MotionProperty, MotionPropertyTarget)> {
  let mut result = target
    .tracks
    .iter()
    .map(|track| (track.property, track.target.clone()))
    .collect::<HashSet<_>>();
  result.extend(
    target
      .transition_end
      .iter()
      .map(|value| (value.property, MotionPropertyTarget::Host)),
  );
  if position {
    for property in [
      MotionProperty::X,
      MotionProperty::Y,
      MotionProperty::Z,
      MotionProperty::LocalPositionX,
      MotionProperty::LocalPositionY,
      MotionProperty::LocalPositionZ,
    ] {
      result.insert((property, MotionPropertyTarget::Host));
    }
  }
  result
}

fn depends_on_completion(
  entries: &[MotionSequenceEntry],
  current: usize,
  expected: usize,
  visited: &mut HashSet<usize>,
) -> bool {
  if !visited.insert(current) {
    return false;
  }
  match schedule(&entries[current]) {
    MotionSequenceSchedule::AfterCompletion { entry, .. } => {
      *entry as usize == expected
        || depends_on_completion(entries, *entry as usize, expected, visited)
    }
    MotionSequenceSchedule::RelativeStart { entry, .. } => {
      depends_on_completion(entries, *entry as usize, expected, visited)
    }
    MotionSequenceSchedule::Label { name, .. } => entries
      .iter()
      .position(
        |entry| matches!(entry, MotionSequenceEntry::Label { name: value, .. } if value == name),
      )
      .is_some_and(|index| depends_on_completion(entries, index, expected, visited)),
    MotionSequenceSchedule::Absolute(_) => false,
  }
}

fn schedule(entry: &MotionSequenceEntry) -> &MotionSequenceSchedule {
  match entry {
    MotionSequenceEntry::Animate { schedule, .. }
    | MotionSequenceEntry::Label { schedule, .. }
    | MotionSequenceEntry::Sound { schedule, .. }
    | MotionSequenceEntry::Particle { schedule, .. } => schedule,
  }
}

fn occurrence_id(playback: ObjectId, index: usize) -> battlement::CommandId {
  let mut bytes = *playback.as_uuid().as_bytes();
  bytes[0] ^= 0xe6;
  let index = u32::try_from(index)
    .expect("sequence has too many entries")
    .to_be_bytes();
  for offset in 0..4 {
    bytes[12 + offset] ^= index[offset];
  }
  battlement::CommandId::from_bytes(bytes).expect("derived occurrence identity is nonzero")
}

fn infinite(entry: &MotionSequenceEntry) -> bool {
  matches!(
    entry,
    MotionSequenceEntry::Animate { target, position, position_transition, .. }
      if target.tracks.iter().any(|track| track.transition.repeat == MotionRepeat::Forever)
        || (position.is_some() && position_transition.repeat == MotionRepeat::Forever)
  )
}

fn add(value: u64, offset: i64) -> Option<u64> {
  Some(if offset >= 0 {
    value.saturating_add(offset as u64)
  } else {
    value.saturating_sub(offset.unsigned_abs())
  })
}

fn property(property: MotionProperty, value: f64) -> MotionPropertyValue {
  MotionPropertyValue {
    property,
    value: MotionValue::Scalar(value as f32),
  }
}

fn parent(host: ObjectId, world: &FakeWorld, ui: &UiWorld) -> Option<ObjectId> {
  world
    .object(host)
    .and_then(|object| object.parent_id())
    .or_else(|| ui.element(host).and_then(|element| element.parent_id()))
}

fn contains(root: ObjectId, mut host: ObjectId, world: &FakeWorld, ui: &UiWorld) -> bool {
  loop {
    if host == root {
      return true;
    }
    let Some(next) = parent(host, world, ui) else {
      return false;
    };
    host = next;
  }
}

//! Existing scoped targets share imperative slots and command-operation outcomes.

use battlement::{
  MotionPlaybackCommand, MotionPlaybackOutcome, MotionRepeat, MotionScopeCommand,
  MotionScopeOperation, MotionSelector, ObjectId,
};
use battlement_ui_fake::UiWorld;

use crate::{
  motion::{MotionWorld, controls},
  motion_playbacks::RunningMotion,
  world::FakeWorld,
};

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
        steps,
      } => {
        let mut selected = Vec::new();
        for (index, step) in steps.iter().enumerate() {
          let mut target = step.target.clone();
          for track in &mut target.tracks {
            track.transition.delay_micros = track
              .transition
              .delay_micros
              .checked_add(
                i64::try_from(step.start_micros).expect("Motion sequence delay overflow"),
              )
              .expect("Motion sequence delay overflow");
          }
          for id in self.select(root, &step.selector, world, ui) {
            controls::validate(&self.entries[&id], &target, blocking);
            selected.push((id, target.clone(), index as u64));
          }
        }
        let infinite = selected.iter().any(|(_, target, _)| {
          target
            .tracks
            .iter()
            .any(|track| track.transition.repeat == MotionRepeat::Forever)
        });
        let running = self.playbacks.register(*playback_id, *generation, infinite);
        let mut grouped = std::collections::HashMap::<_, Vec<_>>::new();
        for (id, target, offset) in &selected {
          grouped
            .entry(*id)
            .or_default()
            .push((target.clone(), *offset));
        }
        for (id, targets) in grouped {
          for address in self.install_imperatives(id, targets, *generation, world, ui, now) {
            self.playbacks.attach(*playback_id, address);
          }
        }
        if selected.is_empty() {
          self
            .playbacks
            .finish(*playback_id, MotionPlaybackOutcome::Completed);
        }
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

//! Bounded playback outcomes shared with the existing command-operation scheduler.

use std::{
  cell::{Cell, RefCell},
  collections::{HashMap, HashSet},
  rc::Rc,
};

use battlement::{
  MotionGeneration, MotionPlaybackEvent, MotionPlaybackOutcome, MotionSlotId, ObjectId,
};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct Address {
  pub(crate) descriptor: ObjectId,
  pub(crate) slot: MotionSlotId,
  pub(crate) generation: MotionGeneration,
}

#[derive(Clone)]
pub(crate) struct RunningMotion {
  outcome: Rc<Cell<Option<MotionPlaybackOutcome>>>,
  cancel: Rc<Cell<bool>>,
  failure: Rc<RefCell<Option<String>>>,
  pub(crate) infinite: bool,
}

impl RunningMotion {
  pub(crate) fn outcome(&self) -> Option<MotionPlaybackOutcome> {
    self.outcome.get()
  }

  pub(crate) fn failure(&self) -> Option<String> {
    self.failure.borrow().clone()
  }

  pub(crate) fn cancel(&self) {
    if self.outcome.get().is_none() {
      self.cancel.set(true);
    }
  }
}

pub(crate) struct Playback {
  pub(crate) generation: u32,
  pub(crate) addresses: Vec<Address>,
  pub(crate) running: RunningMotion,
  emit_event: bool,
}

#[derive(Default)]
pub(crate) struct Playbacks {
  values: HashMap<ObjectId, Playback>,
  events: Vec<MotionPlaybackEvent>,
}

impl Playbacks {
  pub(crate) fn register(
    &mut self,
    id: ObjectId,
    generation: u32,
    infinite: bool,
  ) -> RunningMotion {
    self.register_with_events(id, generation, infinite, true)
  }

  pub(crate) fn register_silent(
    &mut self,
    id: ObjectId,
    generation: u32,
    infinite: bool,
  ) -> RunningMotion {
    self.register_with_events(id, generation, infinite, false)
  }

  fn register_with_events(
    &mut self,
    id: ObjectId,
    generation: u32,
    infinite: bool,
    emit_event: bool,
  ) -> RunningMotion {
    self.finish(id, MotionPlaybackOutcome::Cancelled);
    let running = RunningMotion {
      outcome: Rc::new(Cell::new(None)),
      cancel: Rc::new(Cell::new(false)),
      failure: Rc::new(RefCell::new(None)),
      infinite,
    };
    self.values.insert(
      id,
      Playback {
        generation,
        addresses: Vec::new(),
        running: running.clone(),
        emit_event,
      },
    );
    running
  }

  pub(crate) fn get(&self, id: ObjectId) -> Option<&Playback> {
    self.values.get(&id)
  }

  pub(crate) fn attach(&mut self, id: ObjectId, address: Address) {
    let playback = self.values.get_mut(&id).expect("Motion playback is absent");
    playback
      .addresses
      .retain(|old| old.descriptor != address.descriptor || old.slot != address.slot);
    playback.addresses.push(address);
  }

  pub(crate) fn retain_addresses(&mut self, id: ObjectId, values: &[Address]) {
    if let Some(playback) = self.values.get_mut(&id) {
      playback
        .addresses
        .retain(|address| values.contains(address));
    }
  }

  pub(crate) fn remap(&mut self, remaps: &[(Address, Address)]) {
    for playback in self.values.values_mut() {
      for address in &mut playback.addresses {
        if let Some((_, replacement)) = remaps.iter().find(|(old, _)| old == address) {
          *address = *replacement;
        }
      }
    }
  }

  pub(crate) fn finish(&mut self, id: ObjectId, outcome: MotionPlaybackOutcome) {
    if let Some(playback) = self.values.remove(&id) {
      playback.running.outcome.set(Some(outcome));
      if playback.emit_event {
        self.events.push(MotionPlaybackEvent {
          playback_id: id,
          generation: playback.generation,
          outcome,
        });
      }
    }
  }

  pub(crate) fn fail(&mut self, id: ObjectId, message: &str) {
    if let Some(playback) = self.values.get(&id) {
      playback.running.failure.replace(Some(message.to_owned()));
    }
    self.finish(id, MotionPlaybackOutcome::Failed);
  }

  pub(crate) fn owns_descriptor(&self, descriptor: ObjectId) -> bool {
    self.values.values().any(|playback| {
      playback
        .addresses
        .iter()
        .any(|address| address.descriptor == descriptor)
    })
  }

  pub(crate) fn fail_descriptor(&mut self, descriptor: ObjectId, message: &str) -> Vec<Address> {
    let failed = self
      .values
      .iter()
      .filter(|(_, playback)| {
        playback
          .addresses
          .iter()
          .any(|address| address.descriptor == descriptor)
      })
      .map(|(id, playback)| {
        playback.running.failure.replace(Some(message.to_owned()));
        *id
      })
      .collect::<Vec<_>>();
    let addresses = failed
      .iter()
      .flat_map(|id| self.values[id].addresses.clone())
      .collect();
    for id in failed {
      self.finish(id, MotionPlaybackOutcome::Failed);
    }
    addresses
  }

  pub(crate) fn cancellations(&self) -> Vec<(ObjectId, Vec<Address>)> {
    self
      .values
      .iter()
      .filter(|(_, playback)| playback.running.cancel.get())
      .map(|(id, playback)| (*id, playback.addresses.clone()))
      .collect()
  }

  pub(crate) fn sample(
    &mut self,
    deferred: &HashSet<ObjectId>,
    mut outcome: impl FnMut(Address) -> Option<MotionPlaybackOutcome>,
  ) {
    let mut finished = Vec::new();
    for (id, playback) in &self.values {
      if deferred.contains(id) {
        continue;
      }
      if playback.addresses.is_empty() {
        continue;
      }
      let mut complete = true;
      let mut terminal = MotionPlaybackOutcome::Completed;
      for address in &playback.addresses {
        match outcome(*address) {
          None => complete = false,
          Some(MotionPlaybackOutcome::Completed) => {}
          Some(value) => {
            terminal = value;
            complete = true;
            break;
          }
        }
      }
      if complete {
        finished.push((*id, terminal));
      }
    }
    for (id, outcome) in finished {
      self.finish(id, outcome);
    }
  }

  pub(crate) fn drain(&mut self) -> Vec<MotionPlaybackEvent> {
    std::mem::take(&mut self.events)
  }

  pub(crate) fn has_events(&self) -> bool {
    !self.events.is_empty()
  }
}

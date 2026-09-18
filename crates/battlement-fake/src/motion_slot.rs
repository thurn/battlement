//! Playback identity and clock state independent of target adapters.

use battlement::{
  MotionEventKind, MotionLayer, MotionPlaybackCommand, MotionPlaybackDirection,
  MotionPlaybackOutcome, MotionProperty, MotionSlotDescriptor, MotionValue, ReducedMotionPolicy,
};
use battlement_ui_fake::UiWorld;

use crate::{
  motion_target::{self, Target},
  motion_track::Track,
  world::FakeWorld,
};

#[derive(Clone)]
pub(crate) struct Slot {
  pub(crate) definition: MotionSlotDescriptor,
  pub(crate) tracks: Vec<Track>,
  pub(crate) completion_tracks: Vec<Track>,
  pub(crate) active: bool,
  pub(crate) outcome: Option<MotionPlaybackOutcome>,
  pub(crate) anchor: u64,
  pub(crate) held: u64,
  pub(crate) speed: f64,
  pub(crate) paused: bool,
  scope_paused: bool,
  pub(crate) direction: MotionPlaybackDirection,
  pub(crate) presentation: Vec<(MotionProperty, MotionValue)>,
  started: bool,
  iteration: u32,
  seek: bool,
}

impl Slot {
  pub(crate) fn has_pending_finite_tracks(&self) -> bool {
    self.active
      && self.outcome.is_none()
      && self
        .tracks
        .iter()
        .chain(&self.completion_tracks)
        .any(|track| {
          !track.done && track.definition.transition.repeat != battlement::MotionRepeat::Forever
        })
  }

  pub(crate) fn has_pending_infinite_tracks(&self) -> bool {
    self.active
      && self.outcome.is_none()
      && self
        .tracks
        .iter()
        .chain(&self.completion_tracks)
        .any(|track| {
          !track.done && track.definition.transition.repeat == battlement::MotionRepeat::Forever
        })
  }

  pub(crate) fn new(definition: MotionSlotDescriptor, tracks: Vec<Track>, now: u64) -> Self {
    Self {
      active: matches!(definition.layer, MotionLayer::Animate | MotionLayer::Exit),
      definition,
      tracks,
      completion_tracks: Vec::new(),
      outcome: None,
      anchor: now,
      held: 0,
      speed: 1.0,
      paused: false,
      scope_paused: false,
      direction: MotionPlaybackDirection::Forward,
      presentation: Vec::new(),
      started: false,
      iteration: 0,
      seek: false,
    }
  }

  pub(crate) fn elapsed(&self, now: u64) -> u64 {
    self.held
      + if self.is_paused() {
        0
      } else {
        ((now.saturating_sub(self.anchor) as f64) * self.speed).round_ties_even() as u64
      }
  }

  pub(crate) fn is_paused(&self) -> bool {
    self.paused || self.scope_paused
  }

  pub(crate) fn pause_for_scope(&mut self, now: u64) {
    if self.outcome.is_some() || self.scope_paused {
      return;
    }
    if !self.paused {
      self.held = self.elapsed(now);
    }
    self.scope_paused = true;
  }

  pub(crate) fn resume_for_scope(&mut self, now: u64) {
    if !self.scope_paused {
      return;
    }
    self.scope_paused = false;
    if !self.paused {
      self.anchor = now;
    }
  }

  pub(crate) fn activate(
    &mut self,
    active: bool,
    target: &mut Target,
    world: &FakeWorld,
    ui: &UiWorld,
    now: u64,
  ) {
    if self.active == active {
      return;
    }
    self.active = active;
    if !active {
      return;
    }
    self.anchor = now;
    self.held = 0;
    self.paused = false;
    self.scope_paused = false;
    self.seek = false;
    self.outcome = None;
    self.started = false;
    self.iteration = 0;
    for track in &mut self.tracks {
      track.retarget(target.read(track.definition.property, world, ui));
    }
  }

  pub(crate) fn retarget_position(
    &mut self,
    values: &[battlement::MotionPropertyValue],
    target: &mut Target,
    world: &FakeWorld,
    ui: &UiWorld,
    now: u64,
  ) {
    let elapsed = self.elapsed(now);
    for value in values {
      if let Some(track) = self
        .tracks
        .iter_mut()
        .find(|track| track.definition.property == value.property)
      {
        track.retarget_destination(
          target.read(value.property, world, ui),
          value.value.clone(),
          elapsed,
        );
      }
    }
  }

  pub(crate) fn retarget_from_presentation(
    &mut self,
    target: &mut Target,
    world: &FakeWorld,
    ui: &UiWorld,
    now: u64,
  ) {
    self.anchor = now;
    self.held = 0;
    self.paused = false;
    self.scope_paused = false;
    self.seek = false;
    self.outcome = None;
    self.started = false;
    self.iteration = 0;
    self.presentation.clear();
    for track in &mut self.tracks {
      track.retarget(target.read(track.definition.property, world, ui));
    }
  }

  pub(crate) fn deadline(&self, now: u64) -> Option<u64> {
    if !self.active || self.outcome.is_some() {
      return None;
    }
    if self.is_paused() || self.speed == 0.0 {
      return None;
    }
    let elapsed = self.elapsed(now);
    self
      .tracks
      .iter()
      .chain(&self.completion_tracks)
      .filter(|track| !track.done)
      .filter_map(Track::duration)
      .map(|end| {
        now.saturating_add(((end.saturating_sub(elapsed) as f64 / self.speed).ceil() as u64).max(1))
      })
      .min()
  }

  pub(crate) fn apply(&mut self, command: MotionPlaybackCommand, now: u64) {
    if self.outcome.is_some() {
      return;
    }
    let elapsed = self.elapsed(now);
    match command {
      MotionPlaybackCommand::Play => {
        if self.paused {
          self.paused = false;
          if !self.scope_paused {
            self.anchor = now;
          }
        }
      }
      MotionPlaybackCommand::Pause => {
        if !self.scope_paused {
          self.held = elapsed;
        }
        self.paused = true;
      }
      MotionPlaybackCommand::SetSpeed { value } => {
        self.held = elapsed;
        self.anchor = now;
        self.speed = value;
        if value == 0.0 {
          self.paused = true;
        }
      }
      MotionPlaybackCommand::SetDirection { value } => {
        self.held = elapsed;
        self.anchor = now;
        self.direction = value;
      }
      MotionPlaybackCommand::Seek { elapsed_micros } => {
        self.held = elapsed_micros;
        self.paused = true;
        self.seek = true;
      }
      MotionPlaybackCommand::Replay => {
        self.anchor = now;
        self.held = 0;
        self.paused = false;
        self.scope_paused = false;
        self.outcome = None;
        self.started = false;
        self.iteration = 0;
        for track in &mut self.tracks {
          track.reset();
        }
      }
      MotionPlaybackCommand::Stop
      | MotionPlaybackCommand::Cancel
      | MotionPlaybackCommand::Complete => {
        self.held = elapsed;
        self.paused = true;
        if matches!(command, MotionPlaybackCommand::Complete) {
          self.presentation.clear();
        }
        self.outcome = Some(match command {
          MotionPlaybackCommand::Stop => MotionPlaybackOutcome::Stopped,
          MotionPlaybackCommand::Cancel => MotionPlaybackOutcome::Cancelled,
          _ => MotionPlaybackOutcome::Completed,
        });
      }
    }
  }

  pub(crate) fn sample(
    &mut self,
    target: &mut Target,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
    now: u64,
    policy: ReducedMotionPolicy,
  ) -> Vec<MotionEventKind> {
    if !self.active {
      return Vec::new();
    }
    let elapsed = self.elapsed(now);
    if let Some(outcome) = self.outcome {
      match outcome {
        MotionPlaybackOutcome::Cancelled => {}
        MotionPlaybackOutcome::Completed if self.presentation.is_empty() => {
          for track in &self.tracks {
            target.write(track.definition.property, track.target().clone(), world, ui);
          }
        }
        _ => {
          for (property, value) in &self.presentation {
            target.write(*property, value.clone(), world, ui);
          }
        }
      }
      if outcome == MotionPlaybackOutcome::Completed {
        for value in &self.definition.target.transition_end {
          target.write(value.property, value.value.clone(), world, ui);
        }
      }
      return Vec::new();
    }
    let reduced = policy == ReducedMotionPolicy::Always;
    self.presentation.clear();
    for track in &mut self.tracks {
      let mut value = track.sample(elapsed, self.direction);
      if reduced && motion_target::spatial(track.definition.property) {
        value = track.target().clone();
        track.velocity = 0.0;
        if track.definition.transition.repeat != battlement::MotionRepeat::Forever {
          track.done = true;
        }
      }
      target.write(track.definition.property, value.clone(), world, ui);
      self.presentation.push((track.definition.property, value));
    }
    for track in &mut self.completion_tracks {
      track.sample(elapsed, self.direction);
      if reduced && motion_target::spatial(track.definition.property) {
        track.done = track.definition.transition.repeat != battlement::MotionRepeat::Forever;
      }
    }
    let done = self
      .tracks
      .iter()
      .chain(&self.completion_tracks)
      .all(|track| track.done);
    if done {
      for value in &self.definition.target.transition_end {
        target.write(value.property, value.value.clone(), world, ui);
      }
    }
    if self.seek {
      self.seek = false;
      self.started = true;
      return Vec::new();
    }
    let mut events = Vec::new();
    let delay = self
      .tracks
      .iter()
      .map(|track| track.definition.transition.delay_micros)
      .min()
      .unwrap_or(0)
      .max(0) as u64;
    if !self.started && elapsed >= delay {
      self.started = true;
      if self.definition.callbacks.start {
        events.push(MotionEventKind::Started);
      }
    }
    let iteration = self
      .tracks
      .iter()
      .map(|track| track.iteration)
      .max()
      .unwrap_or(0);
    if iteration > self.iteration {
      if self.definition.callbacks.repeat {
        events.push(MotionEventKind::Repeated {
          first: self.iteration + 1,
          last: iteration,
        });
      }
      self.iteration = iteration;
    }
    if done && !self.is_paused() {
      self.outcome = Some(MotionPlaybackOutcome::Completed);
      if self.definition.callbacks.complete {
        events.push(MotionEventKind::Completed);
      }
    }
    events
  }
}

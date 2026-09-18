//! Motion-value playback using the same slot clock and property sampler.

use battlement::{
  MotionCallbackSubscriptions, MotionGeneration, MotionLayer, MotionPlaybackCommand,
  MotionPlaybackOutcome, MotionProperty, MotionPropertyTarget, MotionPropertyTrack,
  MotionSlotDescriptor, MotionSlotId, MotionTargetDescriptor, MotionValue, ObjectId,
  TransitionDefinition,
};

use crate::{motion_graph_node::Node, motion_slot::Slot, motion_track::Track};

pub(crate) struct Playback {
  pub(crate) value_id: ObjectId,
  pub(crate) slot: Slot,
}

impl Playback {
  pub(crate) fn new(
    value_id: ObjectId,
    generation: u32,
    origin: MotionValue,
    target: MotionValue,
    transition: TransitionDefinition,
    now: u64,
  ) -> Self {
    let track = MotionPropertyTrack {
      property: MotionProperty::Opacity,
      target: MotionPropertyTarget::Host,
      values: vec![target],
      times: None,
      transition,
    };
    Self {
      value_id,
      slot: Slot::new(
        MotionSlotDescriptor {
          slot: MotionSlotId(0),
          generation: MotionGeneration(generation),
          layer: MotionLayer::Animate,
          target: MotionTargetDescriptor {
            tracks: vec![track.clone()],
            transition_end: Vec::new(),
          },
          callbacks: MotionCallbackSubscriptions::default(),
        },
        vec![Track::new(track, origin, 0.0)],
        now,
      ),
    }
  }

  pub(crate) fn target(&self) -> &MotionValue {
    self.slot.tracks[0].target()
  }

  pub(crate) fn sample(&mut self, node: &mut Node, now: u64) -> bool {
    if self.slot.outcome.is_some() {
      return true;
    }
    let elapsed = self.slot.elapsed(now);
    let track = &mut self.slot.tracks[0];
    let value = track.sample(elapsed, self.slot.direction);
    if value.validate().is_err() {
      return false;
    }
    node.set(value, false);
    if track.done {
      self.slot.outcome = Some(MotionPlaybackOutcome::Completed);
    }
    true
  }

  pub(crate) fn apply(&mut self, command: MotionPlaybackCommand, node: &mut Node, now: u64) {
    self.slot.apply(command, now);
    match command {
      MotionPlaybackCommand::Cancel => node.set(self.slot.tracks[0].origin.clone(), true),
      MotionPlaybackCommand::Complete => node.set(self.target().clone(), true),
      MotionPlaybackCommand::Stop => node.stop(),
      MotionPlaybackCommand::SetDirection { .. } => {
        self.slot.held = 0;
        self.slot.anchor = now;
        self.slot.paused = false;
        self.slot.direction = battlement::MotionPlaybackDirection::Forward;
      }
      _ => {}
    }
  }
}

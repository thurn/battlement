//! Bounded shared value graph and command-operation handles for fake presentation.

use std::collections::{HashMap, HashSet};

use battlement::{
  MotionClockSource, MotionDescriptor, MotionPlaybackEvent, MotionPlaybackOutcome, MotionRepeat,
  MotionValue, MotionValueCommand, MotionValueDescriptor, MotionValueEventKind,
  MotionValueOperation, MotionValuePlaybackOperation, MotionValueSample, MotionValueSource,
  MotionValueSubscription, ObjectId,
};

use crate::{
  motion_graph_node::{self, Node},
  motion_graph_playback::Playback,
  motion_playbacks::{Playbacks, RunningMotion},
};

#[derive(Default)]
pub(crate) struct Graph {
  nodes: HashMap<ObjectId, Node>,
  order: Vec<ObjectId>,
  playbacks: HashMap<ObjectId, Playback>,
  statuses: Playbacks,
  samples: HashMap<ObjectId, MotionValueSample>,
  sampled: Option<(u64, u64)>,
  last_frame: Option<u64>,
}

impl Graph {
  pub(crate) fn rebuild<'a>(&mut self, descriptors: impl Iterator<Item = &'a MotionDescriptor>) {
    let mut definitions = HashMap::<ObjectId, &MotionValueDescriptor>::new();
    for descriptor in descriptors {
      for definition in &descriptor.values {
        if let Some(previous) = definitions.insert(definition.value_id, definition) {
          assert_eq!(
            previous, definition,
            "shared Motion value has incompatible definitions"
          );
        }
      }
    }
    self.nodes.retain(|id, _| definitions.contains_key(id));
    for (id, definition) in definitions {
      let compatible = self.nodes.get(&id).is_some_and(|node| {
        std::mem::discriminant(&node.definition.source)
          == std::mem::discriminant(&definition.source)
      });
      if !compatible {
        self.nodes.insert(id, Node::new(definition.clone()));
      }
    }
    let missing = self
      .playbacks
      .iter()
      .filter(|(_, playback)| !self.nodes.contains_key(&playback.value_id))
      .map(|(id, _)| *id)
      .collect::<Vec<_>>();
    for id in missing {
      self.playbacks.remove(&id);
      self.statuses.finish(id, MotionPlaybackOutcome::Cancelled);
    }
    self.order.clear();
    let mut visited = HashSet::new();
    for id in self.nodes.keys() {
      append(*id, &self.nodes, &mut visited, &mut self.order);
    }
    self.invalidate();
  }

  pub(crate) fn invalidate(&mut self) {
    self.sampled = None;
  }

  pub(crate) fn value(&self, id: ObjectId) -> &MotionValue {
    &self
      .nodes
      .get(&id)
      .expect("Motion graph value is absent")
      .value
  }

  pub(crate) fn destination(&self, id: ObjectId) -> &MotionValue {
    self
      .playbacks
      .values()
      .find(|playback| playback.value_id == id)
      .map_or_else(|| self.value(id), Playback::target)
  }

  pub(crate) fn apply(
    &mut self,
    operation: &MotionValueOperation,
    blocking: bool,
    now: u64,
  ) -> Option<RunningMotion> {
    assert!(
      matches!(
        self
          .nodes
          .get(&operation.value_id)
          .expect("Motion value is absent")
          .definition
          .source,
        MotionValueSource::Mutable
      ),
      "only mutable Motion values accept commands"
    );
    if let MotionValueCommand::Animate { transition, .. } = &operation.command {
      assert!(
        !blocking || transition.repeat != MotionRepeat::Forever,
        "infinite Motion cannot block a batch"
      );
    }
    self.invalidate();
    if matches!(
      operation.command,
      MotionValueCommand::Stop | MotionValueCommand::Animate { .. }
    ) {
      let previous = self
        .playbacks
        .iter()
        .filter(|(_, playback)| playback.value_id == operation.value_id)
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
      for id in previous {
        self.playbacks.remove(&id);
        self.statuses.finish(
          id,
          if matches!(operation.command, MotionValueCommand::Stop) {
            MotionPlaybackOutcome::Stopped
          } else {
            MotionPlaybackOutcome::Cancelled
          },
        );
      }
    }
    let node = self.nodes.get_mut(&operation.value_id).unwrap();
    match &operation.command {
      MotionValueCommand::Set(value) => node.set(value.clone(), false),
      MotionValueCommand::Jump(value) => node.set(value.clone(), true),
      MotionValueCommand::Stop => node.stop(),
      MotionValueCommand::Animate {
        playback_id,
        generation,
        target,
        transition,
      } => {
        let running = self.statuses.register(
          *playback_id,
          *generation,
          transition.repeat == MotionRepeat::Forever,
        );
        self.playbacks.insert(
          *playback_id,
          Playback::new(
            operation.value_id,
            *generation,
            node.value.clone(),
            *target.clone(),
            transition.clone(),
            now,
          ),
        );
        return Some(running);
      }
    }
    None
  }

  pub(crate) fn playback(&mut self, operation: MotionValuePlaybackOperation, now: u64) -> bool {
    let Some(playback) = self.playbacks.get_mut(&operation.playback_id) else {
      return false;
    };
    assert_eq!(
      playback.slot.definition.generation.0, operation.generation,
      "Motion playback generation is stale"
    );
    playback.apply(
      operation.command,
      self.nodes.get_mut(&playback.value_id).unwrap(),
      now,
    );
    self.invalidate();
    true
  }

  pub(crate) fn sample(
    &mut self,
    now: u64,
    frame: u64,
    mut clock: impl FnMut(MotionClockSource) -> u64,
  ) -> bool {
    if !self.statuses.cancellations().is_empty() {
      self.invalidate();
    }
    if self.sampled == Some((now, frame)) {
      return false;
    }
    self.sampled = Some((now, frame));
    for (id, _) in self.statuses.cancellations() {
      if let Some(playback) = self.playbacks.get_mut(&id) {
        playback.slot.outcome = Some(MotionPlaybackOutcome::Cancelled);
      }
    }
    for (id, playback) in &mut self.playbacks {
      if !playback.sample(self.nodes.get_mut(&playback.value_id).unwrap(), now) {
        self
          .statuses
          .fail(*id, "Motion produced a non-finite or invalid value.");
        playback.slot.outcome = Some(MotionPlaybackOutcome::Failed);
      }
    }
    for id in self.order.clone() {
      let mut node = self.nodes.remove(&id).unwrap();
      let valid = node.evaluate(&self.nodes, &mut clock);
      self.nodes.insert(id, node);
      if !valid {
        assert!(
          self.fail(id, "Motion graph produced a non-finite or invalid value."),
          "unowned Motion graph produced an invalid value"
        );
      }
    }
    true
  }

  pub(crate) fn fail(&mut self, id: ObjectId, message: &str) -> bool {
    let mut affected = HashSet::new();
    include(id, &self.nodes, &mut affected);
    let mut handled = false;
    for (id, playback) in &mut self.playbacks {
      if affected.contains(&playback.value_id) {
        playback.slot.outcome = Some(MotionPlaybackOutcome::Failed);
        self.statuses.fail(*id, message);
        handled = true;
      }
    }
    handled
  }

  pub(crate) fn capture<'a>(
    &mut self,
    subscriptions: impl Iterator<Item = &'a MotionValueSubscription>,
    frame: u64,
  ) {
    for subscription in subscriptions {
      let node = &self.nodes[&subscription.value_id];
      let requested = match subscription.event {
        MotionValueEventKind::AnimationFrame => self.last_frame != Some(frame),
        MotionValueEventKind::Change | MotionValueEventKind::Velocity => {
          node.changed || node.discontinuity
        }
      };
      if requested {
        self.samples.insert(
          subscription.subscription_id,
          MotionValueSample {
            subscription_id: subscription.subscription_id,
            value_id: subscription.value_id,
            frame,
            value: node.value.clone(),
            velocity: node.velocity.clone(),
            discontinuity: node.discontinuity,
          },
        );
      }
    }
    self.last_frame = Some(frame);
    for node in self.nodes.values_mut() {
      node.changed = false;
      node.discontinuity = false;
    }
    self.playbacks.retain(|id, playback| {
      if let Some(outcome) = playback.slot.outcome {
        self.statuses.finish(*id, outcome);
        false
      } else {
        true
      }
    });
  }

  pub(crate) fn deadline(&self, now: u64) -> Option<u64> {
    self
      .playbacks
      .values()
      .filter_map(|playback| playback.slot.deadline(now))
      .chain(self.nodes.values().filter_map(|node| node.deadline(now)))
      .min()
  }

  pub(crate) fn rebase(&mut self, now: u64) {
    for node in self.nodes.values_mut() {
      node.rebase(now);
    }
    for playback in self.playbacks.values_mut() {
      playback.slot.held = playback.slot.elapsed(now);
      playback.slot.anchor = 0;
    }
    self.last_frame = None;
    self.invalidate();
  }

  pub(crate) fn has_events(&self) -> bool {
    !self.samples.is_empty() || self.statuses.has_events()
  }
  pub(crate) fn drain_samples(&mut self) -> Vec<MotionValueSample> {
    self.samples.drain().map(|(_, sample)| sample).collect()
  }
  pub(crate) fn drain_events(&mut self) -> Vec<MotionPlaybackEvent> {
    self.statuses.drain()
  }
}

fn append(
  id: ObjectId,
  nodes: &HashMap<ObjectId, Node>,
  visited: &mut HashSet<ObjectId>,
  order: &mut Vec<ObjectId>,
) {
  if !visited.insert(id) {
    return;
  }
  for source in motion_graph_node::dependencies(&nodes[&id].definition.source) {
    append(source, nodes, visited, order);
  }
  order.push(id);
}

fn include(id: ObjectId, nodes: &HashMap<ObjectId, Node>, affected: &mut HashSet<ObjectId>) {
  if !affected.insert(id) {
    return;
  }
  for source in motion_graph_node::dependencies(&nodes[&id].definition.source) {
    include(source, nodes, affected);
  }
}

use std::collections::{HashMap, HashSet};

use battlement::{MotionDescriptor, MotionGeneration, MotionValueSubscription, ObjectId, Prop};

use crate::{host_node::HostNode, portal::PortalLayout, render::RenderTree, render_facade};

pub(crate) fn prepare(trees: &mut [RenderTree], previous: &PortalLayout) {
  let mut observations = HashMap::new();
  for tree in trees.iter() {
    self::collect(tree, &mut observations);
  }
  let mut previous_descriptors = HashMap::new();
  for root in &previous.roots {
    self::previous(&root.hosts, &mut previous_descriptors);
  }
  for root in previous.externals.values() {
    self::previous(&root.hosts, &mut previous_descriptors);
  }
  self::previous(&previous.objects, &mut previous_descriptors);
  let mut installed = HashSet::new();
  for tree in trees {
    self::install(tree, &observations, &previous_descriptors, &mut installed);
  }
  for observation in observations.values() {
    assert!(
      installed.contains(&observation.subscription_id),
      "use_motion_value_event requires a mounted native Motion graph owner for value {:?}; \
       bind this value or a dependent value to a host's Motion target or gesture. \
       A standalone observation does not install a graph",
      observation.value_id,
    );
  }
}

fn collect(tree: &RenderTree, observations: &mut HashMap<ObjectId, MotionValueSubscription>) {
  for position in &tree.positions {
    if let Some(component) = &position.component {
      for slot in &component.slots {
        if let Some(observation) = slot.motion_subscription() {
          observations.insert(observation.subscription_id, observation);
        }
      }
    }
    if let Some(suspense) = &position.suspense {
      self::collect(&suspense.primary, observations);
    }
    self::collect(&position.children, observations);
  }
}

fn install(
  tree: &mut RenderTree,
  observations: &HashMap<ObjectId, MotionValueSubscription>,
  previous: &HashMap<ObjectId, &MotionDescriptor>,
  installed: &mut HashSet<ObjectId>,
) {
  for position in &mut tree.positions {
    if let Some(host) = &mut position.host
      && let Prop::Set(descriptor) = host.motion_mut()
    {
      descriptor.value_subscriptions.clear();
      for observation in observations.values() {
        if descriptor
          .values
          .iter()
          .any(|value| value.value_id == observation.value_id)
        {
          descriptor.value_subscriptions.push(*observation);
          installed.insert(observation.subscription_id);
        }
      }
      descriptor
        .value_subscriptions
        .sort_by_key(|value| value.subscription_id);
      if let Some(prior) = previous.get(&descriptor.host_id)
        && descriptor.value_subscriptions != prior.value_subscriptions
        && descriptor.generation <= prior.generation
      {
        render_facade::set_motion_generation(
          descriptor,
          MotionGeneration(
            prior
              .generation
              .0
              .checked_add(1)
              .expect("motion generation exhausted"),
          ),
        );
      }
    }
    if let Some(suspense) = &mut position.suspense {
      self::install(&mut suspense.primary, observations, previous, installed);
    }
    self::install(&mut position.children, observations, previous, installed);
  }
}

fn previous<'a>(hosts: &'a [HostNode], descriptors: &mut HashMap<ObjectId, &'a MotionDescriptor>) {
  for host in hosts {
    if let Some(descriptor) = host.motion_descriptor() {
      descriptors.insert(host.object_id, descriptor);
    }
    self::previous(&host.children, descriptors);
  }
}

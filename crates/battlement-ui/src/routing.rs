use battlement_types::ObjectId;

use crate::{
  UiDocument, UiEvent, UiEventPhase, UiEventSubscription, UiNode, UiVisualElement,
  UiVisualElementProperties,
};

/// One logical subscriber selected for a native UI event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiEventDelivery {
  /// Identity of the subscribed logical element.
  pub object_id: ObjectId,
  /// Route phase that selected the subscription.
  pub phase: UiEventPhase,
}

/// Routes one native event through the current Rust-authored document tree.
///
/// Strict ancestors receive trickle deliveries from root to target and bubble
/// deliveries from target to root. The origin receives only a target delivery.
#[must_use]
pub fn route_event(documents: &[UiDocument], event: &UiEvent) -> Vec<UiEventDelivery> {
  let Some(route) = documents
    .iter()
    .find_map(|document| route_in_document(document, event))
  else {
    return Vec::new();
  };
  route_subscriptions(&route, event)
}

/// Routes an event across target-first identities and their current subscriptions.
#[doc(hidden)]
#[must_use]
pub fn route_subscriptions(
  route: &[(ObjectId, Vec<UiEventSubscription>)],
  event: &UiEvent,
) -> Vec<UiEventDelivery> {
  let kind = event.kind();
  if route.is_empty() || route[0].0 != event.target_id {
    return Vec::new();
  }
  if !kind.propagates() {
    return subscribed(&route[0], kind, UiEventPhase::Target)
      .into_iter()
      .collect();
  }
  let mut result = Vec::new();
  for node in route[1..].iter().rev() {
    if let Some(delivery) = subscribed(node, kind, UiEventPhase::Trickle) {
      result.push(delivery);
    }
  }
  if let Some(delivery) = subscribed(&route[0], kind, UiEventPhase::Target) {
    result.push(delivery);
  }
  for node in &route[1..] {
    if let Some(delivery) = subscribed(node, kind, UiEventPhase::Bubble) {
      result.push(delivery);
    }
  }
  result
}

fn route_in_document(
  document: &UiDocument,
  event: &UiEvent,
) -> Option<Vec<(ObjectId, Vec<UiEventSubscription>)>> {
  if document.root_id == event.target_id {
    return Some(vec![(document.root_id, subscriptions(&document.element))]);
  }
  for child in &document.children {
    if let Some(mut route) = route_in_node(child, event.target_id) {
      route.push((document.root_id, subscriptions(&document.element)));
      return Some(route);
    }
  }
  None
}

fn route_in_node(
  node: &UiNode,
  target_id: ObjectId,
) -> Option<Vec<(ObjectId, Vec<UiEventSubscription>)>> {
  if node.object_id == target_id {
    return Some(vec![(
      node.object_id,
      subscriptions(node.element.visual_element()),
    )]);
  }
  for child in &node.children {
    if let Some(mut route) = route_in_node(child, target_id) {
      route.push((node.object_id, subscriptions(node.element.visual_element())));
      return Some(route);
    }
  }
  None
}

fn subscriptions(value: &UiVisualElement) -> Vec<UiEventSubscription> {
  value
    .events
    .set_value()
    .into_iter()
    .flatten()
    .copied()
    .map(UiEventSubscription::target)
    .chain(
      value
        .event_subscriptions
        .set_value()
        .into_iter()
        .flatten()
        .copied(),
    )
    .collect()
}

fn subscribed(
  node: &(ObjectId, Vec<UiEventSubscription>),
  kind: crate::UiEventKind,
  phase: UiEventPhase,
) -> Option<UiEventDelivery> {
  node
    .1
    .contains(&UiEventSubscription::new(kind, phase))
    .then_some(UiEventDelivery {
      object_id: node.0,
      phase,
    })
}

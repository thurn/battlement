//! Runtime document rendering and event-coverage mutations.

use battlement::{Command, ObjectId, Prop, UiDocument, UiVisualElement};

use crate::{portal::PortalRoot, ui_host_adapter};

pub(crate) fn validate_subscriptions(document: &UiDocument) {
  let authored_events = matches!(&document.element.events, Prop::Set(values) if !values.is_empty());
  let authored_routes =
    matches!(&document.element.event_subscriptions, Prop::Set(values) if !values.is_empty());
  assert!(
    !authored_events && !authored_routes,
    "Reactant owns native event subscriptions"
  );
}

pub(crate) fn render(document: &UiDocument, physical: &PortalRoot) -> UiDocument {
  let mut document = document.clone();
  if !physical.subscriptions.is_empty() {
    document.element.event_subscriptions = Prop::Set(physical.subscriptions.clone());
  }
  document.children = ui_host_adapter::to_ui_nodes(&physical.hosts);
  document
}

pub(crate) fn with_coverage_barrier(
  root_id: ObjectId,
  previous: &PortalRoot,
  desired: &PortalRoot,
  mut groups: Vec<Vec<Command>>,
) -> Vec<Vec<Command>> {
  let mut coverage = self::coverage_groups(root_id, previous, desired);
  if coverage.is_empty() {
    return groups;
  }
  if desired.subscriptions.is_empty() {
    groups.append(&mut coverage);
    groups
  } else {
    coverage.append(&mut groups);
    coverage
  }
}

fn coverage_groups(
  root_id: ObjectId,
  previous: &PortalRoot,
  desired: &PortalRoot,
) -> Vec<Vec<Command>> {
  if previous.subscriptions == desired.subscriptions {
    return Vec::new();
  }
  let mut patch = UiVisualElement::new();
  patch.event_subscriptions = if desired.subscriptions.is_empty() {
    Prop::Reset
  } else {
    Prop::Set(desired.subscriptions.clone())
  };
  vec![vec![Command::update_visual_element(root_id, patch)]]
}

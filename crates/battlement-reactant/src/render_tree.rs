//! Committed render-tree traversal and scheduling.

use std::{any::TypeId, collections::HashSet, rc::Rc};

use battlement::{ObjectId, UiNode};

use crate::{
  effect::EffectOperation,
  error_boundary::ErrorReport,
  hook_storage::HookOwner,
  portal::PortalTarget,
  render::{EventNode, RenderPosition, RenderTree},
  suspense::SuspenseState,
};

impl RenderTree {
  pub(crate) fn hosts(&self) -> Vec<UiNode> {
    let mut hosts = Vec::new();
    self.append_hosts(&mut hosts);
    hosts
  }

  pub(crate) fn event_path(&self, target_id: ObjectId) -> Option<Vec<EventNode>> {
    let mut path = Vec::new();
    self.find_event_path(target_id, &mut path).then_some(path)
  }

  pub(crate) fn validate_model(&self, model: TypeId) {
    for position in &self.positions {
      assert!(
        position
          .handlers
          .iter()
          .all(|handler| handler.model() == model),
        "Reactant handler model type does not match its runtime"
      );
      if let Some(boundary) = &position.error_boundary {
        assert!(
          boundary
            .report
            .as_ref()
            .is_none_or(|report| report.model() == model),
          "Reactant error handler model type does not match its runtime"
        );
      }
      position.children.validate_model(model);
    }
  }

  pub(crate) fn remount_changed_portals(&mut self, targets: &HashSet<PortalTarget>) {
    for position in &mut self.positions {
      if position
        .portal
        .as_ref()
        .is_some_and(|target| targets.contains(target))
      {
        position.children.remount_hosts();
      } else {
        position.children.remount_changed_portals(targets);
      }
    }
  }

  pub(crate) fn commit_hooks(&mut self) {
    for position in &mut self.positions {
      if let Some(component) = &mut position.component {
        component.commit();
      }
      if let Some(suspense) = &position.suspense {
        suspense.commit();
      }
      position.children.commit_hooks();
    }
  }

  pub(crate) fn freeze_store_wakes(&mut self) {
    for position in &mut self.positions {
      if let Some(component) = &mut position.component {
        component.freeze_store_wakes();
      }
      position.children.freeze_store_wakes();
    }
  }

  pub(crate) fn hook_owners(&self, owners: &mut Vec<Rc<HookOwner>>) {
    for position in &self.positions {
      if let Some(component) = &position.component {
        owners.push(component.owner());
      }
      position.children.hook_owners(owners);
    }
  }

  pub(crate) fn take_effect_operations(&mut self, operations: &mut Vec<EffectOperation>) {
    for position in &mut self.positions {
      position.children.take_effect_operations(operations);
      if let Some(component) = &mut position.component {
        component.take_effect_operations(operations);
      }
    }
  }

  pub(crate) fn take_error_reports(&mut self, reports: &mut Vec<ErrorReport>) {
    for position in &mut self.positions {
      if let Some(report) = position
        .error_boundary
        .as_mut()
        .and_then(|boundary| boundary.report.take())
      {
        reports.push(report);
      }
      position.children.take_error_reports(reports);
    }
  }
}

impl RenderPosition {
  pub(crate) fn host_id(&self) -> ObjectId {
    self
      .host
      .as_ref()
      .expect("Reactant portal targets require a host render value")
      .object_id
  }

  pub(crate) fn has_dirty_work(&self) -> bool {
    if self.suspense.as_ref().is_some_and(SuspenseState::dirty) {
      return true;
    }
    let component_dirty = self
      .component
      .as_ref()
      .is_some_and(|component| component.has_pending() || component.context_changed());
    if component_dirty {
      return true;
    }
    self.provider.as_ref().map_or_else(
      || self.children.has_dirty_work(),
      |provider| provider.enter(|| self.children.has_dirty_work()),
    )
  }

  pub(crate) fn adapter_host_mut(&mut self) -> &mut Self {
    if self.host.is_some() {
      return self;
    }
    assert_eq!(
      self.children.positions.len(),
      1,
      "Reactant host adapters require one host render position"
    );
    self.children.positions[0].adapter_host_mut()
  }
}

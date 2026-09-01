//! Committed render-tree traversal and scheduling.

use std::{
  any::{Any, TypeId},
  collections::{HashMap, HashSet},
  rc::Rc,
};

use battlement::{
  MotionDragConstraint, MotionEventKind, MotionGestureEvent, MotionLifecycleEvent,
  MotionPresentationSample, ObjectId, OverlayPlacement, Prop, UiNode, UiVisualElementProperties,
};

use crate::{
  effect::EffectOperation,
  element_ref::AttachmentSet,
  error_boundary::ErrorReport,
  event_dispatch::EventNode,
  geometry::GeometryTarget,
  geometry_effect::GeometryEffectOperation,
  geometry_runtime::GeometryRuntime,
  hook_storage::{HookComponent, HookOwner},
  overlay::OverlayReference,
  portal::PortalTarget,
  render::{RenderPosition, RenderTree},
};

impl RenderTree {
  pub(crate) fn hosts(&self) -> Vec<UiNode> {
    let mut hosts = Vec::new();
    self.append_hosts(&mut hosts);
    hosts
  }

  pub(crate) fn resolve_drag_constraints(&mut self, runtime_id: u64, attachments: &AttachmentSet) {
    for position in &mut self.positions {
      if let Some(element_ref) = &position.drag_constraint_ref {
        let (_, target_id) = attachments
          .geometry_target(runtime_id, element_ref)
          .expect("drag constraint element ref is not attached in the rendered tree");
        let host = position
          .host
          .as_mut()
          .expect("drag constraints require a native host");
        let Prop::Set(descriptor) = &mut host.element.visual_element_mut().motion else {
          panic!("drag constraints require a Motion descriptor");
        };
        let gestures = descriptor
          .gestures
          .as_mut()
          .expect("drag constraints require a gesture descriptor");
        gestures
          .drag
          .as_mut()
          .expect("drag constraints require drag(axis)")
          .constraints = Some(MotionDragConstraint::Element(target_id));
      }
      position
        .children
        .resolve_drag_constraints(runtime_id, attachments);
    }
  }

  pub(crate) fn resolve_overlay_refs(&mut self, runtime_id: u64, attachments: &AttachmentSet) {
    for position in &mut self.positions {
      if let Some(reference) = &position.overlay_reference {
        let host = position
          .host
          .as_mut()
          .expect("overlay metadata requires a public wrapper host");
        host.element.visual_element_mut().overlay_placement = Prop::Set(match reference {
          OverlayReference::Popover { anchor, placement } => OverlayPlacement::Popover {
            anchor: attachments.reference_target(runtime_id, anchor),
            placement: *placement,
          },
          OverlayReference::Modal {
            initial_focus,
            restore_focus,
          } => OverlayPlacement::Modal {
            initial_focus: initial_focus
              .as_ref()
              .map(|value| attachments.reference_target(runtime_id, value)),
            restore_focus: restore_focus
              .as_ref()
              .map(|value| attachments.reference_target(runtime_id, value)),
          },
        });
      }
      position
        .children
        .resolve_overlay_refs(runtime_id, attachments);
    }
  }

  pub(crate) fn event_path(&self, target_id: ObjectId) -> Option<Vec<EventNode>> {
    let mut path = Vec::new();
    self.find_event_path(target_id, &mut path).then_some(path)
  }

  pub(crate) fn pending_hook_lengths(&self, lengths: &mut Vec<usize>) {
    for position in &self.positions {
      if let Some(component) = &position.component {
        component.pending_lengths(lengths);
      }
      if let Some(suspense) = &position.suspense {
        suspense.primary.pending_hook_lengths(lengths);
      }
      position.children.pending_hook_lengths(lengths);
    }
  }

  pub(crate) fn truncate_pending_hooks(&self, lengths: &[usize], cursor: &mut usize) {
    for position in &self.positions {
      if let Some(component) = &position.component {
        component.truncate_pending(lengths, cursor);
      }
      if let Some(suspense) = &position.suspense {
        suspense.primary.truncate_pending_hooks(lengths, cursor);
      }
      position.children.truncate_pending_hooks(lengths, cursor);
    }
  }

  pub(crate) fn unmount_effects(
    &mut self,
    mounted: &[Rc<HookOwner>],
    operations: &mut Vec<EffectOperation>,
  ) {
    for position in &mut self.positions {
      if let Some(suspense) = &mut position.suspense {
        suspense.primary.unmount_effects(mounted, operations);
      }
      position.children.unmount_effects(mounted, operations);
      let Some(component) = &mut position.component else {
        continue;
      };
      if !mounted
        .iter()
        .any(|candidate| component.owner.same(candidate))
      {
        component.unmount(operations);
      }
    }
  }

  pub(crate) fn unmount_all_effects(&mut self, operations: &mut Vec<EffectOperation>) {
    self.unmount_effects(&[], operations);
  }

  pub(crate) fn unmount_geometry_effects(
    &mut self,
    mounted: &[Rc<HookOwner>],
    operations: &mut Vec<GeometryEffectOperation>,
  ) {
    for position in &mut self.positions {
      if let Some(suspense) = &mut position.suspense {
        suspense
          .primary
          .unmount_geometry_effects(mounted, operations);
      }
      position
        .children
        .unmount_geometry_effects(mounted, operations);
      let Some(component) = &mut position.component else {
        continue;
      };
      if !mounted
        .iter()
        .any(|candidate| component.owner.same(candidate))
      {
        component.unmount_geometry_effects(operations);
      }
    }
  }

  pub(crate) fn unmount_all_geometry_effects(
    &mut self,
    operations: &mut Vec<GeometryEffectOperation>,
  ) {
    self.unmount_geometry_effects(&[], operations);
  }

  pub(crate) fn has_pending_hooks(&self) -> bool {
    self.positions.iter().any(|position| {
      let suspense_pending = position
        .suspense
        .as_ref()
        .is_some_and(|suspense| suspense.dirty() || suspense.primary_pending_changed());
      if suspense_pending {
        return true;
      }
      position
        .component
        .as_ref()
        .is_some_and(HookComponent::has_pending)
        || position.children.has_pending_hooks()
    })
  }

  pub(crate) fn has_dirty_work(&self) -> bool {
    self.positions.iter().any(RenderPosition::has_dirty_work)
  }

  pub(crate) fn has_changed_hooks(&self) -> bool {
    self.positions.iter().any(|position| {
      let suspense_changed = position
        .suspense
        .as_ref()
        .is_some_and(|suspense| suspense.dirty() || suspense.primary_pending_changed());
      if suspense_changed {
        return true;
      }
      position
        .component
        .as_ref()
        .is_some_and(HookComponent::has_pending_change)
        || position.children.has_changed_hooks()
    })
  }

  pub(crate) fn discard_pending_hooks(&mut self) {
    for position in &mut self.positions {
      if let Some(component) = &mut position.component {
        component.discard_pending();
      }
      if let Some(suspense) = &mut position.suspense {
        suspense.primary.discard_pending_hooks();
      }
      position.children.discard_pending_hooks();
    }
  }

  pub(crate) fn stabilize_element_hosts(&mut self, object_ids: &HashMap<u64, ObjectId>) {
    for position in &mut self.positions {
      if let (Some(element_ref), Some(host)) = (&position.element_ref, &mut position.host) {
        if let Some(object_id) = object_ids.get(&element_ref.identity()) {
          host.object_id = *object_id;
        }
      }
      if let Some(suspense) = &mut position.suspense {
        suspense.primary.stabilize_element_hosts(object_ids);
      }
      position.children.stabilize_element_hosts(object_ids);
    }
  }

  pub(crate) fn append_hosts(&self, hosts: &mut Vec<UiNode>) {
    for position in &self.positions {
      if let Some(host) = &position.host {
        hosts.push(host.clone());
      } else {
        position.children.append_hosts(hosts);
      }
    }
  }

  pub(crate) fn remount_hosts(&mut self) {
    for position in &mut self.positions {
      if let Some(host) = &mut position.host {
        host.object_id = ObjectId::new_v4();
      }
      position.children.remount_hosts();
    }
  }

  pub(crate) fn find_event_path(&self, target_id: ObjectId, path: &mut Vec<EventNode>) -> bool {
    for position in &self.positions {
      if let Some(host) = &position.host {
        path.push(EventNode {
          object_id: host.object_id,
          handlers: position.handlers.clone(),
        });
        if host.object_id == target_id {
          return true;
        }
        if position.suspense.as_ref().is_some_and(|suspense| {
          suspense.showing_fallback && suspense.primary.find_hidden_event_path(target_id, path)
        }) {
          return true;
        }
        if position.children.find_event_path(target_id, path) {
          return true;
        }
        path.pop();
      } else {
        if position.suspense.as_ref().is_some_and(|suspense| {
          suspense.showing_fallback && suspense.primary.find_hidden_event_path(target_id, path)
        }) {
          return true;
        }
        if position.children.find_event_path(target_id, path) {
          return true;
        }
      }
    }
    false
  }

  fn find_hidden_event_path(&self, target_id: ObjectId, path: &mut Vec<EventNode>) -> bool {
    for position in &self.positions {
      if let Some(host) = &position.host {
        path.push(EventNode {
          object_id: host.object_id,
          handlers: Vec::new(),
        });
        if host.object_id == target_id {
          return true;
        }
        if position
          .suspense
          .as_ref()
          .is_some_and(|suspense| suspense.primary.find_hidden_event_path(target_id, path))
        {
          return true;
        }
        if position.children.find_hidden_event_path(target_id, path) {
          return true;
        }
        path.pop();
      } else {
        if position
          .suspense
          .as_ref()
          .is_some_and(|suspense| suspense.primary.find_hidden_event_path(target_id, path))
        {
          return true;
        }
        if position.children.find_hidden_event_path(target_id, path) {
          return true;
        }
      }
    }
    false
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
      position.motion_callbacks.validate_model(model);
      for registration in &position.motion_callback_history {
        registration.validate_model(model);
      }
      if let Some(boundary) = &position.error_boundary {
        assert!(
          boundary
            .report
            .as_ref()
            .is_none_or(|report| report.model() == model),
          "Reactant error handler model type does not match its runtime"
        );
      }
      if let Some(component) = &position.component {
        assert!(
          component.geometry_effect_model_matches(model),
          "Reactant geometry effect model type does not match its runtime"
        );
      }
      if let Some(presence) = &position.presence {
        assert!(
          presence
            .handler
            .as_ref()
            .is_none_or(|handler| handler.model() == model),
          "presence callback model type does not match its runtime"
        );
      }
      if let Some(suspense) = &position.suspense {
        suspense.primary.validate_model(model);
      }
      position.children.validate_model(model);
    }
  }

  pub(crate) fn apply_motion_event(&mut self, event: &MotionLifecycleEvent) -> bool {
    let mut changed = false;
    for position in &mut self.positions {
      if let Some(presence) = &mut position.presence {
        changed |= presence.apply(event);
      }
      if let Some(suspense) = &mut position.suspense {
        changed |= suspense.primary.apply_motion_event(event);
      }
      changed |= position.children.apply_motion_event(event);
    }
    changed
  }

  pub(crate) fn invoke_motion_event(
    &mut self,
    game: &mut dyn Any,
    event: &MotionLifecycleEvent,
  ) -> bool {
    let mut invoked = false;
    for position in &mut self.positions {
      let matches = position.host.as_ref().is_some_and(|host| {
        let Prop::Set(descriptor) = &host.element.visual_element().motion else {
          return false;
        };
        descriptor.descriptor_id == event.descriptor_id
          && descriptor
            .slots
            .iter()
            .any(|slot| slot.slot == event.slot && slot.generation == event.generation)
      });
      if matches {
        invoked |= position.motion_callbacks.invoke(game, event);
      } else if let Some(index) = position
        .motion_callback_history
        .iter()
        .position(|registration| registration.matches(event))
      {
        invoked |= position.motion_callback_history[index].invoke(game, event);
        if matches!(
          event.kind,
          MotionEventKind::Completed | MotionEventKind::Stopped | MotionEventKind::Cancelled
        ) {
          position.motion_callback_history.remove(index);
        }
      }
      if let Some(suspense) = &mut position.suspense {
        invoked |= suspense.primary.invoke_motion_event(game, event);
      }
      invoked |= position.children.invoke_motion_event(game, event);
    }
    invoked
  }

  pub(crate) fn invoke_motion_sample(
    &mut self,
    game: &mut dyn Any,
    sample: &MotionPresentationSample,
  ) -> bool {
    let mut invoked = false;
    for position in &mut self.positions {
      let matches = position.host.as_ref().is_some_and(|host| {
        let Prop::Set(descriptor) = &host.element.visual_element().motion else {
          return false;
        };
        descriptor.descriptor_id == sample.descriptor_id
          && descriptor
            .slots
            .iter()
            .any(|slot| slot.slot == sample.slot && slot.generation == sample.generation)
      });
      if matches {
        invoked |= position.motion_callbacks.invoke_sample(game, sample);
      }
      if let Some(suspense) = &mut position.suspense {
        invoked |= suspense.primary.invoke_motion_sample(game, sample);
      }
      invoked |= position.children.invoke_motion_sample(game, sample);
    }
    invoked
  }

  pub(crate) fn invoke_motion_gesture(
    &mut self,
    game: &mut dyn Any,
    event: &MotionGestureEvent,
  ) -> bool {
    let mut invoked = false;
    for position in &mut self.positions {
      let matches = position.host.as_ref().is_some_and(|host| {
        let Prop::Set(descriptor) = &host.element.visual_element().motion else {
          return false;
        };
        descriptor.descriptor_id == event.descriptor_id && descriptor.generation == event.generation
      });
      if matches {
        invoked |= position.motion_callbacks.invoke_gesture(game, event);
      }
      if let Some(suspense) = &mut position.suspense {
        invoked |= suspense.primary.invoke_motion_gesture(game, event);
      }
      invoked |= position.children.invoke_motion_gesture(game, event);
    }
    invoked
  }

  pub(crate) fn has_ready_presence(&self) -> bool {
    self.positions.iter().any(|position| {
      position
        .presence
        .as_ref()
        .is_some_and(|presence| presence.ready() && !presence.notified)
        || position
          .suspense
          .as_ref()
          .is_some_and(|suspense| suspense.primary.has_ready_presence())
        || position.children.has_ready_presence()
    })
  }

  pub(crate) fn invoke_ready_presence(&mut self, game: &mut dyn Any) -> bool {
    let mut invoked = false;
    for position in &mut self.positions {
      if let Some(presence) = &mut position.presence
        && presence.ready()
        && !presence.notified
      {
        if let Some(handler) = &presence.handler {
          handler.invoke(game);
        }
        presence.notified = true;
        invoked = true;
      }
      if let Some(suspense) = &mut position.suspense {
        invoked |= suspense.primary.invoke_ready_presence(game);
      }
      invoked |= position.children.invoke_ready_presence(game);
    }
    invoked
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
      if let Some(suspense) = &mut position.suspense {
        suspense.primary.freeze_store_wakes();
      }
      position.children.freeze_store_wakes();
    }
  }

  pub(crate) fn hook_owners(&self, owners: &mut Vec<Rc<HookOwner>>) {
    for position in &self.positions {
      if let Some(component) = &position.component {
        owners.push(component.owner());
      }
      if let Some(suspense) = &position.suspense {
        suspense.primary.hook_owners(owners);
      }
      position.children.hook_owners(owners);
    }
  }

  pub(crate) fn geometry_targets(&self, targets: &mut Vec<GeometryTarget>) {
    for position in &self.positions {
      if let Some(component) = &position.component {
        component.geometry_targets(targets);
      }
      if let Some(suspense) = &position.suspense {
        suspense.primary.geometry_targets(targets);
      }
      position.children.geometry_targets(targets);
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

  pub(crate) fn take_geometry_effect_operations(
    &mut self,
    runtime: &GeometryRuntime,
    operations: &mut Vec<GeometryEffectOperation>,
  ) {
    for position in &mut self.positions {
      if let Some(suspense) = &mut position.suspense {
        suspense
          .primary
          .take_geometry_effect_operations(runtime, operations);
      }
      position
        .children
        .take_geometry_effect_operations(runtime, operations);
      if let Some(component) = &mut position.component {
        component.take_geometry_effect_operations(runtime, operations);
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
    let suspense_dirty = self
      .suspense
      .as_ref()
      .is_some_and(|suspense| suspense.dirty() || suspense.primary_pending_changed());
    if suspense_dirty {
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
}

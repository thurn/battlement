//! Shared placement and logical children for typed world descriptions.

use battlement::{GameObject, GameObjectKind, LocalTransform, ObjectId, Quaternion, Vector3};
use reactant_core::{
  callback::Callback, component::Component, native_host::ObjectRef, render::Render,
};
use uuid::Uuid;

use crate::world::Group;

/// A typed world host with local placement and independently reconciled children.
#[derive(Clone)]
pub struct WorldObject<P> {
  pub(crate) properties: P,
  pub(crate) group: Group,
  describe: fn(&P) -> GameObjectKind,
}

impl<P> WorldObject<P> {
  pub(crate) fn with_properties(properties: P, describe: fn(&P) -> GameObjectKind) -> Self {
    Self {
      properties,
      group: Group::new(),
      describe,
    }
  }

  /// Applies a prepared material and typed renderer-local values to one slot.
  pub fn material(mut self, material: battlement::MaterialInstance) -> Self {
    assert!(
      !self
        .group
        .material_instances
        .iter()
        .any(|m| m.slot == material.slot),
      "duplicate material slot"
    );
    self.group.material_instances.push(material);
    self
  }

  /// Preserves the compatible native host across logical parents and attachments.
  pub fn id(mut self, id: Uuid) -> Self {
    self.group = self.group.id(id);
    self
  }

  /// Appends a logical child with its own native lifetime.
  pub fn child(mut self, child: impl Render) -> Self {
    self.group = self.group.child(child);
    self
  }

  /// Sets the local transform relative to the physical parent.
  pub fn transform(mut self, transform: LocalTransform) -> Self {
    self.group = self.group.transform(transform);
    self
  }

  /// Sets the local position in world units.
  pub fn position(mut self, position: Vector3) -> Self {
    self.group = self.group.position(position);
    self
  }

  /// Sets local orientation; asset geometry is never implicitly reoriented.
  pub fn rotation(mut self, rotation: Quaternion) -> Self {
    self.group = self.group.rotation(rotation);
    self
  }

  /// Scales authored local geometry around its authored origin.
  pub fn scale(mut self, scale: Vector3) -> Self {
    self.group = self.group.scale(scale);
    self
  }

  /// Controls native activation while preserving component state.
  pub fn active(mut self, active: bool) -> Self {
    self.group = self.group.active(active);
    self
  }

  /// Uses the native pointer-following path and emits legacy drag core actions.
  pub fn draggable(mut self, mode: battlement::DragMode) -> Self {
    self.group = self.group.draggable(mode);
    self
  }

  /// Handles the start of this native host's drag gesture.
  pub fn on_drag_start(mut self, callback: impl Fn() + 'static) -> Self {
    self.group = self.group.on_drag_start(callback);
    self
  }

  /// Handles this native host's completed world-space drop.
  pub fn on_drag_end(mut self, callback: impl Fn(Vector3) + 'static) -> Self {
    self.group = self.group.on_drag_end(callback);
    self
  }

  /// Attaches a reference after the host commits.
  pub fn reference(mut self, reference: ObjectRef) -> Self {
    self.group = self.group.reference(reference);
    self
  }

  /// Installs typed logical pointer callbacks.
  pub fn events(mut self, events: crate::world::PointerHandlers) -> Self {
    self.group = self.group.events(events);
    self
  }
  /// Makes this host eligible for keyboard/controller navigation.
  pub fn focusable(mut self, focusable: bool) -> Self {
    self.group = self.group.focusable(focusable);
    self
  }
  /// Installs semantic activation, focus, and navigation callbacks.
  pub fn navigation(mut self, events: crate::world::NavigationHandlers) -> Self {
    self.group = self.group.navigation(events);
    self
  }
  /// Higher interaction layers win before visible depth is compared.
  pub fn interaction_layer(mut self, layer: i32) -> Self {
    self.group = self.group.interaction_layer(layer);
    self
  }
  /// Captures an unprevented primary press until release or capture loss.
  pub fn capture_on_press(mut self, capture: bool) -> Self {
    self.group = self.group.capture_on_press(capture);
    self
  }
  /// Attaches assistive activation to this visible object without changing dragging.
  pub fn accessible_button(mut self, name: trox::LocalizedString, callback: Callback<()>) -> Self {
    self.group = self.group.accessible_button(name, callback);
    self
  }

  /// Handles native or logical descendant activation.
  pub fn on_click(mut self, callback: Callback<()>) -> Self {
    self.group = self.group.on_click(callback);
    self
  }

  /// Converts a leaf description for static application objects, including the input camera.
  /// Logical children, callbacks, refs, and presentation IDs require rendering instead.
  pub fn into_object(self, object_id: ObjectId) -> GameObject {
    self.description().into_object(object_id)
  }

  pub(crate) fn description(&self) -> Group {
    let mut group = self.group.clone();
    group.kind = (self.describe)(&self.properties);
    group
  }
}

impl<P: Clone + 'static> Component for WorldObject<P> {
  fn render(&self) -> impl Render {
    self.description()
  }
}

impl<P: Clone + 'static> reactant_core::prelude::MotionComponent for WorldObject<P> {
  fn with_motion(mut self, motion: reactant_core::motion::MotionProps) -> Self {
    self.group = reactant_core::prelude::MotionComponent::with_motion(self.group, motion);
    self
  }
}

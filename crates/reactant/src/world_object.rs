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

  /// Attaches a reference after the host commits.
  pub fn reference(mut self, reference: ObjectRef) -> Self {
    self.group = self.group.reference(reference);
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

//! Type-erased host descriptions retained by the shared component tree.

use std::any::{Any, TypeId};

use battlement::{Command, ObjectId};

/// Implements one typed native-host catalog behind the heterogeneous render tree.
pub(crate) trait HostAdapter: 'static {
  type Description: Clone + 'static;

  fn requires_remount(previous: &Self::Description, desired: &Self::Description) -> bool;
  fn create_command(node: &HostNode, parent_id: ObjectId, child_index: u32) -> Command;
  fn property_command(
    object_id: ObjectId,
    previous: &Self::Description,
    desired: &Self::Description,
    hierarchy_changed: bool,
  ) -> Option<Command>;
  fn move_command(object_id: ObjectId, parent_id: ObjectId, child_index: u32) -> Command;
  fn index_command(object_id: ObjectId, child_index: u32) -> Command;
  fn destroy_command(object_id: ObjectId) -> Command;
  fn constrains_children(description: &Self::Description) -> bool;
}

/// One native host with an adapter-owned description and shared tree structure.
#[derive(Clone)]
pub(crate) struct HostNode {
  pub(crate) object_id: ObjectId,
  pub(crate) children: Vec<Self>,
  description: Box<dyn ErasedHostDescription>,
}

impl HostNode {
  pub(crate) fn new<A: HostAdapter>(object_id: ObjectId, description: A::Description) -> Self {
    Self {
      object_id,
      children: Vec::new(),
      description: Box::new(AdaptedHost::<A> { description }),
    }
  }

  pub(crate) fn description<A: HostAdapter>(&self) -> &A::Description {
    self
      .description
      .as_any()
      .downcast_ref::<AdaptedHost<A>>()
      .map(|host| &host.description)
      .expect("Reactant host was accessed through the wrong adapter")
  }

  pub(crate) fn description_mut<A: HostAdapter>(&mut self) -> &mut A::Description {
    self
      .description
      .as_any_mut()
      .downcast_mut::<AdaptedHost<A>>()
      .map(|host| &mut host.description)
      .expect("Reactant host was accessed through the wrong adapter")
  }

  pub(crate) fn without_children(&self) -> Self {
    let mut node = self.clone();
    node.children.clear();
    node
  }

  pub(crate) fn requires_remount(&self, desired: &Self) -> bool {
    if self.description.adapter_type() != desired.description.adapter_type() {
      return true;
    }
    self.description.requires_remount(&*desired.description)
  }

  pub(crate) fn create_command(&self, parent_id: ObjectId, child_index: u32) -> Command {
    self
      .description
      .create_command(self, parent_id, child_index)
  }

  pub(crate) fn property_command(
    &self,
    desired: &Self,
    hierarchy_changed: bool,
  ) -> Option<Command> {
    self.assert_same_adapter(desired);
    self
      .description
      .property_command(self.object_id, &*desired.description, hierarchy_changed)
  }

  pub(crate) fn move_command(&self, parent_id: ObjectId, child_index: u32) -> Command {
    self
      .description
      .move_command(self.object_id, parent_id, child_index)
  }

  pub(crate) fn index_command(&self, child_index: u32) -> Command {
    self.description.index_command(self.object_id, child_index)
  }

  pub(crate) fn destroy_command(&self) -> Command {
    self.description.destroy_command(self.object_id)
  }

  pub(crate) fn constrains_children(&self) -> bool {
    self.description.constrains_children()
  }

  fn assert_same_adapter(&self, other: &Self) {
    assert_eq!(
      self.description.adapter_type(),
      other.description.adapter_type(),
      "a retained Reactant host cannot change native adapters"
    );
  }
}

trait ErasedHostDescription {
  fn clone_box(&self) -> Box<dyn ErasedHostDescription>;
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
  fn adapter_type(&self) -> TypeId;
  fn requires_remount(&self, desired: &dyn ErasedHostDescription) -> bool;
  fn create_command(&self, node: &HostNode, parent_id: ObjectId, child_index: u32) -> Command;
  fn property_command(
    &self,
    object_id: ObjectId,
    desired: &dyn ErasedHostDescription,
    hierarchy_changed: bool,
  ) -> Option<Command>;
  fn move_command(&self, object_id: ObjectId, parent_id: ObjectId, child_index: u32) -> Command;
  fn index_command(&self, object_id: ObjectId, child_index: u32) -> Command;
  fn destroy_command(&self, object_id: ObjectId) -> Command;
  fn constrains_children(&self) -> bool;
}

struct AdaptedHost<A: HostAdapter> {
  description: A::Description,
}

impl<A: HostAdapter> ErasedHostDescription for AdaptedHost<A> {
  fn clone_box(&self) -> Box<dyn ErasedHostDescription> {
    Box::new(Self {
      description: self.description.clone(),
    })
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }

  fn adapter_type(&self) -> TypeId {
    TypeId::of::<A>()
  }

  fn requires_remount(&self, desired: &dyn ErasedHostDescription) -> bool {
    A::requires_remount(&self.description, description::<A>(desired))
  }

  fn create_command(&self, node: &HostNode, parent_id: ObjectId, child_index: u32) -> Command {
    A::create_command(node, parent_id, child_index)
  }

  fn property_command(
    &self,
    object_id: ObjectId,
    desired: &dyn ErasedHostDescription,
    hierarchy_changed: bool,
  ) -> Option<Command> {
    A::property_command(
      object_id,
      &self.description,
      description::<A>(desired),
      hierarchy_changed,
    )
  }

  fn move_command(&self, object_id: ObjectId, parent_id: ObjectId, child_index: u32) -> Command {
    A::move_command(object_id, parent_id, child_index)
  }

  fn index_command(&self, object_id: ObjectId, child_index: u32) -> Command {
    A::index_command(object_id, child_index)
  }

  fn destroy_command(&self, object_id: ObjectId) -> Command {
    A::destroy_command(object_id)
  }

  fn constrains_children(&self) -> bool {
    A::constrains_children(&self.description)
  }
}

impl Clone for Box<dyn ErasedHostDescription> {
  fn clone(&self) -> Self {
    self.clone_box()
  }
}

fn description<A: HostAdapter>(value: &dyn ErasedHostDescription) -> &A::Description {
  value
    .as_any()
    .downcast_ref::<AdaptedHost<A>>()
    .map(|host| &host.description)
    .expect("Reactant hosts from different adapters cannot reconcile")
}

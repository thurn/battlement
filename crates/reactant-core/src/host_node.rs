//! Type-erased host descriptions retained by the shared component tree.

use std::any::{Any, TypeId};

use battlement::{Command, GameObject, ObjectId};

/// Implements one typed native-host catalog behind the heterogeneous render tree.
pub trait HostAdapter: 'static {
  /// Typed properties retained at the heterogeneous boundary.
  type Description: Clone + 'static;

  /// Whether the previous native instance can represent the desired declaration.
  fn requires_remount(previous: &Self::Description, desired: &Self::Description) -> bool;
  /// Creates a host beneath its physical attachment.
  fn create_command(node: &HostNode, parent_id: ObjectId, child_index: u32) -> Command;
  /// Returns a sparse single-command property update.
  fn property_command(
    object_id: ObjectId,
    previous: &Self::Description,
    desired: &Self::Description,
    hierarchy_changed: bool,
  ) -> Option<Command>;
  /// Moves a retained host beneath another physical parent.
  fn move_command(object_id: ObjectId, parent_id: ObjectId, child_index: u32) -> Command;
  /// Reorders a host when the native hierarchy has meaningful sibling indices.
  fn index_command(object_id: ObjectId, child_index: u32) -> Option<Command>;
  /// Releases a host and its physical descendants.
  fn destroy_command(object_id: ObjectId) -> Command;
  /// Whether child capacity constrains mutation ordering.
  fn constrains_children(description: &Self::Description) -> bool;
  /// Whether creation includes the entire physical subtree.
  fn creates_children() -> bool {
    true
  }
  /// Additional property mutations for hosts with separate native property commands.
  fn property_commands(
    object_id: ObjectId,
    previous: &Self::Description,
    desired: &Self::Description,
    hierarchy_changed: bool,
  ) -> Vec<Command> {
    Self::property_command(object_id, previous, desired, hierarchy_changed)
      .into_iter()
      .collect()
  }
  /// Returns the snapshot record for an object host, or None for UI.
  fn object(
    _object_id: ObjectId,
    _description: &Self::Description,
    _parent: Option<ObjectId>,
  ) -> Option<GameObject> {
    None
  }
  /// Whether this host attaches directly to a scene container.
  fn scene_root(_description: &Self::Description) -> bool {
    false
  }
  /// Disables native input without changing the retained visual.
  fn inert(_description: &mut Self::Description) {}

  /// Hides a retained host while a suspense fallback is visible.
  fn hide(description: &mut Self::Description);
}

/// One native host with an adapter-owned description and shared tree structure.
#[derive(Clone)]
pub struct HostNode {
  /// Native identity allocated by the shared logical tree.
  pub object_id: ObjectId,
  /// Ordered physical children after attachment projection.
  pub children: Vec<Self>,
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

  /// Reads the description through its declared adapter.
  pub fn description<A: HostAdapter>(&self) -> &A::Description {
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

  pub(crate) fn property_commands(&self, desired: &Self, hierarchy_changed: bool) -> Vec<Command> {
    self.assert_same_adapter(desired);
    self
      .description
      .property_commands(self.object_id, &*desired.description, hierarchy_changed)
  }

  pub(crate) fn move_command(&self, parent_id: ObjectId, child_index: u32) -> Command {
    self
      .description
      .move_command(self.object_id, parent_id, child_index)
  }

  pub(crate) fn index_command(&self, child_index: u32) -> Option<Command> {
    self.description.index_command(self.object_id, child_index)
  }

  pub(crate) fn destroy_command(&self) -> Command {
    self.description.destroy_command(self.object_id)
  }

  pub(crate) fn constrains_children(&self) -> bool {
    self.description.constrains_children()
  }

  pub(crate) fn is_ui(&self) -> bool {
    self.description.adapter_type() == TypeId::of::<crate::ui_host_adapter::UiHostAdapter>()
  }
  pub(crate) fn creates_children(&self) -> bool {
    self.description.creates_children()
  }
  pub(crate) fn object(&self, parent: Option<ObjectId>) -> Option<GameObject> {
    self.description.object(self.object_id, parent)
  }
  pub(crate) fn scene_root(&self) -> bool {
    self.description.scene_root()
  }
  pub(crate) fn inert(&mut self) {
    self.description.inert();
  }
  pub(crate) fn hide(&mut self) {
    self.description.hide();
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
  fn property_commands(
    &self,
    object_id: ObjectId,
    desired: &dyn ErasedHostDescription,
    hierarchy_changed: bool,
  ) -> Vec<Command>;
  fn move_command(&self, object_id: ObjectId, parent_id: ObjectId, child_index: u32) -> Command;
  fn index_command(&self, object_id: ObjectId, child_index: u32) -> Option<Command>;
  fn destroy_command(&self, object_id: ObjectId) -> Command;
  fn constrains_children(&self) -> bool;
  fn creates_children(&self) -> bool;
  fn object(&self, object_id: ObjectId, parent: Option<ObjectId>) -> Option<GameObject>;
  fn scene_root(&self) -> bool;
  fn inert(&mut self);
  fn hide(&mut self);
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

  fn property_commands(
    &self,
    object_id: ObjectId,
    desired: &dyn ErasedHostDescription,
    hierarchy_changed: bool,
  ) -> Vec<Command> {
    A::property_commands(
      object_id,
      &self.description,
      description::<A>(desired),
      hierarchy_changed,
    )
  }

  fn move_command(&self, object_id: ObjectId, parent_id: ObjectId, child_index: u32) -> Command {
    A::move_command(object_id, parent_id, child_index)
  }

  fn index_command(&self, object_id: ObjectId, child_index: u32) -> Option<Command> {
    A::index_command(object_id, child_index)
  }

  fn destroy_command(&self, object_id: ObjectId) -> Command {
    A::destroy_command(object_id)
  }

  fn creates_children(&self) -> bool {
    A::creates_children()
  }
  fn object(&self, object_id: ObjectId, parent: Option<ObjectId>) -> Option<GameObject> {
    A::object(object_id, &self.description, parent)
  }
  fn scene_root(&self) -> bool {
    A::scene_root(&self.description)
  }
  fn inert(&mut self) {
    A::inert(&mut self.description);
  }
  fn hide(&mut self) {
    A::hide(&mut self.description);
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

//! Typed native adapters participate in the shared logical tree.

#![allow(private_interfaces)]

use std::any::TypeId;

use battlement::{Command, CommandBody, ObjectId, Validate};

use crate::{
  callback::Callback,
  element_ref::{self, ElementRef},
  event_handler::Handler,
  host_node::{HostAdapter, HostNode},
  render::{self, Node, Render, RenderSink, RenderTree},
  render_value::Sealed,
};

/// A native host declared by a typed host adapter.
pub struct NativeHost<A: HostAdapter> {
  description: A::Description,
  children: Vec<Node>,
  handlers: Vec<Handler>,
  reference: Option<ElementRef>,
}

/// A committed object reference with the lifetime of its logical owner.
#[derive(Clone)]
pub struct ObjectRef(ElementRef);

/// Returns a stable object reference for the mounted component.
pub fn use_object_ref() -> ObjectRef {
  ObjectRef(element_ref::use_element_ref())
}

impl ObjectRef {
  /// Returns the attached native identity after a successful commit.
  pub fn object_id(&self) -> Option<ObjectId> {
    assert!(
      !crate::context::rendering(),
      "object refs cannot be queried while rendering"
    );
    self.0.geometry_identity().2
  }

  /// Whether the logical owner currently contributes a native object.
  pub fn is_attached(&self) -> bool {
    self.0.is_attached()
  }
}

impl<A: HostAdapter> NativeHost<A> {
  /// Creates a host with adapter-owned properties.
  pub fn new(description: A::Description) -> Self {
    Self {
      description,
      children: Vec::new(),
      handlers: Vec::new(),
      reference: None,
    }
  }

  /// Appends logical children, including portal contributions.
  pub fn child(mut self, child: impl Render) -> Self {
    self.children.push(Node::new(child));
    self
  }

  /// Attaches an object reference at commit time.
  pub fn reference(mut self, reference: ObjectRef) -> Self {
    self.reference = Some(reference.0);
    self
  }

  /// Handles activation along the host's logical ancestry.
  pub fn on_click(mut self, callback: Callback<()>) -> Self {
    self.handlers.push(Handler::world_activation(callback));
    self
  }
}

impl<A: HostAdapter> Render for NativeHost<A> {}

impl<A: HostAdapter> Sealed for NativeHost<A> {
  fn descriptor(&self) -> TypeId {
    TypeId::of::<Self>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    if sink.error.is_some() {
      return;
    }
    let descriptor = self.descriptor();
    let matching = sink.matching_position(descriptor);
    let previous = matching.and_then(|position| position.host.as_ref());
    let mut host = HostNode::new::<A>(
      previous.map_or_else(ObjectId::new_v4, |host| host.object_id),
      self.description.clone(),
    );
    if let Some(object) = host.object(None) {
      Command::new_v4(CommandBody::object_create(object))
        .validate()
        .expect("invalid Reactant object declaration");
    }
    let remount = previous.is_some_and(|previous| previous.requires_remount(&host));
    if remount {
      host.object_id = ObjectId::new_v4();
    }
    let empty = RenderTree::default();
    let committed = matching.map_or(&empty, |position| &position.children);
    let mut children = render::sink_with_scope(committed, sink.variant_scope.clone());
    for child in &self.children {
      child.render_into(&mut children);
    }
    let (mut children, pending) = match RenderSink::finish_child(children) {
      Ok(value) => value,
      Err(error) => {
        sink.error = Some(error);
        return;
      }
    };
    if remount {
      crate::object_layout::replace_descendants(&mut children);
    }
    sink.pending.extend(pending);
    sink.push(descriptor, Some(host), children);
    let position = sink.positions.last_mut().expect("native host was appended");
    position.handlers = self.handlers.clone();
    position.element_ref = self.reference.clone();
  }
}

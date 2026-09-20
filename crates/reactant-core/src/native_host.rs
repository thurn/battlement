//! Typed native adapters participate in the shared logical tree.

#![allow(private_interfaces)]

use std::any::TypeId;

use battlement::{Command, CommandBody, ObjectId, Validate, Vector3};
use uuid::Uuid;

use crate::{
  callback::Callback,
  element_ref::{self, ElementRef},
  event_handler::Handler,
  host_node::{HostAdapter, HostNode},
  motion::MotionProps,
  motion_lifecycle,
  motion_variants::ExitBlueprint,
  render::{self, Node, Render, RenderSink, RenderTree},
  render_facade,
  render_value::Sealed,
};

/// A native host declared by a typed host adapter.
pub struct NativeHost<A: HostAdapter> {
  description: A::Description,
  children: Vec<Node>,
  handlers: Vec<Handler>,
  reference: Option<ElementRef>,
  id: Option<Uuid>,
  motion: MotionProps,
}

/// A committed object reference with the lifetime of its logical owner.
/// UI elements require a different ref type and cannot be local-point targets.
///
/// ```compile_fail
/// use reactant_core::{native_host::ObjectRef, prelude::*};
/// fn wrong_kind(reference: ObjectRef) {
///   View::new().element_ref(reference);
/// }
/// ```
#[derive(Clone)]
pub struct ObjectRef(ElementRef);

impl PartialEq for ObjectRef {
  fn eq(&self, other: &Self) -> bool {
    self.0 == other.0
  }
}

impl Eq for ObjectRef {}

/// Returns a stable object reference for the mounted component.
pub fn use_object_ref() -> ObjectRef {
  ObjectRef(element_ref::use_element_ref())
}

impl ObjectRef {
  pub(crate) fn retain_native_identity(
    &self,
    object_id: ObjectId,
  ) -> std::rc::Rc<crate::native_identity_lease::NativeIdentityLease> {
    self.0.retain_native_identity(object_id)
  }

  /// Describes a required local point without adding an attachment object or host.
  pub fn local_point(&self, offset: Vector3) -> crate::local_point::LocalPoint {
    crate::local_point::LocalPoint::new(self.clone(), offset)
  }

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
      id: None,
      motion: MotionProps::new(),
    }
  }

  /// Forwards native Motion targets, controls, and inherited configuration.
  pub fn motion(mut self, motion: MotionProps) -> Self {
    self.motion = motion;
    self
  }

  /// Preserves this compatible host across logical parents and attachments.
  pub fn id(mut self, id: Uuid) -> Self {
    assert!(!id.is_nil(), "presentation IDs cannot be nil");
    self.id = Some(id);
    self
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

  /// Installs typed logical pointer callbacks.
  pub fn events(mut self, events: crate::pointer_handlers::PointerHandlers) -> Self {
    self.handlers.extend(events.handlers);
    self
  }

  /// Installs semantic focus and navigation callbacks.
  pub fn navigation(mut self, events: crate::navigation_handlers::NavigationHandlers) -> Self {
    self.handlers.extend(events.handlers);
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
    let matching = self.id.map_or_else(
      || sink.matching_position(descriptor),
      |id| sink.identities.matching(id, descriptor),
    );
    let previous = matching.and_then(|position| position.host.as_ref());
    let mut host = HostNode::new::<A>(
      previous.map_or_else(
        || sink.identities.new_host_id(self.id),
        |host| host.object_id,
      ),
      self.description.clone(),
    );
    let object = host.object(None);
    let world_host = object.is_some();
    assert!(
      self.reference.is_none() || world_host,
      "ObjectRef requires a world GameObject host"
    );
    if let Some(object) = object {
      Command::new_v4(CommandBody::object_create(object))
        .validate()
        .expect("invalid Reactant object declaration");
    }
    let remount = previous.is_some_and(|previous| previous.requires_remount(&host));
    if remount {
      host.object_id = ObjectId::new_v4();
    }
    let resolved = sink.variant_scope.resolve(&self.motion);
    let previous_motion = previous.and_then(|host| host.motion_descriptor());
    let callbacks = self.motion.callbacks(&resolved);
    let exit_blueprint = ExitBlueprint::new(self.motion.clone(), sink.variant_scope.clone());
    let history = matching.map_or_else(Vec::new, |position| {
      motion_lifecycle::carry_registrations(
        &position.motion_callback_history,
        previous_motion,
        &position.motion_callbacks,
      )
    });
    let empty = RenderTree::default();
    let committed = matching.map_or(&empty, |position| &position.children);
    let mut children =
      render::sink_with_scope(committed, resolved.child_scope.clone(), sink.identities);
    for child in &self.children {
      child.render_into(&mut children);
    }
    let (children, pending) = match RenderSink::finish_child(children) {
      Ok(value) => value,
      Err(error) => {
        sink.error = Some(error);
        return;
      }
    };
    resolved.complete(
      &sink.variant_scope,
      self.motion.resolved_duration_micros(&resolved),
    );
    *host.motion_mut() =
      render_facade::descriptor(&self.motion, &resolved, host.object_id, previous_motion);
    host.set_world_motion_blocking(self.motion.blocking_command);
    assert!(
      host.motion_descriptor().is_none() || world_host,
      "Native Motion requires a world GameObject host"
    );
    sink.pending.extend(pending);
    sink.push(descriptor, Some(host), children);
    let position = sink.positions.last_mut().expect("native host was appended");
    position.presentation_id = self.id;
    position.handlers = self.handlers.clone();
    position.element_ref = self.reference.clone();
    position.motion_callbacks = callbacks;
    position.motion_callback_history = history;
    position.exit_blueprint = exit_blueprint;
  }
}

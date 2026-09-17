//! Application-wide presentation identity, independent of physical attachment.

#![allow(private_interfaces)]

use std::{any::TypeId, rc::Rc};

use uuid::Uuid;

use crate::{
  key::StructuralRender,
  render::{Node, Render, RenderSink},
  render_value::Sealed,
};

/// Gives a component or structural contribution identity across logical parents.
pub trait IdentityRenderExt: Render + Sized {
  /// Preserves compatible state while this UUID remains declared in the application.
  fn id(self, id: Uuid) -> Identified<Self> {
    assert!(!id.is_nil(), "presentation IDs cannot be nil");
    Identified {
      render: Rc::new(self),
      id,
    }
  }
}

/// A logical contribution with application-wide identity.
pub struct Identified<R> {
  render: Rc<R>,
  id: Uuid,
}

pub(crate) struct IdentifiedMarker;

impl<R: StructuralRender> IdentityRenderExt for R {}
impl<R: Render> Render for Identified<R> {}
impl<R: Render> StructuralRender for Identified<R> {}

impl<R: Render> Sealed for Identified<R> {
  fn descriptor(&self) -> TypeId {
    TypeId::of::<IdentifiedMarker>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    sink.push_identified(self.id, None, |children| {
      Rc::clone(&self.render).render_shared(children)
    });
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    Rc::new(self).render_shared(sink);
  }

  fn render_shared(self: Rc<Self>, sink: &mut RenderSink<'_>) {
    let retained = Node::new(Rc::clone(&self));
    sink.push_identified(self.id, Some(retained), |children| {
      Rc::clone(&self.render).render_shared(children)
    });
  }
}

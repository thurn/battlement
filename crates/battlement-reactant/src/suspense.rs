//! Pending-resource fallback boundaries.

#![allow(private_interfaces)]

use std::{any::TypeId, rc::Rc};

use crate::{
  props::Missing,
  render::{Render, RenderSink, RenderTree},
  render_value::Sealed,
  resource_runtime::{ResourceConsumer, ResourceToken},
};

/// Shows a fallback while its primary subtree waits for resources.
pub struct Suspense<F, C = Missing> {
  fallback: F,
  child: C,
}

#[derive(Clone)]
pub(crate) struct SuspenseState {
  consumer: Rc<ResourceConsumer>,
  rendered: Vec<ResourceToken>,
  attempted_pending: Vec<usize>,
  pub(crate) primary: RenderTree,
  pub(crate) showing_fallback: bool,
}

impl<F> Suspense<F> {
  /// Creates an incomplete boundary with a pending fallback.
  pub const fn new(fallback: F) -> Self {
    Self {
      fallback,
      child: Missing,
    }
  }
}

impl<F> Suspense<F> {
  /// Supplies the primary subtree watched by this boundary.
  pub fn child<C: Render>(self, child: C) -> Suspense<F, C> {
    Suspense {
      fallback: self.fallback,
      child,
    }
  }
}

impl<F: Render, C: Render> Render for Suspense<F, C> {}

impl<F: Render, C: Render> Sealed for Suspense<F, C> {
  fn descriptor(&self) -> TypeId {
    TypeId::of::<SuspenseMarker>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    sink.push_suspense(
      |children| self.child.render_into(children),
      |children| self.fallback.render_into(children),
    );
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    sink.push_suspense(
      |children| self.child.render_owned(children),
      |children| self.fallback.render_owned(children),
    );
  }
}

impl SuspenseState {
  pub(crate) fn new(
    showing_fallback: bool,
    rendered: Vec<ResourceToken>,
    primary: RenderTree,
  ) -> Self {
    let attempted_pending = self::pending_marker(&primary);
    Self {
      consumer: ResourceConsumer::new(),
      rendered,
      attempted_pending,
      primary,
      showing_fallback,
    }
  }

  pub(crate) fn prepare(
    mut self,
    showing_fallback: bool,
    rendered: Vec<ResourceToken>,
    primary: RenderTree,
  ) -> Self {
    self.rendered = rendered;
    self.attempted_pending = self::pending_marker(&primary);
    self.primary = primary;
    self.showing_fallback = showing_fallback;
    self
  }

  pub(crate) fn commit(&self) {
    self.consumer.replace(&self.rendered);
  }

  pub(crate) fn dirty(&self) -> bool {
    self.consumer.dirty()
  }

  pub(crate) fn primary_pending_changed(&self) -> bool {
    self.attempted_pending != self::pending_marker(&self.primary)
  }
}

fn pending_marker(primary: &RenderTree) -> Vec<usize> {
  let mut marker = Vec::new();
  primary.pending_hook_lengths(&mut marker);
  marker
}

pub(crate) struct SuspenseMarker;

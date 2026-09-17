//! Visibility changes that preserve the logical component lifetime.

#![allow(private_interfaces)]

use std::any::TypeId;

use crate::{
  key::StructuralRender,
  render::{Render, RenderSink},
  render_value::Sealed,
};

/// Keeps children mounted while suppressing their native visibility and input.
pub struct VisibilityScope<R = ()> {
  visible: bool,
  child: R,
}

struct VisibilityMarker;

impl VisibilityScope<()> {
  /// Selects whether the retained children are currently shown.
  pub fn new(visible: bool) -> Self {
    Self { visible, child: () }
  }
}

impl<R> VisibilityScope<R> {
  /// Supplies the logical children whose lifetime is independent of visibility.
  pub fn child<C: Render>(self, child: C) -> VisibilityScope<C> {
    VisibilityScope {
      visible: self.visible,
      child,
    }
  }
}

impl<R: Render> Render for VisibilityScope<R> {}
impl<R: Render> StructuralRender for VisibilityScope<R> {}
impl<R: Render> Sealed for VisibilityScope<R> {
  fn descriptor(&self) -> TypeId {
    TypeId::of::<VisibilityMarker>()
  }
  fn render_into(&self, sink: &mut RenderSink<'_>) {
    if sink.error.is_some() {
      return;
    }
    sink.push_nested::<VisibilityMarker>(|children| self.child.render_into(children));
    if let Some(position) = sink.positions.last_mut() {
      position.hidden = !self.visible;
    }
  }
  fn render_owned(self, sink: &mut RenderSink<'_>) {
    if sink.error.is_some() {
      return;
    }
    sink.push_nested::<VisibilityMarker>(|children| self.child.render_owned(children));
    if let Some(position) = sink.positions.last_mut() {
      position.hidden = !self.visible;
    }
  }
}

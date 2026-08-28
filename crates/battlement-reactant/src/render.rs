//! Render values supported by the Reactant tree builder.

use std::{any::TypeId, rc::Rc};

use battlement::{ObjectId, UiNode};

use self::private::Sealed;
use crate::{key::ErasedKey, reconcile};

/// A value Reactant can lower into native host descriptions.
///
/// Text and other raw scalar values deliberately do not implement this trait.
///
/// ```compile_fail
/// use battlement_reactant::render::Render;
///
/// fn accepts_render(_value: impl Render) {}
/// accepts_render("explicit controls are required");
/// ```
///
/// Arbitrary iterators must be collected into a supported structural value.
///
/// ```compile_fail
/// use battlement::Label;
/// use battlement_reactant::render::Render;
///
/// fn accepts_render(_value: impl Render) {}
/// accepts_render([Label::new("one")].into_iter());
/// ```
pub trait Render: private::Sealed + 'static {}

/// Groups children without introducing a native host.
pub struct Fragment<R> {
  children: R,
}

/// Selects one of two heterogeneous render values.
pub enum Either<L, R> {
  /// The left branch.
  Left(L),
  /// The right branch.
  Right(R),
}

/// Stores an immutable type-erased render value.
#[derive(Clone)]
pub struct Node {
  render: Rc<dyn ErasedRender>,
  descriptor: TypeId,
}

impl<R> Fragment<R> {
  /// Creates a hostless group around `children`.
  pub const fn new(children: R) -> Self {
    Self { children }
  }
}

impl<L, R> Either<L, R> {
  /// Selects the left render value.
  pub const fn left(value: L) -> Self {
    Self::Left(value)
  }

  /// Selects the right render value.
  pub const fn right(value: R) -> Self {
    Self::Right(value)
  }
}

impl Node {
  /// Erases an owned render value while retaining its concrete descriptor.
  pub fn new<R: Render>(render: R) -> Self {
    let descriptor = render.descriptor();
    Self {
      render: Rc::new(render),
      descriptor,
    }
  }
}

impl Render for () {}
impl<R: Render> Render for Option<R> {}
impl<R: Render, const N: usize> Render for [R; N] {}
impl<R: Render> Render for Vec<R> {}
impl<R: Render> Render for Rc<R> {}
impl<R: Render> Render for Fragment<R> {}
impl<L: Render, R: Render> Render for Either<L, R> {}
impl Render for Node {}

pub(crate) fn lower<R: Render>(value: R, committed: &RenderTree) -> RenderTree {
  let mut sink = RenderSink::new(committed);
  value.render_into(&mut sink);
  sink.finish()
}

#[derive(Clone, Default)]
pub(crate) struct RenderTree {
  positions: Vec<RenderPosition>,
}

impl RenderTree {
  pub(crate) fn hosts(&self) -> Vec<UiNode> {
    let mut hosts = Vec::new();
    self.append_hosts(&mut hosts);
    hosts
  }

  fn append_hosts(&self, hosts: &mut Vec<UiNode>) {
    for position in &self.positions {
      if let Some(host) = &position.host {
        hosts.push(host.clone());
      } else {
        position.children.append_hosts(hosts);
      }
    }
  }
}

#[derive(Clone)]
struct RenderPosition {
  descriptor: TypeId,
  key: Option<ErasedKey>,
  host: Option<UiNode>,
  children: RenderTree,
}

pub(crate) struct RenderSink<'a> {
  committed: &'a RenderTree,
  positions: Vec<RenderPosition>,
}

impl<'a> RenderSink<'a> {
  fn new(committed: &'a RenderTree) -> Self {
    Self {
      committed,
      positions: Vec::new(),
    }
  }

  pub(crate) fn push_keyed<R: 'static>(
    &mut self,
    key: ErasedKey,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    assert!(
      !self
        .positions
        .iter()
        .any(|position| position.key.as_ref() == Some(&key)),
      "duplicate sibling key"
    );
    let descriptor = TypeId::of::<R>();
    let empty = RenderTree::default();
    let committed = self
      .committed
      .positions
      .iter()
      .find(|position| position.key.as_ref() == Some(&key))
      .filter(|position| position.descriptor == descriptor)
      .map_or(&empty, |position| &position.children);
    let mut children = RenderSink::new(committed);
    render(&mut children);
    self.positions.push(RenderPosition {
      descriptor,
      key: Some(key),
      host: None,
      children: children.finish(),
    });
  }

  fn push_empty<R: 'static>(&mut self) {
    self.push(TypeId::of::<R>(), None, RenderTree::default());
  }

  pub(crate) fn push_host<R: 'static>(&mut self, host: impl FnOnce(ObjectId) -> UiNode) {
    let descriptor = TypeId::of::<R>();
    let previous = self
      .matching_position(descriptor)
      .and_then(|position| position.host.as_ref());
    let mut node = host(previous.map_or_else(ObjectId::new_v4, |node| node.object_id));
    if previous.is_some_and(|value| reconcile::requires_remount(&value.element, &node.element)) {
      node.object_id = ObjectId::new_v4();
    }
    self.push(descriptor, Some(node), RenderTree::default());
  }

  pub(crate) fn push_host_with_children<R: 'static>(
    &mut self,
    host: impl FnOnce(ObjectId, Vec<UiNode>) -> UiNode,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    let descriptor = TypeId::of::<R>();
    let matching = self.matching_position(descriptor);
    let previous = matching.and_then(|position| position.host.as_ref());
    let mut node = host(
      previous.map_or_else(ObjectId::new_v4, |value| value.object_id),
      Vec::new(),
    );
    let remount =
      previous.is_some_and(|value| reconcile::requires_remount(&value.element, &node.element));
    if remount {
      node.object_id = ObjectId::new_v4();
    }
    let empty = RenderTree::default();
    let committed = if remount {
      &empty
    } else {
      matching.map_or(&empty, |position| &position.children)
    };
    let mut children = RenderSink::new(committed);
    render(&mut children);
    let children = children.finish();
    node.children = children.hosts();
    self.push(descriptor, Some(node), children);
  }

  fn push_nested<R: 'static>(&mut self, render: impl FnOnce(&mut RenderSink<'_>)) {
    self.push_nested_descriptor(TypeId::of::<R>(), render);
  }

  fn push_nested_descriptor(
    &mut self,
    descriptor: TypeId,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    let empty = RenderTree::default();
    let committed = self
      .matching_position(descriptor)
      .map_or(&empty, |position| &position.children);
    let mut children = RenderSink::new(committed);
    render(&mut children);
    self.push(descriptor, None, children.finish());
  }

  fn matching_position(&self, descriptor: TypeId) -> Option<&RenderPosition> {
    self
      .committed
      .positions
      .get(self.positions.len())
      .filter(|position| position.key.is_none() && position.descriptor == descriptor)
  }

  fn push(&mut self, descriptor: TypeId, host: Option<UiNode>, children: RenderTree) {
    self.positions.push(RenderPosition {
      descriptor,
      key: None,
      host,
      children,
    });
  }

  fn finish(self) -> RenderTree {
    RenderTree {
      positions: self.positions,
    }
  }
}

trait ErasedRender {
  fn descriptor(&self) -> TypeId;
  fn render_into(&self, sink: &mut RenderSink<'_>);
}

impl<R: Render> ErasedRender for R {
  fn descriptor(&self) -> TypeId {
    Sealed::descriptor(self)
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    Sealed::render_into(self, sink);
  }
}

#[allow(private_interfaces)]
pub(crate) mod private {
  use std::any::TypeId;
  use std::rc::Rc;

  use crate::{
    component::Component,
    context,
    render::{Either, Fragment, Node, RenderSink},
  };

  pub trait Sealed {
    fn descriptor(&self) -> TypeId;
    fn render_into(&self, sink: &mut RenderSink<'_>);
  }

  impl Sealed for () {
    fn descriptor(&self) -> TypeId {
      TypeId::of::<Self>()
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      sink.push_empty::<Self>();
    }
  }

  impl<R: super::Render> Sealed for Option<R> {
    fn descriptor(&self) -> TypeId {
      TypeId::of::<OptionMarker>()
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      sink.push_nested::<OptionMarker>(|children| {
        if let Some(value) = self {
          value.render_into(children);
        }
      });
    }
  }

  impl<R: super::Render, const N: usize> Sealed for [R; N] {
    fn descriptor(&self) -> TypeId {
      TypeId::of::<Self>()
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      for value in self {
        value.render_into(sink);
      }
    }
  }

  impl<R: super::Render> Sealed for Vec<R> {
    fn descriptor(&self) -> TypeId {
      TypeId::of::<Self>()
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      for value in self {
        value.render_into(sink);
      }
    }
  }

  impl<R: super::Render> Sealed for Rc<R> {
    fn descriptor(&self) -> TypeId {
      self.as_ref().descriptor()
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      self.as_ref().render_into(sink);
    }
  }

  impl<R: super::Render> Sealed for Fragment<R> {
    fn descriptor(&self) -> TypeId {
      TypeId::of::<FragmentMarker>()
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      sink.push_nested::<FragmentMarker>(|children| self.children.render_into(children));
    }
  }

  impl<L: super::Render, R: super::Render> Sealed for Either<L, R> {
    fn descriptor(&self) -> TypeId {
      match self {
        Either::Left(value) => value.descriptor(),
        Either::Right(value) => value.descriptor(),
      }
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      match self {
        Either::Left(value) => value.render_into(sink),
        Either::Right(value) => value.render_into(sink),
      }
    }
  }

  impl Sealed for Node {
    fn descriptor(&self) -> TypeId {
      self.descriptor
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      debug_assert_eq!(self.descriptor, self.render.descriptor());
      self.render.render_into(sink);
    }
  }

  impl<C: Component> Sealed for C {
    fn descriptor(&self) -> TypeId {
      TypeId::of::<C>()
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      sink.push_nested::<C>(|children| {
        context::with_component(|| {
          debug_assert!(context::hooks_allowed());
          self.render().render_into(children);
        });
      });
    }
  }

  impl<C: Component> super::Render for C {}

  macro_rules! tuple_render {
    ($($name:ident),+) => {
      impl<$($name: super::Render),+> Sealed for ($($name,)+) {
        fn descriptor(&self) -> TypeId {
          TypeId::of::<Self>()
        }

        fn render_into(&self, sink: &mut RenderSink<'_>) {
          #[allow(non_snake_case)]
          let ($($name,)+) = self;
          $($name.render_into(sink);)+
        }
      }

      impl<$($name: super::Render),+> super::Render for ($($name,)+) {}
    };
  }

  struct FragmentMarker;
  struct OptionMarker;

  tuple_render!(A);
  tuple_render!(A, B);
  tuple_render!(A, B, C);
  tuple_render!(A, B, C, D);
  tuple_render!(A, B, C, D, E);
  tuple_render!(A, B, C, D, E, F);
  tuple_render!(A, B, C, D, E, F, G);
  tuple_render!(A, B, C, D, E, F, G, H);
  tuple_render!(A, B, C, D, E, F, G, H, I);
  tuple_render!(A, B, C, D, E, F, G, H, I, J);
  tuple_render!(A, B, C, D, E, F, G, H, I, J, K);
  tuple_render!(A, B, C, D, E, F, G, H, I, J, K, L);
}

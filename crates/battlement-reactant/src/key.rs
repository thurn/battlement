//! Stable sibling identity for Reactant render values.

#![allow(private_interfaces)]

use std::{
  any::{Any, TypeId},
  hash::{DefaultHasher, Hash, Hasher},
  rc::Rc,
};

use crate::{
  component::{Component, Memo},
  context::Provided,
  error_boundary::ErrorBoundary,
  hooks::Dependencies,
  portal::Portal,
  render::{Node, Render, RenderSink},
  render_value::Sealed,
  suspense::Suspense,
};

/// Adds terminal sibling identity to a non-host render value.
///
/// Keys compare only with values of the same Rust type. Host façades provide
/// their own order-independent inherent `key` method; this adapter remains for
/// components, fragments, portals, boundaries, collections, and other
/// structural render values.
///
/// ```
/// use battlement_reactant::{
///   host::Label,
///   key::KeyRenderExt,
///   render::{Fragment, Render},
/// };
///
/// fn accepts_render(_value: impl Render) {}
/// accepts_render(Fragment::new((Label::new("Ready"), ())).key(7_u64));
/// ```
///
/// Host façades cannot be wrapped in a second structural keyed position, even
/// through fully qualified trait syntax.
///
/// ```compile_fail
/// use battlement_reactant::{host::Button, key::KeyRenderExt};
///
/// let _ = <Button as KeyRenderExt>::key(Button::new("Save"), 7_u64);
/// ```
pub trait KeyRenderExt: Render + Sized {
  /// Assigns typed identity within the render value's sibling list.
  fn key<K: Clone + Eq + Hash + 'static>(self, key: K) -> Keyed<Self, K> {
    Keyed {
      render: self,
      key: Rc::new(key),
    }
  }
}

/// A render value with terminal sibling identity.
pub struct Keyed<R, K> {
  render: R,
  key: Rc<K>,
}

impl<R: StructuralRender> KeyRenderExt for R {}

pub(crate) trait StructuralRender: Render {}

impl StructuralRender for () {}
impl<R: Render> StructuralRender for Option<R> {}
impl<R: Render, const N: usize> StructuralRender for [R; N] {}
impl<R: Render> StructuralRender for Vec<R> {}
impl<R: Render> StructuralRender for Rc<R> {}
impl<R: Render, E: std::error::Error + 'static> StructuralRender for Result<R, E> {}
impl<R: Render> StructuralRender for crate::render::Fragment<R> {}
impl<L: Render, R: Render> StructuralRender for crate::render::Either<L, R> {}
impl StructuralRender for crate::render::Node {}
impl<C: Component> StructuralRender for C {}
impl<C: Component + PartialEq> StructuralRender for Memo<C> {}
impl<T, R> StructuralRender for Provided<T, R>
where
  T: Clone + PartialEq + 'static,
  R: Render,
{
}
impl<F, C, D, O, R> StructuralRender for ErrorBoundary<F, C, D, O>
where
  F: Fn(&crate::runtime::RenderError) -> R + 'static,
  C: Render,
  D: Dependencies,
  O: 'static,
  R: Render,
{
}
impl<R: Render> StructuralRender for Portal<R> {}
impl<F: Render, C: Render> StructuralRender for Suspense<F, C> {}

impl<R, K> Render for Keyed<R, K>
where
  R: Render,
  K: Clone + Eq + Hash + 'static,
{
}

impl<R, K> StructuralRender for Keyed<R, K>
where
  R: Render,
  K: Clone + Eq + Hash + 'static,
{
}

impl<R, K> Sealed for Keyed<R, K>
where
  R: Render,
  K: Clone + Eq + Hash + 'static,
{
  fn descriptor(&self) -> TypeId {
    TypeId::of::<KeyedMarker>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    sink.push_keyed::<KeyedMarker>(ErasedKey::from_rc(Rc::clone(&self.key)), |sink| {
      self.render.render_into(sink);
    });
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    Rc::new(self).render_shared(sink);
  }

  fn render_shared(self: Rc<Self>, sink: &mut RenderSink<'_>) {
    let retained_render = Node {
      render: Rc::clone(&self) as Rc<dyn crate::render_value::ErasedRender>,
      descriptor: TypeId::of::<KeyedMarker>(),
    };
    sink.push_keyed_source::<KeyedMarker>(
      ErasedKey::from_rc(Rc::clone(&self.key)),
      Some(retained_render),
      |sink| self.render.render_into(sink),
    );
  }
}

#[derive(Clone)]
pub(crate) struct ErasedKey {
  type_id: TypeId,
  hash: u64,
  value: Rc<dyn KeyValue>,
}

impl ErasedKey {
  pub(crate) fn from_value<K: Clone + Eq + Hash + 'static>(value: K) -> Self {
    Self::from_rc(Rc::new(value))
  }

  fn from_rc<K: Clone + Eq + Hash + 'static>(value: Rc<K>) -> Self {
    let mut hasher = DefaultHasher::new();
    TypeId::of::<K>().hash(&mut hasher);
    value.hash(&mut hasher);
    Self {
      type_id: TypeId::of::<K>(),
      hash: hasher.finish(),
      value,
    }
  }
}

impl PartialEq for ErasedKey {
  fn eq(&self, other: &Self) -> bool {
    self.type_id == other.type_id
      && self.hash == other.hash
      && self.value.equals(other.value.as_ref())
  }
}

impl Eq for ErasedKey {}

trait KeyValue: Any {
  fn as_any(&self) -> &dyn Any;
  fn equals(&self, other: &dyn KeyValue) -> bool;
}

impl<K: Eq + 'static> KeyValue for K {
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn equals(&self, other: &dyn KeyValue) -> bool {
    other.as_any().downcast_ref::<K>() == Some(self)
  }
}

struct KeyedMarker;

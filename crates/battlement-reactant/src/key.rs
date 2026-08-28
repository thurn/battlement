//! Stable sibling identity for Reactant render values.

#![allow(private_interfaces)]

use std::{
  any::{Any, TypeId},
  hash::{DefaultHasher, Hash, Hasher},
  rc::Rc,
};

use crate::{
  render::{Render, RenderSink},
  render_value::Sealed,
};

/// Adds terminal sibling identity to a render value.
///
/// Keys compare only with values of the same Rust type. Calling `key` consumes
/// the render value, so primitive properties and children must be authored
/// first.
///
/// ```
/// use battlement::Label;
/// use battlement_reactant::{key::KeyRenderExt, render::Render};
///
/// fn accepts_render(_value: impl Render) {}
/// accepts_render(Label::new("Ready").key(7_u64));
/// ```
///
/// ```compile_fail
/// use battlement::{Label, VisualElement};
/// use battlement_reactant::{key::KeyRenderExt, primitive::ContainerRenderExt};
///
/// let _invalid = VisualElement::new().key("panel").child(Label::new("late"));
/// ```
///
/// ```compile_fail
/// use battlement::Label;
/// use battlement_reactant::key::KeyRenderExt;
///
/// let _invalid = Label::new("Ready").key("status").name("late");
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

impl<R: Render> KeyRenderExt for R {}

impl<R, K> Render for Keyed<R, K>
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
    sink.push_keyed::<KeyedMarker>(ErasedKey::new(Rc::clone(&self.key)), |sink| {
      self.render.render_into(sink);
    });
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    sink.push_keyed::<KeyedMarker>(ErasedKey::new(self.key), |sink| {
      self.render.render_owned(sink);
    });
  }
}

#[derive(Clone)]
pub(crate) struct ErasedKey {
  type_id: TypeId,
  hash: u64,
  value: Rc<dyn KeyValue>,
}

impl ErasedKey {
  fn new<K: Clone + Eq + Hash + 'static>(value: Rc<K>) -> Self {
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

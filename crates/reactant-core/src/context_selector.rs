use std::{any::TypeId, rc::Rc};

use crate::{
  context::{self, ContextIdentity},
  hook_storage::{ContextSlot, HookKind},
  hooks,
};

type Equal<T> = Rc<dyn Fn(&T, &T) -> bool>;

#[derive(Clone)]
struct Selected<T> {
  value: T,
  equal: Equal<T>,
}

/// Reads part of a required provider, reevaluating its consumer only when unequal.
pub fn use_required_context_selector<T, V>(
  select: impl Fn(&T) -> V + 'static,
  equal: impl Fn(&V, &V) -> bool + 'static,
) -> V
where
  T: Clone + 'static,
  V: Clone + 'static,
{
  assert!(
    context::hooks_allowed(),
    "Reactant hooks require a component render context"
  );
  let identity = ContextIdentity::of::<T>();
  let equal: Equal<V> = Rc::new(equal);
  let read: Rc<dyn Fn() -> Selected<V>> = Rc::new(move || Selected {
    value: select(&context::read_required::<T>()),
    equal: Rc::clone(&equal),
  });
  let value = context::with_hooks_forbidden(|| read());
  hooks::use_slot(
    HookKind::Context,
    TypeId::of::<Selected<V>>(),
    |_| ContextSlot {
      identity,
      value: value.clone(),
      read: read.clone(),
    },
    |slot| {
      assert!(
        slot.identity == identity,
        "Reactant context identity changed"
      );
      slot.value = value.clone();
      slot.read = read.clone();
      slot.value.value.clone()
    },
  )
}

impl<T> PartialEq for Selected<T> {
  fn eq(&self, other: &Self) -> bool {
    (self.equal)(&self.value, &other.value)
  }
}

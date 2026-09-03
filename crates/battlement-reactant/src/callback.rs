//! Callbacks with optional access to the application's model.

use std::{
  any::{Any, TypeId},
  rc::Rc,
};

/// Converts an ordinary or model-aware callback to a shared event callback.
#[allow(private_interfaces)]
pub trait IntoCallback<A, Signature>: 'static {
  #[doc(hidden)]
  fn into_callback(self) -> Callback<A>;
}

/// A shared event callback retaining its optional application-model requirement.
pub struct Callback<A> {
  pub(crate) model: Option<TypeId>,
  invoke: Rc<Invocation<A>>,
}

/// Distinguishes forwarding a stored callback from converting a closure.
#[doc(hidden)]
pub struct StoredCallback;

impl<A: 'static> IntoCallback<A, StoredCallback> for Callback<A> {
  fn into_callback(self) -> Callback<A> {
    self
  }
}

impl<A> Clone for Callback<A> {
  fn clone(&self) -> Self {
    Self {
      model: self.model,
      invoke: Rc::clone(&self.invoke),
    }
  }
}

impl<A: 'static> Callback<A> {
  pub(crate) fn call(&self, game: &mut dyn Any, value: A) -> bool {
    (self.invoke)(game, value)
  }

  pub(crate) fn map<B: 'static>(self, map: impl Fn(B) -> Option<A> + 'static) -> Callback<B> {
    Callback {
      model: self.model,
      invoke: Rc::new(move |game, value| {
        if let Some(value) = map(value) {
          self.call(game, value)
        } else {
          false
        }
      }),
    }
  }
}

impl<F: Fn() + 'static> IntoCallback<(), fn()> for F {
  fn into_callback(self) -> Callback<()> {
    Callback {
      model: None,
      invoke: Rc::new(move |_, ()| {
        self();
        true
      }),
    }
  }
}

impl<G: 'static, F: Fn(&mut G) + 'static> IntoCallback<(), fn(&mut G)> for F {
  fn into_callback(self) -> Callback<()> {
    Callback {
      model: Some(TypeId::of::<G>()),
      invoke: Rc::new(move |game, ()| {
        self(game.downcast_mut().expect("callback model mismatch"));
        true
      }),
    }
  }
}

impl<A: 'static, F: Fn(A) + 'static> IntoCallback<A, (fn(A),)> for F {
  fn into_callback(self) -> Callback<A> {
    Callback {
      model: None,
      invoke: Rc::new(move |_, value| {
        self(value);
        true
      }),
    }
  }
}

impl<G: 'static, A: 'static, F: Fn(&mut G, A) + 'static> IntoCallback<A, (fn(&mut G, A),)> for F {
  fn into_callback(self) -> Callback<A> {
    Callback {
      model: Some(TypeId::of::<G>()),
      invoke: Rc::new(move |game, value| {
        self(game.downcast_mut().expect("callback model mismatch"), value);
        true
      }),
    }
  }
}

type Invocation<A> = dyn Fn(&mut dyn Any, A) -> bool;

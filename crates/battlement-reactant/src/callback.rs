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
  pub(crate) fn same_identity(&self, other: &Self) -> bool {
    self.model == other.model && Rc::ptr_eq(&self.invoke, &other.invoke)
  }

  /// Creates an application-independent callback.
  #[must_use]
  pub fn new(callback: impl Fn(A) + 'static) -> Self {
    Self {
      model: None,
      invoke: Rc::new(move |_, value| {
        callback(value);
        true
      }),
    }
  }

  /// Creates a callback that accepts and ignores every input.
  #[must_use]
  pub fn noop() -> Self {
    Self::new(drop)
  }

  /// Adapts a new input before forwarding it to this callback.
  #[must_use]
  pub fn map_input<B: 'static>(self, map: impl Fn(B) -> A + 'static) -> Callback<B> {
    self.map(move |value| Some(map(value)))
  }

  /// Conditionally adapts a new input before forwarding it to this callback.
  #[must_use]
  pub fn filter_map_input<B: 'static>(self, map: impl Fn(B) -> Option<A> + 'static) -> Callback<B> {
    self.map(map)
  }

  /// Runs this callback followed by `next` for the same input.
  #[must_use]
  pub fn then(self, next: Callback<A>) -> Self
  where
    A: Clone,
  {
    let model = match (self.model, next.model) {
      (Some(left), Some(right)) => {
        assert_eq!(
          left, right,
          "combined callbacks require the same model type"
        );
        Some(left)
      }
      (left, right) => left.or(right),
    };
    Self {
      model,
      invoke: Rc::new(move |game, value| {
        let handled = self.call(game, value.clone());
        next.call(game, value) || handled
      }),
    }
  }

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

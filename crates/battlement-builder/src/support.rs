//! Shared implementation details referenced by generated builders.

use core::marker::PhantomData;

/// Converts a property value without making untyped `None` ambiguous.
pub trait IntoOption<T> {
  /// Preserves an option or wraps a supplied value.
  fn into_option(self) -> Option<T>;
}

/// An unfilled slot, distinct from the value type even when it is another marker.
pub struct Missing<T: ?Sized>(PhantomData<fn() -> *const T>);

impl<T: ?Sized> Missing<T> {
  /// Creates an empty compile-time property slot.
  pub const fn new() -> Self {
    Self(PhantomData)
  }
}

impl<T: ?Sized> Copy for Missing<T> {}

impl<T: ?Sized> Clone for Missing<T> {
  fn clone(&self) -> Self {
    *self
  }
}

impl<T: ?Sized> Default for Missing<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T> IntoOption<T> for T {
  fn into_option(self) -> Option<T> {
    Some(self)
  }
}

impl<T> IntoOption<T> for Option<T> {
  fn into_option(self) -> Option<T> {
    self
  }
}

impl IntoOption<String> for &str {
  fn into_option(self) -> Option<String> {
    Some(self.to_owned())
  }
}

impl IntoOption<String> for &String {
  fn into_option(self) -> Option<String> {
    Some(self.clone())
  }
}

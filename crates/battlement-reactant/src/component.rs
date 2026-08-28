//! Declarative component values.

use std::{any::TypeId, rc::Rc};

use crate::{
  context,
  render::{Render, RenderSink},
  render_value::Sealed,
};

/// Owns a row or render-prop closure that cannot consume component hooks.
#[derive(Clone, Copy)]
pub struct RenderCallback<F> {
  callback: F,
}

/// Opts a component into prop comparison and subtree bailout.
pub struct Memo<C> {
  component: Rc<C>,
}

/// Produces a render value from owned, immutable props.
///
/// Components and their output are owned and `'static`, so local data must be
/// copied into props rather than borrowed by the mounted tree.
///
/// ```
/// use battlement::Label;
/// use battlement_reactant::{component::Component, render::Render};
///
/// struct Greeting {
///   name: String,
/// }
///
/// impl Component for Greeting {
///   fn render(&self) -> impl Render {
///     Label::new(format!("Hello, {}", self.name))
///   }
/// }
///
/// fn accepts_render(_value: impl Render) {}
/// accepts_render(Greeting { name: "Ada".to_owned() });
/// ```
pub trait Component: 'static {
  /// Describes this component's current child tree.
  fn render(&self) -> impl Render;
}

/// Creates a memoized component boundary.
pub fn memo<C>(component: C) -> Memo<C>
where
  C: Component + PartialEq,
{
  Memo {
    component: Rc::new(component),
  }
}

impl<F> RenderCallback<F> {
  /// Wraps an owned render-producing closure.
  pub const fn new(callback: F) -> Self {
    Self { callback }
  }

  /// Invokes the closure in a hook-forbidden render scope.
  pub fn call<A, R>(&self, argument: A) -> R
  where
    F: Fn(A) -> R,
  {
    context::with_hooks_forbidden(|| (self.callback)(argument))
  }
}

impl<C> Render for Memo<C> where C: Component + PartialEq {}

#[allow(private_interfaces)]
impl<C> Sealed for Memo<C>
where
  C: Component + PartialEq,
{
  fn descriptor(&self) -> TypeId {
    TypeId::of::<Self>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    sink.push_memoized::<Self, C>(Rc::clone(&self.component), |children| {
      debug_assert!(context::hooks_allowed());
      self.component.render().render_owned(children);
    });
  }
}

#[cfg(test)]
mod tests {
  use std::panic::{self, AssertUnwindSafe};

  use crate::{component::RenderCallback, context};

  #[test]
  fn render_callbacks_forbid_hooks_and_restore_the_component_scope() {
    context::with_component(|| {
      assert!(context::hooks_allowed());
      assert!(!RenderCallback::new(|()| context::hooks_allowed()).call(()));
      assert!(context::hooks_allowed());
      let result = panic::catch_unwind(AssertUnwindSafe(|| {
        RenderCallback::new(|()| panic!("fixture panic")).call(())
      }));
      assert!(result.is_err());
      assert!(context::hooks_allowed());
    });
  }
}

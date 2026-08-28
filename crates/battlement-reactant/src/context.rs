use std::cell::Cell;

thread_local! {
  static CURRENT: Cell<RenderContext> = const { Cell::new(RenderContext::Outside) };
}

pub(crate) fn with_component<T>(operation: impl FnOnce() -> T) -> T {
  with(RenderContext::Component, operation)
}

pub(crate) fn with_hooks_forbidden<T>(operation: impl FnOnce() -> T) -> T {
  with(RenderContext::HooksForbidden, operation)
}

pub(crate) fn hooks_allowed() -> bool {
  CURRENT.get() == RenderContext::Component
}

fn with<T>(context: RenderContext, operation: impl FnOnce() -> T) -> T {
  let previous = CURRENT.replace(context);
  let _restore = Restore(previous);
  operation()
}

struct Restore(RenderContext);

impl Drop for Restore {
  fn drop(&mut self) {
    CURRENT.set(self.0);
  }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RenderContext {
  Outside,
  Component,
  HooksForbidden,
}

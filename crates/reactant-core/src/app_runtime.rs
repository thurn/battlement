//! Application orchestration above the shared component runtime.

use std::{any::Any, cell::RefCell, rc::Rc};

use crate::{context::ContextProvider, render::Node};

thread_local! {
  static CURRENT: RefCell<Option<Rc<dyn AppRuntime>>> = const { RefCell::new(None) };
}

/// Application-owned orchestration for context, updates, and callback recovery.
pub trait AppRuntime: Any {
  /// Supplies type-erased application context without changing root ancestry.
  fn context(&self) -> Rc<dyn Any>;
  /// Polls application observations and requests a context refresh when changed.
  fn poll(&self) -> bool;
  /// Runs one input callback at the application's recovery boundary.
  fn callback(&self, callback: &mut dyn FnMut());
  /// Ends application work without joining background computation.
  fn stop(&self);
  /// Current game work owner; ended sessions have no active work.
  fn work_scope(&self) -> Option<u64> {
    None
  }
  /// Consumes at most one publication after downstream admission permits it.
  fn take_output(&self) -> Option<Box<dyn AppOutput>> {
    None
  }
  /// Routes an attributed host failure to its still-current owner.
  fn fail_work(&self, _scope: u64, _message: String) {}
}

/// A consumed publication retained until its native output is admitted.
#[doc(hidden)]
pub trait AppOutput {
  fn scope(&self) -> u64;
  fn submitted(&self);
}

/// Optional application context carried by a stable root provider.
#[doc(hidden)]
#[derive(Clone)]
pub struct ApplicationContext(Option<Rc<dyn Any>>);

impl ApplicationContext {
  /// Returns the attached runtime's context when it has the requested type.
  pub fn value<T: Any>(&self) -> Option<Rc<T>> {
    self.0.as_ref()?.clone().downcast().ok()
  }
}

impl PartialEq for ApplicationContext {
  fn eq(&self, other: &Self) -> bool {
    match (&self.0, &other.0) {
      (None, None) => true,
      (Some(a), Some(b)) => Rc::ptr_eq(a, b),
      _ => false,
    }
  }
}

#[derive(Default)]
pub(crate) struct RuntimeSlot {
  runtime: Option<Rc<dyn AppRuntime>>,
  value: Option<Rc<dyn Any>>,
}

pub(crate) struct CallbackScope(Option<Rc<dyn AppRuntime>>);

impl RuntimeSlot {
  pub(crate) fn runtime(&self) -> Option<Rc<dyn AppRuntime>> {
    self.runtime.clone()
  }
  pub(crate) fn get_or_insert<T: AppRuntime>(&mut self, create: impl FnOnce() -> T) -> Rc<T> {
    if let Some(value) = &self.value {
      return Rc::clone(value)
        .downcast()
        .unwrap_or_else(|_| panic!("application runtime type mismatch"));
    }
    let value = Rc::new(create());
    self.runtime = Some(value.clone());
    self.value = Some(value.clone());
    value
  }

  pub(crate) fn provide(&self, child: Node) -> Node {
    Node::new(
      ContextProvider::new()
        .context(ApplicationContext(
          self.runtime.as_ref().map(|runtime| runtime.context()),
        ))
        .child(child),
    )
  }

  pub(crate) fn poll(&self) -> bool {
    self.runtime.as_ref().is_some_and(|runtime| runtime.poll())
  }

  pub(crate) fn enter(&self) -> CallbackScope {
    CallbackScope(CURRENT.with(|current| current.replace(self.runtime.clone())))
  }

  pub(crate) fn stop(&self) {
    if let Some(runtime) = &self.runtime {
      runtime.stop();
    }
  }
}

impl Drop for CallbackScope {
  fn drop(&mut self) {
    CURRENT.with(|current| current.replace(self.0.take()));
  }
}

pub(crate) fn callback(operation: impl FnOnce()) {
  let runtime = CURRENT.with(|current| current.borrow().clone());
  if let Some(runtime) = runtime {
    let mut operation = Some(operation);
    runtime.callback(&mut || operation.take().expect("callback runs once")());
  } else {
    operation();
  }
}

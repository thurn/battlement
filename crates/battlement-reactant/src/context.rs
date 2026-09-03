//! Logical component context values and providers.

use std::{
  any::{Any, TypeId},
  cell::{Cell, RefCell},
  collections::HashMap,
  marker::PhantomData,
  mem, ptr,
  rc::Rc,
};

use crate::{
  render::{Render, RenderSink},
  render_value::Sealed,
};

thread_local! {
  static CURRENT: Cell<RenderContext> = const { Cell::new(RenderContext::Outside) };
  static DEFAULTS: RefCell<Option<Rc<RefCell<ContextDefaults>>>> = const { RefCell::new(None) };
  static PROVIDERS: RefCell<Vec<ProviderValue>> = const { RefCell::new(Vec::new()) };
}

/// Identifies one optional logical context and its runtime default factory.
pub struct Context<T> {
  default: fn() -> T,
  identity: [u8; 1],
  _type: PhantomData<fn() -> T>,
}

/// Identifies one logical context that requires an ancestor provider.
pub struct RequiredContext<T> {
  identity: [u8; 1],
  _type: PhantomData<fn() -> T>,
}

/// Holds a value until a child is attached to an optional context provider.
pub struct ContextProvider<T> {
  identity: ContextIdentity,
  value: Rc<T>,
}

/// Holds a value until a child is attached to a required context provider.
pub struct RequiredContextProvider<T> {
  identity: ContextIdentity,
  value: Rc<T>,
}

/// Transparently provides a context value to one logical descendant tree.
pub struct Provided<T, R> {
  identity: ContextIdentity,
  value: Rc<T>,
  child: R,
}

impl<T> Copy for Context<T> {}

impl<T> Clone for Context<T> {
  fn clone(&self) -> Self {
    *self
  }
}

impl<T> Copy for RequiredContext<T> {}

impl<T> Clone for RequiredContext<T> {
  fn clone(&self) -> Self {
    *self
  }
}

impl<T> Context<T> {
  /// Creates a context whose default is evaluated once per runtime.
  pub const fn new(default: fn() -> T) -> Self {
    Self {
      default,
      identity: [0],
      _type: PhantomData,
    }
  }

  /// Begins a transparent provider with an owned value.
  pub fn provider(&'static self, value: T) -> ContextProvider<T>
  where
    T: Clone + PartialEq + 'static,
  {
    ContextProvider {
      identity: self.identity(),
      value: Rc::new(value),
    }
  }

  pub(crate) fn read(&'static self) -> T
  where
    T: Clone + 'static,
  {
    let identity = self.identity();
    if let Some(value) = provider_value::<T>(identity) {
      return value;
    }
    let defaults = DEFAULTS
      .with(|current| current.borrow().clone())
      .expect("Reactant context requires a runtime render context");
    if let Some(value) = defaults.borrow().get::<T>(identity) {
      return value;
    }
    let value = with_hooks_forbidden(self.default);
    defaults.borrow_mut().insert(identity, value.clone());
    value
  }

  pub(crate) fn identity(&'static self) -> ContextIdentity {
    ContextIdentity::new(&self.identity, TypeId::of::<T>())
  }
}

impl<T> RequiredContext<T> {
  /// Creates a context that panics when read without a provider.
  pub const fn new() -> Self {
    Self {
      identity: [0],
      _type: PhantomData,
    }
  }

  /// Begins a transparent provider with an owned value.
  pub fn provider(&'static self, value: T) -> RequiredContextProvider<T>
  where
    T: Clone + PartialEq + 'static,
  {
    RequiredContextProvider {
      identity: self.identity(),
      value: Rc::new(value),
    }
  }

  pub(crate) fn read(&'static self) -> T
  where
    T: Clone + 'static,
  {
    provider_value(self.identity()).expect("required Reactant context has no provider")
  }

  pub(crate) fn identity(&'static self) -> ContextIdentity {
    ContextIdentity::new(&self.identity, TypeId::of::<T>())
  }
}

impl<T> Default for RequiredContext<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T> ContextProvider<T> {
  /// Attaches the logical descendant tree that receives this value.
  pub fn child<R: Render>(self, child: R) -> Provided<T, R> {
    Provided {
      identity: self.identity,
      value: self.value,
      child,
    }
  }
}

impl<T> RequiredContextProvider<T> {
  /// Attaches the logical descendant tree that receives this value.
  pub fn child<R: Render>(self, child: R) -> Provided<T, R> {
    Provided {
      identity: self.identity,
      value: self.value,
      child,
    }
  }
}

impl<T, R> Render for Provided<T, R>
where
  T: Clone + PartialEq + 'static,
  R: Render,
{
}

#[allow(private_interfaces)]
impl<T, R> Sealed for Provided<T, R>
where
  T: Clone + PartialEq + 'static,
  R: Render,
{
  fn descriptor(&self) -> TypeId {
    TypeId::of::<ProviderMarker>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    sink.push_provider::<ProviderMarker>(
      ProviderValue::new(self.identity, Rc::clone(&self.value)),
      |children| self.child.render_into(children),
    );
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    sink
      .push_provider::<ProviderMarker>(ProviderValue::new(self.identity, self.value), |children| {
        self.child.render_owned(children)
      });
  }
}

pub(crate) fn with_component<T>(operation: impl FnOnce() -> T) -> T {
  with(RenderContext::Component, operation)
}

pub(crate) fn with_hooks_forbidden<T>(operation: impl FnOnce() -> T) -> T {
  with(RenderContext::HooksForbidden, operation)
}

pub(crate) fn with_runtime<T>(
  defaults: Rc<RefCell<ContextDefaults>>,
  operation: impl FnOnce() -> T,
) -> T {
  let previous = DEFAULTS.with(|current| current.replace(Some(defaults)));
  let previous_providers = PROVIDERS.with(|providers| providers.replace(Vec::new()));
  let _restore = RuntimeRestore {
    previous,
    previous_providers,
  };
  operation()
}

pub(crate) fn hooks_allowed() -> bool {
  CURRENT.get() == RenderContext::Component
}

pub(crate) fn rendering() -> bool {
  CURRENT.get() != RenderContext::Outside
}

#[derive(Default)]
pub(crate) struct ContextDefaults {
  values: HashMap<ContextIdentity, Box<dyn Any>>,
}

impl ContextDefaults {
  fn get<T: Clone + 'static>(&self, identity: ContextIdentity) -> Option<T> {
    self.values.get(&identity).map(|value| {
      value
        .downcast_ref::<T>()
        .expect("Reactant context type changed")
        .clone()
    })
  }

  fn insert<T: 'static>(&mut self, identity: ContextIdentity, value: T) {
    self.values.insert(identity, Box::new(value));
  }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub(crate) struct ContextIdentity {
  address: usize,
  value_type: TypeId,
}

impl ContextIdentity {
  fn new(identity: &'static [u8; 1], value_type: TypeId) -> Self {
    let address = ptr::from_ref(identity) as usize;
    assert_ne!(address, 0, "Reactant context identity must be nonzero");
    Self {
      address,
      value_type,
    }
  }
}

fn provider_value<T: Clone + 'static>(identity: ContextIdentity) -> Option<T> {
  PROVIDERS.with(|providers| {
    providers
      .borrow()
      .iter()
      .rev()
      .find(|provider| provider.identity == identity)
      .map(|provider| {
        provider
          .value
          .downcast_ref::<T>()
          .expect("Reactant context type changed")
          .clone()
      })
  })
}

fn with<T>(context: RenderContext, operation: impl FnOnce() -> T) -> T {
  let previous = CURRENT.replace(context);
  let _restore = RenderRestore(previous);
  operation()
}

struct ProviderMarker;

#[derive(Clone)]
pub(crate) struct ProviderValue {
  identity: ContextIdentity,
  value: Rc<dyn Any>,
}

impl ProviderValue {
  pub(crate) fn new<T: 'static>(identity: ContextIdentity, value: Rc<T>) -> Self {
    Self { identity, value }
  }

  pub(crate) fn enter<R>(&self, operation: impl FnOnce() -> R) -> R {
    PROVIDERS.with(|providers| providers.borrow_mut().push(self.clone()));
    let _restore = ProviderRestore;
    operation()
  }
}

struct RenderRestore(RenderContext);

impl Drop for RenderRestore {
  fn drop(&mut self) {
    CURRENT.set(self.0);
  }
}

struct ProviderRestore;

impl Drop for ProviderRestore {
  fn drop(&mut self) {
    PROVIDERS.with(|providers| {
      providers
        .borrow_mut()
        .pop()
        .expect("context provider scope exists");
    });
  }
}

struct RuntimeRestore {
  previous: Option<Rc<RefCell<ContextDefaults>>>,
  previous_providers: Vec<ProviderValue>,
}

impl Drop for RuntimeRestore {
  fn drop(&mut self) {
    PROVIDERS.with(|providers| {
      assert!(providers.borrow().is_empty());
      providers.replace(mem::take(&mut self.previous_providers));
    });
    DEFAULTS.with(|current| current.replace(self.previous.take()));
  }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RenderContext {
  Outside,
  Component,
  HooksForbidden,
}

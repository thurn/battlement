//! Typed asynchronous resource descriptors.

use std::{
  any::{Any, TypeId},
  cell::RefCell,
  convert::Infallible,
  error::Error,
  fmt,
  future::Future,
  hash::{Hash, Hasher},
  rc::Rc,
  sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
  },
};

use crate::{
  context,
  executor::BoxFuture,
  hook_storage::{HookKind, HookSlot},
  hooks,
  key::StructuralRender,
  render::{Render, RenderSink},
  render_error::RenderError,
  render_value::Sealed,
  resource_cache::ResourceSnapshot,
  resource_runtime::{self, ResourceConsumer, ResourceToken},
};

static NEXT_RESOURCE_ID: AtomicU64 = AtomicU64::new(1);

/// Describes one keyed asynchronous value source.
pub struct Resource<K, T, E = Infallible> {
  id: u64,
  loader: Arc<Loader<K, T, E>>,
}

/// One render-scoped observation of a resource entry.
pub struct ResourceRead<T, E> {
  snapshot: ResourceSnapshot<T, E>,
  token: ResourceToken,
  registration: Rc<ResourceRegistration>,
}

/// Describes the current state of one resource read without exposing its value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceStatus {
  /// The loader is still pending.
  Pending,
  /// The value is available.
  Ready,
  /// The loader returned an error.
  Failed,
}

type Loader<K, T, E> = dyn Fn(K) -> BoxFuture<'static, Result<T, E>> + Send + Sync;

impl<K: Send + 'static, T> Resource<K, T, Infallible> {
  /// Creates an infallible keyed asynchronous resource.
  pub fn new<F, Fut>(loader: F) -> Self
  where
    F: Fn(K) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = T> + Send + 'static,
  {
    let loader = Arc::new(loader);
    Resource::try_new(move |key| {
      let loader = Arc::clone(&loader);
      async move { Ok(loader(key).await) }
    })
  }
}

impl<K, T, E> Resource<K, T, E> {
  /// Creates a fallible keyed asynchronous resource.
  pub fn try_new<F, Fut>(loader: F) -> Self
  where
    F: Fn(K) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
  {
    let loader = Arc::new(loader);
    Self {
      id: NEXT_RESOURCE_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .expect("Reactant resource identity overflow"),
      loader: Arc::new(move |key| Box::pin(loader(key))),
    }
  }

  pub(crate) fn id(&self) -> u64 {
    self.id
  }

  pub(crate) fn load(&self, key: K) -> BoxFuture<'static, Result<T, E>> {
    (self.loader)(key)
  }
}

impl<K, T, E> Clone for Resource<K, T, E> {
  fn clone(&self) -> Self {
    Self {
      id: self.id,
      loader: Arc::clone(&self.loader),
    }
  }
}

impl<K, T, E> fmt::Debug for Resource<K, T, E> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.debug_tuple("Resource").field(&self.id).finish()
  }
}

impl<K, T, E> PartialEq for Resource<K, T, E> {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
  }
}

impl<K, T, E> Eq for Resource<K, T, E> {}

impl<K, T, E> Hash for Resource<K, T, E> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.id.hash(state);
  }
}

impl<T, E> ResourceRead<T, E>
where
  T: Send + Sync + 'static,
  E: Error + Send + Sync + 'static,
{
  /// Returns the observed cache state and subscribes this hook to changes.
  pub fn status(&self) -> ResourceStatus {
    self.registration.push(self.token.clone());
    match &self.snapshot {
      ResourceSnapshot::Pending(_) => ResourceStatus::Pending,
      ResourceSnapshot::Ready(_, _) => ResourceStatus::Ready,
      ResourceSnapshot::Failed(_, _) => ResourceStatus::Failed,
    }
  }

  /// Renders from the ready value, suspends while pending, or propagates failure.
  pub fn then<R>(self, render: impl FnOnce(Arc<T>) -> R + 'static) -> impl Render
  where
    R: Render,
  {
    match self.snapshot {
      ResourceSnapshot::Pending(_) => ResourceThen::Pending(self.token),
      ResourceSnapshot::Ready(_, value) => {
        self.registration.push(self.token);
        ResourceThen::Ready(context::with_hooks_forbidden(|| render(value)))
      }
      ResourceSnapshot::Failed(_, error) => ResourceThen::Failed(error),
    }
  }
}

/// Observes one keyed resource through a positional component hook.
pub fn use_resource<K, T, E>(resource: &Resource<K, T, E>, key: K) -> ResourceRead<T, E>
where
  K: Clone + Eq + Hash + Send + 'static,
  T: Send + Sync + 'static,
  E: Error + Send + Sync + 'static,
{
  let value_type = TypeId::of::<(K, T, E)>();
  let registration = hooks::use_slot(
    HookKind::Resource,
    value_type,
    |_| ResourceSlot::new(value_type),
    ResourceSlot::prepare,
  );
  let observation = resource_runtime::observe(resource, key);
  ResourceRead {
    snapshot: observation.snapshot,
    token: observation.token,
    registration,
  }
}

enum ResourceThen<R, E> {
  Pending(ResourceToken),
  Ready(R),
  Failed(Arc<E>),
}

struct ResourceSlot {
  consumer: Rc<ResourceConsumer>,
  rendered: Rc<ResourceRegistration>,
  value_type: TypeId,
}

struct ResourceRegistration {
  tokens: RefCell<Vec<ResourceToken>>,
}

impl ResourceSlot {
  fn new(value_type: TypeId) -> Self {
    Self {
      consumer: ResourceConsumer::new(),
      rendered: Rc::new(ResourceRegistration::new()),
      value_type,
    }
  }

  fn prepare(&mut self) -> Rc<ResourceRegistration> {
    self.rendered = Rc::new(ResourceRegistration::new());
    Rc::clone(&self.rendered)
  }
}

impl ResourceRegistration {
  fn new() -> Self {
    Self {
      tokens: RefCell::new(Vec::new()),
    }
  }

  fn push(&self, token: ResourceToken) {
    self.tokens.borrow_mut().push(token);
  }
}

impl HookSlot for ResourceSlot {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }

  fn clone_box(&self) -> Box<dyn HookSlot> {
    Box::new(Self {
      consumer: Rc::clone(&self.consumer),
      rendered: Rc::clone(&self.rendered),
      value_type: self.value_type,
    })
  }

  fn commit(&mut self) {
    self.consumer.replace(&self.rendered.tokens.borrow());
  }

  fn discard_pending(&mut self) {}

  fn has_pending(&self) -> bool {
    self.consumer.dirty()
  }

  fn has_pending_change(&self) -> bool {
    self.consumer.dirty()
  }

  fn context_changed(&self) -> bool {
    false
  }

  fn kind(&self) -> HookKind {
    HookKind::Resource
  }

  fn value_type(&self) -> TypeId {
    self.value_type
  }
}

impl<R, E> Render for ResourceThen<R, E>
where
  R: Render,
  E: Error + Send + Sync + 'static,
{
}

impl<R, E> StructuralRender for ResourceThen<R, E>
where
  R: Render,
  E: Error + Send + Sync + 'static,
{
}

impl<R, E> Sealed for ResourceThen<R, E>
where
  R: Render,
  E: Error + Send + Sync + 'static,
{
  fn descriptor(&self) -> TypeId {
    match self {
      Self::Pending(_) => TypeId::of::<PendingMarker>(),
      Self::Ready(value) => value.descriptor(),
      Self::Failed(_) => TypeId::of::<FailedMarker>(),
    }
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    match self {
      Self::Pending(token) => sink.suspend(token.clone()),
      Self::Ready(value) => value.render_into(sink),
      Self::Failed(error) => sink.fail(RenderError::from_shared_resource(Arc::clone(error))),
    }
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    match self {
      Self::Pending(token) => sink.suspend(token),
      Self::Ready(value) => value.render_owned(sink),
      Self::Failed(error) => sink.fail(RenderError::from_shared_resource(error)),
    }
  }
}

struct FailedMarker;
struct PendingMarker;

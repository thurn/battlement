//! Render-scoped access to one runtime resource cache.

use std::{
  cell::RefCell,
  error::Error,
  hash::Hash,
  rc::{Rc, Weak},
};

use crate::{
  executor::Spawner,
  resource::Resource,
  resource_cache::{ResourceCache, ResourceOverlay, ResourceSnapshot, ResourceWake},
};

thread_local! {
  static CURRENT: RefCell<Option<ResourceContext>> = const { RefCell::new(None) };
}

pub(crate) struct ResourceRuntime {
  pub(crate) cache: RefCell<ResourceCache>,
  pub(crate) spawner: Box<dyn Spawner>,
}

pub(crate) struct ResourceObservation<T, E> {
  pub(crate) snapshot: ResourceSnapshot<T, E>,
  pub(crate) token: ResourceToken,
}

pub(crate) struct ResourceConsumer {
  wake: Rc<ResourceWake>,
  runtime: RefCell<Option<Weak<ResourceRuntime>>>,
}

#[derive(Clone)]
pub(crate) struct ResourceToken {
  registration: Rc<dyn TokenRegistration>,
}

#[derive(Clone)]
struct ResourceContext {
  runtime: Rc<ResourceRuntime>,
  overlay: Option<Rc<ResourceOverlay>>,
}

impl ResourceRuntime {
  pub(crate) fn new(spawner: impl Spawner) -> Rc<Self> {
    Rc::new(Self {
      cache: RefCell::new(ResourceCache::new()),
      spawner: Box::new(spawner),
    })
  }
}

impl ResourceConsumer {
  pub(crate) fn new() -> Rc<Self> {
    Rc::new(Self {
      wake: ResourceWake::new(),
      runtime: RefCell::new(None),
    })
  }

  pub(crate) fn replace(&self, tokens: &[ResourceToken]) {
    if let Some(runtime) = self
      .runtime
      .borrow_mut()
      .take()
      .and_then(|runtime| runtime.upgrade())
    {
      runtime.cache.borrow_mut().remove_consumer(self.wake.id());
    }
    let runtime = tokens.first().map(ResourceToken::runtime);
    if let Some(expected) = &runtime {
      assert!(
        tokens
          .iter()
          .all(|token| Weak::ptr_eq(expected, &token.runtime())),
        "one Reactant resource consumer cannot span runtimes"
      );
    }
    for token in tokens {
      token.register(&self.wake);
    }
    *self.runtime.borrow_mut() = runtime;
    self.wake.clear();
  }

  pub(crate) fn dirty(&self) -> bool {
    self.wake.dirty()
  }
}

impl Drop for ResourceConsumer {
  fn drop(&mut self) {
    let Some(runtime) = self
      .runtime
      .get_mut()
      .take()
      .and_then(|runtime| runtime.upgrade())
    else {
      return;
    };
    runtime.cache.borrow_mut().remove_consumer(self.wake.id());
  }
}

impl ResourceToken {
  pub(crate) fn register(&self, wake: &Rc<ResourceWake>) {
    self.registration.register(Rc::downgrade(wake));
  }

  fn runtime(&self) -> Weak<ResourceRuntime> {
    self.registration.runtime()
  }
}

pub(crate) fn observe<K, T, E>(resource: &Resource<K, T, E>, key: K) -> ResourceObservation<T, E>
where
  K: Clone + Eq + Hash + Send + 'static,
  T: Send + Sync + 'static,
  E: Error + Send + Sync + 'static,
{
  let context = CURRENT
    .with(|current| current.borrow().clone())
    .expect("Reactant resources require a runtime render context");
  let (generation, cached) = {
    let mut cache = context.runtime.cache.borrow_mut();
    let generation = cache.request(resource, key.clone(), context.runtime.spawner.as_ref());
    let snapshot = cache
      .snapshot(resource, &key)
      .expect("requested resource entry exists");
    (generation, snapshot)
  };
  let snapshot = context
    .overlay
    .as_ref()
    .and_then(|overlay| overlay.snapshot(resource, &key, generation))
    .unwrap_or(cached);
  ResourceObservation {
    snapshot,
    token: ResourceToken {
      registration: Rc::new(TypedToken {
        runtime: Rc::downgrade(&context.runtime),
        resource: resource.clone(),
        key,
        generation,
      }),
    },
  }
}

pub(crate) fn with_runtime<T>(
  runtime: Rc<ResourceRuntime>,
  overlay: Option<Rc<ResourceOverlay>>,
  operation: impl FnOnce() -> T,
) -> T {
  let previous =
    CURRENT.with(|current| current.replace(Some(ResourceContext { runtime, overlay })));
  let _restore = Restore(previous);
  operation()
}

trait TokenRegistration {
  fn runtime(&self) -> Weak<ResourceRuntime>;
  fn register(&self, wake: Weak<ResourceWake>);
}

struct TypedToken<K, T, E> {
  runtime: Weak<ResourceRuntime>,
  resource: Resource<K, T, E>,
  key: K,
  generation: u64,
}

impl<K, T, E> TokenRegistration for TypedToken<K, T, E>
where
  K: Eq + Hash + 'static,
  T: 'static,
  E: 'static,
{
  fn runtime(&self) -> Weak<ResourceRuntime> {
    self.runtime.clone()
  }

  fn register(&self, wake: Weak<ResourceWake>) {
    let Some(runtime) = self.runtime.upgrade() else {
      return;
    };
    runtime
      .cache
      .borrow_mut()
      .register(&self.resource, &self.key, self.generation, wake);
  }
}

struct Restore(Option<ResourceContext>);

impl Drop for Restore {
  fn drop(&mut self) {
    CURRENT.with(|current| current.replace(self.0.take()));
  }
}

//! Typed asynchronous resource descriptors.

use std::{
  convert::Infallible,
  fmt,
  future::Future,
  hash::{Hash, Hasher},
  sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
  },
};

use crate::executor::BoxFuture;

static NEXT_RESOURCE_ID: AtomicU64 = AtomicU64::new(1);

/// Describes one keyed asynchronous value source.
pub struct Resource<K, T, E = Infallible> {
  id: u64,
  loader: Arc<Loader<K, T, E>>,
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

//! Typed asynchronous resources and runtime cache storage.

use std::{
  any::Any,
  collections::{HashMap, VecDeque},
  convert::Infallible,
  error::Error,
  fmt,
  future::Future,
  hash::{Hash, Hasher},
  panic::{self, AssertUnwindSafe},
  pin::Pin,
  sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Receiver, Sender},
  },
  task::{Context, Poll},
};

use crate::executor::{BoxFuture, SpawnedTask, Spawner};

static NEXT_RESOURCE_ID: AtomicU64 = AtomicU64::new(1);

/// Describes one keyed asynchronous value source.
pub struct Resource<K, T, E = Infallible> {
  id: u64,
  loader: Arc<Loader<K, T, E>>,
}

pub(crate) struct ResourceCache {
  buckets: HashMap<u64, Box<dyn Any>>,
  next_generation: u64,
  completions: Receiver<Completion>,
  completion_sender: Sender<Completion>,
  deferred: VecDeque<Completion>,
}

pub(crate) struct FrozenCompletions {
  operations: VecDeque<Completion>,
}

type PanicPayload = Box<dyn Any + Send + 'static>;
type Completion = Box<dyn CompletionOperation>;
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

impl ResourceCache {
  pub(crate) fn new() -> Self {
    let (completion_sender, completions) = mpsc::channel();
    Self {
      buckets: HashMap::new(),
      next_generation: 1,
      completions,
      completion_sender,
      deferred: VecDeque::new(),
    }
  }

  #[allow(dead_code)]
  pub(crate) fn request<K, T, E>(
    &mut self,
    resource: &Resource<K, T, E>,
    key: K,
    spawner: &dyn Spawner,
  ) -> u64
  where
    K: Clone + Eq + Hash + Send + 'static,
    T: Send + 'static,
    E: Error + Send + Sync + 'static,
  {
    if let Some(generation) = self.generation(resource, &key) {
      return generation;
    }
    self.start(resource, key, spawner)
  }

  #[allow(dead_code)]
  pub(crate) fn restart<K, T, E>(
    &mut self,
    resource: &Resource<K, T, E>,
    key: K,
    spawner: &dyn Spawner,
  ) -> u64
  where
    K: Clone + Eq + Hash + Send + 'static,
    T: Send + 'static,
    E: Error + Send + Sync + 'static,
  {
    self.bucket_mut(resource).entries.remove(&key);
    self.start(resource, key, spawner)
  }

  pub(crate) fn freeze(&mut self) -> FrozenCompletions {
    let mut operations = std::mem::take(&mut self.deferred);
    operations.extend(self.completions.try_iter());
    FrozenCompletions { operations }
  }

  pub(crate) fn restore(&mut self, frozen: FrozenCompletions) {
    assert!(
      self.deferred.is_empty(),
      "Reactant has only one resource completion transaction"
    );
    self.deferred = frozen.operations;
  }

  pub(crate) fn current_panic(&self, frozen: &mut FrozenCompletions) -> Option<PanicPayload> {
    frozen.operations.iter_mut().find_map(|completion| {
      completion
        .is_current(self)
        .then(|| completion.take_panic())
        .flatten()
    })
  }

  pub(crate) fn apply(&mut self, mut frozen: FrozenCompletions) -> Result<(), PanicPayload> {
    while let Some(mut completion) = frozen.operations.pop_front() {
      if !completion.is_current(self) {
        continue;
      }
      if let Some(payload) = completion.take_panic() {
        return Err(payload);
      }
      completion.apply(self);
    }
    Ok(())
  }

  fn start<K, T, E>(&mut self, resource: &Resource<K, T, E>, key: K, spawner: &dyn Spawner) -> u64
  where
    K: Clone + Eq + Hash + Send + 'static,
    T: Send + 'static,
    E: Error + Send + Sync + 'static,
  {
    let future = (resource.loader)(key.clone());
    let generation = self.next_generation;
    self.next_generation = self
      .next_generation
      .checked_add(1)
      .expect("Reactant resource generation overflow");
    self.bucket_mut(resource).entries.insert(
      key.clone(),
      CacheEntry::Pending {
        generation,
        task: None,
      },
    );
    let id = resource.id;
    let completion_key = key.clone();
    let sender = self.completion_sender.clone();
    let spawned = panic::catch_unwind(AssertUnwindSafe(|| {
      spawner.spawn(Box::pin(async move {
        let outcome = CatchPanic::new(future).await;
        let _ = sender.send(self::completion(id, completion_key, generation, outcome));
      }))
    }));
    let task = match spawned {
      Ok(task) => task,
      Err(payload) => {
        self.bucket_mut(resource).entries.remove(&key);
        panic::resume_unwind(payload);
      }
    };
    let entry = self
      .bucket_mut(resource)
      .entries
      .get_mut(&key)
      .expect("new resource cache entry exists");
    if let CacheEntry::Pending { task: slot, .. } = entry {
      *slot = Some(task);
    }
    generation
  }

  fn generation<K, T, E>(&self, resource: &Resource<K, T, E>, key: &K) -> Option<u64>
  where
    K: Eq + Hash + 'static,
    T: 'static,
    E: 'static,
  {
    self
      .buckets
      .get(&resource.id)
      .and_then(|bucket| bucket.downcast_ref::<ResourceBucket<K, T, E>>())
      .and_then(|bucket| bucket.entries.get(key))
      .map(CacheEntry::generation)
  }

  #[cfg(test)]
  pub(crate) fn is_pending<K, T, E>(&self, resource: &Resource<K, T, E>, key: &K) -> bool
  where
    K: Eq + Hash + 'static,
    T: 'static,
    E: 'static,
  {
    self
      .buckets
      .get(&resource.id)
      .and_then(|bucket| bucket.downcast_ref::<ResourceBucket<K, T, E>>())
      .and_then(|bucket| bucket.entries.get(key))
      .is_some_and(|entry| matches!(entry, CacheEntry::Pending { .. }))
  }

  fn bucket_mut<K: 'static, T: 'static, E: 'static>(
    &mut self,
    resource: &Resource<K, T, E>,
  ) -> &mut ResourceBucket<K, T, E> {
    self
      .buckets
      .entry(resource.id)
      .or_insert_with(|| Box::new(ResourceBucket::<K, T, E>::default()))
      .downcast_mut()
      .expect("a resource identity always has one key, value, and error shape")
  }
}

struct ResourceBucket<K, T, E> {
  entries: HashMap<K, CacheEntry<T, E>>,
}

impl<K, T, E> Default for ResourceBucket<K, T, E> {
  fn default() -> Self {
    Self {
      entries: HashMap::new(),
    }
  }
}

#[allow(dead_code)]
enum CacheEntry<T, E> {
  Pending {
    generation: u64,
    task: Option<SpawnedTask>,
  },
  Ready {
    generation: u64,
    value: Arc<T>,
  },
  Failed {
    generation: u64,
    error: Arc<E>,
  },
}

impl<T, E> CacheEntry<T, E> {
  fn generation(&self) -> u64 {
    match self {
      Self::Pending { generation, .. }
      | Self::Ready { generation, .. }
      | Self::Failed { generation, .. } => *generation,
    }
  }
}

struct CatchPanic<T> {
  future: BoxFuture<'static, T>,
}

impl<T> CatchPanic<T> {
  fn new(future: BoxFuture<'static, T>) -> Self {
    Self { future }
  }
}

impl<T> Future for CatchPanic<T> {
  type Output = Result<T, PanicPayload>;

  fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
    match panic::catch_unwind(AssertUnwindSafe(|| self.future.as_mut().poll(context))) {
      Ok(Poll::Ready(output)) => Poll::Ready(Ok(output)),
      Ok(Poll::Pending) => Poll::Pending,
      Err(payload) => Poll::Ready(Err(payload)),
    }
  }
}

trait CompletionOperation: Send {
  fn is_current(&self, cache: &ResourceCache) -> bool;
  fn take_panic(&mut self) -> Option<PanicPayload>;
  fn apply(self: Box<Self>, cache: &mut ResourceCache);
}

struct ResourceCompletion<K, T, E> {
  id: u64,
  key: K,
  generation: u64,
  outcome: Option<Result<Result<T, E>, PanicPayload>>,
}

impl<K, T, E> CompletionOperation for ResourceCompletion<K, T, E>
where
  K: Eq + Hash + Send + 'static,
  T: Send + 'static,
  E: Send + 'static,
{
  fn is_current(&self, cache: &ResourceCache) -> bool {
    let Some(bucket) = cache.buckets.get(&self.id) else {
      return false;
    };
    let bucket = bucket
      .downcast_ref::<ResourceBucket<K, T, E>>()
      .expect("a resource identity always has one key, value, and error shape");
    matches!(
      bucket.entries.get(&self.key),
      Some(CacheEntry::Pending { generation, .. }) if *generation == self.generation
    )
  }

  fn take_panic(&mut self) -> Option<PanicPayload> {
    if !matches!(self.outcome, Some(Err(_))) {
      return None;
    }
    match self.outcome.take().expect("completion outcome exists") {
      Err(payload) => Some(payload),
      Ok(_) => unreachable!("checked panic outcome"),
    }
  }

  fn apply(mut self: Box<Self>, cache: &mut ResourceCache) {
    let bucket = cache
      .buckets
      .get_mut(&self.id)
      .expect("current resource bucket exists")
      .downcast_mut::<ResourceBucket<K, T, E>>()
      .expect("a resource identity always has one key, value, and error shape");
    let entry = bucket
      .entries
      .remove(&self.key)
      .expect("current resource entry exists");
    if let CacheEntry::Pending {
      task: Some(task), ..
    } = entry
    {
      task.disarm();
    }
    match self.outcome.take().expect("completion outcome exists") {
      Ok(Ok(value)) => {
        bucket.entries.insert(
          self.key,
          CacheEntry::Ready {
            generation: self.generation,
            value: Arc::new(value),
          },
        );
      }
      Ok(Err(error)) => {
        bucket.entries.insert(
          self.key,
          CacheEntry::Failed {
            generation: self.generation,
            error: Arc::new(error),
          },
        );
      }
      Err(_) => unreachable!("current panic is delivered before application"),
    }
  }
}

fn completion<K, T, E>(
  id: u64,
  key: K,
  generation: u64,
  outcome: Result<Result<T, E>, PanicPayload>,
) -> Completion
where
  K: Eq + Hash + Send + 'static,
  T: Send + 'static,
  E: Send + 'static,
{
  Box::new(ResourceCompletion {
    id,
    key,
    generation,
    outcome: Some(outcome),
  })
}

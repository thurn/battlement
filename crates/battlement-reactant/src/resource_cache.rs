//! Runtime-wide asynchronous resource cache storage.

use std::{
  any::Any,
  cell::Cell,
  collections::{HashMap, VecDeque},
  error::Error,
  future::Future,
  hash::Hash,
  mem,
  panic::{self, AssertUnwindSafe},
  pin::Pin,
  rc::{Rc, Weak},
  sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Receiver, Sender},
  },
  task::{Context, Poll},
  thread,
};

use crate::{
  executor::{BoxFuture, SpawnedTask, Spawner},
  resource::Resource,
};

static NEXT_CONSUMER_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) struct ResourceCache {
  buckets: HashMap<u64, Box<dyn ErasedBucket>>,
  next_generation: u64,
  completions: Receiver<Completion>,
  completion_sender: Sender<Completion>,
  deferred: VecDeque<Completion>,
}

pub(crate) struct FrozenCompletions {
  operations: VecDeque<Completion>,
}

pub(crate) struct ResourceOverlay {
  entries: Vec<Box<dyn OverlayValue>>,
}

pub(crate) struct ResourceWake {
  id: u64,
  dirty: Cell<bool>,
}

pub(crate) enum ResourceSnapshot<T, E> {
  Pending(u64),
  Ready(u64, Arc<T>),
  Failed(u64, Arc<E>),
}

pub(crate) type PanicPayload = Box<dyn Any + Send + 'static>;
type Completion = Box<dyn CompletionOperation>;
type CompletionOutcome<T, E> = Result<Result<Arc<T>, Arc<E>>, PanicPayload>;

impl ResourceWake {
  pub(crate) fn new() -> Rc<Self> {
    Rc::new(Self {
      id: NEXT_CONSUMER_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .expect("Reactant resource consumer identity overflow"),
      dirty: Cell::new(false),
    })
  }

  pub(crate) fn id(&self) -> u64 {
    self.id
  }

  pub(crate) fn clear(&self) {
    self.dirty.set(false);
  }

  pub(crate) fn dirty(&self) -> bool {
    self.dirty.get()
  }

  fn mark(&self) {
    self.dirty.set(true);
  }
}

impl<T, E> Clone for ResourceSnapshot<T, E> {
  fn clone(&self) -> Self {
    match self {
      Self::Pending(generation) => Self::Pending(*generation),
      Self::Ready(generation, value) => Self::Ready(*generation, Arc::clone(value)),
      Self::Failed(generation, error) => Self::Failed(*generation, Arc::clone(error)),
    }
  }
}

impl ResourceOverlay {
  pub(crate) fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }

  pub(crate) fn snapshot<K, T, E>(
    &self,
    resource: &Resource<K, T, E>,
    key: &K,
    generation: u64,
  ) -> Option<ResourceSnapshot<T, E>>
  where
    K: Eq + 'static,
    T: 'static,
    E: 'static,
  {
    self.entries.iter().find_map(|entry| {
      entry
        .as_any()
        .downcast_ref::<TypedOverlay<K, T, E>>()
        .filter(|entry| entry.id == resource.id())
        .filter(|entry| entry.generation == generation)
        .filter(|entry| &entry.key == key)
        .map(|entry| entry.snapshot.clone())
    })
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
    T: Send + Sync + 'static,
    E: Error + Send + Sync + 'static,
  {
    if let Some(generation) = self.generation(resource, &key) {
      return generation;
    }
    self.start(resource, key, spawner)
  }

  pub(crate) fn invalidate<K, T, E>(
    &mut self,
    resource: &Resource<K, T, E>,
    key: &K,
  ) -> Result<(), PanicPayload>
  where
    K: Eq + Hash + 'static,
    T: 'static,
    E: 'static,
  {
    let mut entry = self
      .buckets
      .get_mut(&resource.id())
      .and_then(|bucket| {
        bucket
          .as_any_mut()
          .downcast_mut::<ResourceBucket<K, T, E>>()
      })
      .and_then(|bucket| bucket.entries.remove(key));
    if let Some(entry) = &mut entry {
      entry.mark_consumers();
    }
    self::cancel_tasks(entry.and_then(CacheEntry::into_task))
  }

  pub(crate) fn clear<K, T, E>(
    &mut self,
    resource: &Resource<K, T, E>,
  ) -> Result<(), PanicPayload> {
    let Some(mut bucket) = self.buckets.remove(&resource.id()) else {
      return Ok(());
    };
    self::cancel_tasks(bucket.take_tasks(true))
  }

  pub(crate) fn cancel_all(&mut self) -> Result<(), PanicPayload> {
    let tasks = self
      .buckets
      .drain()
      .flat_map(|(_, mut bucket)| bucket.take_tasks(false))
      .collect::<Vec<_>>();
    self::cancel_tasks(tasks)
  }

  pub(crate) fn freeze(&mut self) -> FrozenCompletions {
    let mut operations = mem::take(&mut self.deferred);
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

  pub(crate) fn overlay(&self, frozen: &FrozenCompletions) -> ResourceOverlay {
    for completion in &frozen.operations {
      completion.mark_consumers(self);
    }
    ResourceOverlay {
      entries: frozen
        .operations
        .iter()
        .filter_map(|completion| completion.overlay(self))
        .collect(),
    }
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

  pub(crate) fn snapshot<K, T, E>(
    &self,
    resource: &Resource<K, T, E>,
    key: &K,
  ) -> Option<ResourceSnapshot<T, E>>
  where
    K: Eq + Hash + 'static,
    T: 'static,
    E: 'static,
  {
    self
      .bucket(resource)
      .and_then(|bucket| bucket.entries.get(key))
      .map(CacheEntry::snapshot)
  }

  pub(crate) fn register<K, T, E>(
    &mut self,
    resource: &Resource<K, T, E>,
    key: &K,
    generation: u64,
    wake: Weak<ResourceWake>,
  ) where
    K: Eq + Hash + 'static,
    T: 'static,
    E: 'static,
  {
    let Some(entry) = self
      .bucket_mut(resource)
      .entries
      .get_mut(key)
      .filter(|entry| entry.generation() == generation)
    else {
      return;
    };
    let id = wake
      .upgrade()
      .expect("resource consumer remains alive while registering")
      .id();
    entry.consumers_mut().insert(id, wake);
  }

  pub(crate) fn remove_consumer(&mut self, id: u64) {
    for bucket in self.buckets.values_mut() {
      bucket.remove_consumer(id);
    }
  }

  #[cfg(test)]
  pub(crate) fn is_pending<K, T, E>(&self, resource: &Resource<K, T, E>, key: &K) -> bool
  where
    K: Eq + Hash + 'static,
    T: 'static,
    E: 'static,
  {
    self
      .bucket(resource)
      .and_then(|bucket| bucket.entries.get(key))
      .is_some_and(|entry| matches!(entry, CacheEntry::Pending { .. }))
  }

  fn start<K, T, E>(&mut self, resource: &Resource<K, T, E>, key: K, spawner: &dyn Spawner) -> u64
  where
    K: Clone + Eq + Hash + Send + 'static,
    T: Send + Sync + 'static,
    E: Error + Send + Sync + 'static,
  {
    let future = resource.load(key.clone());
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
        consumers: HashMap::new(),
      },
    );
    let id = resource.id();
    let completion_key = key.clone();
    let sender = self.completion_sender.clone();
    let spawned = panic::catch_unwind(AssertUnwindSafe(|| {
      spawner.spawn(Box::pin(async move {
        let outcome = CatchPanic::new(future)
          .await
          .map(|outcome| outcome.map(Arc::new).map_err(Arc::new));
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
      .bucket(resource)
      .and_then(|bucket| bucket.entries.get(key))
      .map(CacheEntry::generation)
  }

  fn bucket<K, T, E>(&self, resource: &Resource<K, T, E>) -> Option<&ResourceBucket<K, T, E>>
  where
    K: 'static,
    T: 'static,
    E: 'static,
  {
    self.buckets.get(&resource.id()).map(|bucket| {
      bucket
        .as_any()
        .downcast_ref()
        .expect("a resource identity always has one key, value, and error shape")
    })
  }

  fn bucket_mut<K: 'static, T: 'static, E: 'static>(
    &mut self,
    resource: &Resource<K, T, E>,
  ) -> &mut ResourceBucket<K, T, E> {
    self
      .buckets
      .entry(resource.id())
      .or_insert_with(|| Box::new(ResourceBucket::<K, T, E>::default()))
      .as_any_mut()
      .downcast_mut()
      .expect("a resource identity always has one key, value, and error shape")
  }
}

impl Drop for ResourceCache {
  fn drop(&mut self) {
    let Err(payload) = self.cancel_all() else {
      return;
    };
    if !thread::panicking() {
      panic::resume_unwind(payload);
    }
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

impl<K: 'static, T: 'static, E: 'static> ErasedBucket for ResourceBucket<K, T, E> {
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }

  fn take_tasks(&mut self, notify: bool) -> Vec<SpawnedTask> {
    mem::take(&mut self.entries)
      .into_values()
      .filter_map(|mut entry| {
        if notify {
          entry.mark_consumers();
        }
        entry.into_task()
      })
      .collect()
  }

  fn remove_consumer(&mut self, id: u64) {
    for entry in self.entries.values_mut() {
      entry.consumers_mut().remove(&id);
    }
  }
}

#[allow(dead_code)]
enum CacheEntry<T, E> {
  Pending {
    generation: u64,
    task: Option<SpawnedTask>,
    consumers: HashMap<u64, Weak<ResourceWake>>,
  },
  Ready {
    generation: u64,
    value: Arc<T>,
    consumers: HashMap<u64, Weak<ResourceWake>>,
  },
  Failed {
    generation: u64,
    error: Arc<E>,
    consumers: HashMap<u64, Weak<ResourceWake>>,
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

  fn snapshot(&self) -> ResourceSnapshot<T, E> {
    match self {
      Self::Pending { generation, .. } => ResourceSnapshot::Pending(*generation),
      Self::Ready {
        generation, value, ..
      } => ResourceSnapshot::Ready(*generation, Arc::clone(value)),
      Self::Failed {
        generation, error, ..
      } => ResourceSnapshot::Failed(*generation, Arc::clone(error)),
    }
  }

  fn consumers_mut(&mut self) -> &mut HashMap<u64, Weak<ResourceWake>> {
    match self {
      Self::Pending { consumers, .. }
      | Self::Ready { consumers, .. }
      | Self::Failed { consumers, .. } => consumers,
    }
  }

  fn consumers(&self) -> &HashMap<u64, Weak<ResourceWake>> {
    match self {
      Self::Pending { consumers, .. }
      | Self::Ready { consumers, .. }
      | Self::Failed { consumers, .. } => consumers,
    }
  }

  fn mark_consumers(&mut self) {
    self.consumers_mut().retain(|_, consumer| {
      let Some(wake) = consumer.upgrade() else {
        return false;
      };
      wake.mark();
      true
    });
  }

  fn into_pending(self) -> (Option<SpawnedTask>, HashMap<u64, Weak<ResourceWake>>) {
    match self {
      Self::Pending {
        task, consumers, ..
      } => (task, consumers),
      Self::Ready { .. } | Self::Failed { .. } => {
        panic!("only pending resource entries receive completions")
      }
    }
  }

  fn into_task(self) -> Option<SpawnedTask> {
    match self {
      Self::Pending { task, .. } => task,
      Self::Ready { .. } | Self::Failed { .. } => None,
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

trait ErasedBucket: Any {
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
  fn take_tasks(&mut self, notify: bool) -> Vec<SpawnedTask>;
  fn remove_consumer(&mut self, id: u64);
}

trait CompletionOperation: Send {
  fn is_current(&self, cache: &ResourceCache) -> bool;
  fn mark_consumers(&self, cache: &ResourceCache);
  fn take_panic(&mut self) -> Option<PanicPayload>;
  fn overlay(&self, cache: &ResourceCache) -> Option<Box<dyn OverlayValue>>;
  fn apply(self: Box<Self>, cache: &mut ResourceCache);
}

trait OverlayValue {
  fn as_any(&self) -> &dyn Any;
}

struct ResourceCompletion<K, T, E> {
  id: u64,
  key: K,
  generation: u64,
  outcome: Option<CompletionOutcome<T, E>>,
}

struct TypedOverlay<K, T, E> {
  id: u64,
  key: K,
  generation: u64,
  snapshot: ResourceSnapshot<T, E>,
}

impl<K: 'static, T: 'static, E: 'static> OverlayValue for TypedOverlay<K, T, E> {
  fn as_any(&self) -> &dyn Any {
    self
  }
}

impl<K, T, E> CompletionOperation for ResourceCompletion<K, T, E>
where
  K: Clone + Eq + Hash + Send + 'static,
  T: Send + Sync + 'static,
  E: Send + Sync + 'static,
{
  fn is_current(&self, cache: &ResourceCache) -> bool {
    let Some(bucket) = cache.buckets.get(&self.id) else {
      return false;
    };
    let bucket = bucket
      .as_any()
      .downcast_ref::<ResourceBucket<K, T, E>>()
      .expect("a resource identity always has one key, value, and error shape");
    matches!(
      bucket.entries.get(&self.key),
      Some(CacheEntry::Pending { generation, .. }) if *generation == self.generation
    )
  }

  fn mark_consumers(&self, cache: &ResourceCache) {
    let Some(bucket) = cache.buckets.get(&self.id) else {
      return;
    };
    let bucket = bucket
      .as_any()
      .downcast_ref::<ResourceBucket<K, T, E>>()
      .expect("a resource identity always has one key, value, and error shape");
    let Some(entry) = bucket.entries.get(&self.key) else {
      return;
    };
    if entry.generation() != self.generation {
      return;
    }
    for consumer in entry.consumers().values() {
      if let Some(wake) = consumer.upgrade() {
        wake.mark();
      }
    }
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

  fn overlay(&self, cache: &ResourceCache) -> Option<Box<dyn OverlayValue>> {
    if !self.is_current(cache) {
      return None;
    }
    let snapshot = match self.outcome.as_ref()? {
      Ok(Ok(value)) => ResourceSnapshot::Ready(self.generation, Arc::clone(value)),
      Ok(Err(error)) => ResourceSnapshot::Failed(self.generation, Arc::clone(error)),
      Err(_) => return None,
    };
    Some(Box::new(TypedOverlay {
      id: self.id,
      key: self.key.clone(),
      generation: self.generation,
      snapshot,
    }))
  }

  fn apply(mut self: Box<Self>, cache: &mut ResourceCache) {
    let bucket = cache
      .buckets
      .get_mut(&self.id)
      .expect("current resource bucket exists")
      .as_any_mut()
      .downcast_mut::<ResourceBucket<K, T, E>>()
      .expect("a resource identity always has one key, value, and error shape");
    let entry = bucket
      .entries
      .remove(&self.key)
      .expect("current resource entry exists");
    let (task, mut consumers) = entry.into_pending();
    if let Some(task) = task {
      task.disarm();
    }
    consumers.retain(|_, consumer| {
      let Some(wake) = consumer.upgrade() else {
        return false;
      };
      wake.mark();
      true
    });
    match self.outcome.take().expect("completion outcome exists") {
      Ok(Ok(value)) => {
        bucket.entries.insert(
          self.key,
          CacheEntry::Ready {
            generation: self.generation,
            value,
            consumers,
          },
        );
      }
      Ok(Err(error)) => {
        bucket.entries.insert(
          self.key,
          CacheEntry::Failed {
            generation: self.generation,
            error,
            consumers,
          },
        );
      }
      Err(_) => unreachable!("current panic is delivered before application"),
    }
  }
}

fn cancel_tasks(tasks: impl IntoIterator<Item = SpawnedTask>) -> Result<(), PanicPayload> {
  let mut first_panic = None;
  for task in tasks {
    if let Err(payload) = panic::catch_unwind(AssertUnwindSafe(|| task.cancel())) {
      first_panic.get_or_insert(payload);
    }
  }
  match first_panic {
    Some(payload) => Err(payload),
    None => Ok(()),
  }
}

fn completion<K, T, E>(
  id: u64,
  key: K,
  generation: u64,
  outcome: CompletionOutcome<T, E>,
) -> Completion
where
  K: Clone + Eq + Hash + Send + 'static,
  T: Send + Sync + 'static,
  E: Send + Sync + 'static,
{
  Box::new(ResourceCompletion {
    id,
    key,
    generation,
    outcome: Some(outcome),
  })
}

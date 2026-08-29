//! Runtime-wide asynchronous resource cache storage.

use std::{
  any::Any,
  collections::{HashMap, VecDeque},
  error::Error,
  future::Future,
  hash::Hash,
  mem,
  panic::{self, AssertUnwindSafe},
  pin::Pin,
  sync::{
    Arc,
    mpsc::{self, Receiver, Sender},
  },
  task::{Context, Poll},
  thread,
};

use crate::{
  executor::{BoxFuture, SpawnedTask, Spawner},
  resource::Resource,
};

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

pub(crate) type PanicPayload = Box<dyn Any + Send + 'static>;
type Completion = Box<dyn CompletionOperation>;

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
    let task = self
      .buckets
      .get_mut(&resource.id())
      .and_then(|bucket| {
        bucket
          .as_any_mut()
          .downcast_mut::<ResourceBucket<K, T, E>>()
      })
      .and_then(|bucket| bucket.entries.remove(key))
      .and_then(CacheEntry::into_task);
    self::cancel_tasks(task)
  }

  pub(crate) fn clear<K, T, E>(
    &mut self,
    resource: &Resource<K, T, E>,
  ) -> Result<(), PanicPayload> {
    let Some(mut bucket) = self.buckets.remove(&resource.id()) else {
      return Ok(());
    };
    self::cancel_tasks(bucket.take_tasks())
  }

  pub(crate) fn cancel_all(&mut self) -> Result<(), PanicPayload> {
    let tasks = self
      .buckets
      .drain()
      .flat_map(|(_, mut bucket)| bucket.take_tasks())
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
    T: Send + 'static,
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
      },
    );
    let id = resource.id();
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

  fn take_tasks(&mut self) -> Vec<SpawnedTask> {
    mem::take(&mut self.entries)
      .into_values()
      .filter_map(CacheEntry::into_task)
      .collect()
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
  fn take_tasks(&mut self) -> Vec<SpawnedTask>;
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
      .as_any()
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
      .as_any_mut()
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

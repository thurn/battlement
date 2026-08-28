use std::{
  collections::VecDeque,
  error::Error,
  fmt,
  future::Future,
  panic::{self, AssertUnwindSafe},
  pin::Pin,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
  },
  task::{Context, Poll, Wake, Waker},
  thread,
};

use battlement::{
  GameObject, GameObjectKind, ObjectId, ParentScene, PreparedAsset, Scene, SceneId, SessionId,
  Snapshot, UiDocument, UiDocumentState,
};
use uuid::Uuid;

use crate::{
  executor::{BoxFuture, SpawnedTask, Spawner},
  resource::{Resource, ResourceCache},
  runtime::Reactant,
};

#[derive(Clone)]
struct ManualSpawner {
  tasks: Arc<Mutex<VecDeque<BoxFuture<'static, ()>>>>,
  calls: Arc<AtomicUsize>,
}

struct InlineSpawner {
  calls: Arc<AtomicUsize>,
}

struct PanickingSpawner {
  calls: Arc<AtomicUsize>,
}

#[derive(Debug)]
struct LoadError;

struct NoopWake;

impl ManualSpawner {
  fn new() -> Self {
    Self {
      tasks: Arc::new(Mutex::new(VecDeque::new())),
      calls: Arc::new(AtomicUsize::new(0)),
    }
  }

  fn calls(&self) -> usize {
    self.calls.load(Ordering::Relaxed)
  }

  fn run_next(&self) {
    let task = self
      .tasks
      .lock()
      .expect("task queue lock")
      .pop_front()
      .expect("queued task");
    self::run_ready(task);
  }
}

impl Spawner for ManualSpawner {
  fn spawn(&self, task: BoxFuture<'static, ()>) -> SpawnedTask {
    self.calls.fetch_add(1, Ordering::Relaxed);
    self.tasks.lock().expect("task queue lock").push_back(task);
    SpawnedTask::detached()
  }
}

impl Spawner for InlineSpawner {
  fn spawn(&self, task: BoxFuture<'static, ()>) -> SpawnedTask {
    self.calls.fetch_add(1, Ordering::Relaxed);
    self::run_ready(task);
    SpawnedTask::detached()
  }
}

impl Spawner for PanickingSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    self.calls.fetch_add(1, Ordering::Relaxed);
    panic!("executor rejected task");
  }
}

impl fmt::Display for LoadError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("load failed")
  }
}

impl Error for LoadError {}

impl Wake for NoopWake {
  fn wake(self: Arc<Self>) {}
}

#[test]
fn resource_identity_and_cache_entries_are_shared_for_each_key_generation() {
  let spawner = ManualSpawner::new();
  let resource = Resource::<u32, u32>::new(|key| async move { key * 2 });
  let other = Resource::<u32, u32>::new(|key| async move { key * 3 });
  let mut cache = ResourceCache::new();

  assert_eq!(resource, resource.clone());
  assert_ne!(resource, other);
  assert_eq!(cache.request(&resource, 7, &spawner), 1);
  assert_eq!(cache.request(&resource.clone(), 7, &spawner), 1);
  assert_eq!(cache.request(&resource, 8, &spawner), 2);
  assert_eq!(cache.request(&other, 7, &spawner), 3);
  assert_eq!(spawner.calls(), 3);

  for _ in 0..3 {
    spawner.run_next();
  }
  self::apply_completions(&mut cache).expect("loads complete");
  assert_eq!(cache.request(&resource, 7, &spawner), 1);
  assert_eq!(spawner.calls(), 3);
}

#[test]
fn synchronous_completion_is_queued_until_the_task_handle_is_installed() {
  let calls = Arc::new(AtomicUsize::new(0));
  let spawner = InlineSpawner {
    calls: Arc::clone(&calls),
  };
  let resource = Resource::<u32, u32, LoadError>::try_new(|key| async move { Ok(key + 1) });
  let mut cache = ResourceCache::new();

  assert_eq!(cache.request(&resource, 4, &spawner), 1);
  assert_eq!(calls.load(Ordering::Relaxed), 1);
  self::apply_completions(&mut cache).expect("inline completion applies later");
  assert_eq!(cache.request(&resource, 4, &spawner), 1);
  assert_eq!(calls.load(Ordering::Relaxed), 1);
}

#[test]
fn stale_panics_are_suppressed_but_the_current_task_panics_on_the_engine_thread() {
  let attempts = Arc::new(AtomicUsize::new(0));
  let resource = Resource::<u32, u32, LoadError>::try_new({
    let attempts = Arc::clone(&attempts);
    move |key| {
      let attempt = attempts.fetch_add(1, Ordering::Relaxed);
      async move {
        assert_ne!(attempt, 0, "stale task panic");
        Ok(key)
      }
    }
  });
  let spawner = ManualSpawner::new();
  let mut cache = ResourceCache::new();

  assert_eq!(cache.request(&resource, 9, &spawner), 1);
  assert_eq!(cache.restart(&resource, 9, &spawner), 2);
  thread::spawn({
    let spawner = spawner.clone();
    move || spawner.run_next()
  })
  .join()
  .expect("task panic is captured");
  self::apply_completions(&mut cache).expect("stale panic is ignored");
  assert_eq!(cache.request(&resource, 9, &spawner), 2);
  spawner.run_next();
  self::apply_completions(&mut cache).expect("current replacement completes");

  let current = Resource::<u32, u32, LoadError>::try_new(|_| async move {
    panic!("current task panic");
  });
  let runtime_spawner = ManualSpawner::new();
  let executor = runtime_spawner.clone();
  let mut reactant = Reactant::<()>::new(runtime_spawner);
  let _ = reactant
    .begin_session(&mut ())
    .expect("empty session renders")
    .into_parts(Snapshot::new_with_main_camera(
      SessionId::new_v4(),
      vec![PreparedAsset::Scene("test/scene".into())],
      vec![Scene::new(SceneId::new_v4(), "test/scene")],
      Vec::new(),
    ))
    .1
    .into_groups();
  reactant.request_resource(&current, 1);
  thread::spawn(move || executor.run_next())
    .join()
    .expect("task panic is captured");
  let _ = reactant
    .refresh(&mut ())
    .expect("refresh does not freeze resource completions")
    .into_groups();
  let engine_thread = thread::current().id();
  let delivered = match panic::catch_unwind(AssertUnwindSafe(|| reactant.poll(&mut ()))) {
    Err(payload) => payload,
    Ok(_) => panic!("current task panic must reach the engine entry"),
  };
  assert_eq!(thread::current().id(), engine_thread);
  assert_eq!(self::panic_message(delivered), "current task panic");
  assert!(panic::catch_unwind(AssertUnwindSafe(|| reactant.poll(&mut ()))).is_err());
}

#[test]
fn registered_roots_share_the_runtime_resource_generation() {
  let spawner = ManualSpawner::new();
  let resource = Resource::<u32, u32>::new(|key| async move { key });
  let mut reactant = Reactant::<()>::new(spawner.clone());

  reactant.register_root(self::document(1), |_| ());
  assert_eq!(reactant.request_resource(&resource, 5), 1);
  reactant.register_root(self::document(2), |_| ());
  assert_eq!(reactant.request_resource(&resource, 5), 1);
  assert_eq!(spawner.calls(), 1);
}

#[test]
fn failed_session_restores_frozen_completions_for_the_corrected_retry() {
  let spawner = ManualSpawner::new();
  let executor = spawner.clone();
  let resource = Resource::<u32, u32>::new(|key| async move { key });
  let failing = Arc::new(AtomicBool::new(true));
  let document = self::document(3);
  let mut reactant = Reactant::<()>::new(spawner.clone());
  reactant.register_root(document.clone(), {
    let failing = Arc::clone(&failing);
    move |_| {
      if failing.load(Ordering::Relaxed) {
        Err(LoadError)
      } else {
        Ok(())
      }
    }
  });

  assert_eq!(reactant.request_resource(&resource, 7), 1);
  executor.run_next();
  assert!(reactant.begin_session(&mut ()).is_err());
  assert!(reactant.resource_is_pending(&resource, &7));
  assert_eq!(reactant.request_resource(&resource, 7), 1);
  assert_eq!(spawner.calls(), 1);

  failing.store(false, Ordering::Relaxed);
  let _ = reactant
    .begin_session(&mut ())
    .expect("corrected session renders")
    .into_parts(self::snapshot(&[document]))
    .1
    .into_groups();
  assert!(!reactant.resource_is_pending(&resource, &7));
  assert_eq!(reactant.request_resource(&resource, 7), 1);
  assert_eq!(spawner.calls(), 1);
}

#[test]
fn loader_and_executor_panics_leave_no_pending_cache_entry() {
  let loader_calls = Arc::new(AtomicUsize::new(0));
  let resource = Resource::<u32, u32, LoadError>::try_new({
    let loader_calls = Arc::clone(&loader_calls);
    move |_| {
      loader_calls.fetch_add(1, Ordering::Relaxed);
      panic!("loader construction failed");
      #[allow(unreachable_code)]
      std::future::ready(Ok(1))
    }
  });
  let mut cache = ResourceCache::new();
  let idle = ManualSpawner::new();
  for _ in 0..2 {
    assert!(panic::catch_unwind(AssertUnwindSafe(|| cache.request(&resource, 1, &idle))).is_err());
  }
  assert_eq!(loader_calls.load(Ordering::Relaxed), 2);

  let spawn_calls = Arc::new(AtomicUsize::new(0));
  let panicking = PanickingSpawner {
    calls: Arc::clone(&spawn_calls),
  };
  let resource = Resource::<u32, u32>::new(|key| async move { key });
  for _ in 0..2 {
    assert!(
      panic::catch_unwind(AssertUnwindSafe(|| cache.request(&resource, 1, &panicking))).is_err()
    );
  }
  assert_eq!(spawn_calls.load(Ordering::Relaxed), 2);
}

fn run_ready(mut task: Pin<Box<dyn Future<Output = ()> + Send>>) {
  let waker = Waker::from(Arc::new(NoopWake));
  let mut context = Context::from_waker(&waker);
  assert_eq!(task.as_mut().poll(&mut context), Poll::Ready(()));
}

fn apply_completions(cache: &mut ResourceCache) -> Result<(), Box<dyn std::any::Any + Send>> {
  let frozen = cache.freeze();
  cache.apply(frozen)
}

fn document(id: u128) -> UiDocument {
  UiDocument::with_root_id(
    ObjectId::from_uuid(Uuid::from_u128(id)).expect("nonzero fixture object ID"),
    ObjectId::from_uuid(Uuid::from_u128(id + 100)).expect("nonzero fixture root ID"),
  )
}

fn snapshot(documents: &[UiDocument]) -> Snapshot {
  Snapshot::new_with_main_camera(
    SessionId::new_v4(),
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(SceneId::new_v4(), "test/scene")],
    documents
      .iter()
      .map(|document| {
        GameObject::new(
          document.document_id,
          GameObjectKind::UiDocument(UiDocumentState::new(document.root_id)),
        )
        .parent_scene(ParentScene::Persistent)
      })
      .collect(),
  )
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> &'static str {
  *payload
    .downcast::<&'static str>()
    .expect("fixture panic payload is static text")
}

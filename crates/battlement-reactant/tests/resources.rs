use std::{
  cell::Cell,
  collections::VecDeque,
  error::Error,
  fmt,
  future::Future,
  panic::{self, AssertUnwindSafe},
  pin::Pin,
  rc::Rc,
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
  },
  task::{Context, Poll, Wake, Waker},
};

use battlement::{
  CommandBody, GameObject, GameObjectKind, Label, ObjectId, ParentScene, PreparedAsset, Scene,
  SceneId, SessionId, Snapshot, UiDocument, UiDocumentState,
};
use battlement_fake::battlement_ui_fake::UiWorld;
use battlement_reactant::{
  component::{self, Component},
  error_boundary::ErrorBoundary,
  executor::{BoxFuture, SpawnedTask, Spawner},
  hooks,
  render::{Either, Render},
  resource::{Resource, ResourceStatus, use_resource},
  runtime::{Reactant, ReactantCommit, RenderError},
  suspense::Suspense,
};

#[derive(Clone)]
struct ManualSpawner {
  tasks: Arc<Mutex<VecDeque<BoxFuture<'static, ()>>>>,
  calls: Arc<AtomicUsize>,
}

struct PairReads {
  resource: Resource<u32, u32>,
}

struct StatusReads {
  ready: Resource<u32, u32>,
  failed: Resource<u32, u32, LoadError>,
  renders: Rc<Cell<usize>>,
}

struct FailedRead {
  resource: Resource<u32, u32, LoadError>,
}

struct NestedRead {
  resource: Resource<u32, u32>,
}

struct BareRead {
  resource: Resource<u32, u32>,
}

struct InvalidThen {
  resource: Resource<u32, u32>,
}

struct RetryRead {
  resource: Resource<u32, u32>,
  fail: bool,
}

struct Visibility {
  load: bool,
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

impl Component for PairReads {
  fn render(&self) -> impl Render {
    let first = use_resource(&self.resource, 1);
    let second = use_resource(&self.resource, 2);
    let shared = use_resource(&self.resource, 1);
    Suspense::new(Label::new("pending")).child((
      first.then(|value| Label::new(format!("first:{value}"))),
      second.then(|value| Label::new(format!("second:{value}"))),
      shared.then(|value| Label::new(format!("shared:{value}"))),
    ))
  }
}

impl PartialEq for StatusReads {
  fn eq(&self, other: &Self) -> bool {
    self.ready == other.ready && self.failed == other.failed
  }
}

impl Component for StatusReads {
  fn render(&self) -> impl Render {
    self.renders.set(self.renders.get() + 1);
    (
      Label::new(self::status_text(use_resource(&self.ready, 1).status())),
      Label::new(self::status_text(use_resource(&self.failed, 2).status())),
    )
  }
}

impl Component for FailedRead {
  fn render(&self) -> impl Render {
    Suspense::new(Label::new("pending"))
      .child(use_resource(&self.resource, 1).then(|value| Label::new(value.to_string())))
  }
}

impl Component for NestedRead {
  fn render(&self) -> impl Render {
    Suspense::new(Label::new("outer pending")).child(
      Suspense::new(Label::new("inner pending"))
        .child(use_resource(&self.resource, 1).then(|value| Label::new(value.to_string()))),
    )
  }
}

impl Component for BareRead {
  fn render(&self) -> impl Render {
    use_resource(&self.resource, 1).then(|value| Label::new(value.to_string()))
  }
}

impl Component for InvalidThen {
  fn render(&self) -> impl Render {
    use_resource(&self.resource, 1).then(|value| {
      let _ = hooks::use_state(0_u8);
      Label::new(value.to_string())
    })
  }
}

impl Component for RetryRead {
  fn render(&self) -> impl Render {
    let content =
      use_resource(&self.resource, 1).then(|value| Label::new(format!("ready:{value}")));
    if self.fail {
      Err(LoadError)
    } else {
      Ok(content)
    }
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
fn sibling_and_shared_reads_start_together_before_the_fallback_commits() {
  let spawner = ManualSpawner::new();
  let resource = Resource::new(|key| async move { key * 10 });
  let document = self::document();
  let mut reactant = Reactant::new(spawner.clone());
  reactant.register_root(document.clone(), move |_| PairReads {
    resource: resource.clone(),
  });
  let mut world = self::begin(&mut reactant, &document);

  assert_eq!(self::texts(&world, document.root_id), ["pending"]);
  assert_eq!(spawner.calls(), 2);
  spawner.run_next();
  self::apply(&mut world, reactant.poll(&mut ()).unwrap());
  assert_eq!(self::texts(&world, document.root_id), ["pending"]);
  spawner.run_next();
  self::apply(&mut world, reactant.poll(&mut ()).unwrap());
  assert_eq!(
    self::texts(&world, document.root_id),
    ["first:10", "second:20", "shared:10"]
  );
  assert_eq!(spawner.calls(), 2);
}

#[test]
fn status_consumers_wake_in_every_state_and_defeat_memo_bailout() {
  let spawner = ManualSpawner::new();
  let ready = Resource::new(|key| async move { key });
  let failed = Resource::try_new(|_: u32| async move { Err::<u32, _>(LoadError) });
  let renders = Rc::new(Cell::new(0));
  let document = self::document();
  let mut reactant = Reactant::new(spawner.clone());
  let view_ready = ready.clone();
  let view_failed = failed.clone();
  let view_renders = Rc::clone(&renders);
  reactant.register_root(document.clone(), move |_| {
    component::memo(StatusReads {
      ready: view_ready.clone(),
      failed: view_failed.clone(),
      renders: Rc::clone(&view_renders),
    })
  });
  let mut world = self::begin(&mut reactant, &document);

  assert_eq!(
    self::texts(&world, document.root_id),
    ["Pending", "Pending"]
  );
  spawner.run_next();
  self::apply(&mut world, reactant.poll(&mut ()).unwrap());
  assert_eq!(self::texts(&world, document.root_id), ["Ready", "Pending"]);
  spawner.run_next();
  self::apply(&mut world, reactant.poll(&mut ()).unwrap());
  assert_eq!(self::texts(&world, document.root_id), ["Ready", "Failed"]);

  reactant.invalidate(&ready, &1);
  reactant.invalidate(&failed, &2);
  self::apply(&mut world, reactant.poll(&mut ()).unwrap());
  assert_eq!(
    self::texts(&world, document.root_id),
    ["Pending", "Pending"]
  );
  assert_eq!(spawner.calls(), 4);
  assert_eq!(renders.get(), 4);
}

#[test]
fn failed_reads_reach_the_nearest_error_boundary_as_the_concrete_error() {
  let spawner = ManualSpawner::new();
  let resource = Resource::try_new(|_: u32| async move { Err::<u32, _>(LoadError) });
  let document = self::document();
  let caught = Rc::new(Cell::new(false));
  let view_caught = Rc::clone(&caught);
  let mut reactant = Reactant::new(spawner.clone());
  reactant.register_root(document.clone(), move |_| {
    let caught = Rc::clone(&view_caught);
    ErrorBoundary::new(|_: &RenderError| Label::new("outer")).child(
      ErrorBoundary::new(move |error: &RenderError| {
        caught.set(error.downcast_ref::<LoadError>().is_some());
        Label::new("failed")
      })
      .child(FailedRead {
        resource: resource.clone(),
      }),
    )
  });
  let mut world = self::begin(&mut reactant, &document);

  assert_eq!(self::texts(&world, document.root_id), ["pending"]);
  spawner.run_next();
  self::apply(&mut world, reactant.poll(&mut ()).unwrap());
  assert_eq!(self::texts(&world, document.root_id), ["failed"]);
  assert!(caught.get());
}

#[test]
fn nested_suspense_consumes_pending_reads_at_the_nearest_boundary() {
  let spawner = ManualSpawner::new();
  let resource = Resource::new(|key| async move { key });
  let document = self::document();
  let mut reactant = Reactant::new(spawner.clone());
  reactant.register_root(document.clone(), move |_| NestedRead {
    resource: resource.clone(),
  });
  let mut world = self::begin(&mut reactant, &document);

  assert_eq!(self::texts(&world, document.root_id), ["inner pending"]);
  spawner.run_next();
  self::apply(&mut world, reactant.poll(&mut ()).unwrap());
  assert_eq!(self::texts(&world, document.root_id), ["1"]);
}

#[test]
fn a_missing_boundary_panics_atomically_and_poisons_the_runtime() {
  let spawner = ManualSpawner::new();
  let resource = Resource::new(|key| async move { key });
  let document = self::document();
  let view_resource = resource.clone();
  let mut game = Visibility { load: false };
  let mut reactant = Reactant::new(spawner);
  reactant.register_root(document.clone(), move |game: &Visibility| {
    if game.load {
      Either::Left(BareRead {
        resource: view_resource.clone(),
      })
    } else {
      Either::Right(Label::new("stable"))
    }
  });
  let world = self::begin_with(&mut reactant, &mut game, &document);
  assert_eq!(self::texts(&world, document.root_id), ["stable"]);

  game.load = true;
  let payload = panic::catch_unwind(AssertUnwindSafe(|| {
    let _ = reactant.refresh(&mut game);
  }))
  .expect_err("pending read without Suspense must panic");
  assert_eq!(
    self::panic_message(payload),
    "pending Reactant resource read requires a Suspense boundary"
  );
  assert_eq!(self::texts(&world, document.root_id), ["stable"]);
  assert!(panic::catch_unwind(AssertUnwindSafe(|| reactant.poll(&mut game))).is_err());
}

#[test]
fn resource_hooks_and_ready_mapping_closures_reject_forbidden_contexts() {
  let resource = Resource::new(|key| async move { key });
  let document = self::document();
  let mut outside = Reactant::new(ManualSpawner::new());
  let outside_resource = resource.clone();
  outside.register_root(document.clone(), move |_| {
    use_resource(&outside_resource, 1).then(|value| Label::new(value.to_string()))
  });
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let _ = outside.begin_session(&mut ());
    }))
    .is_err()
  );
  assert!(panic::catch_unwind(AssertUnwindSafe(|| outside.shutdown(&mut ()))).is_err());

  let spawner = ManualSpawner::new();
  let mut mapped = Reactant::new(spawner.clone());
  mapped.preload(&resource, 1);
  spawner.run_next();
  mapped.register_root(document, move |_| InvalidThen {
    resource: resource.clone(),
  });
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let _ = mapped.begin_session(&mut ());
    }))
    .is_err()
  );
  assert!(panic::catch_unwind(AssertUnwindSafe(|| mapped.shutdown(&mut ()))).is_err());
}

#[test]
fn completed_preload_is_ready_in_the_first_session_without_reloading() {
  let spawner = ManualSpawner::new();
  let resource = Resource::new(|key| async move { key * 10 });
  let document = self::document();
  let mut reactant = Reactant::new(spawner.clone());
  reactant.preload(&resource, 1);
  spawner.run_next();
  reactant.register_root(document.clone(), move |_| BareRead {
    resource: resource.clone(),
  });

  let world = self::begin(&mut reactant, &document);

  assert_eq!(self::texts(&world, document.root_id), ["10"]);
  assert_eq!(spawner.calls(), 1);
}

#[test]
fn failed_session_restores_a_completion_for_the_corrected_retry() {
  let spawner = ManualSpawner::new();
  let resource = Resource::new(|key| async move { key * 10 });
  let document = self::document();
  let mut game = Visibility { load: true };
  let mut reactant = Reactant::new(spawner.clone());
  reactant.preload(&resource, 1);
  spawner.run_next();
  reactant.register_root(document.clone(), move |game: &Visibility| RetryRead {
    resource: resource.clone(),
    fail: game.load,
  });

  assert!(reactant.begin_session(&mut game).is_err());
  game.load = false;
  let world = self::begin_with(&mut reactant, &mut game, &document);

  assert_eq!(self::texts(&world, document.root_id), ["ready:10"]);
  assert_eq!(spawner.calls(), 1);
}

fn status_text(status: ResourceStatus) -> &'static str {
  match status {
    ResourceStatus::Pending => "Pending",
    ResourceStatus::Ready => "Ready",
    ResourceStatus::Failed => "Failed",
  }
}

fn run_ready(mut task: Pin<Box<dyn Future<Output = ()> + Send>>) {
  let waker = Waker::from(Arc::new(NoopWake));
  let mut context = Context::from_waker(&waker);
  assert_eq!(task.as_mut().poll(&mut context), Poll::Ready(()));
}

fn begin(reactant: &mut Reactant<()>, document: &UiDocument) -> UiWorld {
  self::begin_with(reactant, &mut (), document)
}

fn begin_with<G: 'static>(
  reactant: &mut Reactant<G>,
  game: &mut G,
  document: &UiDocument,
) -> UiWorld {
  let rendered = reactant
    .begin_session(game)
    .unwrap()
    .into_parts(self::snapshot(document))
    .0;
  let mut world = UiWorld::default();
  world.replace(rendered.ui).unwrap();
  world
}

fn apply(world: &mut UiWorld, commit: ReactantCommit) {
  for body in commit.into_groups().into_iter().flatten() {
    match body {
      CommandBody::VisualElementCreate(value) => world.create(*value).unwrap(),
      CommandBody::VisualElementUpdate(value) => world.update(*value).unwrap(),
      CommandBody::VisualElementDestroy(value) => world.destroy(value.object_id).unwrap(),
      _ => panic!("Reactant emitted a non-UI command"),
    }
  }
}

fn texts(world: &UiWorld, root: ObjectId) -> Vec<&str> {
  world
    .element(root)
    .unwrap()
    .children()
    .iter()
    .map(|child| world.element(*child).unwrap().text().unwrap())
    .collect()
}

fn document() -> UiDocument {
  UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4())
}

fn snapshot(document: &UiDocument) -> Snapshot {
  Snapshot::new_with_main_camera(
    SessionId::new_v4(),
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(SceneId::new_v4(), "test/scene")],
    vec![
      GameObject::new(
        document.document_id,
        GameObjectKind::UiDocument(UiDocumentState::new(document.root_id)),
      )
      .parent_scene(ParentScene::Persistent),
    ],
  )
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> &'static str {
  *payload
    .downcast::<&'static str>()
    .expect("fixture panic payload is static text")
}

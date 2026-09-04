mod runtime_support;

use std::{
  collections::HashMap,
  num::NonZeroU64,
  panic::{self, AssertUnwindSafe},
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
  },
  thread,
};

use battlement::{
  CameraState, ClickEvent, CommandBody, GameObject, GameObjectKind, GeometryGeneration,
  GeometryObservationBatch, ObjectId, PanelScaleMode, PanelSettings, ParentScene, PreparedAsset,
  Scene, SceneId, SessionId, Snapshot, UiDocument, UiDocumentState, UiEvent,
};
use battlement_fake::battlement_ui_fake::UiWorld;
use battlement_reactant::{
  component::Component,
  executor::{BoxFuture, SpawnedTask, Spawner},
  external_store::{ExternalStore, StoreNotify, Subscription},
  hooks::use_external_store,
  render::Render,
  runtime::{Reactant, ReactantCommit},
};

struct IdleSpawner;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

#[derive(Clone)]
struct TestStore {
  name: &'static str,
  state: Arc<StoreState>,
}

struct StoreState {
  value: AtomicUsize,
  snapshot_reads: AtomicUsize,
  subscriptions: AtomicUsize,
  unsubscriptions: AtomicUsize,
  next_listener: AtomicUsize,
  change_on_subscribe: AtomicBool,
  notify_on_unsubscribe: AtomicBool,
  unstable: AtomicBool,
  listeners: Mutex<HashMap<usize, StoreNotify>>,
  log: Arc<Mutex<Vec<String>>>,
}

impl PartialEq for TestStore {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.state, &other.state)
  }
}

impl Eq for TestStore {}

impl ExternalStore for TestStore {
  type Snapshot = usize;

  fn snapshot(&self) -> Self::Snapshot {
    self.state.snapshot_reads.fetch_add(1, Ordering::Relaxed);
    if self.state.unstable.load(Ordering::Relaxed) {
      return self.state.value.fetch_add(1, Ordering::Relaxed);
    }
    self.state.value.load(Ordering::Acquire)
  }

  fn subscribe(&self, notify: StoreNotify) -> Subscription {
    self.state.subscriptions.fetch_add(1, Ordering::Relaxed);
    self
      .state
      .log
      .lock()
      .unwrap()
      .push(format!("subscribe {}", self.name));
    if self.state.change_on_subscribe.swap(false, Ordering::AcqRel) {
      self.state.value.fetch_add(1, Ordering::AcqRel);
    }
    let listener = self.state.next_listener.fetch_add(1, Ordering::Relaxed);
    self
      .state
      .listeners
      .lock()
      .unwrap()
      .insert(listener, notify.clone());
    let state = Arc::clone(&self.state);
    let name = self.name;
    Subscription::new(move || {
      if state.notify_on_unsubscribe.load(Ordering::Relaxed) {
        notify.notify();
      }
      state.listeners.lock().unwrap().remove(&listener);
      state.unsubscriptions.fetch_add(1, Ordering::Relaxed);
      state
        .log
        .lock()
        .unwrap()
        .push(format!("unsubscribe {name}"));
    })
  }
}

impl TestStore {
  fn new(name: &'static str, value: usize, log: Arc<Mutex<Vec<String>>>) -> Self {
    Self {
      name,
      state: Arc::new(StoreState {
        value: AtomicUsize::new(value),
        snapshot_reads: AtomicUsize::new(0),
        subscriptions: AtomicUsize::new(0),
        unsubscriptions: AtomicUsize::new(0),
        next_listener: AtomicUsize::new(0),
        change_on_subscribe: AtomicBool::new(false),
        notify_on_unsubscribe: AtomicBool::new(false),
        unstable: AtomicBool::new(false),
        listeners: Mutex::new(HashMap::new()),
        log,
      }),
    }
  }

  fn update_from_thread(&self, value: usize, notifications: usize) {
    self.state.value.store(value, Ordering::Release);
    let listeners = self
      .state
      .listeners
      .lock()
      .unwrap()
      .values()
      .cloned()
      .collect::<Vec<_>>();
    thread::spawn(move || {
      for _ in 0..notifications {
        for notify in &listeners {
          notify.notify();
        }
      }
    })
    .join()
    .unwrap();
  }
}

#[derive(Clone)]
struct StoreView {
  store: TestStore,
  renders: Arc<AtomicUsize>,
}

impl Component for StoreView {
  fn render(&self) -> impl Render {
    self.renders.fetch_add(1, Ordering::Relaxed);
    let snapshot = use_external_store(self.store.clone());
    battlement_reactant::host::Label::new(trox::ls(format!("{} {snapshot}", self.store.name)))
  }
}

struct SwapGame {
  store: TestStore,
  visible: bool,
}

#[test]
fn subscribe_recheck_closes_the_race_and_reuses_the_subscription() {
  let store = TestStore::new("store", 0, Arc::default());
  store
    .state
    .change_on_subscribe
    .store(true, Ordering::Relaxed);
  let renders = Arc::new(AtomicUsize::new(0));
  let document = self::document();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  let view_store = store.clone();
  let view_renders = Arc::clone(&renders);
  reactant.register_root(document.clone(), move |_: &()| StoreView {
    store: view_store.clone(),
    renders: Arc::clone(&view_renders),
  });

  let initial = self::begin(&mut reactant, &mut (), &document);
  let label = initial.ui[0].children[0].object_id;
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();
  assert_eq!(world.element(label).unwrap().text(), Some("store 1"));
  assert_eq!(store.state.subscriptions.load(Ordering::Relaxed), 1);
  assert_eq!(
    renders.load(Ordering::Relaxed),
    2,
    "the stale render retried"
  );

  assert!(reactant.refresh(&mut ()).unwrap().is_empty());
  assert_eq!(store.state.subscriptions.load(Ordering::Relaxed), 1);
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn coalesced_thread_wakes_are_consumed_by_every_active_entry() {
  let store = TestStore::new("store", 0, Arc::default());
  let renders = Arc::new(AtomicUsize::new(0));
  let document = self::document();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  let view_store = store.clone();
  let view_renders = Arc::clone(&renders);
  reactant.register_root(document.clone(), move |_: &()| StoreView {
    store: view_store.clone(),
    renders: Arc::clone(&view_renders),
  });
  let initial = self::begin(&mut reactant, &mut (), &document);
  let label = initial.ui[0].children[0].object_id;
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();

  store.update_from_thread(1, 4);
  self::apply(&mut world, reactant.poll(&mut ()).unwrap());
  assert_eq!(world.element(label).unwrap().text(), Some("store 1"));
  assert_eq!(renders.load(Ordering::Relaxed), 2);

  store.update_from_thread(2, 1);
  self::apply(
    &mut world,
    reactant
      .dispatch(
        &mut (),
        UiEvent::click(ObjectId::new_v4(), ClickEvent::NavigationSubmit),
      )
      .unwrap()
      .into_commit(),
  );
  assert_eq!(world.element(label).unwrap().text(), Some("store 2"));

  store.update_from_thread(3, 1);
  self::apply(&mut world, reactant.refresh(&mut ()).unwrap());
  assert_eq!(world.element(label).unwrap().text(), Some("store 3"));

  store.update_from_thread(4, 1);
  self::apply(
    &mut world,
    reactant
      .observe_geometry(&mut (), self::geometry())
      .unwrap(),
  );
  assert_eq!(world.element(label).unwrap().text(), Some("store 4"));

  store.update_from_thread(5, 1);
  let reconnected = self::begin(&mut reactant, &mut (), &document);
  world.replace(reconnected.ui).unwrap();
  assert_eq!(world.element(label).unwrap().text(), Some("store 5"));
  assert_eq!(store.state.subscriptions.load(Ordering::Relaxed), 1);
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn source_swaps_overlap_and_retired_wakes_cannot_dirty_the_new_generation() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let first = TestStore::new("first", 1, Arc::clone(&log));
  let second = TestStore::new("second", 2, Arc::clone(&log));
  first
    .state
    .notify_on_unsubscribe
    .store(true, Ordering::Relaxed);
  let renders = Arc::new(AtomicUsize::new(0));
  let document = self::document();
  let mut game = SwapGame {
    store: first.clone(),
    visible: true,
  };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  let view_renders = Arc::clone(&renders);
  reactant.register_root(document.clone(), move |game: &SwapGame| {
    game.visible.then(|| StoreView {
      store: game.store.clone(),
      renders: Arc::clone(&view_renders),
    })
  });
  let initial = self::begin(&mut reactant, &mut game, &document);
  let label = initial.ui[0].children[0].object_id;
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();

  game.store = second.clone();
  self::apply(&mut world, reactant.refresh(&mut game).unwrap());
  assert_eq!(world.element(label).unwrap().text(), Some("second 2"));
  assert_eq!(
    &*log.lock().unwrap(),
    &["subscribe first", "subscribe second", "unsubscribe first"]
  );
  let rendered = renders.load(Ordering::Relaxed);
  assert!(reactant.poll(&mut game).unwrap().is_empty());
  assert_eq!(renders.load(Ordering::Relaxed), rendered);

  first.update_from_thread(9, 1);
  assert!(reactant.poll(&mut game).unwrap().is_empty());
  second.update_from_thread(3, 1);
  self::apply(&mut world, reactant.poll(&mut game).unwrap());
  assert_eq!(world.element(label).unwrap().text(), Some("second 3"));
  assert_eq!(second.state.subscriptions.load(Ordering::Relaxed), 1);

  game.visible = false;
  self::apply(&mut world, reactant.refresh(&mut game).unwrap());
  assert_eq!(second.state.unsubscriptions.load(Ordering::Relaxed), 1);
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn retry_exhaustion_panics_and_poisons_the_runtime() {
  let store = TestStore::new("unstable", 0, Arc::default());
  store.state.unstable.store(true, Ordering::Relaxed);
  let document = self::document();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  let view_store = store.clone();
  reactant.register_root(document.clone(), move |_: &()| StoreView {
    store: view_store.clone(),
    renders: Arc::default(),
  });

  let failure = panic::catch_unwind(AssertUnwindSafe(|| {
    let _ = reactant.begin_session(&mut ());
  }));
  assert!(failure.is_err());
  assert_eq!(store.state.subscriptions.load(Ordering::Relaxed), 1);
  assert_eq!(store.state.unsubscriptions.load(Ordering::Relaxed), 1);
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let _ = reactant.begin_session(&mut ());
    }))
    .is_err()
  );
}

fn begin<G: 'static>(reactant: &mut Reactant<G>, game: &mut G, document: &UiDocument) -> Snapshot {
  reactant
    .begin_session(game)
    .unwrap()
    .into_parts(self::snapshot(document))
    .0
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

fn document() -> UiDocument {
  UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4())
}

fn snapshot(document: &UiDocument) -> Snapshot {
  let scene_id = SceneId::new_v4();
  let camera_id = ObjectId::new_v4();
  Snapshot::new(
    SessionId::new_v4(),
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(scene_id, "test/scene")],
    vec![
      GameObject::new(camera_id, CameraState::new()),
      GameObject::new(
        document.document_id,
        GameObjectKind::UiDocument(UiDocumentState::new(document.root_id).panel_settings(
          PanelSettings::new().scale_mode(PanelScaleMode::ConstantLogicalPixelSize),
        )),
      )
      .parent_scene(ParentScene::Persistent),
    ],
    camera_id,
  )
}

fn geometry() -> GeometryObservationBatch {
  GeometryObservationBatch {
    generation: GeometryGeneration(NonZeroU64::new(1).unwrap()),
    changed: Vec::new(),
  }
}

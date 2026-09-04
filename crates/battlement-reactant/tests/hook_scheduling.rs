mod runtime_support;

use std::{
  cell::{Cell, RefCell},
  collections::HashMap,
  panic::{self, AssertUnwindSafe},
  rc::Rc,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
  },
};

use battlement::{
  CameraState, ClickEvent, CommandBody, GameObject, GameObjectKind, ObjectId, PanelScaleMode,
  PanelSettings, ParentScene, PreparedAsset, Scene, SceneId, SessionId, Snapshot, UiDocument,
  UiDocumentState, UiEvent,
};
use battlement_fake::battlement_ui_fake::UiWorld;
use battlement_reactant::{
  component::{self, Component},
  context::Context,
  executor::{BoxFuture, SpawnedTask, Spawner},
  external_store::{ExternalStore, StoreNotify, Subscription},
  hooks::{self, Callback, ReducerDispatch, StateSetter},
  render::Render,
  runtime::{Reactant, ReactantCommit},
};

static THEME: Context<usize> = Context::new(|| 0);

type StoredCallback = Callback<Box<dyn Fn() -> usize>>;

struct IdleSpawner;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

#[derive(Clone)]
struct TestStore {
  state: Arc<StoreState>,
}

struct StoreState {
  value: AtomicUsize,
  change_on_subscribe: AtomicBool,
  snapshot_reads: AtomicUsize,
  next_listener: AtomicUsize,
  listeners: Mutex<HashMap<usize, StoreNotify>>,
}

impl PartialEq for TestStore {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.state, &other.state)
  }
}

impl ExternalStore for TestStore {
  type Snapshot = usize;

  fn snapshot(&self) -> Self::Snapshot {
    self.state.snapshot_reads.fetch_add(1, Ordering::Relaxed);
    self.state.value.load(Ordering::Acquire)
  }

  fn subscribe(&self, notify: StoreNotify) -> Subscription {
    if self.state.change_on_subscribe.swap(false, Ordering::AcqRel) {
      self.state.value.fetch_add(1, Ordering::AcqRel);
    }
    let listener = self.state.next_listener.fetch_add(1, Ordering::Relaxed);
    self
      .state
      .listeners
      .lock()
      .unwrap()
      .insert(listener, notify);
    let state = Arc::clone(&self.state);
    Subscription::new(move || {
      state.listeners.lock().unwrap().remove(&listener);
    })
  }
}

impl TestStore {
  fn new(value: usize) -> Self {
    Self {
      state: Arc::new(StoreState {
        value: AtomicUsize::new(value),
        change_on_subscribe: AtomicBool::new(false),
        snapshot_reads: AtomicUsize::new(0),
        next_listener: AtomicUsize::new(0),
        listeners: Mutex::new(HashMap::new()),
      }),
    }
  }

  fn set(&self, value: usize) {
    self.state.value.store(value, Ordering::Release);
    for notify in self.state.listeners.lock().unwrap().values() {
      notify.notify();
    }
  }
}

#[derive(Default)]
struct Handles {
  callback: Option<StoredCallback>,
  dispatch: Option<ReducerDispatch<usize>>,
  reference: Option<hooks::Ref<usize>>,
  setter: Option<StateSetter<usize>>,
}

struct Game {
  callback_dependency: usize,
  callback_value: usize,
  memo_dependency: usize,
  store: TestStore,
  theme: usize,
}

#[derive(Clone)]
struct HookMatrix {
  calculations: Rc<Cell<usize>>,
  callback_dependency: usize,
  effect_updates: bool,
  effect_setups: Rc<Cell<usize>>,
  handles: Rc<RefCell<Handles>>,
  renders: Rc<Cell<usize>>,
  memo_dependency: usize,
  store: TestStore,
}

impl PartialEq for HookMatrix {
  fn eq(&self, other: &Self) -> bool {
    (
      self.callback_dependency,
      self.effect_updates,
      self.memo_dependency,
      &self.store,
    ) == (
      other.callback_dependency,
      other.effect_updates,
      other.memo_dependency,
      &other.store,
    ) && Rc::ptr_eq(&self.handles, &other.handles)
  }
}

impl Component for HookMatrix {
  fn render(&self) -> impl Render {
    self.renders.set(self.renders.get() + 1);
    let (state, setter) = hooks::use_state(0_usize);
    let (reduced, dispatch) = hooks::use_reducer(|value, action| value + action, 0_usize);
    let reference = hooks::use_ref(4_usize);
    let stored = hooks::use_external_store(self.store.clone());
    let theme = hooks::use_context(&THEME);
    let calculations = Rc::clone(&self.calculations);
    let memo_dependency = self.memo_dependency;
    let memoized = hooks::use_memo(
      move || {
        calculations.set(calculations.get() + 1);
        memo_dependency * 10
      },
      self.memo_dependency,
    );
    let callback_dependency = self.callback_dependency;
    let callback = hooks::use_callback(
      Box::new(move || callback_dependency) as Box<dyn Fn() -> usize>,
      self.callback_dependency,
    );
    let effect_setups = Rc::clone(&self.effect_setups);
    let effect_setter = setter.clone();
    let effect_updates = self.effect_updates;
    hooks::use_effect(
      move || {
        effect_setups.set(effect_setups.get() + 1);
        if effect_updates {
          effect_setter.update(|value| value + 1);
        }
      },
      (),
    );
    self.handles.replace(Handles {
      callback: Some(callback.clone()),
      dispatch: Some(dispatch),
      reference: Some(reference),
      setter: Some(setter),
    });
    (
      battlement_reactant::host::Button::new(trox::assert_localized("Invoke callback")).on_click(
        move |game: &mut Game| {
          game.callback_value = callback();
        },
      ),
      battlement_reactant::host::Label::new(trox::assert_localized(format!(
        "state={state} reducer={reduced} store={stored} theme={theme} memo={memoized}"
      ))),
    )
  }
}

struct FailingCallback {
  setter: Rc<RefCell<Option<StateSetter<usize>>>>,
}

struct SessionCounter {
  setter: Rc<RefCell<Option<StateSetter<usize>>>>,
}

impl Component for FailingCallback {
  fn render(&self) -> impl Render {
    let (value, setter) = hooks::use_state(0_usize);
    self.setter.replace(Some(setter.clone()));
    let callback = hooks::use_callback(
      move || {
        setter.update(|current| current + 1);
        panic!("callback failed");
      },
      (),
    );
    (
      battlement_reactant::host::Button::new(trox::assert_localized("Fail"))
        .on_click(move |_game: &mut Game| callback()),
      battlement_reactant::host::Label::new(trox::assert_localized(format!("value={value}"))),
    )
  }
}

impl Component for SessionCounter {
  fn render(&self) -> impl Render {
    let (value, setter) = hooks::use_state(0_usize);
    self.setter.replace(Some(setter));
    battlement_reactant::host::Label::new(trox::assert_localized(format!("value={value}")))
  }
}

#[derive(Clone, Copy, Debug)]
enum Entry {
  BeginSession,
  Dispatch,
  Poll,
  Refresh,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Source {
  Callback,
  Context,
  Effect,
  Memo,
  Reducer,
  Ref,
  State,
  Store,
}

impl Entry {
  const ALL: [Self; 4] = [
    Self::Dispatch,
    Self::Refresh,
    Self::Poll,
    Self::BeginSession,
  ];
}

impl Source {
  const ALL: [Self; 8] = [
    Self::State,
    Self::Reducer,
    Self::Store,
    Self::Effect,
    Self::Context,
    Self::Memo,
    Self::Callback,
    Self::Ref,
  ];

  fn eligible(self, entry: Entry) -> bool {
    match self {
      Self::State | Self::Reducer | Self::Store => true,
      Self::Effect => !matches!(entry, Entry::BeginSession),
      Self::Callback | Self::Context | Self::Memo | Self::Ref => !matches!(entry, Entry::Poll),
    }
  }
}

#[test]
fn public_hook_lifecycle_matrix_and_transactional_failures() {
  for source in Source::ALL {
    for entry in Entry::ALL {
      if source.eligible(entry) {
        self::exercise_entry(source, entry);
      }
    }
  }
  self::store_retry_applies_queued_state_once();
  self::unconsumed_session_poisons_without_committing();
  self::callback_failure_poisons_without_committing();
}

fn exercise_entry(source: Source, entry: Entry) {
  let store = TestStore::new(0);
  let handles = Rc::new(RefCell::new(Handles::default()));
  let calculations = Rc::new(Cell::new(0));
  let effect_setups = Rc::new(Cell::new(0));
  let renders = Rc::new(Cell::new(0));
  let document = self::document();
  let mut game = Game {
    callback_dependency: 0,
    callback_value: usize::MAX,
    memo_dependency: 0,
    store: store.clone(),
    theme: 0,
  };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  let view_handles = Rc::clone(&handles);
  let view_calculations = Rc::clone(&calculations);
  let view_effect_setups = Rc::clone(&effect_setups);
  let view_renders = Rc::clone(&renders);
  reactant.register_root(document.clone(), move |game: &Game| {
    THEME
      .provider(game.theme)
      .child(component::memo(HookMatrix {
        calculations: Rc::clone(&view_calculations),
        callback_dependency: game.callback_dependency,
        effect_updates: matches!(source, Source::Effect),
        effect_setups: Rc::clone(&view_effect_setups),
        handles: Rc::clone(&view_handles),
        renders: Rc::clone(&view_renders),
        memo_dependency: game.memo_dependency,
        store: game.store.clone(),
      }))
  });
  let initial = self::begin(&mut reactant, &mut game, &document);
  let button = initial.ui[0].children[0].object_id;
  let label = initial.ui[0].children[1].object_id;
  let first_callback = handles.borrow().callback.clone().unwrap();
  let first_reference = handles.borrow().reference.clone().unwrap();
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();

  match source {
    Source::Callback => game.callback_dependency = 1,
    Source::Context => game.theme = 5,
    Source::Effect => {}
    Source::Memo => game.memo_dependency = 1,
    Source::Reducer => handles.borrow().dispatch.clone().unwrap().send(2),
    Source::Ref => {
      first_reference.replace(9);
    }
    Source::State => handles
      .borrow()
      .setter
      .clone()
      .unwrap()
      .update(|value| value + 1),
    Source::Store => store.set(3),
  }

  match entry {
    Entry::BeginSession => {
      let snapshot = reactant
        .begin_session(&mut game)
        .unwrap()
        .into_parts(self::snapshot(&document))
        .0;
      world.replace(snapshot.ui).unwrap();
    }
    Entry::Dispatch => self::apply(
      &mut world,
      reactant
        .dispatch(
          &mut game,
          UiEvent::click(
            if matches!(
              source,
              Source::Callback | Source::Context | Source::Memo | Source::Ref
            ) {
              button
            } else {
              ObjectId::new_v4()
            },
            ClickEvent::NavigationSubmit,
          ),
        )
        .unwrap()
        .into_commit(),
    ),
    Entry::Poll => self::apply(&mut world, reactant.poll(&mut game).unwrap()),
    Entry::Refresh => self::apply(&mut world, reactant.refresh(&mut game).unwrap()),
  }

  let expected_state = usize::from(matches!(source, Source::Effect | Source::State));
  let expected_reducer = 2 * usize::from(matches!(source, Source::Reducer));
  let expected_store = 3 * usize::from(matches!(source, Source::Store));
  let expected_theme = 5 * usize::from(matches!(source, Source::Context));
  let expected_memo = 10 * usize::from(matches!(source, Source::Memo));
  assert_eq!(
    world.element(label).unwrap().text(),
    Some(
      format!(
        "state={expected_state} reducer={expected_reducer} store={expected_store} \
         theme={expected_theme} memo={expected_memo}"
      )
      .as_str()
    ),
    "{source:?} through {entry:?}"
  );
  assert_eq!(
    first_reference.get(),
    if matches!(source, Source::Ref) { 9 } else { 4 },
    "{source:?} through {entry:?}"
  );
  assert!(first_reference == handles.borrow().reference.clone().unwrap());
  assert_eq!(
    handles.borrow().callback.as_ref().unwrap()(),
    usize::from(matches!(source, Source::Callback))
  );
  let latest_callback = handles.borrow().callback.clone().unwrap();
  if matches!(source, Source::Callback) {
    assert!(first_callback != latest_callback);
  } else {
    assert!(first_callback == latest_callback);
  }
  assert_eq!(
    calculations.get(),
    1 + usize::from(matches!(source, Source::Memo)),
    "{source:?} through {entry:?}"
  );
  assert_eq!(
    effect_setups.get(),
    usize::from(!matches!(entry, Entry::BeginSession))
  );
  assert_eq!(
    renders.get(),
    if matches!(source, Source::Ref) { 1 } else { 2 },
    "{source:?} through {entry:?}"
  );
  let completed_renders = renders.get();
  let completed_reads = store.state.snapshot_reads.load(Ordering::Relaxed);
  assert!(
    reactant.poll(&mut game).unwrap().is_empty(),
    "{source:?} through {entry:?}"
  );
  assert_eq!(renders.get(), completed_renders);
  assert_eq!(
    store.state.snapshot_reads.load(Ordering::Relaxed),
    completed_reads,
    "successful {entry:?} left frozen store work after {source:?}"
  );
  let _ = reactant.shutdown(&mut game).into_groups();
}

fn store_retry_applies_queued_state_once() {
  let first = TestStore::new(0);
  let second = TestStore::new(10);
  second
    .state
    .change_on_subscribe
    .store(true, Ordering::Relaxed);
  let handles = Rc::new(RefCell::new(Handles::default()));
  let renders = Rc::new(Cell::new(0));
  let document = self::document();
  let mut game = Game {
    callback_dependency: 0,
    callback_value: 0,
    memo_dependency: 0,
    store: first,
    theme: 0,
  };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  let view_handles = Rc::clone(&handles);
  let view_renders = Rc::clone(&renders);
  reactant.register_root(document.clone(), move |game: &Game| HookMatrix {
    calculations: Rc::new(Cell::new(0)),
    callback_dependency: 0,
    effect_updates: false,
    effect_setups: Rc::new(Cell::new(0)),
    handles: Rc::clone(&view_handles),
    renders: Rc::clone(&view_renders),
    memo_dependency: 0,
    store: game.store.clone(),
  });
  let initial = self::begin(&mut reactant, &mut game, &document);
  let label = initial.ui[0].children[1].object_id;
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();
  handles
    .borrow()
    .setter
    .clone()
    .unwrap()
    .update(|value| value + 1);
  game.store = second;

  self::apply(&mut world, reactant.refresh(&mut game).unwrap());

  assert_eq!(
    world.element(label).unwrap().text(),
    Some("state=1 reducer=0 store=11 theme=0 memo=0")
  );
  assert_eq!(renders.get(), 3, "the subscription recheck retries once");
  assert!(reactant.poll(&mut game).unwrap().is_empty());
  let _ = reactant.shutdown(&mut game).into_groups();
}

fn unconsumed_session_poisons_without_committing() {
  let setter = Rc::new(RefCell::new(None));
  let document = self::document();
  let mut game = Game {
    callback_dependency: 0,
    callback_value: 0,
    memo_dependency: 0,
    store: TestStore::new(0),
    theme: 0,
  };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  let view_setter = Rc::clone(&setter);
  reactant.register_root(document.clone(), move |_: &Game| SessionCounter {
    setter: Rc::clone(&view_setter),
  });
  let initial = self::begin(&mut reactant, &mut game, &document);
  let label = initial.ui[0].children[0].object_id;
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();
  setter.borrow().clone().unwrap().set(1);

  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let _session = reactant.begin_session(&mut game).unwrap();
    }))
    .is_err()
  );
  assert_eq!(world.element(label).unwrap().text(), Some("value=0"));
  assert!(panic::catch_unwind(AssertUnwindSafe(|| reactant.poll(&mut game))).is_err());
}

fn callback_failure_poisons_without_committing() {
  let setter = Rc::new(RefCell::new(None));
  let document = self::document();
  let mut game = Game {
    callback_dependency: 0,
    callback_value: 0,
    memo_dependency: 0,
    store: TestStore::new(0),
    theme: 0,
  };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  let view_setter = Rc::clone(&setter);
  reactant.register_root(document.clone(), move |_: &Game| FailingCallback {
    setter: Rc::clone(&view_setter),
  });
  let initial = self::begin(&mut reactant, &mut game, &document);
  let button = initial.ui[0].children[0].object_id;
  let label = initial.ui[0].children[1].object_id;
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();

  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      reactant.dispatch(
        &mut game,
        UiEvent::click(button, ClickEvent::NavigationSubmit),
      )
    }))
    .is_err()
  );
  assert_eq!(world.element(label).unwrap().text(), Some("value=0"));
  assert!(setter.borrow().is_some());
  assert!(panic::catch_unwind(AssertUnwindSafe(|| reactant.poll(&mut game))).is_err());
}

fn begin(reactant: &mut Reactant<Game>, game: &mut Game, document: &UiDocument) -> Snapshot {
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

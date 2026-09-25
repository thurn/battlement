use std::{
  cell::{Cell, RefCell},
  rc::Rc,
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::Duration,
};

use battlement::{ObjectId, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{
  Application, GameConsumer, GameOutput, GameStatus, ReducerDispatch, ReducerHandle,
  host::ButtonHost, prelude::*,
};
use reactant_rules::{GameReducer, ReducerGame, ReducerOutput};
use reactant_testing::Display;
use trox::ls;

const ROOT: ObjectId = object_id!("0a8b6b93-330b-4c66-b397-020e86780227");
const TIMEOUT: Duration = Duration::from_secs(10);

type CounterGame = ReducerGame<Counter>;

#[derive(Default)]
struct Probe {
  clones: AtomicUsize,
  events: AtomicUsize,
  validations: AtomicUsize,
}

struct State {
  value: usize,
  probe: Arc<Probe>,
}

impl Clone for State {
  fn clone(&self) -> Self {
    self.probe.clones.fetch_add(1, Ordering::SeqCst);
    Self {
      value: self.value,
      probe: self.probe.clone(),
    }
  }
}

struct Counter;

impl GameReducer for Counter {
  type State = State;
  type Action = usize;
  type Event = usize;
  type Rejection = &'static str;

  fn validate(state: &State, action: &usize) -> Result<(), Self::Rejection> {
    state.probe.validations.fetch_add(1, Ordering::SeqCst);
    if *action == 0 {
      Err("empty action")
    } else {
      Ok(())
    }
  }

  fn reduce(&mut self, state: &mut State, action: usize, output: &mut ReducerOutput<Self>) {
    for _ in 0..action {
      state.value += 1;
      output.publish(state, || {
        state.probe.events.fetch_add(1, Ordering::SeqCst);
        state.value
      });
    }
  }
}

#[derive(Clone, Default)]
struct Mount {
  initialized: Rc<Cell<usize>>,
  handle: Rc<RefCell<Option<ReducerHandle<Counter>>>>,
  probe: Arc<Probe>,
}

struct CounterView;

impl Component for CounterView {
  fn render(&self) -> impl Render {
    let value = reactant::use_reducer_selector::<Counter, _>(|state| state.value);
    View::new().name(format!("presented-{value}"))
  }
}

impl Component for Mount {
  fn render(&self) -> impl Render {
    let (key, replace) = reactant::hooks::use_state(0_usize);
    let (renders, rerender) = reactant::hooks::use_state(0_usize);
    let initialized = self.initialized.clone();
    let probe = self.probe.clone();
    let game = reactant::use_game_reducer(
      key,
      move || {
        initialized.set(initialized.get() + 1);
        State {
          value: key * 100,
          probe,
        }
      },
      Counter,
    );
    *self.handle.borrow_mut() = Some(game.clone());
    (
      View::new().name(format!("accepted-{}", game.accepted().state.value)),
      GameRoot::new(CounterView),
      ButtonHost::new(ls("Render"))
        .name("render")
        .on_click(move || rerender.set(renders + 1)),
      ButtonHost::new(ls("Replace"))
        .name("replace")
        .on_click(move || replace.set(key + 1)),
    )
  }
}

fn setup() -> (
  Display<reactant::ApplicationEngine>,
  Mount,
  GameConsumer<CounterGame>,
) {
  let mount = Mount::default();
  let component = mount.clone();
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("reducer/scene");
  let mut display = Display::mount(
    move || {
      Application::new("reducer/scene")
        .child(component.clone())
        .document(|mut document| {
          document.root_id = ROOT;
          document
        })
    },
    assets,
  );
  display.flush();
  let consumer = display.with_engine(|engine| engine.game_consumer::<CounterGame>());
  let game = mount.handle.borrow().as_ref().unwrap().clone();
  assert_eq!(game.status(), GameStatus::Busy);
  assert_eq!(
    game.dispatch(game.accepted().version, 0),
    ReducerDispatch::Busy
  );
  assert_eq!(mount.probe.validations.load(Ordering::SeqCst), 0);
  self::output(&consumer).submitted();
  (display, mount, consumer)
}

fn output(consumer: &GameConsumer<CounterGame>) -> GameOutput<CounterGame> {
  assert!(consumer.wait_for_output(TIMEOUT));
  consumer.take_output().expect("published output")
}

#[test]
fn lazy_stable_sessions_guard_input_and_accept_only_final_submission() {
  let (mut display, mount, consumer) = self::setup();
  let game = mount.handle.borrow().as_ref().unwrap().clone();
  let initial = game.accepted().version;
  assert_eq!(initial.revision, 0);
  assert_eq!(game.status(), GameStatus::Ready);
  display.click_ui(display.find_ui(ROOT, "render"));
  assert!(game == *mount.handle.borrow().as_ref().unwrap());
  assert_eq!(mount.initialized.get(), 1);
  assert_eq!(
    game.dispatch(initial, 0),
    ReducerDispatch::Rejected("empty action")
  );
  assert_eq!(game.dispatch(initial, 2), ReducerDispatch::Started);
  let validations = mount.probe.validations.load(Ordering::SeqCst);
  assert_eq!(game.dispatch(initial, 0), ReducerDispatch::Busy);
  assert_eq!(mount.probe.validations.load(Ordering::SeqCst), validations);
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  assert_eq!(game.accepted().state.value, 0);
  let first = self::output(&consumer);
  assert_eq!(first.animation(), Some(&1));
  assert_eq!(game.presented().state.value, 1);
  assert_eq!(game.presented().version.revision, 1);
  first.submitted();
  display.poll();
  let _ = display.find_ui(ROOT, "presented-1");
  let _ = display.find_ui(ROOT, "accepted-0");
  self::output(&consumer).submitted();
  let final_output = self::output(&consumer);
  assert!(final_output.is_final());
  assert_eq!(game.accepted().version, initial);
  assert_eq!(game.accepted().state.value, 0);
  final_output.submitted();
  final_output.submitted();
  first.submitted();
  assert_eq!(game.accepted().state.value, 2);
  assert_eq!(game.accepted().version.revision, 1);
  assert_eq!(game.presented().version, game.accepted().version);
  assert_eq!(game.dispatch(initial, 1), ReducerDispatch::Stale);
  display.poll();
  let _ = display.find_ui(ROOT, "accepted-2");
  display.click_ui(display.find_ui(ROOT, "replace"));
  display.flush();
  let replacement = mount.handle.borrow().as_ref().unwrap().clone();
  assert_eq!(mount.initialized.get(), 2);
  assert!(game != replacement);
  assert_ne!(replacement.accepted().version.session, initial.session);
  assert_eq!(
    game.dispatch(game.accepted().version, 1),
    ReducerDispatch::Stale
  );
  assert_eq!(replacement.dispatch(initial, 1), ReducerDispatch::Stale);
  final_output.submitted();
  assert_eq!(replacement.accepted().state.value, 100);
  drop(display);
  assert_eq!(
    replacement.dispatch(replacement.accepted().version, 1),
    ReducerDispatch::Stale
  );
}

#[test]
fn reducer_backpressure_reserves_before_snapshots_and_events_and_replacement_cancels() {
  let (mut display, mount, consumer) = self::setup();
  let game = mount.handle.borrow().as_ref().unwrap().clone();
  let clones = mount.probe.clones.load(Ordering::SeqCst);
  assert_eq!(
    game.dispatch(game.accepted().version, 34),
    ReducerDispatch::Started
  );
  assert!(consumer.wait_for_publication(TIMEOUT, |o| o.waiting_for_capacity));
  assert_eq!(consumer.publication_observation().published, 32);
  assert_eq!(mount.probe.events.load(Ordering::SeqCst), 32);
  assert_eq!(mount.probe.clones.load(Ordering::SeqCst) - clones, 33);
  let held = self::output(&consumer);
  assert!(consumer.wait_for_publication(TIMEOUT, |o| o.waiting_for_capacity && o.published == 33));
  assert_eq!(mount.probe.events.load(Ordering::SeqCst), 33);
  assert_eq!(game.accepted().state.value, 0);
  display.click_ui(display.find_ui(ROOT, "replace"));
  display.flush();
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  held.submitted();
  assert_eq!(game.accepted().state.value, 0);
  assert_eq!(game.status(), GameStatus::Stopped);
  assert!(consumer.take_output().is_none());
  assert_eq!(
    mount
      .handle
      .borrow()
      .as_ref()
      .unwrap()
      .accepted()
      .state
      .value,
    100
  );
}

#[test]
fn submission_failure_preserves_the_last_accepted_revision() {
  for accept_final in [false, true] {
    let (mut display, mount, consumer) = self::setup();
    let game = mount.handle.borrow().as_ref().unwrap().clone();
    let initial = game.accepted().version;
    assert_eq!(game.dispatch(initial, 1), ReducerDispatch::Started);
    self::output(&consumer).submitted();
    let final_output = self::output(&consumer);
    assert!(consumer.wait_for_worker_stopped(TIMEOUT));
    if accept_final {
      final_output.submitted();
    }
    consumer.fail("injected host submission failure");
    final_output.submitted();
    assert_eq!(game.status(), GameStatus::Failed);
    assert_eq!(game.accepted().state.value, usize::from(accept_final));
    assert_eq!(game.accepted().version.revision, u64::from(accept_final));
    assert_eq!(
      game.dispatch(game.accepted().version, 1),
      ReducerDispatch::Failed
    );
    assert!(game.diagnostic().unwrap().contains("injected host"));
    display.click_ui(display.find_ui(ROOT, "replace"));
    display.flush();
    assert_eq!(
      mount.handle.borrow().as_ref().unwrap().status(),
      GameStatus::Ready
    );
  }
}

//! Integration evidence for the synchronous test host. Toy rules expose ordered
//! checkpoints so these tests can isolate delivery and virtual-time behavior.
//! Ordinary game tests should assert their own displayed pieces, not this probe.
use battlement::{Connect, ObjectId, ScreenSize, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{
  Application, ApplicationEngine, GameHandle, GameRoot, SnapshotAnimation,
  prelude::*,
  rules::{ChoiceOwner, ChoicePolicy, ExecutionMode, Game, RulesWorker},
};
use reactant_testing::{ActionResult, Display};
use std::{cell::RefCell, rc::Rc, time::Duration};
use trox::ls;
const ROOT: ObjectId = object_id!("78100000-0000-4000-8000-000000000001");
struct Counter;
struct Policy;
impl ChoicePolicy<Counter> for Policy {
  fn owner(&self, _: &usize, _: &()) -> ChoiceOwner {
    ChoiceOwner::Policy
  }
  fn choose(&mut self, _: &usize, _: &()) -> usize {
    unreachable!()
  }
}
impl Game for Counter {
  type State = usize;
  type Action = ();
  type StateAnimation = usize;
  type Prompt<'a> = ();
  type Context = ExecutionMode<Self, Policy>;
  fn logical_clone(state: &usize) -> usize {
    *state
  }
  fn is_legal_action(state: &usize, _: &()) -> bool {
    *state == 0
  }
  fn execute(cx: &mut Self::Context, state: &mut usize, _: ()) {
    for n in 1..=40 {
      *state = n;
      cx.present(state, || n);
    }
  }
}
struct Session(Rc<RefCell<Vec<usize>>>);
struct Screen {
  game: GameHandle<Counter>,
  observed: Rc<RefCell<Vec<usize>>>,
}
impl Component for Session {
  fn render(&self) -> impl Render {
    let game = reactant::use_game::<Counter, _>((), 0, |connection| ExecutionMode::Interactive {
      connection,
      policy: Policy,
    });
    GameRoot::new(Screen {
      game,
      observed: self.0.clone(),
    })
  }
}
impl Component for Screen {
  fn render(&self) -> impl Render {
    let value = *reactant::use_game_state::<Counter>();
    let observed = self.observed.clone();
    reactant::use_animate::<Counter>(move |n| {
      observed.borrow_mut().push(*n);
      Some(SnapshotAnimation::wait(Duration::from_millis(1)))
    });
    let game = self.game.clone();
    (
      Button::new(ls("Run")).host_name("run").on_press(move || {
        game.dispatch(());
      }),
      Label::new(ls(value.to_string())).name("value"),
    )
  }
}
fn application(worker: RulesWorker, observed: Rc<RefCell<Vec<usize>>>) -> Application {
  Application::new("inline/scene")
    .rules_worker(worker)
    .child(Session(observed))
    .document(|mut d| {
      d.root_id = ROOT;
      d
    })
}
fn assets() -> FakeAssetCatalog {
  let mut a = FakeAssetCatalog::new();
  a.add_scene("inline/scene");
  a
}
fn connect() -> Connect {
  Connect::new("test", "test", ScreenSize::new(1920, 1080))
}

fn counter(worker: RulesWorker, events: Rc<RefCell<Vec<usize>>>) -> Display {
  Display::connect_application::<Counter>(
    move |clock| {
      ApplicationEngine::with_clock(
        move || self::application(worker.clone(), events.clone()),
        move || clock.now(),
      )
    },
    self::assets(),
    self::connect(),
  )
}

#[test]
fn virtual_time_does_not_depend_on_worker_scheduling() {
  // Pending worker publications must be received at the current virtual instant,
  // before advancing the same forty milliseconds used by the inline application.
  let mut inline = self::counter(RulesWorker::inline(), Rc::default());
  let mut worker = self::counter(RulesWorker::default(), Rc::default());
  inline.click_ui(inline.find_ui(ROOT, "run"));
  worker.click_ui(worker.find_ui(ROOT, "run"));
  inline.advance(Duration::from_millis(40));
  worker.advance(Duration::from_millis(40));
  assert_eq!(
    inline.ui_element(inline.find_ui(ROOT, "value")).text(),
    Some("40")
  );
  assert_eq!(
    worker.ui_element(worker.find_ui(ROOT, "value")).text(),
    Some("40")
  );
  assert_eq!(worker.presentation_time(), inline.presentation_time());
}

#[test]
fn inline_and_worker_paths_deliver_the_same_ordered_presentation() {
  // One action exceeds the worker FIFO capacity. Inline execution collects it on
  // the caller; both paths must still render every checkpoint exactly once before
  // accepting the final state. Engine::poll panics for the inline instance.
  let inline_events = Rc::new(RefCell::new(Vec::new()));
  let events = inline_events.clone();
  let mut inline = self::counter(RulesWorker::inline(), events);
  inline.click_ui(inline.find_ui(ROOT, "run"));
  inline.settle();
  assert_eq!(*inline_events.borrow(), (1..=40).collect::<Vec<_>>());
  assert_eq!(
    inline.ui_element(inline.find_ui(ROOT, "value")).text(),
    Some("40")
  );
  assert_eq!(inline.presentation_time(), Duration::from_millis(40));
  let threaded_events = Rc::new(RefCell::new(Vec::new()));
  let events = threaded_events.clone();
  let mut threaded = self::counter(RulesWorker::default(), events);
  let run = threaded.find_ui(ROOT, "run");
  assert_eq!(
    threaded.action(|d| d.click_ui(run)),
    ActionResult::Completed
  );
  assert_eq!(*threaded_events.borrow(), *inline_events.borrow());
  assert_eq!(
    threaded.ui_element(threaded.find_ui(ROOT, "value")).text(),
    Some("40")
  );
}

#[test]
fn reconnect_discards_old_output_and_mounts_a_fresh_inline_session() {
  // Replacement happens while output is queued. Old completion and Motion must
  // not install state into the fresh session, even though the engine is reused.
  let events = Rc::new(RefCell::new(Vec::new()));
  let mut display = Display::connect_application::<Counter>(
    move |clock| {
      ApplicationEngine::with_clock(
        move || application(RulesWorker::inline(), events.clone()),
        move || clock.now(),
      )
    },
    assets(),
    connect(),
  );
  display.click_ui(display.find_ui(ROOT, "run"));
  display.reconnect();
  display.settle();
  assert_eq!(
    display.ui_element(display.find_ui(ROOT, "value")).text(),
    Some("0")
  );
  display.click_ui(display.find_ui(ROOT, "run"));
  display.settle();
  assert_eq!(
    display.ui_element(display.find_ui(ROOT, "value")).text(),
    Some("40")
  );
}

struct MountGate(Children);

impl Component for MountGate {
  fn render(&self) -> impl Render {
    let (ready, set_ready) = reactant::hooks::use_state(false);
    reactant::hooks::use_effect(move || set_ready.set(true), ());
    ready.then(|| self.0.render())
  }
}

#[test]
fn nested_initialization_effects_settle_before_the_game_mounts() {
  for worker in [RulesWorker::inline(), RulesWorker::default()] {
    let mut display = Display::connect_application::<Counter>(
      move |clock| {
        ApplicationEngine::with_clock(
          move || {
            let mut children: Children = Session(Rc::default()).into();
            for _ in 0..8 {
              children = MountGate(children).into();
            }
            Application::new("inline/scene")
              .rules_worker(worker.clone())
              .child(children.render())
          },
          move || clock.now(),
        )
      },
      self::assets(),
      self::connect(),
    );
    display.expect_button("Run");
    assert_eq!(display.game_state::<Counter>(), Some(0));
    assert_eq!(display.presentation_time(), Duration::ZERO);
  }
}

struct ExternalResource(Resource<(), u32>);
impl Component for ExternalResource {
  fn render(&self) -> impl Render {
    Suspense::new(Label::new(ls("Loading"))).child(
      reactant::resource::use_resource(&self.0, ()).then(|value| Label::new(ls(value.to_string()))),
    )
  }
}

#[test]
#[should_panic(expected = "inline application cannot spawn asynchronous resources")]
fn an_external_resource_cannot_silently_enter_the_fast_lane() {
  // Even an immediately ready future is an undeclared execution dependency here.
  // Reject spawning it before its first poll, rather than waiting for a loading UI
  // or deciding completion from the test's expected value. Real resource behavior
  // belongs in the worker/integration lane, with its actual executor.
  Display::connect_application::<Counter>(
    |clock| {
      ApplicationEngine::with_clock(
        || {
          Application::new("inline/scene")
            .rules_worker(RulesWorker::inline())
            .child(ExternalResource(Resource::new(|()| std::future::ready(42))))
        },
        move || clock.now(),
      )
    },
    assets(),
    connect(),
  );
}

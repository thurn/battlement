#[path = "support/session_game.rs"]
mod session_game;

use std::{
  panic::{self, AssertUnwindSafe},
  sync::{Arc, atomic::Ordering},
  time::Duration,
};

use battlement_fake::assets::FakeAssetCatalog;
use reactant::{
  DispatchResult, GameConsumer, GameHandle, GameOutput, GameStatus, app::App, host::ButtonHost,
  prelude::*,
};
use reactant_rules::Game;
use reactant_testing::Display;
use session_game::{Action, Context, Counter, Probe, Prompt};
use trox::ls;

const TIMEOUT: Duration = Duration::from_secs(10);

struct GameView;
impl Component for GameView {
  fn render(&self) -> impl Render {
    let state = reactant::use_game_state::<Counter>();
    let status = reactant::use_game_status::<Counter>();
    let prompt = reactant::use_game_prompt::<Counter>();
    View::new().name("game").child((
      Label::new(ls(format!(
        "{}:{status:?}:{}",
        state.value,
        prompt.is_some()
      )))
      .name("game-status"),
      prompt.map(|prompt| {
        ButtonHost::new(ls("Answer"))
          .key(prompt.handle.clone())
          .name("answer")
          .on_click(move |_: &mut usize| {
            let Prompt::Number(number) = &prompt.prompt;
            prompt.handle.submit(number.as_ref(), 5);
          })
      }),
      ButtonHost::new(ls("Invalid callback"))
        .name("invalid")
        .on_click(|_: &mut usize| panic!("detailed callback failure")),
    ))
  }
}

fn setup(
  probe: &Arc<Probe>,
) -> (
  Display<App<usize>>,
  GameHandle<Counter>,
  GameConsumer<Counter>,
  battlement::ObjectId,
) {
  let mut app = App::with_model("app/content", 0_usize).root(|count| {
    View::new().child((
      Label::new(ls(count.to_string())).name("menu-count"),
      ButtonHost::new(ls("Menu"))
        .name("menu")
        .on_click(|count: &mut usize| *count += 1),
      GameRoot::new(GameView),
    ))
  });
  let game = app.start_game::<Counter>(probe.state(), |connection| {
    Context::interactive(connection, probe.clone())
  });
  let consumer = app.game_consumer::<Counter>();
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("app/content");
  (Display::connect(app, assets), game, consumer, root)
}
fn output(consumer: &GameConsumer<Counter>) -> GameOutput<Counter> {
  assert!(consumer.wait_for_output(TIMEOUT));
  consumer.take_output().expect("published output")
}
fn assert_ui(display: &Display<App<usize>>, root: battlement::ObjectId, value: &str) {
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "game-status"))
      .text(),
    Some(value)
  );
  assert_eq!(display.presentation_time(), Duration::ZERO);
  assert_eq!(display.frame(), 0);
}

#[test]
fn app_dispatch_typed_answers_and_submission_preserve_accepted_state_and_context() {
  let probe = Arc::new(Probe::default());
  let (mut display, game, consumer, root) = self::setup(&probe);
  assert_eq!(probe.contexts.load(Ordering::SeqCst), 1);
  assert_eq!(probe.executions.load(Ordering::SeqCst), 0);
  assert_eq!(game.status(), GameStatus::Busy);
  assert_eq!(game.dispatch(Action::Illegal), DispatchResult::Busy);
  assert_eq!(probe.validations.load(Ordering::SeqCst), 0);
  self::output(&consumer).submitted();
  assert_eq!(game.status(), GameStatus::Ready);
  let clones = probe.clones.load(Ordering::SeqCst);
  assert!(panic::catch_unwind(AssertUnwindSafe(|| game.dispatch(Action::Illegal))).is_err());
  assert_eq!(probe.clones.load(Ordering::SeqCst), clones);
  let handle = game.clone();
  assert_eq!(handle.dispatch(Action::Choice), DispatchResult::Started);
  assert!(consumer.wait_for_worker_started(TIMEOUT));
  let prompt = self::output(&consumer);
  prompt.submitted();
  display.poll();
  self::assert_ui(&display, root, "0:Busy:true");
  display.click_ui(display.find_ui(root, "menu"));
  display.click_ui(display.find_ui(root, "answer"));
  display.poll();
  self::assert_ui(&display, root, "0:Busy:false");
  let final_output = self::output(&consumer);
  assert!(final_output.is_final());
  assert!(final_output.animation().is_none());
  assert_eq!(game.accepted_state().value, 0);
  assert_eq!(handle.dispatch(Action::Illegal), DispatchResult::Busy);
  display.poll();
  self::assert_ui(&display, root, "5:Busy:false");
  final_output.submitted();
  final_output.submitted();
  assert_eq!(game.status(), GameStatus::Ready);
  assert_eq!(game.accepted_state().value, 5);
  let mut copy = game.accepted_state();
  copy.value = 999;
  assert_eq!(game.accepted_state().value, 5);
  assert_eq!(game.dispatch(Action::Add), DispatchResult::Started);
  self::output(&consumer).submitted();
  assert_eq!(game.accepted_state().value, 7);
  assert_eq!(probe.contexts.load(Ordering::SeqCst), 1);
  let mut simulation = probe.state();
  let mut context = Context::simulation(probe.clone());
  Counter::execute(&mut context, &mut simulation, Action::Choice);
  Counter::execute(&mut context, &mut simulation, Action::Add);
  assert_eq!(game.accepted_state().value, simulation.value);
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  game.stop();
  game.stop();
  assert_eq!(handle.status(), GameStatus::Stopped);
  assert_eq!(handle.accepted_state().value, 7);
  assert!(panic::catch_unwind(AssertUnwindSafe(|| handle.dispatch(Action::Add))).is_err());
}

#[test]
fn replacement_attaches_immediately_but_waits_for_old_cleanup_and_preserves_menus() {
  let probe = Arc::new(Probe::default());
  let (mut display, old, old_consumer, root) = self::setup(&probe);
  self::output(&old_consumer).submitted();
  old.dispatch(Action::HoldThenFail);
  probe.wait_held();
  let menu = display.find_ui(root, "menu");
  let old_game = display.find_ui(root, "game");
  let mut current = old.clone();
  let mut current_consumer = None;
  let mut superseded = Vec::new();
  for value in 10..15 {
    let (next, consumer) = display.with_engine(|app| {
      let mut state = probe.state();
      state.value = value;
      let next = app.start_game::<Counter>(state, |connection| {
        Context::interactive(connection, probe.clone())
      });
      (next, app.game_consumer::<Counter>())
    });
    assert_eq!(current.status(), GameStatus::Stopped);
    superseded.push(current);
    current = next;
    current_consumer = Some(consumer);
    self::output(current_consumer.as_ref().unwrap()).submitted();
    let validations = probe.validations.load(Ordering::SeqCst);
    assert_eq!(current.dispatch(Action::Illegal), DispatchResult::Busy);
    assert_eq!(probe.validations.load(Ordering::SeqCst), validations);
    display.poll();
    assert_eq!(display.find_ui(root, "menu"), menu);
    assert_ne!(display.find_ui(root, "game"), old_game);
    display.click_ui(menu);
  }
  assert_eq!(probe.contexts.load(Ordering::SeqCst), 6);
  assert_eq!(probe.executions.load(Ordering::SeqCst), 1);
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "menu-count"))
      .text(),
    Some("5")
  );
  probe.release();
  // The ended consumer's lifecycle remains observable after public Stopped.
  // The latest consumer has no worker until admission becomes Ready.
  assert!(old_consumer.wait_for_worker_stopped(TIMEOUT));
  let current_consumer = current_consumer.unwrap();
  assert_eq!(current.status(), GameStatus::Ready);
  assert!(current.diagnostic().is_none());
  assert_eq!(current.dispatch(Action::Add), DispatchResult::Started);
  self::output(&current_consumer).submitted();
  assert_eq!(current.accepted_state().value, 15);
  assert!(
    superseded
      .iter()
      .all(|handle| handle.status() == GameStatus::Stopped)
  );
  assert!(current_consumer.wait_for_worker_stopped(TIMEOUT));
  assert_eq!(probe.drops.load(Ordering::SeqCst), 5);
  drop(display);
  assert_eq!(current.status(), GameStatus::Stopped);
  assert_eq!(probe.drops.load(Ordering::SeqCst), 6);
}

#[test]
fn worker_and_callback_failures_retain_accepted_state_and_allow_explicit_recovery() {
  let probe = Arc::new(Probe::default());
  let (mut display, game, consumer, root) = self::setup(&probe);
  self::output(&consumer).submitted();
  game.dispatch(Action::Add);
  self::output(&consumer).submitted();
  game.dispatch(Action::Fail);
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  assert_eq!(game.status(), GameStatus::Failed);
  assert_eq!(game.accepted_state().value, 1);
  assert!(
    game
      .diagnostic()
      .unwrap()
      .contains("deliberate worker failure")
  );
  let (next, next_consumer) = display.with_engine(|app| {
    let next = app.start_game::<Counter>(game.accepted_state(), |connection| {
      Context::interactive(connection, probe.clone())
    });
    (next, app.game_consumer::<Counter>())
  });
  let initial = self::output(&next_consumer);
  initial.submitted();
  next.dispatch(Action::Add);
  self::output(&next_consumer).submitted();
  display.poll();
  display.click_ui(display.find_ui(root, "invalid"));
  assert_eq!(next.status(), GameStatus::Failed);
  assert_eq!(next.accepted_state().value, 2);
  assert!(
    next
      .diagnostic()
      .unwrap()
      .contains("detailed callback failure")
  );
  initial.submitted();
  assert!(next_consumer.take_output().is_none());
  display.click_ui(display.find_ui(root, "menu"));
  self::assert_ui(&display, root, "2:Failed:false");
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "menu-count"))
      .text(),
    Some("1")
  );
}

#[test]
fn first_attachment_keeps_the_already_connected_menu_lifetime() {
  let app = App::with_model("app/content", 0_usize).root(|count| {
    View::new().child((
      ButtonHost::new(ls(count.to_string()))
        .name("menu")
        .on_click(|count: &mut usize| *count += 1),
      GameRoot::new(GameView),
    ))
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("app/content");
  let mut display = Display::connect(app, assets);
  let menu = display.find_ui(root, "menu");
  display.click_ui(menu);
  let probe = Arc::new(Probe::default());
  let thread = std::thread::current().id();
  let game = display.with_engine(|app| {
    app.start_game::<Counter>(probe.state(), |connection| {
      assert_eq!(std::thread::current().id(), thread);
      Context::interactive(connection, probe.clone())
    })
  });
  display.poll();
  assert_eq!(display.find_ui(root, "menu"), menu);
  assert_eq!(display.ui_element(menu).text(), Some("1"));
  self::assert_ui(&display, root, "0:Busy:false");
  drop(display);
  assert_eq!(game.status(), GameStatus::Stopped);
}

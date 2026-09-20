#[path = "support/failure_recorder.rs"]
mod failure_recorder;

use std::time::Duration;

use battlement::{Command, ObjectId};
use battlement_cloud::diagnostics::{DiagnosticsCommand, DiagnosticsMetadata};
use battlement_fake::assets::FakeAssetCatalog;
use failure_recorder::FailureRecorder;
use reactant::{
  GameConsumer, GameHandle, GameStatus, host::ButtonHost, prelude::*, testing::App,
  testing::GameApp,
};
use reactant_core::app_output::DeliveryLimits;
use reactant_rules::{ChoiceOwner, ChoicePolicy, DisplayConnection, ExecutionMode, Game};
use reactant_testing::Display;
use trox::ls;

const TIMEOUT: Duration = Duration::from_secs(10);

type TestDisplay = Display<FailureRecorder<App<usize>>>;

struct QueueGame;
struct Policy;
struct Context(ExecutionMode<QueueGame, Policy>);

impl ChoicePolicy<QueueGame> for Policy {
  fn owner(&self, _: &usize, _: &()) -> ChoiceOwner {
    ChoiceOwner::Policy
  }
  fn choose(&mut self, _: &usize, _: &()) -> usize {
    unreachable!()
  }
}
impl Game for QueueGame {
  type State = usize;
  type Action = usize;
  type StateAnimation = Duration;
  type Prompt<'a> = ();
  type Context = Context;
  fn logical_clone(state: &usize) -> usize {
    *state
  }
  fn is_legal_action(_: &usize, _: &usize) -> bool {
    true
  }
  fn execute(context: &mut Context, state: &mut usize, count: usize) {
    if count == usize::MAX - 2 {
      *state = 1;
      context.0.present(state, || Duration::from_millis(200));
      *state = usize::MAX - 1;
      return;
    }
    if count >= usize::MAX - 1 {
      *state = count;
      context.0.present(state, || Duration::from_millis(200));
      return;
    }
    for _ in 0..count {
      *state += 1;
      context.0.present(state, || Duration::from_millis(200));
    }
  }
}

fn context(connection: DisplayConnection<QueueGame>) -> Context {
  Context(ExecutionMode::Interactive {
    connection,
    policy: Policy,
  })
}

struct QueueView;
impl Component for QueueView {
  fn render(&self) -> impl Render {
    let state = *reactant::use_game_state::<QueueGame>();
    reactant::use_animate::<QueueGame>(|duration| Some(SnapshotAnimation::wait(*duration)));
    let app = reactant::app_context::use_app();
    reactant::hooks::use_effect(
      move || {
        if state == usize::MAX - 1 {
          app.send(Command::diagnostics(DiagnosticsCommand::SetMetadata(
            DiagnosticsMetadata {
              key: "proof".to_owned(),
              value: Some("failed host".to_owned()),
            },
          )));
        }
      },
      state,
    );
    Label::new(ls(if state == usize::MAX {
      "oversized".repeat(4096)
    } else {
      state.to_string()
    }))
    .name("stage")
    .key(state)
  }
}

fn setup(
  limits: DeliveryLimits,
) -> (
  TestDisplay,
  GameHandle<QueueGame>,
  GameConsumer<QueueGame>,
  ObjectId,
) {
  let mut app = App::with_model("queue/content", 0_usize)
    .delivery_limits(limits)
    .root(|menu| {
      View::new().child((
        ButtonHost::new(ls("Menu"))
          .name("menu")
          .on_click(|menu: &mut usize| *menu += 1),
        Label::new(ls(menu.to_string())).name("menu-count"),
        GameRoot::new(QueueView),
      ))
    });
  let game = app.start_game::<QueueGame>(0, self::context);
  let consumer = app.game_consumer::<QueueGame>();
  consumer.resume_automatic_submission();
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("queue/content");
  let mut display = Display::connect(FailureRecorder::new(app), assets);
  display.poll();
  assert_eq!(game.status(), GameStatus::Ready);
  (display, game, consumer, root)
}

fn stage(display: &TestDisplay, root: ObjectId) -> usize {
  display
    .ui_element(display.find_ui(root, "stage"))
    .text()
    .unwrap()
    .parse()
    .unwrap()
}

fn submit_all(
  display: &mut TestDisplay,
  game: &GameHandle<QueueGame>,
  consumer: &GameConsumer<QueueGame>,
) {
  for _ in 0..100 {
    if game.status() == GameStatus::Ready {
      return;
    }
    assert!(consumer.wait_for_output(TIMEOUT));
    display.poll();
  }
  panic!("output failed to reach submission");
}

#[test]
fn rust_submits_all_snapshots_and_accepts_before_host_time_or_frames_advance() {
  let (mut display, game, consumer, root) = self::setup(DeliveryLimits::default());
  game.dispatch(3);
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  self::submit_all(&mut display, &game, &consumer);
  assert_eq!(game.accepted_state(), 3);
  assert_eq!(self::stage(&display, root), 1);
  assert_eq!(display.presentation_time(), Duration::ZERO);
  assert_eq!(display.frame(), 0);
  display.advance_time(Duration::from_millis(200));
  assert_eq!(self::stage(&display, root), 2);
  display.advance_time(Duration::from_millis(200));
  assert_eq!(self::stage(&display, root), 3);
  display.settle();
  assert_eq!(display.with_engine(|app| app.retained_gameplay_bytes()), 0);
  game.dispatch(0);
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  let before = display.presentation_time();
  self::submit_all(&mut display, &game, &consumer);
  assert_eq!(display.presentation_time(), before);
  assert_eq!(display.frame(), 0);
}

#[test]
fn full_gameplay_budget_holds_final_acceptance_but_menus_and_stop_remain_live() {
  let (mut display, game, consumer, root) = self::setup(DeliveryLimits {
    gameplay_bytes: 8192,
    ..DeliveryLimits::default()
  });
  let menu = display.find_ui(root, "menu");
  game.dispatch(40);
  assert!(consumer.wait_for_publication(TIMEOUT, |o| o.waiting_for_capacity));
  for _ in 0..5 {
    display.poll();
  }
  assert!(consumer.wait_for_publication(TIMEOUT, |o| o.waiting_for_capacity));
  let held = consumer.publication_observation();
  assert_eq!(game.status(), GameStatus::Busy);
  assert_eq!(game.accepted_state(), 0);
  assert_eq!(held.published - held.consumed, 32);
  for _ in 0..5 {
    display.poll();
  }
  assert_eq!(
    consumer.publication_observation().builders_started,
    held.builders_started
  );
  for _ in 0..40 {
    display.click_ui(menu);
  }
  assert_eq!(game.status(), GameStatus::Busy);
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "menu-count"))
      .text(),
    Some("40")
  );
  assert_eq!(self::stage(&display, root), 1);
  let active_stage = display.find_ui(root, "stage");
  let container = display.ui_element(root).children()[0];
  game.stop();
  display.poll();
  assert!(
    !display
      .ui_element(container)
      .children()
      .contains(&active_stage)
  );
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  assert_eq!(display.find_ui(root, "menu"), menu);
  assert_eq!(display.with_engine(|app| app.retained_gameplay_bytes()), 0);
  assert_eq!(display.presentation_time(), Duration::ZERO);
  let replacement = display.with_engine(|app| app.start_game::<QueueGame>(77, self::context));
  display.poll();
  display.poll();
  assert_eq!(replacement.status(), GameStatus::Ready);
  assert_eq!(self::stage(&display, root), 77);
  assert_eq!(display.find_ui(root, "menu"), menu);
}

#[test]
fn capacity_release_resumes_each_snapshot_once_and_oversized_output_fails_locally() {
  let (mut display, game, consumer, root) = self::setup(DeliveryLimits {
    gameplay_bytes: 8192,
    ..DeliveryLimits::default()
  });
  game.dispatch(3);
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  for _ in 0..5 {
    display.poll();
  }
  assert_eq!(game.status(), GameStatus::Busy);
  assert_eq!(game.accepted_state(), 0);
  for expected in 1..=3 {
    for _ in 0..5 {
      display.poll();
    }
    assert_eq!(self::stage(&display, root), expected);
    display.advance_time(Duration::from_millis(200));
  }
  self::submit_all(&mut display, &game, &consumer);
  assert_eq!(game.accepted_state(), 3);
  assert_eq!(consumer.publication_observation().consumed, 4);
  assert_eq!(display.with_engine(|app| app.retained_gameplay_bytes()), 0);

  let (mut display, game, consumer, _) = self::setup(DeliveryLimits {
    message_bytes: 4096,
    ..DeliveryLimits::default()
  });
  game.dispatch(usize::MAX);
  assert!(consumer.wait_for_output(TIMEOUT));
  display.poll();
  assert_eq!(game.status(), GameStatus::Failed);
  assert_eq!(game.accepted_state(), 0);
  assert!(game.diagnostic().unwrap().contains("response limit"));
}

#[test]
fn binary_host_failure_cancels_game_output_without_replaying_effects_or_losing_menu() {
  let (mut display, game, consumer, root) = self::setup(DeliveryLimits::default());
  let menu = display.find_ui(root, "menu");
  game.dispatch(usize::MAX - 1);
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  display.poll();
  assert_eq!(game.status(), GameStatus::Failed);
  assert_eq!(game.accepted_state(), 0);
  assert!(game.diagnostic().unwrap().contains("Diagnostics"));
  display.poll();
  display.click_ui(menu);
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "menu-count"))
      .text(),
    Some("1")
  );
  assert_eq!(display.with_engine(|app| app.retained_gameplay_bytes()), 0);
  assert_eq!(display.presentation_time(), Duration::ZERO);
  assert_eq!(display.frame(), 0);
  assert_eq!(self::stage(&display, root), usize::MAX - 1);
}

#[test]
fn host_failure_after_acceptance_keeps_recoverable_rules_and_persistent_menu() {
  let (mut display, game, consumer, root) = self::setup(DeliveryLimits::default());
  let menu = display.find_ui(root, "menu");
  display.click_ui(menu);
  game.dispatch(usize::MAX - 2);
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  self::submit_all(&mut display, &game, &consumer);
  assert_eq!(game.status(), GameStatus::Ready);
  assert_eq!(game.accepted_state(), usize::MAX - 1);
  assert_eq!(self::stage(&display, root), 1);
  display.advance_time(Duration::from_millis(200));
  assert_eq!(game.status(), GameStatus::Failed);
  assert_eq!(game.accepted_state(), usize::MAX - 1);
  assert!(game.diagnostic().unwrap().contains("Diagnostics"));
  display.poll();
  assert_eq!(display.with_engine(|app| app.retained_gameplay_bytes()), 0);
  // The accepted state remains available even when the host cannot display it.
  assert_eq!(game.accepted_state(), usize::MAX - 1);
  let replacement = display.with_engine(|app| app.start_game::<QueueGame>(77, self::context));
  display.poll();
  display.poll();
  display.with_engine(|app| app.replay_failure());
  display.poll();
  assert_eq!(replacement.status(), GameStatus::Ready);
  assert_eq!(replacement.accepted_state(), 77);
  assert!(replacement.diagnostic().is_none());
  assert_eq!(self::stage(&display, root), 77);
  assert_eq!(display.find_ui(root, "menu"), menu);
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "menu-count"))
      .text(),
    Some("1")
  );
}

use std::{cell::RefCell, rc::Rc, time::Duration};

use battlement::{ObjectId, Prop, StyleValue};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{
  GameConsumer, GameHandle, GamePresentation,
  animation_controls::{AnimationSequence, MotionSelector},
  prelude::*,
  rules::{ChoiceOwner, ChoicePolicy, DisplayConnection, ExecutionMode, Game},
  testing::App,
  testing::GameApp,
};
use reactant_core::app_output::DeliveryLimits;
use reactant_testing::Display;
use trox::ls;

const TIMEOUT: Duration = Duration::from_secs(10);

struct PausableGame;
struct Policy;
struct Context(ExecutionMode<PausableGame, Policy>);
struct Board {
  controls: Rc<RefCell<Vec<GamePresentation>>>,
}

#[derive(Clone, Copy)]
enum Animation {
  Move,
  Hold,
}

impl ChoicePolicy<PausableGame> for Policy {
  fn owner(&self, _: &u32, _: &()) -> ChoiceOwner {
    ChoiceOwner::Policy
  }

  fn choose(&mut self, _: &u32, _: &()) -> usize {
    unreachable!()
  }
}

impl Game for PausableGame {
  type State = u32;
  type Action = u32;
  type StateAnimation = Animation;
  type Prompt<'a> = ();
  type Context = Context;

  fn logical_clone(state: &u32) -> u32 {
    *state
  }

  fn is_legal_action(_: &u32, _: &u32) -> bool {
    true
  }

  fn execute(context: &mut Context, state: &mut u32, count: u32) {
    for _ in 0..count {
      *state += 1;
      context.0.present(state, || {
        if *state == 1 {
          Animation::Move
        } else {
          Animation::Hold
        }
      });
    }
  }
}

fn context(connection: DisplayConnection<PausableGame>) -> Context {
  Context(ExecutionMode::Interactive {
    connection,
    policy: Policy,
  })
}

impl Component for Board {
  fn render(&self) -> impl Render {
    let stage = *reactant::use_game_state::<PausableGame>();
    let scope = reactant::animation_controls::use_animation_scope();
    let animation_scope = scope.clone();
    reactant::use_animate::<PausableGame>(move |animation| {
      Some(match animation {
        Animation::Move => SnapshotAnimation::sequence(
          animation_scope.clone(),
          AnimationSequence::new().animate(
            MotionSelector::name("card"),
            StyleTarget::new().x(20.0),
            Transition::tween().duration_secs(0.2).ease(Easing::Linear),
          ),
        ),
        Animation::Hold => SnapshotAnimation::wait(Duration::from_millis(200)),
      })
    });
    let first = reactant::use_game_presentation();
    let second = reactant::use_game_presentation();
    self.controls.replace(vec![first.clone(), second.clone()]);
    let pause_first = first.clone();
    let pause_second = second.clone();
    View::new()
      .motion(MotionProps::new().animation_scope(scope))
      .child((
        Label::new(ls(stage.to_string())).name("stage"),
        View::new()
          .name("card")
          .style(Style::new().width(10.px()).height(10.px()))
          .motion(MotionProps::new().motion_name("card")),
        reactant::host::ButtonHost::new(ls("Pause first"))
          .name("pause-first")
          .on_click(move || pause_first.pause()),
        reactant::host::ButtonHost::new(ls("Resume first"))
          .name("resume-first")
          .on_click(move || first.resume()),
        reactant::host::ButtonHost::new(ls("Pause second"))
          .name("pause-second")
          .on_click(move || pause_second.pause()),
        reactant::host::ButtonHost::new(ls("Resume second"))
          .name("resume-second")
          .on_click(move || second.resume()),
      ))
  }
}

type TestDisplay = Display<App<u32>>;
type Fixture = (
  TestDisplay,
  GameHandle<PausableGame>,
  GameConsumer<PausableGame>,
  ObjectId,
  Rc<RefCell<Vec<GamePresentation>>>,
);

fn fixture(limits: DeliveryLimits) -> Fixture {
  let controls = Rc::new(RefCell::new(Vec::new()));
  let board_controls = controls.clone();
  let mut app = App::with_model("gameplay-pause/content", 0_u32)
    .delivery_limits(limits)
    .root(move |menu| {
      View::new().child((
        reactant::host::ButtonHost::new(ls("Menu"))
          .name("menu")
          .on_click(|menu: &mut u32| *menu += 1),
        Label::new(ls(menu.to_string())).name("menu-count"),
        GameRoot::new(Board {
          controls: board_controls.clone(),
        }),
      ))
    });
  let game = app.start_game::<PausableGame>(0, self::context);
  let consumer = app.game_consumer::<PausableGame>();
  consumer.resume_automatic_submission();
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("gameplay-pause/content");
  let mut display = Display::connect(app, assets);
  display.poll();
  (display, game, consumer, root, controls)
}

fn submit_all(
  display: &mut TestDisplay,
  game: &GameHandle<PausableGame>,
  consumer: &GameConsumer<PausableGame>,
) {
  for _ in 0..100 {
    if game.status() == GameStatus::Ready {
      return;
    }
    assert!(consumer.wait_for_output(TIMEOUT));
    display.poll();
  }
  panic!("game output did not reach submission");
}

fn text(display: &TestDisplay, root: ObjectId, name: &str) -> String {
  display
    .ui_element(display.find_ui(root, name))
    .text()
    .unwrap()
    .to_owned()
}

fn x(display: &TestDisplay, root: ObjectId) -> f64 {
  let Prop::Set(StyleValue::Value(translation)) = display
    .ui_element(display.find_ui(root, "card"))
    .style()
    .translate
  else {
    panic!("card translation is absent")
  };
  let battlement::Length::Px(value) = translation.x else {
    panic!("card translation is not pixels")
  };
  f64::from(value)
}

#[test]
fn pause_freezes_motion_and_wait_while_menu_and_workers_continue() {
  let (mut display, game, consumer, root, _) = fixture(DeliveryLimits::default());
  game.dispatch(3);
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  submit_all(&mut display, &game, &consumer);
  assert_eq!(game.accepted_state(), 3);
  assert_eq!(text(&display, root, "stage"), "1");

  display.advance_time(Duration::from_millis(50));
  assert_eq!(x(&display, root), 5.0);
  display.click_ui(display.find_ui(root, "pause-first"));
  display.click_ui(display.find_ui(root, "pause-second"));
  display.advance_time(Duration::from_secs(5));
  assert_eq!(x(&display, root), 5.0);
  assert_eq!(text(&display, root, "stage"), "1");

  display.click_ui(display.find_ui(root, "menu"));
  assert_eq!(text(&display, root, "menu-count"), "1");
  display.click_ui(display.find_ui(root, "resume-first"));
  display.advance_time(Duration::from_secs(1));
  assert_eq!(x(&display, root), 5.0);
  display.click_ui(display.find_ui(root, "resume-second"));
  display.advance_time(Duration::from_millis(149));
  assert_eq!(text(&display, root, "stage"), "1");
  display.advance_time(Duration::from_millis(1));
  assert_eq!(x(&display, root), 20.0);
  assert_eq!(text(&display, root, "stage"), "2");
  display.advance_time(Duration::from_millis(199));
  assert_eq!(text(&display, root, "stage"), "2");
  display.advance_time(Duration::from_millis(1));
  assert_eq!(text(&display, root, "stage"), "3");
  assert_eq!(display.frame(), 0);
}

#[test]
fn paused_budget_backpressures_and_old_owner_cannot_resume_replacement() {
  let (mut display, game, consumer, root, controls) = fixture(DeliveryLimits {
    gameplay_bytes: 8192,
    ..DeliveryLimits::default()
  });
  let old_owner = controls.borrow()[0].clone();
  old_owner.pause();
  display.poll();
  game.dispatch(40);
  assert!(consumer.wait_for_publication(TIMEOUT, |value| value.waiting_for_capacity));
  for _ in 0..5 {
    display.poll();
  }
  assert_eq!(game.status(), GameStatus::Busy);
  assert_eq!(game.accepted_state(), 0);
  assert_eq!(text(&display, root, "stage"), "0");
  display.click_ui(display.find_ui(root, "menu"));
  assert_eq!(text(&display, root, "menu-count"), "1");

  old_owner.resume();
  display.poll();
  for _ in 0..45 {
    display.advance_time(Duration::from_millis(200));
    display.poll();
    if game.status() == GameStatus::Ready {
      break;
    }
  }
  submit_all(&mut display, &game, &consumer);
  assert_eq!(game.accepted_state(), 40);

  old_owner.pause();
  display.poll();
  game.stop();
  display.poll();
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  let replacement = display.with_engine(|app| app.start_game::<PausableGame>(77, self::context));
  display.poll();
  display.poll();
  assert_eq!(replacement.status(), GameStatus::Ready);
  assert_eq!(text(&display, root, "stage"), "77");
  old_owner.resume();
  display.poll();
  assert_eq!(text(&display, root, "stage"), "77");
}

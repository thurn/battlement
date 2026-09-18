use std::time::Duration;

use battlement::{ObjectId, ParentScene, Prop, StyleValue, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{
  GameConsumer, GameHandle,
  animation_controls::{self, AnimationSequence, MotionSelector, SequencePosition},
  app::App,
  prelude::*,
  rules::{ChoiceOwner, ChoicePolicy, ExecutionMode, Game},
  world,
};
use reactant_testing::Display;
use trox::ls;

const TIMEOUT: Duration = Duration::from_secs(10);
const LAYOUT_OBJECT: ObjectId = object_id!("92500000-0000-4000-8000-000000000001");

struct AnimationGame;
struct Policy;
struct Board {
  layout_destination: world::LayoutDestination,
}

#[derive(Clone, Copy)]
enum Action {
  Gameplay,
  Cosmetic,
  Layout,
}

enum StateAnimation {
  Placement,
  Wait,
  Cosmetic,
  Layout,
}

impl ChoicePolicy<AnimationGame> for Policy {
  fn owner(&self, _: &u32, _: &()) -> ChoiceOwner {
    ChoiceOwner::Policy
  }

  fn choose(&mut self, _: &u32, _: &()) -> usize {
    unreachable!()
  }
}

impl Game for AnimationGame {
  type State = u32;
  type Action = Action;
  type StateAnimation = StateAnimation;
  type Prompt<'a> = ();
  type Context = ExecutionMode<Self, Policy>;

  fn logical_clone(state: &u32) -> u32 {
    *state
  }

  fn is_legal_action(state: &u32, action: &Action) -> bool {
    matches!(
      (state, action),
      (0, Action::Gameplay) | (2, Action::Cosmetic) | (4, Action::Layout)
    )
  }

  fn execute(context: &mut Self::Context, state: &mut u32, action: Action) {
    match action {
      Action::Gameplay => {
        *state = 1;
        context.present(state, || StateAnimation::Placement);
        context.present(state, || StateAnimation::Wait);
        *state = 2;
      }
      Action::Cosmetic => {
        *state = 3;
        context.present(state, || StateAnimation::Cosmetic);
        *state = 4;
      }
      Action::Layout => {
        *state = 5;
        context.present(state, || StateAnimation::Layout);
        *state = 6;
      }
    }
  }
}

impl Component for Board {
  fn render(&self) -> impl Render {
    let stage = *reactant::use_game_state::<AnimationGame>();
    let ui_scope = animation_controls::use_animation_scope();
    let event_ui_scope = ui_scope.clone();
    reactant::use_animate::<AnimationGame>(move |event| {
      Some(match event {
        StateAnimation::Placement => SnapshotAnimation::sequence(
          event_ui_scope.clone(),
          AnimationSequence::new()
            .animate(
              MotionSelector::name("short"),
              StyleTarget::new().x(10.0),
              Transition::tween().duration_secs(0.1).ease(Easing::Linear),
            )
            .then(
              MotionSelector::name("long"),
              StyleTarget::new().x(20.0),
              Transition::tween().duration_secs(0.2).ease(Easing::Linear),
            )
            .at(SequencePosition::WithPrevious(0.0))
            .then(
              MotionSelector::name("short"),
              StyleTarget::new().x(-20.0),
              Transition::tween().duration_secs(0.1).ease(Easing::Linear),
            )
            .at(SequencePosition::Absolute(Duration::from_millis(100)))
            .replace(),
        ),
        StateAnimation::Wait => SnapshotAnimation::wait(Duration::from_millis(100)),
        StateAnimation::Cosmetic => SnapshotAnimation::sequence(
          event_ui_scope.clone(),
          AnimationSequence::new().animate(
            MotionSelector::name("short"),
            StyleTarget::new().x(30.0),
            Transition::tween().duration_secs(0.5).ease(Easing::Linear),
          ),
        )
        .nonblocking(),
        StateAnimation::Layout => return None,
      })
    });
    let (local, set_local) = reactant::hooks::use_state(0_u32);
    let interface = View::new()
      .motion(MotionProps::new().animation_scope(ui_scope))
      .child((
        Label::new(ls(stage.to_string())).name("stage"),
        reactant::host::ButtonHost::new(ls("Local rerender"))
          .name("local-rerender")
          .on_click(move || set_local.set(local + 1)),
        Label::new(ls(local.to_string())).name("local"),
        View::new()
          .name("short")
          .style(Style::new().width(10.px()).height(10.px()))
          .motion(
            MotionProps::new()
              .motion_name("short")
              .animate(StyleTarget::new().x(if stage >= 1 { -20.0 } else { 0.0 }))
              .transition(Transition::tween().duration_secs(0.2).ease(Easing::Linear)),
          ),
        View::new()
          .name("long")
          .style(Style::new().width(10.px()).height(10.px()))
          .motion(MotionProps::new().motion_name("long")),
      ));
    let width = if stage >= 5 { 10.0 } else { 4.0 };
    let layout = world::Flex::new()
      .extent((width, 2.0))
      .child(world::LayoutChild::new(
        self.layout_destination.clone(),
        world::LayoutBox::new(1.0, 1.0),
        world::Group::new(),
      ));
    (
      interface,
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        MotionConfig::new(layout)
          .transition(Transition::tween().duration_secs(0.2).ease(Easing::Linear)),
      ),
    )
  }
}

fn fixture() -> (
  Display<App>,
  GameHandle<AnimationGame>,
  GameConsumer<AnimationGame>,
  ObjectId,
) {
  let mut app = App::new("snapshot-animation/content").ui(GameRoot::new(Board {
    layout_destination: world::LayoutDestination::new(*LAYOUT_OBJECT.as_uuid()),
  }));
  let game = app.start_game::<AnimationGame>(0, |connection| ExecutionMode::Interactive {
    connection,
    policy: Policy,
  });
  let consumer = app.game_consumer::<AnimationGame>();
  consumer.resume_automatic_submission();
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("snapshot-animation/content");
  let mut display = Display::connect(app, assets);
  display.poll();
  (display, game, consumer, root)
}

fn submit_all(
  display: &mut Display<App>,
  game: &GameHandle<AnimationGame>,
  consumer: &GameConsumer<AnimationGame>,
) {
  for _ in 0..20 {
    if game.status() == reactant::GameStatus::Ready {
      return;
    }
    assert!(consumer.wait_for_output(TIMEOUT));
    display.poll();
  }
  panic!("game output did not reach submission");
}

fn text(display: &Display<App>, root: ObjectId, name: &str) -> String {
  display
    .ui_element(display.find_ui(root, name))
    .text()
    .unwrap()
    .to_owned()
}

fn x(display: &Display<App>, root: ObjectId, name: &str) -> f64 {
  let Prop::Set(StyleValue::Value(translate)) = display
    .ui_element(display.find_ui(root, name))
    .style()
    .translate
  else {
    panic!("animated x presentation is absent")
  };
  let battlement::Length::Px(value) = translate.x else {
    panic!("animated x presentation is not pixels")
  };
  f64::from(value)
}

#[test]
fn snapshot_events_submit_once_with_blocking_parallel_motion_waits_and_nonblocking_work() {
  let (mut display, game, consumer, root) = fixture();
  game.dispatch(Action::Gameplay);
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  submit_all(&mut display, &game, &consumer);
  assert_eq!(game.accepted_state(), 2);
  assert_eq!(text(&display, root, "stage"), "1");

  display.advance_time(Duration::from_millis(50));
  assert_eq!(x(&display, root, "short"), 5.0);
  assert_eq!(x(&display, root, "long"), 5.0);
  display.click_ui(display.find_ui(root, "local-rerender"));

  display.advance_time(Duration::from_millis(149));
  assert_eq!(text(&display, root, "stage"), "1");
  display.advance_time(Duration::from_millis(1));
  assert_eq!(x(&display, root, "short"), -20.0);
  assert_eq!(x(&display, root, "long"), 20.0);
  assert_eq!(text(&display, root, "stage"), "1");
  display.advance_time(Duration::from_millis(99));
  assert_eq!(text(&display, root, "stage"), "1");
  display.advance_time(Duration::from_millis(1));
  assert_eq!(text(&display, root, "stage"), "2");
  assert_eq!(display.frame(), 0);

  game.dispatch(Action::Cosmetic);
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  submit_all(&mut display, &game, &consumer);
  assert_eq!(game.accepted_state(), 4);
  assert_eq!(text(&display, root, "stage"), "4");
  assert_eq!(x(&display, root, "short"), -20.0);
  display.advance_time(Duration::from_millis(250));
  assert_eq!(x(&display, root, "short"), 5.0);
  assert_eq!(text(&display, root, "stage"), "4");
  display.advance_time(Duration::from_millis(250));
  assert_eq!(x(&display, root, "short"), 30.0);
  assert_eq!(display.frame(), 0);

  display.clear_commands();
  game.dispatch(Action::Layout);
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  submit_all(&mut display, &game, &consumer);
  assert_eq!(game.accepted_state(), 6);
  assert_eq!(text(&display, root, "stage"), "5");
  display.advance_time(Duration::from_millis(199));
  assert_eq!(text(&display, root, "stage"), "5");
  display.advance_time(Duration::from_millis(1));
  assert_eq!(text(&display, root, "stage"), "6");
  assert_eq!(display.frame(), 0);
}

#[test]
fn reconnect_does_not_replay_a_consumed_snapshot_event() {
  let (mut display, game, consumer, root) = fixture();
  game.dispatch(Action::Gameplay);
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  submit_all(&mut display, &game, &consumer);
  display.advance_time(Duration::from_millis(300));
  assert_eq!(text(&display, root, "stage"), "2");

  display.reconnect();
  assert_eq!(text(&display, root, "stage"), "2");
  game.dispatch(Action::Cosmetic);
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  submit_all(&mut display, &game, &consumer);
  assert_eq!(text(&display, root, "stage"), "4");
}

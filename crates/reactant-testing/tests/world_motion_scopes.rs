use std::{
  cell::{Cell, RefCell},
  rc::Rc,
  time::Duration,
};

use battlement::{FloatValue, ObjectId, ParentScene, Prop, StyleValue, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{
  animation_controls::{self, AnimationScope, AnimationSequence, MotionSelector},
  app::App,
  prelude::*,
  world,
};
use reactant_testing::Display;

const WORLD: ObjectId = object_id!("321a0000-0000-4000-8000-000000000031");
const OUTSIDE: ObjectId = object_id!("321a0000-0000-4000-8000-000000000032");

#[derive(Clone, Default)]
struct Probe(Rc<RefCell<Option<(AnimationScope, AnimationScope)>>>);
struct ScopedScene(Probe);

impl Component for ScopedScene {
  fn render(&self) -> impl Render {
    let ui_scope = animation_controls::use_animation_scope();
    let world_scope = animation_controls::use_animation_scope();
    self
      .0
      .0
      .replace(Some((ui_scope.clone(), world_scope.clone())));
    (
      View::new()
        .motion(MotionProps::new().animation_scope(ui_scope))
        .child(
          View::new()
            .name("opacity")
            .style(Style::new().opacity(0.0))
            .motion(MotionProps::new().motion_name("subject")),
        ),
      View::new()
        .name("outside")
        .style(Style::new().opacity(0.0))
        .motion(MotionProps::new().motion_name("subject")),
      world::SceneRoot::new(ParentScene::PrimaryScene).child((
        world::Group::new()
          .child(
            world::Group::new()
              .id(*WORLD.as_uuid())
              .motion(MotionProps::new().motion_name("subject")),
          )
          .motion(MotionProps::new().animation_scope(world_scope)),
        world::Group::new()
          .id(*OUTSIDE.as_uuid())
          .motion(MotionProps::new().motion_name("subject")),
      )),
    )
  }
}

fn fixture() -> (Display<App>, ObjectId, Probe) {
  let probe = Probe::default();
  let app = App::new("motion/scene").ui(ScopedScene(probe.clone()));
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  (Display::connect(app, assets), root, probe)
}

fn presented(display: &Display<App>, root: ObjectId, value: f32) {
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "opacity"))
      .style()
      .opacity,
    Prop::Set(StyleValue::Value(FloatValue(value)))
  );
  assert_eq!(
    display.object(WORLD).unwrap().local_transform().position.x,
    f64::from(value)
  );
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "outside"))
      .style()
      .opacity,
    Prop::Set(StyleValue::Value(FloatValue(0.0)))
  );
  assert_eq!(
    display
      .object(OUTSIDE)
      .unwrap()
      .local_transform()
      .position
      .x,
    0.0
  );
}

fn sequence(target: StyleTarget) -> AnimationSequence {
  AnimationSequence::new().animate(
    MotionSelector::name("subject"),
    target,
    Transition::tween().duration_secs(1.0).ease(Easing::Linear),
  )
}

#[test]
fn scoped_ui_and_world_targets_select_only_their_own_descendants_and_share_playback_controls() {
  let (mut display, root, probe) = fixture();
  let (ui, world) = probe.0.borrow().as_ref().unwrap().clone();
  let ui_playback = ui.start(sequence(StyleTarget::new().opacity(1.0)));
  let world_playback = world.start(sequence(StyleTarget::new().local_position_x(1.0)));
  let completed = Rc::new(Cell::new(0));
  for playback in [&ui_playback, &world_playback] {
    let counter = completed.clone();
    playback.on_complete(move || counter.set(counter.get() + 1));
  }
  display.poll();
  display.advance_time(Duration::from_millis(250));
  presented(&display, root, 0.25);
  ui_playback.pause();
  world_playback.pause();
  display.poll();
  display.advance_time(Duration::from_millis(500));
  presented(&display, root, 0.25);
  ui_playback.play();
  world_playback.play();
  display.poll();
  display.settle();
  presented(&display, root, 1.0);
  assert_eq!(completed.get(), 2);
  assert_eq!(display.presentation_time(), Duration::from_millis(1500));
  assert_eq!(display.frame(), 0);
}

#[test]
fn scoped_set_and_stop_use_the_current_host_selection() {
  let (mut display, root, probe) = fixture();
  let (ui, world) = probe.0.borrow().as_ref().unwrap().clone();
  ui.set(MotionSelector::Children, StyleTarget::new().opacity(0.5));
  world.set(
    MotionSelector::Children,
    StyleTarget::new().local_position_x(0.5),
  );
  display.poll();
  presented(&display, root, 0.5);
  let first = ui.start(sequence(StyleTarget::new().opacity(1.0)));
  let second = world.start(sequence(StyleTarget::new().local_position_x(1.0)));
  let stopped = Rc::new(Cell::new(0));
  for playback in [&first, &second] {
    let counter = stopped.clone();
    playback.on_stop(move || counter.set(counter.get() + 1));
  }
  display.poll();
  display.advance_time(Duration::from_millis(500));
  presented(&display, root, 0.75);
  ui.stop(MotionSelector::Descendants);
  world.stop(MotionSelector::Descendants);
  display.poll();
  display.advance_time(Duration::from_secs(1));
  presented(&display, root, 0.75);
  assert_eq!(stopped.get(), 2);
}

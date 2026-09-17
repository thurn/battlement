use std::{
  boxed::Box as Boxed,
  cell::{Cell, RefCell},
  rc::Rc,
  time::Duration,
};

use battlement::{FloatValue, ObjectId, ParentScene, Prop, StyleValue, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{app::App, prelude::*, world};
use reactant_testing::Display;

const WORLD: ObjectId = object_id!("321a0000-0000-4000-8000-000000000021");

#[derive(Clone, Default)]
struct Probe {
  value: Rc<RefCell<Option<TypedMotionValue<f32>>>>,
  renders: Rc<Cell<usize>>,
}

struct ValueScene(Probe);
impl Component for ValueScene {
  fn render(&self) -> impl Render {
    self.0.renders.set(self.0.renders.get() + 1);
    let value = use_motion_value(0.0f32);
    self.0.value.replace(Some(value.clone()));
    (
      View::new()
        .name("opacity")
        .animate(StyleTarget::new().opacity_value(value.clone())),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .id(*WORLD.as_uuid())
          .animate(StyleTarget::new().local_position_x_value(value)),
      ),
    )
  }
}

fn fixture() -> (Display<App>, ObjectId, Probe) {
  let probe = Probe::default();
  let app = App::new("motion/scene").ui(ValueScene(probe.clone()));
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  (Display::connect(app, assets), root, probe)
}

fn presented(display: &Display<App>, root: ObjectId, expected: f32) {
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "opacity"))
      .style()
      .opacity,
    Prop::Set(StyleValue::Value(FloatValue(expected)))
  );
  assert!(
    (display.object(WORLD).unwrap().local_transform().position.x - f64::from(expected)).abs()
      < 0.00001
  );
}

fn linear() -> Transition {
  Transition::tween().duration_secs(1.0).ease(Easing::Linear)
}

#[test]
fn value_playback_samples_both_adapters_and_obeys_pause_speed_and_completion() {
  let (mut display, root, probe) = fixture();
  let renders = probe.renders.get();
  let completed = Rc::new(Cell::new(0));
  let playback = probe
    .value
    .borrow()
    .as_ref()
    .unwrap()
    .animate(1.0, linear());
  let counter = completed.clone();
  playback.on_complete(move || counter.set(counter.get() + 1));
  display.poll();
  display.advance_time(Duration::from_millis(250));
  presented(&display, root, 0.25);
  playback.pause();
  display.poll();
  display.advance_time(Duration::from_millis(500));
  presented(&display, root, 0.25);
  assert_eq!(probe.renders.get(), renders);
  playback.set_speed(2.0);
  playback.play();
  display.poll();
  display.settle();
  presented(&display, root, 1.0);
  assert_eq!(completed.get(), 1);
  assert_eq!(display.presentation_time(), Duration::from_millis(1125));
  assert_eq!(display.frame(), 0);
}

#[test]
fn value_retarget_and_stop_report_one_terminal_outcome_from_the_presented_pose() {
  let (mut display, root, probe) = fixture();
  let value = probe.value.borrow().as_ref().unwrap().clone();
  let cancelled = Rc::new(Cell::new(0));
  let stopped = Rc::new(Cell::new(0));
  let first = value.animate(1.0, linear());
  let counter = cancelled.clone();
  first.on_cancel(move || counter.set(counter.get() + 1));
  display.poll();
  display.advance_time(Duration::from_millis(250));
  let second = value.animate(0.0, linear());
  let counter = stopped.clone();
  second.on_stop(move || counter.set(counter.get() + 1));
  display.poll();
  presented(&display, root, 0.25);
  assert_eq!(cancelled.get(), 1);
  display.advance_time(Duration::from_millis(500));
  presented(&display, root, 0.125);
  second.stop();
  display.poll();
  display.advance_time(Duration::from_secs(1));
  presented(&display, root, 0.125);
  assert_eq!(stopped.get(), 1);
  assert_eq!(cancelled.get(), 1);
}

#[test]
fn value_failure_keeps_finite_presentation_and_reports_failed_once() {
  let (mut display, root, probe) = fixture();
  let failed = Rc::new(Cell::new(0));
  let playback = probe
    .value
    .borrow()
    .as_ref()
    .unwrap()
    .animate(f32::MAX, Transition::spring().stiffness(100.0).damping(1.0));
  let counter = failed.clone();
  playback.on_failed(move || counter.set(counter.get() + 1));
  display.poll();
  display.advance_time(Duration::from_millis(100));
  assert_eq!(failed.get(), 0);
  display.advance_time(Duration::from_millis(100));
  assert_eq!(failed.get(), 1);
  display.advance_time(Duration::from_secs(1));
  assert_eq!(failed.get(), 1);
  assert!(
    display
      .object(WORLD)
      .unwrap()
      .local_transform()
      .position
      .x
      .is_finite()
  );
  let Prop::Set(StyleValue::Value(FloatValue(value))) = display
    .ui_element(display.find_ui(root, "opacity"))
    .style()
    .opacity
  else {
    panic!("opacity absent");
  };
  assert!(value.is_finite());
}

#[test]
fn snapshot_cancels_session_owned_value_playback_and_retains_its_presented_value() {
  let (mut display, root, probe) = fixture();
  let playback = probe
    .value
    .borrow()
    .as_ref()
    .unwrap()
    .animate(1.0, linear());
  display.poll();
  display.advance_time(Duration::from_millis(250));
  playback.pause();
  display.poll();
  let cancelled = Rc::new(Cell::new(0));
  let counter = cancelled.clone();
  playback.on_cancel(move || counter.set(counter.get() + 1));
  display.reconnect();
  presented(&display, root, 0.25);
  display.advance_time(Duration::from_millis(500));
  presented(&display, root, 0.25);
  playback.play();
  display.poll();
  display.advance_time(Duration::from_millis(250));
  presented(&display, root, 0.25);
  assert_eq!(cancelled.get(), 1);
  let next = probe
    .value
    .borrow()
    .as_ref()
    .unwrap()
    .animate(1.0, linear());
  display.poll();
  display.advance_time(Duration::from_millis(250));
  presented(&display, root, 0.4375);
  next.stop();
  display.poll();
}

struct DerivedScene {
  probe: Probe,
  frames: Rc<RefCell<Vec<f32>>>,
}
impl Component for DerivedScene {
  fn render(&self) -> impl Render {
    let value = use_motion_value(0.0f32);
    self.probe.value.replace(Some(value.clone()));
    let opacity = use_transform(
      value.clone(),
      InputRange::new([0.0, 1.0]),
      OutputRange::new([0.0, 0.5]),
    );
    let squared =
      use_motion_expression(MotionExpression::input(value.clone()).multiply(value.clone()));
    let frames = self.frames.clone();
    use_motion_value_event(value, MotionValueEvent::AnimationFrame, move |value| {
      frames.borrow_mut().push(value)
    });
    (
      View::new()
        .name("opacity")
        .animate(StyleTarget::new().opacity_value(opacity)),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .id(*WORLD.as_uuid())
          .animate(StyleTarget::new().local_position_x_value(squared)),
      ),
    )
  }
}

#[test]
fn derived_graph_bindings_update_without_synthesizing_animation_frames() {
  let probe = Probe::default();
  let frames = Rc::new(RefCell::new(Vec::new()));
  let app = App::new("motion/scene").ui(DerivedScene {
    probe: probe.clone(),
    frames: frames.clone(),
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  let mut display = Display::connect(app, assets);
  let frame_count = frames.borrow().len();
  let playback = probe
    .value
    .borrow()
    .as_ref()
    .unwrap()
    .animate(1.0, linear());
  display.poll();
  display.advance_time(Duration::from_millis(500));
  presented(&display, root, 0.25);
  assert_eq!(frames.borrow().len(), frame_count);
  display.advance_frame();
  assert_eq!(frames.borrow().len(), frame_count + 1);
  assert_eq!(frames.borrow().last(), Some(&0.5));
  display.poll();
  assert_eq!(frames.borrow().len(), frame_count + 1);
  assert_eq!(display.presentation_time(), Duration::from_millis(500));
  playback.stop();
  display.poll();
}

struct TimeScene;
impl Component for TimeScene {
  fn render(&self) -> impl Render {
    let time = use_time();
    let seconds = use_motion_expression(MotionExpression::seconds(time).clamp(0.0, 1.0));
    (
      View::new()
        .name("opacity")
        .animate(StyleTarget::new().opacity_value(seconds.clone())),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .id(*WORLD.as_uuid())
          .animate(StyleTarget::new().local_position_x_value(seconds)),
      ),
    )
  }
}

#[test]
fn time_graph_reads_virtual_time_without_creating_a_second_clock() {
  let app = App::new("motion/scene").ui(TimeScene);
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  let mut display = Display::connect(app, assets);
  display.advance_time(Duration::from_millis(250));
  presented(&display, root, 0.25);
  display.advance_frame();
  display.advance_frame();
  presented(&display, root, 0.25);
  assert_eq!(display.presentation_time(), Duration::from_millis(250));
}

#[path = "support/batch_engine.rs"]
mod batch_engine;

#[test]
fn value_operations_preserve_command_blocking_flags_in_verified_batches() {
  for blocking in [false, true] {
    let probe = Probe::default();
    let app = App::new("motion/scene").ui(ValueScene(probe.clone()));
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene("motion/scene");
    let mut display = Display::connect(batch_engine::BatchEngine::new(app), assets);
    let value_id = probe.value.borrow().as_ref().unwrap().id();
    let mut transition = battlement::TransitionDefinition::tween();
    transition.generator = battlement::TransitionGenerator::Tween {
      duration_micros: 1_000_000,
      easings: vec![battlement::MotionEasing::Linear],
      times: None,
    };
    let mut command = battlement::Command::new_v4(battlement::CommandBody::MotionValue(
      battlement::MotionValueOperation {
        value_id,
        command: battlement::MotionValueCommand::Animate {
          playback_id: ObjectId::new_v4(),
          generation: 1,
          target: Boxed::new(battlement::MotionValue::Scalar(1.0)),
          transition,
        },
      },
    ));
    command.blocking = blocking;
    let marker = ObjectId::new_v4();
    let next = battlement::Command::new_v4(battlement::CommandBody::object_create(
      battlement::GameObject::new(marker, battlement::GameObjectKind::Empty),
    ));
    display.with_engine(|engine| engine.queue(vec![vec![command], vec![next]]));
    display.poll();
    display.advance_time(Duration::from_millis(500));
    assert_eq!(display.object(marker).is_some(), !blocking);
    assert_eq!(
      display.object(WORLD).unwrap().local_transform().position.x,
      0.5
    );
    display.advance_time(Duration::from_millis(500));
    assert!(display.object(marker).is_some());
    display.with_engine(|engine| assert!(engine.failures.is_empty()));
    assert_eq!(display.frame(), 0);
  }
}

struct SpringScene(Probe);
impl Component for SpringScene {
  fn render(&self) -> impl Render {
    let value = use_motion_value(0.0f32);
    self.0.value.replace(Some(value.clone()));
    let spring = use_spring(value, SpringOptions::new().stiffness(100.0).damping(10.0));
    (
      View::new()
        .name("opacity")
        .animate(StyleTarget::new().opacity_value(spring.clone())),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .id(*WORLD.as_uuid())
          .animate(StyleTarget::new().local_position_x_value(spring)),
      ),
    )
  }
}

#[test]
fn a_passive_spring_settles_through_existing_time_and_keeps_its_phase_across_reconnect() {
  let probe = Probe::default();
  let app = App::new("motion/scene").ui(SpringScene(probe.clone()));
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  let mut display = Display::connect(app, assets);
  probe.value.borrow().as_ref().unwrap().set(1.0);
  display.poll();
  display.advance_time(Duration::from_millis(100));
  let before = display.object(WORLD).unwrap().local_transform().position.x;
  assert!(before > 0.0 && before < 1.0);
  display.reconnect();
  let after = display.object(WORLD).unwrap().local_transform().position.x;
  assert!(
    (before - after).abs() < 0.00001,
    "before {before}, after {after}"
  );
  display.settle();
  presented(&display, root, 1.0);
  assert_eq!(display.frame(), 0);
}

struct ComposedScale;
impl Component for ComposedScale {
  fn render(&self) -> impl Render {
    let factor = use_motion_value(2.0f32);
    let (bound, set_bound) = reactant::hooks::use_state(true);
    let target = StyleTarget::new().scale(3.0);
    (
      reactant::host::ButtonHost::new(trox::ls("Unbind"))
        .name("unbind")
        .on_click(set_bound.update_callback(|_| false)),
      View::new()
        .name("scale")
        .animate(if bound {
          target.scale_factor(factor)
        } else {
          target
        })
        .transition(linear()),
    )
  }
}

#[test]
fn removing_a_scale_contribution_restores_the_uncomposed_track() {
  let app = App::new("motion/scene").ui(ComposedScale);
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  let mut display = Display::connect(app, assets);
  let scale = display.find_ui(root, "scale");
  display.advance_time(Duration::from_millis(500));
  assert_eq!(
    display.ui_element(scale).style().scale,
    Prop::Set(StyleValue::Value(battlement::Scale::uniform(4.0)))
  );
  display.click_ui(display.find_ui(root, "unbind"));
  assert_eq!(
    display.ui_element(scale).style().scale,
    Prop::Set(StyleValue::Value(battlement::Scale::uniform(2.0)))
  );
  display.advance_time(Duration::from_millis(500));
  assert_eq!(
    display.ui_element(scale).style().scale,
    Prop::Set(StyleValue::Value(battlement::Scale::uniform(2.5)))
  );
}

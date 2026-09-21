use std::{
  cell::{Cell, RefCell},
  rc::Rc,
  time::Duration,
};

use battlement::{FloatValue, ObjectId, ParentScene, Prop, StyleValue, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{animation_controls, host::ButtonHost, prelude::*, testing::App, world};
use reactant_testing::{Display, temporal::Clock};
use trox::ls;

const WORLD: ObjectId = object_id!("321a0000-0000-4000-8000-000000000011");

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Pose {
  End,
  Back,
  Overflow,
  FinishOnly,
}

#[derive(Clone, Default)]
struct Probe {
  playback: Rc<RefCell<Option<AnimationPlayback>>>,
  completed: Rc<Cell<usize>>,
  cancelled: Rc<Cell<usize>>,
  failed: Rc<Cell<usize>>,
}

struct Controlled(Probe);
impl Component for Controlled {
  fn render(&self) -> impl Render {
    let controls = animation_controls::use_animation_controls::<Pose>();
    let finish = controls.clone();
    let finish_probe = self.0.clone();
    let start = controls.clone();
    let back = controls.clone();
    let overflow = controls.clone();
    let overflow_probe = self.0.clone();
    let start_probe = self.0.clone();
    let back_probe = self.0.clone();
    MotionConfig::new((
      ButtonHost::new(ls("Finish only"))
        .name("finish-only")
        .on_click(move || {
          track(
            &finish_probe,
            finish.start(animation_controls::ControlTarget::Variant(Pose::FinishOnly)),
          );
        }),
      ButtonHost::new(ls("Start"))
        .name("start")
        .on_click(move || {
          track(
            &start_probe,
            start.start(animation_controls::ControlTarget::Variant(Pose::End)),
          );
        }),
      ButtonHost::new(ls("Back")).name("back").on_click(move || {
        track(
          &back_probe,
          back.start(animation_controls::ControlTarget::Variant(Pose::Back)),
        );
      }),
      ButtonHost::new(ls("Overflow"))
        .name("overflow")
        .on_click(move || {
          track(
            &overflow_probe,
            overflow.start(animation_controls::ControlTarget::Variant(Pose::Overflow)),
          );
        }),
      View::new()
        .name("opacity")
        .style(Style::new().opacity(0.0))
        .animation_controls(controls.clone())
        .variants(
          Variants::<Pose>::new()
            .target(
              Pose::FinishOnly,
              MotionTarget::new(StyleTarget::new().opacity(1.0))
                .transition_end(StyleTarget::new().scale(2.0)),
            )
            .target(Pose::End, StyleTarget::new().opacity(1.0).scale(2.0))
            .target(Pose::Back, StyleTarget::new().opacity(0.0))
            .target(
              Pose::Overflow,
              MotionTarget::new(StyleTarget::new().opacity(f32::MAX))
                .transition(Transition::spring().stiffness(100.0).damping(1.0)),
            ),
        ),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new().id(*WORLD.as_uuid()).motion(
          MotionProps::new()
            .animation_controls(controls)
            .variants(
              Variants::<Pose>::new()
                .target(
                  Pose::FinishOnly,
                  MotionTarget::new(StyleTarget::new().local_position_x(1.0))
                    .transition_end(StyleTarget::new().local_position_z(4.0)),
                )
                .target(
                  Pose::End,
                  StyleTarget::new()
                    .local_position_x(1.0)
                    .local_position_z(4.0),
                )
                .target(Pose::Back, StyleTarget::new().local_position_x(0.0))
                .target(
                  Pose::Overflow,
                  MotionTarget::new(StyleTarget::new().local_position_x(f32::MAX))
                    .transition(Transition::spring().stiffness(100.0).damping(1.0)),
                ),
            )
            .animate(StyleTarget::new().local_position_y(4.0)),
        ),
      ),
    ))
    .transition(Transition::tween().duration_secs(1.0).ease(Easing::Linear))
  }
}

fn track(probe: &Probe, playback: AnimationPlayback) {
  let completed = probe.completed.clone();
  playback.on_complete(move || completed.set(completed.get() + 1));
  let cancelled = probe.cancelled.clone();
  playback.on_cancel(move || cancelled.set(cancelled.get() + 1));
  let failed = probe.failed.clone();
  playback.on_failed(move || failed.set(failed.get() + 1));
  probe.playback.replace(Some(playback));
}

fn fixture() -> (Display<App>, ObjectId, Probe) {
  let probe = Probe::default();
  let app = App::new("motion/scene").ui(Controlled(probe.clone()));
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  (Display::connect(app, assets), root, probe)
}

fn position(display: &Display<App>, root: ObjectId, x: f64, y: f64) {
  let pose = display.object(WORLD).unwrap().local_transform().position;
  assert!((pose.x - x).abs() < 0.00001, "{pose:?}");
  assert!((pose.y - y).abs() < 0.00001, "{pose:?}");
  let opacity = display.find_ui(root, "opacity");
  let Prop::Set(StyleValue::Value(FloatValue(value))) = display.ui_element(opacity).style().opacity
  else {
    panic!("opacity is not presented");
  };
  assert!(
    (f64::from(value) - x).abs() < 0.00001,
    "opacity {value}, x {x}"
  );
}

#[test]
fn typed_controls_pause_and_complete_across_hosts_without_restarting_disjoint_placement() {
  let (mut display, root, probe) = fixture();
  Clock::advance(&mut display, Duration::from_millis(250));
  display.click_ui(display.find_ui(root, "start"));
  Clock::advance(&mut display, Duration::from_millis(250));
  position(&display, root, 0.25, 2.0);
  probe.playback.borrow().as_ref().unwrap().pause();
  display.poll();
  Clock::advance(&mut display, Duration::from_millis(250));
  position(&display, root, 0.25, 3.0);
  probe.playback.borrow().as_ref().unwrap().set_speed(2.0);
  probe.playback.borrow().as_ref().unwrap().play();
  display.poll();
  display.settle();
  position(&display, root, 1.0, 4.0);
  assert_eq!(probe.completed.get(), 1);
  assert_eq!(probe.cancelled.get(), 0);
  assert_eq!(display.presentation_time(), Duration::from_millis(1125));
  assert_eq!(display.frame(), 0);
}

#[test]
fn a_new_control_start_interrupts_from_the_actual_pose_and_reports_the_old_outcome_once() {
  let (mut display, root, probe) = fixture();
  display.click_ui(display.find_ui(root, "start"));
  Clock::advance(&mut display, Duration::from_millis(250));
  position(&display, root, 0.25, 1.0);
  let _previous = probe.playback.borrow().as_ref().unwrap().clone();
  display.click_ui(display.find_ui(root, "back"));
  position(&display, root, 0.25, 1.0);
  assert_eq!(probe.cancelled.get(), 1);
  Clock::advance(&mut display, Duration::from_millis(500));
  position(&display, root, 0.125, 3.0);
  display.settle();
  position(&display, root, 0.0, 4.0);
  assert_eq!(probe.completed.get(), 1);
  assert_eq!(probe.cancelled.get(), 1);
}

#[path = "support/batch_engine.rs"]
mod batch_engine;

#[test]
fn command_blocking_flags_control_later_groups_while_nonblocking_motion_keeps_running() {
  for blocking in [true, false] {
    let app = App::new("motion/scene").ui(Controlled(Probe::default()));
    let root = app.root_document().root_id;
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene("motion/scene");
    let mut display = Display::connect(batch_engine::BatchEngine::new(app), assets);
    let opacity = display.find_ui(root, "opacity");
    let Prop::Set(descriptor) =
      &battlement::UiVisualElementProperties::visual_element(display.ui_element(opacity).element())
        .motion
    else {
      panic!("control binding is absent");
    };
    let control_id = descriptor.control_id.unwrap();
    let target = descriptor.named_targets[0].name.clone();
    let mut start = battlement::Command::new_v4(battlement::CommandBody::MotionControl(
      battlement::MotionControlOperation {
        control_id,
        command: battlement::MotionControlCommand::Start {
          playback_id: ObjectId::new_v4(),
          generation: 1,
          target: battlement::MotionControlTarget::Variant(target),
        },
      },
    ));
    start.blocking = blocking;
    let marker = ObjectId::new_v4();
    let show_marker = battlement::Command::new_v4(battlement::CommandBody::object_create(
      battlement::GameObject::new(marker, battlement::GameObjectKind::Empty),
    ));
    display.with_engine(|engine| engine.queue(vec![vec![start], vec![show_marker]]));
    display.poll();
    assert_eq!(display.object(marker).is_some(), !blocking);
    Clock::advance(&mut display, Duration::from_millis(500));
    assert_eq!(display.object(marker).is_some(), !blocking);
    let x = display.object(WORLD).unwrap().local_transform().position.x;
    assert!((x - 0.5).abs() < 0.00001);
    Clock::advance(&mut display, Duration::from_millis(500));
    assert!(display.object(marker).is_some());
    assert_eq!(
      display.object(WORLD).unwrap().local_transform().position.x,
      1.0
    );
    assert_eq!(display.frame(), 0);
  }
}

#[test]
fn a_sampler_failure_reports_one_failed_outcome_and_releases_its_command_operation() {
  let (mut display, root, probe) = fixture();
  display.click_ui(display.find_ui(root, "overflow"));
  Clock::advance(&mut display, Duration::from_millis(100));
  assert_eq!(probe.failed.get(), 0);
  Clock::advance(&mut display, Duration::from_millis(100));
  assert_eq!(probe.failed.get(), 1);
  assert_eq!(probe.completed.get(), 0);
  assert_eq!(probe.cancelled.get(), 0);
  Clock::advance(&mut display, Duration::from_secs(1));
  assert_eq!(probe.failed.get(), 1);
  assert!(
    display
      .object(WORLD)
      .unwrap()
      .local_transform()
      .position
      .x
      .is_finite()
  );
  assert_eq!(display.frame(), 0);
}

#[test]
fn failed_motion_cancels_blocked_groups_and_reports_nonblocking_failures_after_continuation() {
  for blocking in [true, false] {
    let app = App::new("motion/scene").ui(Controlled(Probe::default()));
    let root = app.root_document().root_id;
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene("motion/scene");
    let mut display = Display::connect(batch_engine::BatchEngine::new(app), assets);
    let opacity = display.find_ui(root, "opacity");
    let Prop::Set(descriptor) =
      &battlement::UiVisualElementProperties::visual_element(display.ui_element(opacity).element())
        .motion
    else {
      panic!("control binding is absent");
    };
    let target = descriptor
      .named_targets
      .iter()
      .find(|target| {
        target.target.tracks[0].values == vec![battlement::MotionValue::Scalar(f32::MAX)]
      })
      .expect("overflow variant")
      .name
      .clone();
    let mut start = battlement::Command::new_v4(battlement::CommandBody::MotionControl(
      battlement::MotionControlOperation {
        control_id: descriptor.control_id.unwrap(),
        command: battlement::MotionControlCommand::Start {
          playback_id: ObjectId::new_v4(),
          generation: 1,
          target: battlement::MotionControlTarget::Variant(target),
        },
      },
    ));
    start.blocking = blocking;
    let command_id = start.command_id;
    let marker = ObjectId::new_v4();
    let show_marker = battlement::Command::new_v4(battlement::CommandBody::object_create(
      battlement::GameObject::new(marker, battlement::GameObjectKind::Empty),
    ));
    display.with_engine(|engine| engine.queue(vec![vec![start], vec![show_marker]]));
    display.poll();
    assert_eq!(display.object(marker).is_some(), !blocking);
    Clock::advance(&mut display, Duration::from_millis(200));
    assert_eq!(display.object(marker).is_some(), !blocking);
    display.with_engine(|engine| assert_eq!(engine.failures, vec![(blocking, command_id)]));
    Clock::advance(&mut display, Duration::from_secs(1));
    display.with_engine(|engine| assert_eq!(engine.failures.len(), 1));
    assert_eq!(display.frame(), 0);
  }
}

#[test]
fn replacing_one_imperative_property_preserves_the_other_tracks_original_timeline() {
  let (mut display, root, probe) = fixture();
  display.click_ui(display.find_ui(root, "start"));
  Clock::advance(&mut display, Duration::from_millis(250));
  let previous = probe.playback.borrow().as_ref().unwrap().clone();
  display.click_ui(display.find_ui(root, "back"));
  for expected in [1.0, 2.0, 3.0, 4.0] {
    assert_eq!(
      display.object(WORLD).unwrap().local_transform().position.z,
      expected
    );
    assert_eq!(
      display
        .ui_element(display.find_ui(root, "opacity"))
        .style()
        .scale,
      Prop::Set(StyleValue::Value(battlement::Scale::uniform(
        (1.0 + expected / 4.0) as f32
      )))
    );
    Clock::advance(&mut display, Duration::from_millis(250));
  }
  assert_eq!(probe.cancelled.get(), 1);
  drop(previous);
}

#[test]
fn retargeting_all_tracks_keeps_disjoint_transition_end_on_its_original_clock() {
  let (mut display, root, _) = fixture();
  display.click_ui(display.find_ui(root, "finish-only"));
  Clock::advance(&mut display, Duration::from_millis(250));
  display.click_ui(display.find_ui(root, "back"));
  for elapsed in [250, 250, 249] {
    Clock::advance(&mut display, Duration::from_millis(elapsed));
    assert_eq!(
      display.object(WORLD).unwrap().local_transform().position.z,
      0.0
    );
  }
  Clock::advance(&mut display, Duration::from_millis(1));
  assert_eq!(
    display.object(WORLD).unwrap().local_transform().position.z,
    4.0
  );
  let opacity = display.find_ui(root, "opacity");
  assert_eq!(
    display.ui_element(opacity).style().scale,
    Prop::Set(StyleValue::Value(battlement::Scale::uniform(2.0)))
  );
}

#[test]
#[should_panic(expected = "blocking Motion controls require a mounted target")]
fn an_empty_blocking_control_fails_instead_of_stranding_the_next_group() {
  let app = App::new("motion/scene").ui(View::new());
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  let mut display = Display::connect(batch_engine::BatchEngine::new(app), assets);
  let start = battlement::Command::new_v4(battlement::CommandBody::MotionControl(
    battlement::MotionControlOperation {
      control_id: ObjectId::new_v4(),
      command: battlement::MotionControlCommand::Start {
        playback_id: ObjectId::new_v4(),
        generation: 1,
        target: battlement::MotionControlTarget::Variant("end".into()),
      },
    },
  ));
  display.with_engine(|engine| engine.queue(vec![vec![start]]));
  display.poll();
}

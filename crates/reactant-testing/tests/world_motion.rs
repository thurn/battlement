use std::{cell::Cell, rc::Rc, time::Duration};

use battlement::{FloatValue, ObjectId, ParentScene, Prop, StyleValue, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{prelude::*, testing::App, world};
use reactant_testing::Display;

const WORLD: ObjectId = object_id!("321a0000-0000-4000-8000-000000000001");

struct MotionScene(Rc<Cell<usize>>);

impl Component for MotionScene {
  fn render(&self) -> impl Render {
    self.0.set(self.0.get() + 1);
    let transition = Transition::tween().duration_secs(1.0).ease(Easing::Linear);
    (
      View::new()
        .name("opacity")
        .initial(StyleTarget::new().opacity(0.0))
        .animate(StyleTarget::new().opacity(1.0))
        .transition(transition.clone()),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .id(*WORLD.as_uuid())
          .initial(StyleTarget::new().local_position_x(0.0))
          .animate(StyleTarget::new().local_position_x(1.0))
          .transition(transition),
      ),
    )
  }
}

#[test]
fn ui_and_world_sample_the_same_clock_without_rendering_rules_or_frames() {
  let renders = Rc::new(Cell::new(0));
  let app = App::new("motion/scene").ui(MotionScene(renders.clone()));
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  let mut display = Display::connect(app, assets);
  let opacity = display.find_ui(root, "opacity");
  let render_count = renders.get();
  let frame = display.frame();
  for expected in [0.0, 0.25, 0.5, 0.75, 1.0] {
    if expected > 0.0 {
      display.advance_time(Duration::from_millis(250));
    }
    assert_eq!(
      display.ui_element(opacity).style().opacity,
      Prop::Set(StyleValue::Value(FloatValue(expected)))
    );
    let position = display.object(WORLD).unwrap().local_transform().position;
    assert!(
      (position.x - f64::from(expected)).abs() < 0.00001,
      "{position:?}"
    );
    assert_eq!(renders.get(), render_count);
    assert_eq!(display.frame(), frame);
  }
  let time = display.presentation_time();
  display.advance_frame();
  assert_eq!(display.presentation_time(), time);
  assert_eq!(display.frame(), frame + 1);
  assert_eq!(renders.get(), render_count);
}

#[derive(Clone, Default)]
struct PlaybackProbe {
  app: Rc<std::cell::RefCell<Option<reactant::app_context::AppHandle>>>,
  slots: Rc<std::cell::RefCell<Vec<battlement::MotionLifecycleEvent>>>,
  completed: Rc<Cell<usize>>,
  stopped: Rc<Cell<usize>>,
}

impl PlaybackProbe {
  fn target(&self, style: StyleTarget) -> MotionTarget {
    let slots = self.slots.clone();
    let completed = self.completed.clone();
    let stopped = self.stopped.clone();
    MotionTarget::new(style)
      .on_start_event(move |_: &mut (), event| slots.borrow_mut().push(event.clone()))
      .on_complete(move |_: &mut ()| completed.set(completed.get() + 1))
      .on_stop(move |_: &mut ()| stopped.set(stopped.get() + 1))
  }

  fn command(&self, command: battlement::MotionPlaybackCommand) {
    let app = self.app.borrow();
    for slot in self.slots.borrow().iter() {
      app.as_ref().unwrap().send(battlement::Command::new_v4(
        battlement::CommandBody::MotionPlayback(battlement::MotionPlaybackOperation {
          descriptor_id: slot.descriptor_id,
          slot: slot.slot,
          generation: slot.generation,
          command,
        }),
      ));
    }
  }
}

struct ControlledScene(PlaybackProbe);

impl Component for ControlledScene {
  fn render(&self) -> impl Render {
    self.0.app.replace(Some(reactant::app_context::use_app()));
    MotionConfig::new((
      View::new()
        .name("opacity")
        .initial(StyleTarget::new().opacity(0.0))
        .animate(self.0.target(StyleTarget::new().opacity(1.0))),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .id(*WORLD.as_uuid())
          .initial(StyleTarget::new().local_position_x(0.0))
          .animate(self.0.target(StyleTarget::new().local_position_x(1.0))),
      ),
    ))
    .transition(Transition::tween().duration_secs(1.0).ease(Easing::Linear))
  }
}

fn controlled() -> (Display<App>, ObjectId, PlaybackProbe) {
  let probe = PlaybackProbe::default();
  let app = App::new("motion/scene").ui(ControlledScene(probe.clone()));
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  let display = Display::connect(app, assets);
  let opacity = display.find_ui(root, "opacity");
  assert_eq!(probe.slots.borrow().len(), 2);
  (display, opacity, probe)
}

fn assert_pair(display: &Display<App>, opacity: ObjectId, expected: f32) {
  let Prop::Set(StyleValue::Value(FloatValue(value))) = display.ui_element(opacity).style().opacity
  else {
    panic!("opacity is not presented");
  };
  assert!(
    (value - expected).abs() < 0.00001,
    "opacity {value}, expected {expected}"
  );
  let position = display.object(WORLD).unwrap().local_transform().position.x;
  assert!(
    (position - f64::from(expected)).abs() < 0.00001,
    "position {position}, expected {expected}"
  );
}

#[test]
fn pause_resume_speed_and_completion_share_the_existing_temporal_driver() {
  let (mut display, opacity, probe) = controlled();
  display.advance_time(Duration::from_millis(250));
  probe.command(battlement::MotionPlaybackCommand::Pause);
  display.poll();
  display.advance_time(Duration::from_millis(400));
  assert_pair(&display, opacity, 0.25);
  probe.command(battlement::MotionPlaybackCommand::SetSpeed { value: 2.0 });
  probe.command(battlement::MotionPlaybackCommand::Play);
  display.poll();
  display.advance_time(Duration::from_millis(125));
  assert_pair(&display, opacity, 0.5);
  probe.command(battlement::MotionPlaybackCommand::Play);
  display.poll();
  display.settle();
  assert_pair(&display, opacity, 1.0);
  assert_eq!(probe.completed.get(), 2);
  assert_eq!(display.presentation_time(), Duration::from_millis(1025));
  assert_eq!(display.frame(), 0);
}

#[test]
fn stopped_slots_hold_the_displayed_pose_and_emit_only_one_terminal_boundary() {
  let (mut display, opacity, probe) = controlled();
  display.advance_time(Duration::from_millis(250));
  probe.command(battlement::MotionPlaybackCommand::Stop);
  display.poll();
  display.settle();
  assert_eq!(display.presentation_time(), Duration::from_millis(250));
  display.advance_time(Duration::from_secs(1));
  probe.command(battlement::MotionPlaybackCommand::Complete);
  display.poll();
  assert_pair(&display, opacity, 0.25);
  assert_eq!(probe.stopped.get(), 2);
  assert_eq!(probe.completed.get(), 0);
}

#[test]
fn explicit_completion_applies_the_target_after_an_intermediate_sample() {
  let (mut display, opacity, probe) = controlled();
  display.advance_time(Duration::from_millis(250));
  probe.command(battlement::MotionPlaybackCommand::Complete);
  display.poll();
  assert_pair(&display, opacity, 1.0);
  assert_eq!(probe.completed.get(), 2);
  assert_eq!(display.presentation_time(), Duration::from_millis(250));
}

#[test]
fn reconnect_preserves_paused_motion_and_its_original_origin() {
  let (mut display, opacity, probe) = controlled();
  display.advance_time(Duration::from_millis(250));
  probe.command(battlement::MotionPlaybackCommand::Pause);
  display.poll();
  display.reconnect();
  assert_pair(&display, opacity, 0.25);
  display.advance_time(Duration::from_millis(100));
  assert_pair(&display, opacity, 0.25);
  probe.command(battlement::MotionPlaybackCommand::Play);
  display.poll();
  display.advance_time(Duration::from_millis(250));
  assert_pair(&display, opacity, 0.5);
  display.settle();
  assert_pair(&display, opacity, 1.0);
  assert_eq!(probe.completed.get(), 2);
  assert_eq!(probe.slots.borrow().len(), 2);
}

#[test]
fn geometric_hover_composes_with_moving_placement_and_releases_to_the_latest_base() {
  let app = App::new("motion/scene")
    .ui(
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .id(*WORLD.as_uuid())
          .child(world::BoxHitRegion::new().size(battlement::Vector3::new(8.0, 8.0, 0.2)))
          .animate(StyleTarget::new().local_position_x(4.0).local_offset_y(0.2))
          .while_hover(StyleTarget::new().local_offset_y(0.4))
          .transition(Transition::tween().duration_secs(1.0).ease(Easing::Linear)),
      ),
    )
    .document(|mut document| {
      document.element.picking_mode = Prop::Set(battlement::PickingMode::Ignore);
      document
    })
    .camera(|camera| {
      world::Camera::new()
        .orthographic(5.0)
        .position(battlement::Vector3::new(0.0, 0.0, -10.0))
        .into_object(camera.object_id)
    });
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  let mut display = Display::connect(app, assets);
  display.advance_time(Duration::from_millis(500));
  display.pointer_move(0, battlement::PanelPoint::new(960.0, 540.0), false);
  display.advance_time(Duration::from_millis(250));
  let pose = display.object(WORLD).unwrap().local_transform().position;
  assert!((pose.x - 3.0).abs() < 0.00001, "{pose:?}");
  assert!((pose.y - 0.175).abs() < 0.00001, "{pose:?}");
  display.pointer_move(0, battlement::PanelPoint::new(1.0, 1.0), false);
  display.advance_time(Duration::from_millis(250));
  let pose = display.object(WORLD).unwrap().local_transform().position;
  assert!((pose.x - 4.0).abs() < 0.00001, "{pose:?}");
  assert!((pose.y - 0.18125).abs() < 0.00001, "{pose:?}");
  display.reconnect();
  display.advance_time(Duration::from_millis(250));
  let pose = display.object(WORLD).unwrap().local_transform().position;
  assert!((pose.y - 0.1875).abs() < 0.00001, "{pose:?}");
  display.settle();
  let pose = display.object(WORLD).unwrap().local_transform().position;
  assert!((pose.y - 0.2).abs() < 0.00001, "{pose:?}");
  assert_eq!(display.presentation_time(), Duration::from_millis(750));
  assert_eq!(display.frame(), 0);
}

#[test]
fn a_stationary_pointer_is_repicked_as_a_target_crosses_its_screen_position() {
  let app = App::with_model("motion/scene", (0_u32, 0_u32))
    .root(|_| {
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .id(*WORLD.as_uuid())
          .child(
            world::BoxHitRegion::new()
              .size(battlement::Vector3::new(2.0, 2.0, 0.2))
              .events(
                world::PointerHandlers::new()
                  .on_pointer_enter(
                    |model: &mut (u32, u32),
                     _: reactant::event::ReactantEvent<battlement::PointerBoundaryEvent>| {
                      model.0 += 1;
                    },
                  )
                  .on_pointer_leave(
                    |model: &mut (u32, u32),
                     _: reactant::event::ReactantEvent<battlement::PointerBoundaryEvent>| {
                      model.1 += 1;
                    },
                  ),
              ),
          )
          .initial(StyleTarget::new().local_position_x(-2.0))
          .animate(StyleTarget::new().local_position_x(2.0))
          .while_hover(StyleTarget::new().local_offset_y(0.5))
          .transition(Transition::tween().duration_secs(1.0).ease(Easing::Linear)),
      )
    })
    .document(|mut document| {
      document.element.picking_mode = Prop::Set(battlement::PickingMode::Ignore);
      document
    })
    .camera(|camera| {
      world::Camera::new()
        .orthographic(5.0)
        .position(battlement::Vector3::new(0.0, 0.0, -10.0))
        .into_object(camera.object_id)
    });
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  let mut display = Display::connect(app, assets);
  let stationary = battlement::PanelPoint::new(960.0, 540.0);
  display.pointer_move(0, stationary, false);
  assert_eq!(display.with_engine(|app| *app.model()), (0, 0));
  display.advance_time(Duration::from_millis(500));
  assert_eq!(display.with_engine(|app| *app.model()), (1, 0));
  display.advance_time(Duration::from_millis(100));
  assert!(display.object(WORLD).unwrap().local_transform().position.y > 0.0);
  display.advance_time(Duration::from_millis(300));
  assert_eq!(display.with_engine(|app| *app.model()), (1, 1));
  display.settle();
  let pose = display.object(WORLD).unwrap().local_transform().position;
  assert!((pose.x - 2.0).abs() < 0.00001, "{pose:?}");
  assert!(pose.y.abs() < 0.00001, "{pose:?}");
}

struct NativeHoverScene(Rc<Cell<usize>>);

impl Component for NativeHoverScene {
  fn render(&self) -> impl Render {
    self.0.set(self.0.get() + 1);
    world::SceneRoot::new(ParentScene::PrimaryScene).child(
      world::Group::new()
        .id(*WORLD.as_uuid())
        .child(world::BoxHitRegion::new().size(battlement::Vector3::new(2.0, 2.0, 0.2)))
        .while_hover(StyleTarget::new().local_offset_y(0.5))
        .transition(Transition::tween().duration_secs(1.0).ease(Easing::Linear)),
    )
  }
}

#[test]
fn native_only_hover_does_not_evaluate_rust() {
  let renders = Rc::new(Cell::new(0));
  let app = App::new("motion/scene")
    .ui(NativeHoverScene(renders.clone()))
    .document(|mut document| {
      document.element.picking_mode = Prop::Set(battlement::PickingMode::Ignore);
      document
    })
    .camera(|camera| {
      world::Camera::new()
        .orthographic(5.0)
        .position(battlement::Vector3::new(0.0, 0.0, -10.0))
        .into_object(camera.object_id)
    });
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  let mut display = Display::connect(app, assets);
  let render_count = renders.get();
  display.pointer_move(0, battlement::PanelPoint::new(960.0, 540.0), false);
  display.advance_time(Duration::from_millis(500));
  assert!(display.object(WORLD).unwrap().local_transform().position.y > 0.0);
  assert_eq!(renders.get(), render_count);
}

#[test]
fn a_retarget_uses_the_presented_pose_and_keeps_spring_velocity() {
  let app = App::with_model("motion/scene", false).root(|reversed| {
    let transition = Transition::spring().stiffness(100.0).damping(10.0);
    let target = if *reversed { 0.0 } else { 1.0 };
    (
      reactant::host::ButtonHost::new(trox::ls("Retarget"))
        .name("retarget")
        .on_click(|reversed: &mut bool| *reversed = true),
      View::new()
        .name("opacity")
        .initial(StyleTarget::new().opacity(0.0))
        .animate(StyleTarget::new().opacity(target))
        .transition(transition.clone()),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .id(*WORLD.as_uuid())
          .initial(StyleTarget::new().local_position_x(0.0))
          .animate(StyleTarget::new().local_position_x(target))
          .transition(transition),
      ),
    )
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  let mut display = Display::connect(app, assets);
  display.advance_time(Duration::from_millis(100));
  let before = display.object(WORLD).unwrap().local_transform().position.x;
  assert!((0.3..0.4).contains(&before));
  display.click_ui(display.find_ui(root, "retarget"));
  let replaced = display.object(WORLD).unwrap().local_transform().position.x;
  assert!((before - replaced).abs() < 0.00001);
  display.advance_time(Duration::from_millis(1));
  let moving = display.object(WORLD).unwrap().local_transform().position.x;
  assert!(
    moving > before,
    "the spring lost its incoming positive velocity"
  );
  let opacity = display.find_ui(root, "opacity");
  let Prop::Set(StyleValue::Value(FloatValue(value))) = display.ui_element(opacity).style().opacity
  else {
    panic!("opacity is not presented");
  };
  assert!((moving - f64::from(value)).abs() < 0.00001);
  display.settle();
  assert!(
    display
      .object(WORLD)
      .unwrap()
      .local_transform()
      .position
      .x
      .abs()
      < 0.00001
  );
  assert_eq!(
    display.ui_element(opacity).style().opacity,
    Prop::Set(StyleValue::Value(FloatValue(0.0)))
  );
  assert_eq!(display.frame(), 0);
}

struct ClockScene(Rc<std::cell::RefCell<Option<reactant::motion_value::ControlledMotionClock>>>);
impl Component for ClockScene {
  fn render(&self) -> impl Render {
    let clock = reactant::motion_value::use_controlled_motion_clock();
    self.0.replace(Some(clock.clone()));
    MotionConfig::new(MotionScene(Rc::new(Cell::new(0))))
      .time_source(MotionTimeSource::Controlled(clock))
  }
}

#[test]
fn controlled_motion_does_not_charge_virtual_time_or_frames() {
  let clock = Rc::new(std::cell::RefCell::new(None));
  let app = App::new("motion/scene").ui(ClockScene(clock.clone()));
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  let mut display = Display::connect(app, assets);
  let opacity = display.find_ui(root, "opacity");
  display.settle();
  assert_eq!(display.presentation_time(), Duration::ZERO);
  display.advance_time(Duration::from_secs(2));
  display.advance_frame();
  assert_pair(&display, opacity, 0.0);
  clock
    .borrow()
    .as_ref()
    .unwrap()
    .advance(Duration::from_millis(125));
  display.poll();
  assert_pair(&display, opacity, 0.125);
  assert_eq!(display.presentation_time(), Duration::from_secs(2));
  assert_eq!(display.frame(), 1);
}

#[test]
fn inherited_reduced_motion_snaps_world_placement_and_preserves_opacity_timing() {
  let app = App::new("motion/scene").ui(
    MotionConfig::new(MotionScene(Rc::new(Cell::new(0)))).reduced_motion(ReducedMotion::Always),
  );
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  let mut display = Display::connect(app, assets);
  let opacity = display.find_ui(root, "opacity");
  assert_eq!(
    display.object(WORLD).unwrap().local_transform().position.x,
    1.0
  );
  display.advance_time(Duration::from_millis(250));
  assert_eq!(
    display.object(WORLD).unwrap().local_transform().position.x,
    1.0
  );
  assert_eq!(
    display.ui_element(opacity).style().opacity,
    Prop::Set(StyleValue::Value(FloatValue(0.25)))
  );
  assert_eq!(display.frame(), 0);
}

#[test]
fn a_world_exit_keeps_the_visual_until_the_sampled_motion_finishes() {
  let app = App::with_model("motion/scene", true).root(|visible| {
    (
      reactant::host::ButtonHost::new(trox::ls("Remove"))
        .name("remove")
        .on_click(|visible: &mut bool| *visible = false),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(AnimatePresence::new().child(
        visible.then(|| {
          world::Group::new()
            .id(*WORLD.as_uuid())
            .animate(StyleTarget::new().local_position_x(0.0))
            .exit(StyleTarget::new().local_position_x(1.0))
            .transition(Transition::tween().duration_secs(0.2).ease(Easing::Linear))
            .key("moving")
        }),
      )),
    )
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  let mut display = Display::connect(app, assets);
  display.click_ui(display.find_ui(root, "remove"));
  assert!(display.object(WORLD).is_some());
  display.advance_time(Duration::from_millis(100));
  assert!((display.object(WORLD).unwrap().local_transform().position.x - 0.5).abs() < 0.00001);
  display.settle();
  assert!(display.object(WORLD).is_none());
  assert_eq!(display.presentation_time(), Duration::from_millis(200));
  assert_eq!(display.frame(), 0);
}

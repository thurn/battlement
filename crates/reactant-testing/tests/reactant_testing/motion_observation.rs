use std::{
  cell::{Cell, RefCell},
  rc::Rc,
  time::Duration,
};

use battlement::{ObjectId, ParentScene, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{
  hooks::{self, StateSetter},
  motion_value,
  prelude::*,
  testing::App,
  world,
};
use reactant_testing::{Display, temporal::Clock};

const WORLD: ObjectId = object_id!("321a0000-0000-4000-8000-000000000071");

#[derive(Clone, Default)]
struct Probe {
  samples: Rc<Cell<usize>>,
  renders: Rc<Cell<usize>>,
  mounted: Rc<RefCell<Option<StateSetter<bool>>>>,
  value: Rc<RefCell<Option<TypedMotionValue<f32>>>>,
}

struct Observer {
  value: TypedMotionValue<f32>,
  event: MotionValueEvent,
  samples: Rc<Cell<usize>>,
}

struct OwnedScene(Probe);

struct StandaloneScene(MotionValueEvent);

impl Component for Observer {
  fn render(&self) -> impl Render {
    let samples = Rc::clone(&self.samples);
    motion_value::use_motion_value_event(self.value.clone(), self.event, move |_| {
      samples.set(samples.get() + 1);
    });
  }
}

impl Component for OwnedScene {
  fn render(&self) -> impl Render {
    self.0.renders.set(self.0.renders.get() + 1);
    let (mounted, set_mounted) = hooks::use_state(true);
    self.0.mounted.replace(Some(set_mounted));
    let value = motion_value::use_motion_value(0.0f32);
    self.0.value.replace(Some(value.clone()));
    let derived = motion_value::use_transform(
      value.clone(),
      InputRange::new([0.0, 1.0]),
      OutputRange::new([0.0, 0.5]),
    );
    // The observer is rendered after the two shared graph owners.
    (
      View::new().animate(StyleTarget::new().opacity_value(derived)),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .id(*WORLD.as_uuid())
          .animate(StyleTarget::new().local_position_x_value(value.clone())),
      ),
      mounted.then(|| Observer {
        value,
        event: MotionValueEvent::AnimationFrame,
        samples: Rc::clone(&self.0.samples),
      }),
    )
  }
}

impl Component for StandaloneScene {
  fn render(&self) -> impl Render {
    motion_value::use_motion_value_event(motion_value::use_time(), self.0, |_| {});
    View::new()
  }
}

fn assets() -> FakeAssetCatalog {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  assets
}

#[test]
#[should_panic(expected = "use_motion_value_event requires a mounted native Motion graph owner")]
fn standalone_frame_observation_has_an_actionable_ownership_diagnostic() {
  let _ = Display::connect(
    App::new("motion/scene").ui(StandaloneScene(MotionValueEvent::AnimationFrame)),
    self::assets(),
  );
}

#[test]
#[should_panic(expected = "use_motion_value_event requires a mounted native Motion graph owner")]
fn standalone_change_observation_has_an_actionable_ownership_diagnostic() {
  let _ = Display::connect(
    App::new("motion/scene").ui(StandaloneScene(MotionValueEvent::Change)),
    self::assets(),
  );
}

#[test]
fn shared_graph_observation_mounts_after_its_owners_and_cleans_up_without_frame_renders() {
  let probe = Probe::default();
  let mut display = Display::connect(
    App::new("motion/scene").ui(OwnedScene(probe.clone())),
    self::assets(),
  );
  let renders = probe.renders.get();
  let samples = probe.samples.get();
  Clock::advance(&mut display, Duration::from_millis(20));
  display.advance_frame();
  assert_eq!(probe.samples.get(), samples + 1);
  assert_eq!(probe.renders.get(), renders);

  probe.mounted.borrow().as_ref().unwrap().set(false);
  display.poll();
  let renders = probe.renders.get();
  let samples = probe.samples.get();
  Clock::advance(&mut display, Duration::from_millis(20));
  display.advance_frame();
  assert_eq!(probe.samples.get(), samples);
  assert_eq!(probe.renders.get(), renders);

  probe.value.borrow().as_ref().unwrap().set(0.5);
  display.poll();
  assert_eq!(
    display.object(WORLD).unwrap().local_transform().position.x,
    0.5
  );

  probe.mounted.borrow().as_ref().unwrap().set(true);
  display.poll();
  let renders = probe.renders.get();
  let samples = probe.samples.get();
  display.advance_frame();
  assert_eq!(probe.samples.get(), samples + 1);
  assert_eq!(probe.renders.get(), renders);
  display.reconnect();
  let renders = probe.renders.get();
  let samples = probe.samples.get();
  display.advance_frame();
  assert_eq!(probe.samples.get(), samples + 1);
  assert_eq!(probe.renders.get(), renders);
}

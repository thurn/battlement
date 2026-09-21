use std::{
  cell::{Cell, RefCell},
  rc::Rc,
  time::Duration,
};

use battlement::{ObjectId, ParentScene, Vector3};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{
  animation_controls::{self, AnimationScope, AnimationSequence, MotionSelector, SequenceTarget},
  hooks,
  host::ButtonHost,
  native_host::{self, ObjectRef},
  prelude::*,
  testing::App,
  world,
};
use reactant_testing::{Display, temporal::Clock};
use trox::ls;

#[derive(Clone, Default)]
struct Probe {
  active: Rc<Cell<usize>>,
  scope: Rc<RefCell<Option<AnimationScope>>>,
  target: Rc<RefCell<Option<ObjectRef>>>,
  projectile: Rc<RefCell<Option<ObjectRef>>>,
}

struct Target(Probe);

impl Component for Target {
  fn render(&self) -> impl Render {
    let reference = native_host::use_object_ref();
    self.0.target.replace(Some(reference.clone()));
    let active = self.0.active.clone();
    hooks::use_effect(
      move || {
        active.set(active.get() + 1);
        move || active.set(active.get() - 1)
      },
      (),
    );
    world::Group::new()
      .reference(reference)
      .motion(MotionProps::new().motion_name("target"))
  }
}

struct Scene {
  open: bool,
  probe: Probe,
}

impl Component for Scene {
  fn render(&self) -> impl Render {
    let scope = animation_controls::use_animation_scope();
    let projectile = native_host::use_object_ref();
    self.probe.scope.replace(Some(scope.clone()));
    self.probe.projectile.replace(Some(projectile.clone()));
    (
      ButtonHost::new(ls("Destroy retained target"))
        .name("destroy")
        .on_click(|open: &mut bool| *open = false),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .child((
            world::Group::new()
              .position(Vector3::new(-2.0, 0.0, 0.0))
              .reference(projectile)
              .motion(MotionProps::new().motion_name("projectile")),
            AnimatePresence::new().child(
              self
                .open
                .then(|| Target(self.probe.clone()).key("retained-target")),
            ),
          ))
          .motion(MotionProps::new().animation_scope(scope)),
      ),
    )
  }
}

fn fixture() -> (Display<App<bool>>, ObjectId, Probe) {
  let probe = Probe::default();
  let scene_probe = probe.clone();
  let app = App::with_model("retention/scene", true).root(move |open| Scene {
    open: *open,
    probe: scene_probe.clone(),
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("retention/scene");
  (Display::connect(app, assets), root, probe)
}

fn start_retaining_sequences(probe: &Probe) -> (AnimationPlayback, AnimationPlayback, ObjectId) {
  let scope = probe.scope.borrow().as_ref().unwrap().clone();
  let target = probe.target.borrow().as_ref().unwrap().clone();
  let projectile = probe.projectile.borrow().as_ref().unwrap().clone();
  let target_id = target.object_id().unwrap();
  let dissolve = scope.start(AnimationSequence::new().animate(
    MotionSelector::object(target.clone()),
    StyleTarget::new().local_scale_x(0.0),
    Transition::tween().duration_secs(0.4).ease(Easing::Linear),
  ));
  let projectile = scope.start(
    AnimationSequence::new().animate(
      MotionSelector::object(projectile),
      SequenceTarget::new(StyleTarget::new())
        .position(target.local_point(Vector3::new(1.0, 0.0, 0.0)).follow()),
      Transition::tween().duration_secs(0.7).ease(Easing::Linear),
    ),
  );
  (dissolve, projectile, target_id)
}

#[test]
fn final_effect_use_retains_the_original_anchor_after_logical_unmount() {
  let (mut display, root, probe) = fixture();
  let (dissolve, projectile, target_id) = start_retaining_sequences(&probe);
  display.poll();
  drop(dissolve);
  drop(projectile);

  display.click_ui(display.find_ui(root, "destroy"));
  display.poll();
  assert_eq!(probe.active.get(), 0, "logical effect survived unmount");
  assert!(!probe.target.borrow().as_ref().unwrap().is_attached());
  assert!(display.object(target_id).is_some());

  Clock::advance(&mut display, Duration::from_millis(400));
  assert!(
    display.object(target_id).is_some(),
    "first effect released anchor"
  );
  Clock::advance(&mut display, Duration::from_millis(699));
  assert!(display.object(target_id).is_some());
  Clock::advance(&mut display, Duration::from_millis(1));
  assert!(display.object(target_id).is_none());
  assert_eq!(probe.active.get(), 0);
  assert_eq!(display.frame(), 0);
}

#[test]
fn named_selector_retains_the_exact_snapshotted_target_after_logical_unmount() {
  let (mut display, root, probe) = fixture();
  let scope = probe.scope.borrow().as_ref().unwrap().clone();
  let target_id = probe.target.borrow().as_ref().unwrap().object_id().unwrap();
  let playback = scope.start(AnimationSequence::new().animate(
    MotionSelector::name("target"),
    StyleTarget::new().local_scale_x(0.0),
    Transition::tween().duration_secs(0.4).ease(Easing::Linear),
  ));
  display.poll();
  drop(playback);

  display.click_ui(display.find_ui(root, "destroy"));
  display.poll();
  assert!(display.object(target_id).is_some());
  Clock::advance(&mut display, Duration::from_millis(399));
  assert!(display.object(target_id).is_some());
  Clock::advance(&mut display, Duration::from_millis(1));
  assert!(display.object(target_id).is_none());
}

#[test]
fn stopped_retaining_sequence_releases_without_reviving_destroyed_callbacks() {
  let (mut display, root, probe) = fixture();
  let (dissolve, projectile, target_id) = start_retaining_sequences(&probe);
  display.poll();
  display.click_ui(display.find_ui(root, "destroy"));
  display.poll();
  assert!(display.object(target_id).is_some());

  dissolve.stop();
  projectile.stop();
  display.poll();
  assert!(display.object(target_id).is_none());
  assert_eq!(probe.active.get(), 0);
}

#[test]
fn public_scope_api_keeps_cosmetic_sequences_nonblocking_and_gameplay_sequences_blocking() {
  let (mut display, _root, probe) = fixture();
  let scope = probe.scope.borrow().as_ref().unwrap().clone();
  let projectile = probe.projectile.borrow().as_ref().unwrap().clone();
  let sequence = || {
    AnimationSequence::new().animate(
      MotionSelector::object(projectile.clone()),
      StyleTarget::new().local_position_y(1.0),
      Transition::tween().duration_secs(0.01),
    )
  };

  display.clear_commands();
  scope.start(sequence());
  display.poll();
  Clock::advance(&mut display, Duration::from_millis(10));
  assert_scope_command_blocking(&display, false);

  display.clear_commands();
  scope.start_blocking(sequence());
  display.poll();
  Clock::advance(&mut display, Duration::from_millis(10));
  assert_scope_command_blocking(&display, true);
}

#[test]
fn reset_reconnect_releases_playback_identity_leases() {
  let probe = Probe::default();
  let scene_probe = probe.clone();
  let app = App::with_model("retention/scene", true)
    .root(move |open| Scene {
      open: *open,
      probe: scene_probe.clone(),
    })
    .reset_on_reconnect();
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("retention/scene");
  let mut display = Display::connect(app, assets);
  let (dissolve, projectile, target_id) = start_retaining_sequences(&probe);
  display.poll();
  drop(dissolve);
  drop(projectile);
  display.click_ui(display.find_ui(root, "destroy"));
  display.poll();
  assert!(display.object(target_id).is_some());

  display.reconnect();
  display.poll();
  assert!(display.object(target_id).is_none());
  Clock::advance(&mut display, Duration::from_secs(1));
  assert!(display.object(target_id).is_none());
}

fn assert_scope_command_blocking(display: &Display<App<bool>>, expected: bool) {
  let command = display
    .commands()
    .iter()
    .find(|entry| {
      matches!(
        entry.command.body,
        battlement::CommandBody::MotionScope(battlement::MotionScopeOperation {
          command: battlement::MotionScopeCommand::Start { .. },
          ..
        })
      )
    })
    .expect("public animation scope command is absent");
  assert_eq!(command.command.blocking, expected);
}

use std::{
  cell::RefCell,
  panic::{self, AssertUnwindSafe},
  rc::Rc,
};

use battlement::{GameObjectKind, ObjectId, ParentScene, Quaternion, Vector3, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{app::App, hooks, host::ButtonHost, native_host, prelude::*, world};
use reactant_testing::Display;
use trox::ls;

const CARD: ObjectId = object_id!("38100000-0000-4000-8000-000000000001");
const HIT: ObjectId = object_id!("38100000-0000-4000-8000-000000000002");
const FACE: ObjectId = object_id!("38100000-0000-4000-8000-000000000003");

struct Scene(Rc<RefCell<Option<ObjectRef>>>);
impl Component for Scene {
  fn render(&self) -> impl Render {
    let reference = native_host::use_object_ref();
    self.0.replace(Some(reference.clone()));
    let (expanded, expand) = hooks::use_state(false);
    let (moved, move_card) = hooks::use_state(false);
    let (hidden, hide) = hooks::use_state(false);
    let (destroyed, destroy) = hooks::use_state(false);
    let (count, increment) = hooks::use_state(0);
    let required = reference.local_point(Vector3::new(1.0, 0.0, 0.0)).follow();
    let card = (!destroyed).then(|| {
      world::Group::new()
        .id(*CARD.as_uuid())
        .reference(reference)
        .active(!hidden)
        .rotation(Quaternion::new(
          0.0,
          0.0,
          2_f64.sqrt() / 2.0,
          2_f64.sqrt() / 2.0,
        ))
        .on_click(increment.update_callback(|v| v + 1))
        .child((
          world::Sprite::new()
            .id(*FACE.as_uuid())
            .texture("test/face")
            .size(if expanded { 3.0 } else { 1.0 }, 2.0),
          world::BoxHitRegion::new()
            .id(*HIT.as_uuid())
            .size(Vector3::new(if expanded { 3.0 } else { 1.0 }, 2.0, 0.2))
            .center(Vector3::new(if expanded { 0.75 } else { 0.0 }, 0.0, 0.0)),
        ))
    });
    (
      ButtonHost::new(ls("Resize"))
        .name("resize")
        .on_click(expand.update_callback(|v| !v)),
      ButtonHost::new(ls("Move"))
        .name("move")
        .on_click(move_card.update_callback(|v| !v)),
      ButtonHost::new(ls("Hide"))
        .name("hide")
        .on_click(hide.update_callback(|v| !v)),
      ButtonHost::new(ls("Destroy"))
        .name("destroy")
        .on_click(destroy.update_callback(|v| !v)),
      ButtonHost::new(ls("Dependent change"))
        .name("dependent")
        .on_click(move || {
          let _target = required.resolve();
          increment.set(99);
        }),
      Label::new(ls(count.to_string())).name("count"),
      world::SceneRoot::new(ParentScene::PrimaryScene).child((
        world::Group::new()
          .scale(Vector3::new(2.0, 3.0, 1.0))
          .child((!moved).then(|| card.clone())),
        world::Group::new()
          .position(Vector3::new(10.0, 0.0, 0.0))
          .child(moved.then_some(card)),
      )),
    )
  }
}
fn fixture() -> (Display<App>, ObjectId, ObjectRef) {
  let reference = Rc::new(RefCell::new(None));
  let app = App::new("test/scene").ui(Scene(reference.clone()));
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("test/scene");
  assets.add_texture("test/face");
  let display = Display::connect(app, assets);
  let reference = reference.borrow().as_ref().unwrap().clone();
  (display, root, reference)
}
fn click(display: &mut Display<App>, root: ObjectId, name: &str) {
  let button = display.find_ui(root, name);
  display.click_ui(button);
}
fn assert_point(actual: Vector3, expected: Vector3) {
  assert!((actual.x - expected.x).abs() < 1e-9);
  assert!((actual.y - expected.y).abs() < 1e-9);
  assert!((actual.z - expected.z).abs() < 1e-9);
}
#[test]
fn independent_geometry_updates_with_visuals_and_bubbles_to_parent() {
  let (mut display, root, _) = self::fixture();
  display.click(HIT);
  assert_eq!(
    display.ui_element(display.find_ui(root, "count")).text(),
    Some("1")
  );
  self::click(&mut display, root, "resize");
  let GameObjectKind::Image { image } = display.object(FACE).unwrap().kind() else {
    panic!("face");
  };
  assert_eq!(image.width, 3.0);
  let GameObjectKind::BoxHitRegion { region } = display.object(HIT).unwrap().kind() else {
    panic!("hit region");
  };
  assert_eq!(region.size, Vector3::new(3.0, 2.0, 0.2));
  assert_eq!(region.center, Vector3::new(0.75, 0.0, 0.0));
  display.click(HIT);
  assert_eq!(
    display.ui_element(display.find_ui(root, "count")).text(),
    Some("2")
  );
  assert_eq!(display.frame(), 0);
  assert_eq!(display.presentation_time(), std::time::Duration::ZERO);
}
#[test]
fn typed_local_points_follow_transforms_moves_and_hidden_lifetimes() {
  let (mut display, root, reference) = self::fixture();
  let point = reference.local_point(Vector3::new(1.0, 0.0, 0.0));
  let target = point.follow().resolve();
  assert_eq!(target.tracking(), world::PointTracking::FollowLive);
  assert_eq!(
    point.capture_at_start().resolve().tracking(),
    world::PointTracking::CaptureAtStart
  );
  self::assert_point(
    display.world_point(target.object_id(), target.offset()),
    Vector3::new(0.0, 3.0, 0.0),
  );
  self::click(&mut display, root, "move");
  assert_eq!(reference.object_id(), Some(target.object_id()));
  self::assert_point(
    display.world_point(target.object_id(), target.offset()),
    Vector3::new(10.0, 1.0, 0.0),
  );
  self::click(&mut display, root, "hide");
  assert_eq!(reference.object_id(), Some(target.object_id()));
  self::click(&mut display, root, "hide");
  self::click(&mut display, root, "destroy");
  assert!(!reference.is_attached());
  assert!(display.object(target.object_id()).is_none());
  self::click(&mut display, root, "destroy");
  assert_ne!(reference.object_id(), Some(target.object_id()));
  assert!(display.object(target.object_id()).is_none());
  self::click(&mut display, root, "destroy");
  let failure = panic::catch_unwind(AssertUnwindSafe(|| {
    self::click(&mut display, root, "dependent")
  }));
  assert!(failure.is_err());
  assert_eq!(
    display.ui_element(display.find_ui(root, "count")).text(),
    Some("0")
  );
}

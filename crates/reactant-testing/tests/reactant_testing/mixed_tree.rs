use std::{
  cell::{Cell, RefCell},
  rc::Rc,
};

use battlement::{ObjectId, ParentScene, Vector3};
use battlement_fake::assets::{FakeAssetCatalog, FakePrefab};
use reactant::{
  callback::Callback,
  hooks,
  host::ButtonHost,
  native_host,
  portal::{self, PortalTarget},
  prelude::*,
  testing::App,
};
use reactant_testing::Display;
use trox::ls;

#[derive(Clone, Default, PartialEq)]
struct Count(u32);

#[derive(Clone, Default)]
struct Probe {
  visual: Rc<RefCell<Option<ObjectRef>>>,
  renders: Rc<Cell<usize>>,
  mounts: Rc<Cell<usize>>,
  fail: Rc<Cell<bool>>,
}

struct Mixed {
  target: PortalTarget,
  probe: Probe,
}

struct Visual(Probe);

struct Details;
struct LateFailure(Rc<Cell<bool>>);
impl Component for LateFailure {
  fn render(&self) -> impl Render {
    if self.0.get() {
      Err(std::io::Error::other("rejected mixed render"))
    } else {
      Ok(())
    }
  }
}

impl Component for Visual {
  fn render(&self) -> impl Render {
    let count = hooks::use_context::<Count>().0;
    let reference = native_host::use_object_ref();
    self.0.visual.replace(Some(reference.clone()));
    let mounts = Rc::clone(&self.0.mounts);
    hooks::use_effect(move || mounts.set(mounts.get() + 1), ());
    Prefab::at("mixed/visual")
      .position(Vector3::new(f64::from(count), 0.0, 0.0))
      .reference(reference)
      .on_click(Callback::noop())
  }
}

impl Component for Details {
  fn render(&self) -> impl Render {
    let count = hooks::use_context::<Count>().0;
    ButtonHost::new(ls(format!("Count {count}"))).name("details")
  }
}

impl Component for Mixed {
  fn render(&self) -> impl Render {
    self.probe.renders.set(self.probe.renders.get() + 1);
    let (count, set_count) = hooks::use_state(0_u32);
    let (world, set_world) = hooks::use_state(true);
    let (ui, set_ui) = hooks::use_state(true);
    let fail = self.probe.fail.clone();
    let reject = set_count.clone();
    (
      ButtonHost::new(ls("Reject render"))
        .name("reject")
        .on_click(move || {
          fail.set(true);
          reject.set(42);
        }),
      ButtonHost::new(ls("Toggle world"))
        .name("world")
        .on_click(set_world.update_callback(|value| !value)),
      ButtonHost::new(ls("Toggle UI"))
        .name("ui")
        .on_click(set_ui.update_callback(|value| !value)),
      ContextProvider::new().context(Count(count)).child(
        SceneRoot::new(ParentScene::PrimaryScene).child(
          WorldGroup::new()
            .on_click(set_count.update_callback(|value| value + 1))
            .child((
              world.then(|| Visual(self.probe.clone())),
              ui.then(|| portal::create_portal(Details, self.target.clone())),
            )),
        ),
      ),
      View::new()
        .name("physical-target")
        .portal_target(self.target.clone()),
      LateFailure(self.probe.fail.clone()),
    )
  }
}

fn fixture() -> (Display<App>, ObjectId, Probe) {
  let mut app = App::new("mixed/scene");
  let target = app.create_portal_target();
  let probe = Probe::default();
  app = app.ui(Mixed {
    target,
    probe: probe.clone(),
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("mixed/scene");
  assets.add_prefab("mixed/visual", FakePrefab::new().with_pointer_collider());
  let mut display = Display::connect(app, assets);
  display.settle();
  (display, root, probe)
}

fn visual(probe: &Probe) -> ObjectId {
  probe.visual.borrow().as_ref().unwrap().object_id().unwrap()
}

#[test]
fn world_and_portal_share_one_counter_context_and_logical_event_path() {
  let (mut display, root, probe) = self::fixture();
  let visual = self::visual(&probe);
  let details = display.find_ui(root, "details");
  let renders = probe.renders.get();
  let frames = display.frame();
  let time = display.presentation_time();
  display.click_ui(details);
  assert_eq!(probe.renders.get(), renders + 1);
  assert_eq!(self::visual(&probe), visual);
  assert_eq!(display.find_ui(root, "details"), details);
  assert_eq!(display.ui_element(details).text(), Some("Count 1"));
  assert_eq!(
    display.object(visual).unwrap().local_transform().position.x,
    1.0
  );
  display.click(visual);
  assert_eq!(display.ui_element(details).text(), Some("Count 2"));
  assert_eq!(
    display.object(visual).unwrap().local_transform().position.x,
    2.0
  );
  display.activate(visual);
  assert_eq!(display.ui_element(details).text(), Some("Count 3"));
  assert_eq!(
    display.object(visual).unwrap().local_transform().position.x,
    3.0
  );
  assert_eq!(probe.mounts.get(), 1);
  assert_eq!(display.frame(), frames);
  assert_eq!(display.presentation_time(), time);
}

#[test]
fn removing_either_contribution_preserves_its_sibling_and_committed_ref() {
  let (mut display, root, probe) = self::fixture();
  let visual = self::visual(&probe);
  let reference = probe.visual.borrow().as_ref().unwrap().clone();
  let toggle_ui = display.find_ui(root, "ui");
  display.click_ui(toggle_ui);
  assert_eq!(self::visual(&probe), visual);
  assert!(reference.is_attached());
  display.click(visual);
  display.click_ui(toggle_ui);
  let details = display.find_ui(root, "details");
  assert_eq!(display.ui_element(details).text(), Some("Count 1"));
  let toggle_world = display.find_ui(root, "world");
  display.click_ui(toggle_world);
  assert!(!reference.is_attached());
  assert!(display.object(visual).is_none());
  assert_eq!(display.find_ui(root, "details"), details);
  display.click_ui(details);
  assert_eq!(display.ui_element(details).text(), Some("Count 2"));
  display.click_ui(toggle_world);
  assert_ne!(self::visual(&probe), visual);
  assert_eq!(
    display
      .object(self::visual(&probe))
      .unwrap()
      .local_transform()
      .position
      .x,
    2.0
  );
  assert_eq!(display.find_ui(root, "details"), details);
}

#[test]
fn failed_mixed_render_leaves_both_physical_contributions_and_refs_committed() {
  let (mut display, root, probe) = self::fixture();
  let visual = self::visual(&probe);
  let details = display.find_ui(root, "details");
  let reject = display.find_ui(root, "reject");
  assert!(
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| display.click_ui(reject))).is_err()
  );
  assert_eq!(self::visual(&probe), visual);
  assert_eq!(
    display.object(visual).unwrap().local_transform().position.x,
    0.0
  );
  assert_eq!(display.ui_element(details).text(), Some("Count 0"));
  let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(display)));
}

#[test]
fn reconnect_rebuilds_both_attachments_without_remounting_the_logical_owner() {
  let (mut display, root, probe) = self::fixture();
  let visual = self::visual(&probe);
  display.click(visual);
  display.reconnect();
  display.settle();
  assert_eq!(self::visual(&probe), visual);
  assert_eq!(
    display.object(visual).unwrap().local_transform().position.x,
    1.0
  );
  assert_eq!(
    display.ui_element(display.find_ui(root, "details")).text(),
    Some("Count 1")
  );
  assert_eq!(probe.mounts.get(), 1);
}

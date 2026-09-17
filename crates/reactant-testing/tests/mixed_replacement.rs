use std::{
  cell::{Cell, RefCell},
  rc::Rc,
};

use battlement::{ObjectId, ParentScene, Vector3};
use battlement_fake::assets::{FakeAssetCatalog, FakePrefab};
use reactant::{
  app::App,
  hooks, native_host,
  portal::{self, PortalTarget},
  prelude::*,
};
use reactant_testing::Display;
use trox::ls;

#[derive(Clone, Default)]
struct Probe {
  reference: Rc<RefCell<Option<ObjectRef>>>,
  mounts: Rc<Cell<usize>>,
}

struct Screen {
  target: PortalTarget,
  probe: Probe,
}
struct Child {
  target: PortalTarget,
  probe: Probe,
}

impl Component for Screen {
  fn render(&self) -> impl Render {
    let (alternate, set_alternate) = hooks::use_state(false);
    let (persistent, set_persistent) = hooks::use_state(false);
    (
      Button::new(ls("Replace prefab"))
        .host_name("replace")
        .on_press(set_alternate.update_callback(|value| !value)),
      Button::new(ls("Change scene"))
        .host_name("scene")
        .on_press(set_persistent.update_callback(|value| !value)),
      SceneRoot::new(if persistent {
        ParentScene::Persistent
      } else {
        ParentScene::PrimaryScene
      })
      .child(
        Prefab::at(if alternate {
          "mixed/second"
        } else {
          "mixed/first"
        })
        .child(Child {
          target: self.target.clone(),
          probe: self.probe.clone(),
        }),
      ),
      View::new().portal_target(self.target.clone()),
    )
  }
}

impl Component for Child {
  fn render(&self) -> impl Render {
    let (count, set_count) = hooks::use_state(0_u32);
    let reference = native_host::use_object_ref();
    self.probe.reference.replace(Some(reference.clone()));
    let mounts = self.probe.mounts.clone();
    hooks::use_effect(move || mounts.set(mounts.get() + 1), ());
    WorldGroup::new()
      .reference(reference)
      .position(Vector3::new(f64::from(count), 0.0, 0.0))
      .child(portal::create_portal(
        Button::new(ls(format!("Count {count}")))
          .host_name("counter")
          .on_press(set_count.update_callback(|value| value + 1)),
        self.target.clone(),
      ))
  }
}

fn object(probe: &Probe) -> ObjectId {
  probe
    .reference
    .borrow()
    .as_ref()
    .unwrap()
    .object_id()
    .unwrap()
}

#[test]
fn replacing_physical_attachments_preserves_descendant_hooks_portals_and_refs() {
  let mut app = App::new("mixed/scene");
  let target = app.create_portal_target();
  let probe = Probe::default();
  app = app.ui(Screen {
    target,
    probe: probe.clone(),
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("mixed/scene");
  assets.add_prefab("mixed/first", FakePrefab::new());
  assets.add_prefab("mixed/second", FakePrefab::new());
  let mut display = Display::connect(app, assets);
  display.settle();
  let counter = display.find_ui(root, "counter");
  display.click_ui(counter);
  let reference = probe.reference.borrow().as_ref().unwrap().clone();
  for control in ["replace", "scene"] {
    let previous = self::object(&probe);
    display.click_ui(display.find_ui(root, control));
    let current = self::object(&probe);
    assert_ne!(current, previous);
    assert!(display.object(previous).is_none());
    assert_eq!(reference.object_id(), Some(current));
    assert_eq!(
      display
        .object(current)
        .unwrap()
        .local_transform()
        .position
        .x,
      1.0
    );
    assert_eq!(display.find_ui(root, "counter"), counter);
    assert_eq!(display.ui_element(counter).text(), Some("Count 1"));
    assert_eq!(probe.mounts.get(), 1);
  }
  display.click_ui(counter);
  assert_eq!(display.ui_element(counter).text(), Some("Count 2"));
}

use std::{
  cell::RefCell,
  rc::Rc,
  sync::{Arc, Mutex},
  task::{Poll, Waker},
};

use battlement::{ParentScene, Vector3};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{
  hooks, native_host,
  portal::{self, PortalTarget},
  prelude::*,
  testing::App,
};
use reactant_testing::Display;
use trox::ls;

struct Source {
  ready: bool,
  value: u32,
  wake: Option<Waker>,
}
struct Screen {
  resource: Resource<(), u32>,
  source: Arc<Mutex<Source>>,
  target: PortalTarget,
  reference: Rc<RefCell<Option<ObjectRef>>>,
}
struct Mixed {
  value: u32,
  target: PortalTarget,
  reference: Rc<RefCell<Option<ObjectRef>>>,
}

impl Component for Screen {
  fn render(&self) -> impl Render {
    let control = use_resource_control(&self.resource);
    let source = self.source.clone();
    let target = self.target.clone();
    let reference = self.reference.clone();
    (
      Button::new(ls("Refetch"))
        .host_name("refetch")
        .on_press(move || {
          source.lock().unwrap().ready = false;
          control.invalidate(());
        }),
      Suspense::new(Label::new(ls("Loading")).name("loading")).child(
        use_resource(&self.resource, ()).then(move |value| Mixed {
          value: *value,
          target: target.clone(),
          reference: reference.clone(),
        }),
      ),
      View::new().portal_target(self.target.clone()),
    )
  }
}

impl Component for Mixed {
  fn render(&self) -> impl Render {
    let (count, set_count) = hooks::use_state(0_u32);
    let reference = native_host::use_object_ref();
    self.reference.replace(Some(reference.clone()));
    View::new().child(
      SceneRoot::new(ParentScene::Persistent).child(
        WorldGroup::new()
          .reference(reference)
          .position(Vector3::new(f64::from(self.value), 0.0, 0.0))
          .child(portal::create_portal(
            Button::new(ls(format!("Local {count}")))
              .host_name("local")
              .on_press(set_count.update_callback(|value| value + 1)),
            self.target.clone(),
          )),
      ),
    )
  }
}

#[test]
fn suspense_hides_detached_world_roots_and_restores_the_same_logical_state() {
  let source = Arc::new(Mutex::new(Source {
    ready: true,
    value: 1,
    wake: None,
  }));
  let loader = source.clone();
  let resource = Resource::new(move |()| {
    let source = loader.clone();
    std::future::poll_fn(move |context| {
      let mut source = source.lock().unwrap();
      if source.ready {
        Poll::Ready(source.value)
      } else {
        source.wake = Some(context.waker().clone());
        Poll::Pending
      }
    })
  });
  let mut app = App::new("mixed/scene");
  let target = app.create_portal_target();
  let reference = Rc::new(RefCell::new(None));
  app = app.ui(Screen {
    resource,
    source: source.clone(),
    target,
    reference: reference.clone(),
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("mixed/scene");
  let mut display = Display::connect(app, assets);
  display.poll();
  display.settle();
  let native = reference.borrow().as_ref().unwrap().object_id().unwrap();
  let local = display.find_ui(root, "local");
  display.click_ui(local);
  assert!(display.object(native).unwrap().active_in_hierarchy());
  let refetch = display.find_ui(root, "refetch");
  display.click_ui(refetch);
  display.poll();
  assert_eq!(
    display.ui_element(display.find_ui(root, "loading")).text(),
    Some("Loading")
  );
  assert!(!display.object(native).unwrap().active_in_hierarchy());
  assert!(reference.borrow().as_ref().unwrap().is_attached());
  {
    let mut source = source.lock().unwrap();
    source.ready = true;
    source.value = 2;
    source.wake.take().unwrap().wake();
  }
  display.poll();
  display.settle();
  assert_eq!(
    reference.borrow().as_ref().unwrap().object_id(),
    Some(native)
  );
  assert!(display.object(native).unwrap().active_in_hierarchy());
  assert_eq!(
    display.object(native).unwrap().local_transform().position.x,
    2.0
  );
  assert_eq!(display.find_ui(root, "local"), local);
  assert_eq!(display.ui_element(local).text(), Some("Local 1"));
}

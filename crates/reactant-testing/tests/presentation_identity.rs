use std::{
  cell::{Cell, RefCell},
  rc::Rc,
};

use battlement::ObjectId;
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{app::App, element_ref, hooks, host::ButtonHost, prelude::*};
use reactant_testing::Display;
use trox::ls;

#[derive(Clone, Default, PartialEq)]
struct Location(u32);

#[derive(Clone, Default)]
struct Probe {
  reference: Rc<RefCell<Option<ElementRef>>>,
  active: Rc<Cell<usize>>,
  events: Rc<RefCell<Vec<String>>>,
}

struct Counter(Probe);
struct Screen {
  id: ObjectId,
  probe: Probe,
}

impl Component for Counter {
  fn render(&self) -> impl Render {
    let (count, set_count) = hooks::use_state(0_u32);
    let location = hooks::use_context::<Location>().0;
    let reference = element_ref::use_element_ref();
    self.0.reference.replace(Some(reference.clone()));
    let probe = self.0.clone();
    hooks::use_effect(
      move || {
        assert_eq!(probe.active.replace(1), 0);
        probe.events.borrow_mut().push(format!("setup {location}"));
        move || {
          assert_eq!(probe.active.replace(0), 1);
          probe
            .events
            .borrow_mut()
            .push(format!("cleanup {location}"));
        }
      },
      location,
    );
    ButtonHost::new(ls(format!("Count {count} at {location}")))
      .name("counter")
      .element_ref(reference)
      .on_click(set_count.update_callback(|value| value + 1))
  }
}

impl Component for Screen {
  fn render(&self) -> impl Render {
    let (moved, set_moved) = hooks::use_state(false);
    (
      ButtonHost::new(ls("Move"))
        .name("move")
        .on_click(set_moved.update_callback(|value| !value)),
      (!moved).then(|| {
        View::new().name("old-parent").child(
          ContextProvider::new()
            .context(Location(1))
            .child(Counter(self.probe.clone()).id(*self.id.as_uuid())),
        )
      }),
      View::new().name("new-parent").child(
        ContextProvider::new()
          .context(Location(2))
          .child(moved.then(|| Counter(self.probe.clone()).id(*self.id.as_uuid()))),
      ),
    )
  }
}

#[test]
fn moving_out_of_a_deleted_parent_preserves_counter_ref_and_one_subscription() {
  let probe = Probe::default();
  let app = App::new("identity/scene").ui(Screen {
    id: ObjectId::new_v4(),
    probe: probe.clone(),
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("identity/scene");
  let mut display = Display::connect(app, assets);
  display.settle();
  display.poll();
  let counter = display.find_ui(root, "counter");
  let reference = probe.reference.borrow().as_ref().unwrap().clone();
  display.click_ui(counter);
  assert_eq!(display.ui_element(counter).text(), Some("Count 1 at 1"));
  display.click_ui(display.find_ui(root, "move"));
  assert_eq!(display.find_ui(root, "counter"), counter);
  assert!(reference.is_attached());
  assert_eq!(display.ui_element(counter).text(), Some("Count 1 at 2"));
  display.poll();
  assert_eq!(*probe.events.borrow(), ["setup 1", "cleanup 1", "setup 2"]);
  assert_eq!(probe.active.get(), 1);
  display.click_ui(counter);
  assert_eq!(display.ui_element(counter).text(), Some("Count 2 at 2"));
}

fn location_parent(location: u32, probe: Probe, children: impl Render) -> impl Render {
  let capture = Rc::clone(&probe.events);
  let bubble = Rc::clone(&probe.events);
  ContextProvider::new().context(Location(location)).child(
    View::new()
      .on_click_capture(move || capture.borrow_mut().push(format!("capture {location}")))
      .on_click(move || bubble.borrow_mut().push(format!("bubble {location}")))
      .child(children),
  )
}

#[test]
fn identified_counter_moves_across_roots_and_a_portal_with_new_logical_ancestry() {
  let id = ObjectId::new_v4();
  let probe = Probe::default();
  let mut app = App::with_model("identity/scene", 0_u32);
  let target = app.create_portal_target();
  let first_probe = probe.clone();
  let portal_target = target.clone();
  app = app.root(move |location| {
    (
      ButtonHost::new(ls("Move"))
        .name("move")
        .on_click(|location: &mut u32| *location = (*location + 1) % 3),
      (*location == 0).then(|| {
        self::location_parent(
          1,
          first_probe.clone(),
          Counter(first_probe.clone()).id(*id.as_uuid()),
        )
      }),
      (*location == 2).then(|| {
        self::location_parent(
          3,
          first_probe.clone(),
          reactant::portal::create_portal(
            Counter(first_probe.clone()).id(*id.as_uuid()),
            portal_target.clone(),
          ),
        )
      }),
    )
  });
  let first_root = app.root_document().root_id;
  let document = battlement::UiDocument::new(ObjectId::new_v4());
  let second_root = document.root_id;
  let second_probe = probe.clone();
  app = app.additional_root(document, move |location| {
    (
      (*location == 1).then(|| {
        self::location_parent(
          2,
          second_probe.clone(),
          Counter(second_probe.clone()).id(*id.as_uuid()),
        )
      }),
      View::new()
        .name("portal-destination")
        .portal_target(target.clone()),
    )
  });
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("identity/scene");
  let mut display = Display::connect(app, assets);
  display.settle();
  display.poll();
  let counter = display.find_ui(first_root, "counter");
  let reference = probe.reference.borrow().as_ref().unwrap().clone();
  let time = display.presentation_time();
  let frame = display.frame();
  for (step, root, location) in [
    (0, first_root, 1),
    (1, second_root, 2),
    (2, second_root, 3),
    (3, first_root, 1),
  ] {
    if step > 0 {
      display.click_ui(display.find_ui(first_root, "move"));
      display.poll();
    }
    let observed = display.with_engine(|app| app.presentation(*id.as_uuid()).unwrap());
    assert_eq!(observed.native_objects, [counter]);
    assert_eq!(
      observed.logical_root,
      if location == 2 {
        second_root
      } else {
        first_root
      }
    );
    assert_eq!(display.find_ui(root, "counter"), counter);
    assert_eq!(display.ui_element(counter).document_root_id(), root);
    assert!(reference.is_attached());
    assert_eq!(
      probe.active.get(),
      1,
      "step {step}, events {:?}",
      probe.events.borrow()
    );
    probe.events.borrow_mut().clear();
    display.click_ui(counter);
    display.poll();
    assert_eq!(
      display.ui_element(counter).text(),
      Some(format!("Count {} at {location}", step + 1).as_str())
    );
    assert_eq!(
      *probe.events.borrow(),
      [format!("capture {location}"), format!("bubble {location}")]
    );
  }
  assert_eq!(display.presentation_time(), time);
  assert_eq!(display.frame(), frame);
}

struct DuplicateWorld {
  id: ObjectId,
}

impl Component for DuplicateWorld {
  fn render(&self) -> impl Render {
    let (duplicate, set_duplicate) = hooks::use_state(false);
    (
      ButtonHost::new(ls("Duplicate"))
        .name("duplicate")
        .on_click(set_duplicate.update_callback(|_| true)),
      SceneRoot::new(battlement::ParentScene::PrimaryScene)
        .child(duplicate.then(|| WorldGroup::new().id(*self.id.as_uuid()))),
    )
  }
}

struct RetainedIdentity {
  id: ObjectId,
  probe: Probe,
  renders: Rc<Cell<usize>>,
}
impl Component for RetainedIdentity {
  fn render(&self) -> impl Render {
    self.renders.set(self.renders.get() + 1);
    Counter(self.probe.clone()).id(*self.id.as_uuid())
  }
}

#[test]
fn duplicate_in_a_sparse_world_update_rejects_against_an_unevaluated_ui_root() {
  let id = ObjectId::new_v4();
  let probe = Probe::default();
  let renders = Rc::new(Cell::new(0));
  let document = battlement::UiDocument::new(ObjectId::new_v4());
  let second = document.root_id;
  let updater_id = ObjectId::new_v4();
  let app = App::new("identity/scene")
    .ui(RetainedIdentity {
      id,
      probe: probe.clone(),
      renders: Rc::clone(&renders),
    })
    .additional_root(document, move |_| {
      DuplicateWorld { id }.id(*updater_id.as_uuid())
    });
  let first = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("identity/scene");
  let mut display = Display::connect(app, assets);
  display.settle();
  display.poll();
  let counter = display.find_ui(first, "counter");
  display.click_ui(counter);
  display.poll();
  assert_eq!(renders.get(), 1, "a counter update stays local");
  let duplicate = display.find_ui(second, "duplicate");
  assert!(
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| display.click_ui(duplicate))).is_err()
  );
  assert_eq!(
    renders.get(),
    1,
    "UUID validation includes retained roots without reevaluation"
  );
  assert_eq!(display.ui_element(counter).text(), Some("Count 1 at 0"));
  assert_eq!(
    display.with_engine(|app| app.presentation(*id.as_uuid()).unwrap().native_objects),
    [counter]
  );
  assert_eq!(probe.active.get(), 1);
  let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(display)));
  assert_eq!(
    probe.active.get(),
    0,
    "a rejected update retains ownership for cleanup"
  );
}

#[test]
fn reconnect_reset_starts_fresh_identified_hooks_and_handles() {
  let id = ObjectId::new_v4();
  let probe = Probe::default();
  let app = App::new("identity/scene")
    .ui(Counter(probe.clone()).id(*id.as_uuid()))
    .reset_on_reconnect();
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("identity/scene");
  let mut display = Display::connect(app, assets);
  display.poll();
  let old = display.find_ui(root, "counter");
  let old_ref = probe.reference.borrow().as_ref().unwrap().clone();
  display.click_ui(old);
  display.reconnect();
  display.poll();
  let new = display.find_ui(root, "counter");
  assert_ne!(old, new);
  assert!(!old_ref.is_attached());
  assert_eq!(display.ui_element(new).text(), Some("Count 0 at 0"));
  assert_eq!(probe.active.get(), 1);
}

struct DifferentCounter(Probe);
impl Component for DifferentCounter {
  fn render(&self) -> impl Render {
    let (count, set_count) = hooks::use_state(90_u32);
    let reference = element_ref::use_element_ref();
    self.0.reference.replace(Some(reference.clone()));
    ButtonHost::new(ls(format!("Different {count}")))
      .name("counter")
      .element_ref(reference)
      .on_click(set_count.update_callback(|value| value + 1))
  }
}

#[test]
fn a_different_component_type_with_the_same_uuid_starts_fresh_hooks() {
  let id = ObjectId::new_v4();
  let probe = Probe::default();
  let captured = probe.clone();
  let app = App::with_model("identity/scene", false).root(move |changed| {
    (
      ButtonHost::new(ls("Replace"))
        .name("replace")
        .on_click(|changed: &mut bool| *changed = true),
      if *changed {
        Node::new(DifferentCounter(captured.clone()).id(*id.as_uuid()))
      } else {
        Node::new(Counter(captured.clone()).id(*id.as_uuid()))
      },
    )
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("identity/scene");
  let mut display = Display::connect(app, assets);
  display.poll();
  let old = display.find_ui(root, "counter");
  let old_ref = probe.reference.borrow().as_ref().unwrap().clone();
  display.click_ui(old);
  display.click_ui(display.find_ui(root, "replace"));
  display.poll();
  let new = display.find_ui(root, "counter");
  assert_ne!(old, new);
  assert!(!old_ref.is_attached());
  assert_eq!(display.ui_element(new).text(), Some("Different 90"));
  assert_eq!(probe.active.get(), 0);
}

#[test]
fn equal_keys_under_distinct_parents_remain_independent() {
  let left = Probe::default();
  let right = Probe::default();
  let app = App::new("identity/scene").ui((
    View::new().name("left").child(Counter(left).key("counter")),
    View::new()
      .name("right")
      .child(Counter(right).key("counter")),
  ));
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("identity/scene");
  let mut display = Display::connect(app, assets);
  let left = display.find_ui(display.find_ui(root, "left"), "counter");
  let right = display.find_ui(display.find_ui(root, "right"), "counter");
  display.click_ui(left);
  assert_eq!(display.ui_element(left).text(), Some("Count 1 at 0"));
  assert_eq!(display.ui_element(right).text(), Some("Count 0 at 0"));
}

use std::{
  collections::HashMap,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
  },
};

use battlement_fake::assets::FakeAssetCatalog;
use reactant::{app::App, prelude::*};
use reactant_testing::Display;
use trox::ls;

#[derive(Clone, Default)]
struct Source(Arc<SourceState>);
#[derive(Default)]
struct SourceState {
  value: AtomicUsize,
  next: AtomicUsize,
  listeners: Mutex<HashMap<usize, StoreNotify>>,
}
impl PartialEq for Source {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}
impl ExternalStore for Source {
  type Snapshot = usize;
  fn snapshot(&self) -> usize {
    self.0.value.load(Ordering::Acquire)
  }
  fn subscribe(&self, notify: StoreNotify) -> Subscription {
    let id = self.0.next.fetch_add(1, Ordering::Relaxed);
    self.0.listeners.lock().unwrap().insert(id, notify);
    let state = self.0.clone();
    Subscription::new(move || {
      state.listeners.lock().unwrap().remove(&id);
    })
  }
}
impl Source {
  fn set(&self, value: usize) {
    self.0.value.store(value, Ordering::Release);
    for notify in self.0.listeners.lock().unwrap().values() {
      notify.notify();
    }
  }
}

struct Reader {
  source: Source,
  name: &'static str,
  write: Arc<AtomicBool>,
  reads: Arc<Mutex<Vec<(&'static str, usize)>>>,
}
impl Component for Reader {
  fn render(&self) -> impl Render {
    let value = use_external_store(self.source.clone());
    self.reads.lock().unwrap().push((self.name, value));
    if self.write.swap(false, Ordering::AcqRel) {
      self.source.set(value + 1);
    }
    Label::new(ls(value.to_string())).name(self.name)
  }
}

#[test]
fn render_time_writes_are_observed_only_by_a_later_complete_render() {
  let source = Source::default();
  let write = Arc::new(AtomicBool::new(false));
  let reads = Arc::new(Mutex::new(Vec::new()));
  let events = Arc::new(AtomicUsize::new(0));
  let callback_events = events.clone();
  let callback_source = source.clone();
  let callback_write = write.clone();
  let app = App::new("stores/scene").ui((
    reactant::host::ButtonHost::new(ls("Change"))
      .name("change")
      .on_click(move || {
        callback_events.fetch_add(1, Ordering::Relaxed);
        callback_write.store(true, Ordering::Release);
        callback_source.set(1);
      }),
    Reader {
      source: source.clone(),
      name: "left",
      write: write.clone(),
      reads: reads.clone(),
    },
    Reader {
      source: source.clone(),
      name: "right",
      write: Arc::new(AtomicBool::new(false)),
      reads: reads.clone(),
    },
  ));
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("stores/scene");
  let mut display = Display::connect(app, assets);
  reads.lock().unwrap().clear();
  display.click_ui(display.find_ui(root, "change"));
  assert_eq!(&reads.lock().unwrap()[..2], &[("left", 1), ("right", 1)]);
  display.poll();
  for name in ["left", "right"] {
    assert_eq!(
      display.ui_element(display.find_ui(root, name)).text(),
      Some("2")
    );
  }
  assert_eq!(events.load(Ordering::Relaxed), 1);
  assert_eq!(display.frame(), 0);
}

#[derive(Clone)]
struct Selection(u32);
struct SelectedReader(DisplayStore<(u32, u32)>, Arc<AtomicUsize>);
impl PartialEq for SelectedReader {
  fn eq(&self, other: &Self) -> bool {
    self.0 == other.0 && Arc::ptr_eq(&self.1, &other.1)
  }
}
impl Component for SelectedReader {
  fn render(&self) -> impl Render {
    self.1.fetch_add(1, Ordering::Relaxed);
    let value = use_external_store_selector_with(
      self.0.clone(),
      |state| Selection(state.0),
      |a, b| a.0 == b.0,
    );
    Label::new(ls(value.0.to_string())).name("selected")
  }
}

struct OtherField(DisplayStore<(u32, u32)>);
impl Component for OtherField {
  fn render(&self) -> impl Render {
    let value = use_external_store(self.0.clone());
    Label::new(ls(value.1.to_string()))
  }
}

#[test]
fn unrelated_store_fields_skip_subscriber_evaluation_with_explicit_equality() {
  let store = DisplayStore::new((1, 0));
  let renders = Arc::new(AtomicUsize::new(0));
  let app = App::new("stores/scene").ui((
    reactant::component::memo(SelectedReader(store.clone(), renders.clone())),
    OtherField(store.clone()),
  ));
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("stores/scene");
  let mut display = Display::connect(app, assets);
  let first = renders.load(Ordering::Relaxed);
  store.update(|state| state.1 += 1);
  display.poll();
  assert_eq!(renders.load(Ordering::Relaxed), first);
  store.update(|state| state.0 = 2);
  display.poll();
  assert_eq!(renders.load(Ordering::Relaxed), first + 1);
  assert_eq!(
    display.ui_element(display.find_ui(root, "selected")).text(),
    Some("2")
  );
}

struct Consumer(Arc<AtomicUsize>);
impl Component for Consumer {
  fn render(&self) -> impl Render {
    self.0.fetch_add(1, Ordering::Relaxed);
    let source = use_required_context::<Source>();
    let value = use_external_store_selector(source, |value| *value);
    let (local, setter) = use_state(0);
    reactant::host::ButtonHost::new(ls(format!("{value} / local {local}")))
      .name("consumer")
      .on_click(setter.update_callback(|count| count + 1))
  }
}
struct Moving {
  first: Source,
  second: Source,
  renders: Arc<AtomicUsize>,
  id: battlement::ObjectId,
}
impl Component for Moving {
  fn render(&self) -> impl Render {
    let (moved, set_moved) = use_state(false);
    (
      reactant::host::ButtonHost::new(ls("Move"))
        .name("move")
        .on_click(set_moved.update_callback(|value| !value)),
      ContextProvider::new()
        .context(self.first.clone())
        .child((!moved).then(|| Consumer(self.renders.clone()).id(*self.id.as_uuid()))),
      ContextProvider::new()
        .context(self.second.clone())
        .child(moved.then(|| Consumer(self.renders.clone()).id(*self.id.as_uuid()))),
    )
  }
}

#[test]
fn moving_a_consumer_keeps_hooks_and_replaces_its_subscription_once() {
  let first = Source::default();
  first.set(1);
  let second = Source::default();
  second.set(9);
  let renders = Arc::new(AtomicUsize::new(0));
  let app = App::new("stores/scene").ui(Moving {
    first: first.clone(),
    second: second.clone(),
    renders: renders.clone(),
    id: battlement::ObjectId::new_v4(),
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("stores/scene");
  let mut display = Display::connect(app, assets);
  let consumer = display.find_ui(root, "consumer");
  display.click_ui(consumer);
  let before = renders.load(Ordering::Relaxed);
  display.click_ui(display.find_ui(root, "move"));
  assert_eq!(display.find_ui(root, "consumer"), consumer);
  assert_eq!(display.ui_element(consumer).text(), Some("9 / local 1"));
  assert_eq!(renders.load(Ordering::Relaxed), before + 1);
  assert!(first.0.listeners.lock().unwrap().is_empty());
  assert_eq!(second.0.listeners.lock().unwrap().len(), 1);
  assert_eq!(second.0.next.load(Ordering::Relaxed), 1);
  first.set(2);
  display.poll();
  assert_eq!(renders.load(Ordering::Relaxed), before + 1);
  second.set(10);
  display.poll();
  assert_eq!(display.ui_element(consumer).text(), Some("10 / local 1"));
  assert_eq!(renders.load(Ordering::Relaxed), before + 2);
  drop(display);
  assert!(second.0.listeners.lock().unwrap().is_empty());
}

struct Rejected;
impl Component for Rejected {
  fn render(&self) -> impl Render {
    Err::<(), _>(std::io::Error::other("reject proposed store consumer"))
  }
}

#[test]
fn abandoned_render_never_installs_a_new_subscription() {
  let source = Source::default();
  let proposed = source.clone();
  let app = App::with_model("stores/scene", false).root(move |show| {
    (
      reactant::host::ButtonHost::new(ls("Propose"))
        .name("propose")
        .on_click(|show: &mut bool| *show = true),
      show.then(|| {
        (
          Reader {
            source: proposed.clone(),
            name: "proposed",
            write: Arc::default(),
            reads: Arc::default(),
          },
          Rejected,
        )
      }),
    )
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("stores/scene");
  let mut display = Display::connect(app, assets);
  let button = display.find_ui(root, "propose");
  assert!(
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| display.click_ui(button))).is_err()
  );
  assert_eq!(source.0.next.load(Ordering::Relaxed), 0);
  assert!(source.0.listeners.lock().unwrap().is_empty());
  let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(display)));
}

struct NestedDisplay(Source);
impl Component for NestedDisplay {
  fn render(&self) -> impl Render {
    let outer = use_external_store(self.0.clone());
    self.0.set(outer + 1);
    let app = App::new("nested/scene").ui(Reader {
      source: self.0.clone(),
      name: "nested",
      write: Arc::default(),
      reads: Arc::default(),
    });
    let root = app.root_document().root_id;
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene("nested/scene");
    let display = Display::connect(app, assets);
    assert_eq!(
      display.ui_element(display.find_ui(root, "nested")).text(),
      Some((outer + 1).to_string().as_str())
    );
    let still_outer = use_external_store(self.0.clone());
    assert_eq!(still_outer, outer);
    Label::new(ls(outer.to_string()))
  }
}

#[test]
fn nested_displays_capture_their_own_render_version_and_restore_the_outer_version() {
  let app = App::new("stores/scene").ui(NestedDisplay(Source::default()));
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("stores/scene");
  let _display = Display::connect(app, assets);
}

#[test]
fn a_semantic_store_write_updates_a_memoized_consumer_in_that_response() {
  let store = DisplayStore::new((0, 0));
  let renders = Arc::new(AtomicUsize::new(0));
  let callback_store = store.clone();
  let app = App::new("stores/scene").ui((
    reactant::host::ButtonHost::new(ls("Write"))
      .name("write")
      .on_click(move || callback_store.update(|value| value.0 += 1)),
    reactant::component::memo(SelectedReader(store, renders.clone())),
  ));
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("stores/scene");
  let mut display = Display::connect(app, assets);
  let before = renders.load(Ordering::Relaxed);
  display.click_ui(display.find_ui(root, "write"));
  assert_eq!(
    display.ui_element(display.find_ui(root, "selected")).text(),
    Some("1")
  );
  assert_eq!(renders.load(Ordering::Relaxed), before + 1);
  assert_eq!(display.frame(), 0);
}

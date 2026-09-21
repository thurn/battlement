use std::{
  cell::{Cell, RefCell},
  collections::BTreeMap,
  path::{Path, PathBuf},
  rc::Rc,
  time::Duration,
};

use battlement::{Connect, DragMode, ObjectId, ParentScene, ScreenSize, Vector3, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{Application, host::ButtonHost, prelude::*, world::BoxHitRegion};
use reactant_testing::Display;
use trox::ls;

const ROOT: ObjectId = object_id!("7d75c092-7143-49cc-adbd-8b9090194a02");
const DRAG_HOST: ObjectId = object_id!("870d1a2b-df7c-4331-a674-ecbf3af5027f");

fn catalog() -> FakeAssetCatalog {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("application-services/scene");
  assets
}

fn application(component: impl Render) -> Application {
  Application::new("application-services/scene")
    .child(component)
    .document(|mut document| {
      document.root_id = ROOT;
      document
    })
}

struct RescheduledTimeout;

impl Component for RescheduledTimeout {
  fn render(&self) -> impl Render {
    let (long, set_long) = reactant::hooks::use_state(false);
    let (fired, set_fired) = reactant::hooks::use_state(false);
    reactant::use_timeout(
      if long {
        Duration::from_secs(10)
      } else {
        Duration::from_secs(1)
      },
      move || set_fired.set(true),
    );
    (
      ButtonHost::new(ls("Reschedule"))
        .name("reschedule")
        .on_click(move || set_long.set(true)),
      View::new().name(if fired { "fired" } else { "waiting" }),
    )
  }
}

#[test]
fn changing_a_timeout_reschedules_its_deadline() {
  let mut display = Display::mount(|| application(RescheduledTimeout), catalog());
  display.poll();
  let button = display.find_ui(ROOT, "reschedule");
  display.click_ui(button);
  display.poll();
  display.advance_time(Duration::from_secs(1));
  display.poll();
  let _ = display.find_ui(ROOT, "waiting");
  display.advance_time(Duration::from_secs(9));
  display.poll();
  let _ = display.find_ui(ROOT, "fired");
}

struct MissedInterval;

impl Component for MissedInterval {
  fn render(&self) -> impl Render {
    let (count, set_count) = reactant::hooks::use_state(0_u32);
    reactant::use_interval(Duration::from_secs(10), move || {
      set_count.update(|count| count + 1);
    });
    View::new().name(format!("count-{count}"))
  }
}

#[test]
fn a_missed_interval_runs_at_most_once_per_poll() {
  let mut display = Display::mount(|| application(MissedInterval), catalog());
  display.poll();
  display.advance_time(Duration::from_secs(35));
  display.poll();
  let _ = display.find_ui(ROOT, "count-1");
  display.advance_time(Duration::from_secs(10));
  display.poll();
  let _ = display.find_ui(ROOT, "count-2");
}

struct ConditionalDrag;

impl Component for ConditionalDrag {
  fn render(&self) -> impl Render {
    let (enabled, set_enabled) = reactant::hooks::use_state(false);
    let mut host = BoxHitRegion::new()
      .id(*DRAG_HOST.as_uuid())
      .size(Vector3::ONE)
      .draggable(DragMode::SnapToPointer);
    if enabled {
      host = host.on_drag_start(|| {});
    }
    (
      ButtonHost::new(ls("Enable drag callback"))
        .name("enable-drag-callback")
        .on_click(move || set_enabled.set(true)),
      SceneRoot::new(ParentScene::PrimaryScene).child(host),
    )
  }
}

#[test]
fn adding_a_drag_callback_does_not_change_the_component_hook_count() {
  let mut display = Display::mount(|| application(ConditionalDrag), catalog());
  let button = display.find_ui(ROOT, "enable-drag-callback");
  display.click_ui(button);
  assert_eq!(
    display.object(DRAG_HOST).unwrap().drag_mode(),
    Some(DragMode::SnapToPointer)
  );
}

struct ChangedPersistenceFile;

impl Component for ChangedPersistenceFile {
  fn render(&self) -> impl Render {
    let (changed, set_changed) = reactant::hooks::use_state(false);
    let _state =
      reactant::use_persistent_state::<u32>(if changed { "second.json" } else { "first.json" });
    ButtonHost::new(ls("Change persistence file"))
      .name("change-persistence-file")
      .on_click(move || set_changed.set(true))
  }
}

#[test]
#[should_panic(expected = "persistent state file name cannot change while mounted")]
fn persistence_file_names_are_mount_stable() {
  let mut display = Display::mount(|| application(ChangedPersistenceFile), catalog());
  let button = display.find_ui(ROOT, "change-persistence-file");
  display.click_ui(button);
}

#[derive(Default)]
struct MemoryPersistence {
  values: RefCell<BTreeMap<PathBuf, Vec<u8>>>,
  fail: Cell<bool>,
}

impl reactant::PersistenceBackend for MemoryPersistence {
  fn load(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
    if self.fail.get() {
      return Err("injected failure".to_owned());
    }
    Ok(self.values.borrow().get(path).cloned())
  }

  fn store(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
    if self.fail.get() {
      return Err("injected failure".to_owned());
    }
    self
      .values
      .borrow_mut()
      .insert(path.to_owned(), bytes.to_vec());
    Ok(())
  }

  fn remove(&self, path: &Path) -> Result<(), String> {
    if self.fail.get() {
      return Err("injected failure".to_owned());
    }
    self.values.borrow_mut().remove(path);
    Ok(())
  }
}

struct PersistenceControls {
  backend: Rc<MemoryPersistence>,
}

impl Component for PersistenceControls {
  fn render(&self) -> impl Render {
    let state = reactant::use_persistent_state_with::<u32>("value.json", self.backend.clone());
    let update = state.clone();
    let clear = state.clone();
    (
      View::new().name(match (state.value(), state.error()) {
        (Some(value), _) => format!("value-{value}"),
        (None, Some(_)) => "error".to_owned(),
        (None, None) => "absent".to_owned(),
      }),
      ButtonHost::new(ls("Store"))
        .name("store")
        .on_click(move || update.update(7)),
      ButtonHost::new(ls("Remove"))
        .name("remove")
        .on_click(move || clear.clear()),
    )
  }
}

#[test]
fn injected_persistence_reports_absence_updates_removes_and_errors() {
  let backend = Rc::new(MemoryPersistence::default());
  let component_backend = backend.clone();
  let mut display = Display::mount_with(
    move || {
      application(PersistenceControls {
        backend: component_backend.clone(),
      })
    },
    catalog(),
    Connect::new("test", "test", ScreenSize::new(1_920, 1_080)).persistent_data_path("memory"),
  );
  let _ = display.find_ui(ROOT, "absent");
  let store = display.find_ui(ROOT, "store");
  display.click_ui(store);
  let _ = display.find_ui(ROOT, "value-7");
  let remove = display.find_ui(ROOT, "remove");
  display.click_ui(remove);
  let _ = display.find_ui(ROOT, "absent");
  backend.fail.set(true);
  display.click_ui(store);
  let _ = display.find_ui(ROOT, "error");
}

struct FilesystemPersistence;

impl Component for FilesystemPersistence {
  fn render(&self) -> impl Render {
    let state = reactant::use_persistent_state::<u32>("value.json");
    let update = state.clone();
    (
      View::new().name(match state.value() {
        Some(value) => format!("value-{value}"),
        None => "absent".to_owned(),
      }),
      ButtonHost::new(ls("Store"))
        .name("store")
        .on_click(move || update.update(11)),
    )
  }
}

#[test]
fn default_persistence_backend_remains_filesystem_backed() {
  let directory = std::env::temp_dir().join(format!("reactant-persistence-{}", ObjectId::new_v4()));
  let connect = Connect::new("test", "test", ScreenSize::new(1_920, 1_080))
    .persistent_data_path(directory.to_string_lossy());
  let mut first = Display::mount_with(
    || application(FilesystemPersistence),
    catalog(),
    connect.clone(),
  );
  let store = first.find_ui(ROOT, "store");
  first.click_ui(store);
  let _ = first.find_ui(ROOT, "value-11");
  drop(first);

  let restored = Display::mount_with(|| application(FilesystemPersistence), catalog(), connect);
  let _ = restored.find_ui(ROOT, "value-11");
  std::fs::remove_dir_all(directory).expect("temporary persistence cleanup");
}

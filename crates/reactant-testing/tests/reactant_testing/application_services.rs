use std::{
  cell::RefCell,
  collections::BTreeMap,
  path::{Path, PathBuf},
  rc::Rc,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
  },
  time::Duration,
};

use battlement::{Connect, DragMode, ObjectId, ParentScene, ScreenSize, Vector3, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{Application, host::ButtonHost, prelude::*, world::BoxHitRegion};
use reactant_testing::{Display, temporal::Clock};
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
  Clock::advance(&mut display, Duration::from_secs(1));
  display.poll();
  let _ = display.find_ui(ROOT, "waiting");
  Clock::advance(&mut display, Duration::from_secs(9));
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
  Clock::advance(&mut display, Duration::from_secs(35));
  display.poll();
  let _ = display.find_ui(ROOT, "count-1");
  Clock::advance(&mut display, Duration::from_secs(10));
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
  values: Mutex<BTreeMap<PathBuf, Vec<u8>>>,
  fail: AtomicBool,
}

impl reactant::PersistenceBackend for MemoryPersistence {
  fn start(self: Arc<Self>, request: PersistenceRequest, complete: PersistenceCompletion) {
    complete(request.id, request.execute(self.as_ref()));
  }
  fn load(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
    if self.fail.load(Ordering::SeqCst) {
      return Err("injected failure".to_owned());
    }
    Ok(self.values.lock().unwrap().get(path).cloned())
  }

  fn store(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
    if self.fail.load(Ordering::SeqCst) {
      return Err("injected failure".to_owned());
    }
    self
      .values
      .lock()
      .unwrap()
      .insert(path.to_owned(), bytes.to_vec());
    Ok(())
  }

  fn remove(&self, path: &Path) -> Result<(), String> {
    if self.fail.load(Ordering::SeqCst) {
      return Err("injected failure".to_owned());
    }
    self.values.lock().unwrap().remove(path);
    Ok(())
  }
}

struct PersistenceControls {
  backend: Arc<MemoryPersistence>,
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
  let backend = Arc::new(MemoryPersistence::default());
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
  backend.fail.store(true, Ordering::SeqCst);
  display.click_ui(store);
  let _ = display.find_ui(ROOT, "error");
}

struct FilesystemPersistence(Rc<RefCell<Option<PersistentState<u32>>>>);

impl Component for FilesystemPersistence {
  fn render(&self) -> impl Render {
    let state = reactant::use_persistent_state::<u32>("value.json");
    *self.0.borrow_mut() = Some(state.clone());
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
  let captured = Rc::new(RefCell::new(None::<PersistentState<u32>>));
  let capture = captured.clone();
  let mut first = Display::mount_with(
    move || application(FilesystemPersistence(capture.clone())),
    catalog(),
    connect.clone(),
  );
  first.flush();
  assert!(
    captured
      .borrow()
      .as_ref()
      .unwrap()
      .wait_for_idle(Duration::from_secs(10))
  );
  first.flush();
  let store = first.find_ui(ROOT, "store");
  first.click_ui(store);
  assert!(
    captured
      .borrow()
      .as_ref()
      .unwrap()
      .wait_for_idle(Duration::from_secs(10))
  );
  first.flush();
  let _ = first.find_ui(ROOT, "value-11");
  drop(first);

  let capture = captured.clone();
  let mut restored = Display::mount_with(
    move || application(FilesystemPersistence(capture.clone())),
    catalog(),
    connect,
  );
  restored.flush();
  assert!(
    captured
      .borrow()
      .as_ref()
      .unwrap()
      .wait_for_idle(Duration::from_secs(10))
  );
  restored.flush();
  let _ = restored.find_ui(ROOT, "value-11");
  std::fs::remove_dir_all(directory).expect("temporary persistence cleanup");
}

#[derive(Default)]
struct DeferredPersistence {
  calls: Mutex<Vec<(PersistenceRequest, PersistenceCompletion)>>,
}

impl PersistenceBackend for DeferredPersistence {
  fn load(&self, _: &Path) -> Result<Option<Vec<u8>>, String> {
    unreachable!()
  }
  fn store(&self, _: &Path, _: &[u8]) -> Result<(), String> {
    unreachable!()
  }
  fn remove(&self, _: &Path) -> Result<(), String> {
    unreachable!()
  }
  fn start(self: Arc<Self>, request: PersistenceRequest, complete: PersistenceCompletion) {
    self.calls.lock().unwrap().push((request, complete));
  }
}

struct DeferredControls {
  backend: Arc<DeferredPersistence>,
}

impl Component for DeferredControls {
  fn render(&self) -> impl Render {
    let state = reactant::use_persistent_state_with::<u32>("async.json", self.backend.clone());
    let status = format!(
      "saved-{:?}-desired-{:?}-pending-{}",
      state.value(),
      state.desired(),
      state.pending().is_some()
    );
    let clear = state.clone();
    (
      View::new().name(status),
      ButtonHost::new(ls("Save"))
        .name("save")
        .on_click(move || state.update(12)),
      ButtonHost::new(ls("Clear"))
        .name("clear")
        .on_click(move || clear.clear()),
    )
  }
}

#[test]
fn async_persistence_wakes_the_component_and_unmount_drains_queued_work() {
  let backend = Arc::new(DeferredPersistence::default());
  let source = backend.clone();
  let mut display = Display::mount_with(
    move || {
      application(DeferredControls {
        backend: source.clone(),
      })
    },
    catalog(),
    Connect::new("test", "test", ScreenSize::new(1_920, 1_080)).persistent_data_path("memory"),
  );
  display.flush();
  while display.with_engine(|engine| engine.has_ready_changes()) {
    display.flush();
  }
  let _ = display.find_ui(ROOT, "saved-None-desired-None-pending-true");
  assert_eq!(backend.calls.lock().unwrap().len(), 1);
  let (read, done) = backend.calls.lock().unwrap().remove(0);
  done(read.id, Ok(Some(b"5".to_vec())));
  display.flush();
  let _ = display.find_ui(ROOT, "saved-Some(5)-desired-Some(5)-pending-false");
  display.click_ui(display.find_ui(ROOT, "save"));
  let _ = display.find_ui(ROOT, "saved-Some(5)-desired-Some(12)-pending-true");
  let (save, done_save) = backend.calls.lock().unwrap().remove(0);
  display.click_ui(display.find_ui(ROOT, "clear"));
  assert!(backend.calls.lock().unwrap().is_empty());
  drop(display);
  done_save(save.id, Ok(None));
  let (delete, done_delete) = backend.calls.lock().unwrap().remove(0);
  assert_eq!(delete.operation, PersistenceOperation::Remove);
  done_delete(delete.id, Ok(None));
  done_save(save.id, Ok(None));
  assert!(backend.calls.lock().unwrap().is_empty());
}

struct PausableTimer {
  paused: bool,
  duration: Duration,
  on_fire: Rc<dyn Fn()>,
}

struct TimerControls;

impl Component for PausableTimer {
  fn render(&self) -> impl Render {
    let on_fire = self.on_fire.clone();
    reactant::use_pausable_timeout(self.duration, self.paused, move || on_fire());
  }
}

impl Component for TimerControls {
  fn render(&self) -> impl Render {
    let (paused, set_paused) = reactant::hooks::use_state(false);
    let (mounted, set_mounted) = reactant::hooks::use_state(true);
    let (long, set_long) = reactant::hooks::use_state(false);
    let (count, set_count) = reactant::hooks::use_state(0);
    (
      ButtonHost::new(ls("Toggle pause"))
        .name("pause")
        .on_click(move || set_paused.update(|v| !v)),
      ButtonHost::new(ls("Toggle mount"))
        .name("mount")
        .on_click(move || set_mounted.update(|v| !v)),
      ButtonHost::new(ls("Change delay"))
        .name("delay")
        .on_click(move || set_long.set(true)),
      View::new().name(format!("count-{count}")),
      mounted.then(|| PausableTimer {
        paused,
        duration: Duration::from_secs(if long { 5 } else { 2 }),
        on_fire: Rc::new(move || set_count.set(count + 1)),
      }),
    )
  }
}

#[test]
fn pausing_a_timer_preserves_remaining_time_and_does_not_rearm_after_firing() {
  let mut display = Display::mount(|| application(TimerControls), catalog());
  display.poll();
  Clock::advance(&mut display, Duration::from_millis(800));
  display.poll();
  display.click_ui(display.find_ui(ROOT, "pause"));
  display.poll();
  Clock::advance(&mut display, Duration::from_secs(30));
  display.poll();
  let _ = display.find_ui(ROOT, "count-0");
  display.click_ui(display.find_ui(ROOT, "pause"));
  display.poll();
  Clock::advance(&mut display, Duration::from_millis(1199));
  display.poll();
  let _ = display.find_ui(ROOT, "count-0");
  Clock::advance(&mut display, Duration::from_millis(1));
  display.poll();
  let _ = display.find_ui(ROOT, "count-1");
  display.click_ui(display.find_ui(ROOT, "pause"));
  display.poll();
  display.click_ui(display.find_ui(ROOT, "pause"));
  display.poll();
  Clock::advance(&mut display, Duration::from_secs(20));
  display.poll();
  let _ = display.find_ui(ROOT, "count-1");
}

#[test]
fn a_paused_timer_can_be_rescheduled_or_unmounted_without_leaking_a_callback() {
  let mut display = Display::mount(|| application(TimerControls), catalog());
  display.poll();
  Clock::advance(&mut display, Duration::from_secs(1));
  display.poll();
  display.click_ui(display.find_ui(ROOT, "pause"));
  display.poll();
  display.click_ui(display.find_ui(ROOT, "delay"));
  display.poll();
  Clock::advance(&mut display, Duration::from_secs(20));
  display.poll();
  let _ = display.find_ui(ROOT, "count-0");
  display.click_ui(display.find_ui(ROOT, "pause"));
  display.poll();
  Clock::advance(&mut display, Duration::from_secs(4));
  display.poll();
  let _ = display.find_ui(ROOT, "count-0");
  display.click_ui(display.find_ui(ROOT, "mount"));
  display.poll();
  Clock::advance(&mut display, Duration::from_secs(20));
  display.poll();
  let _ = display.find_ui(ROOT, "count-0");
  display.click_ui(display.find_ui(ROOT, "mount"));
  display.poll();
  Clock::advance(&mut display, Duration::from_secs(5));
  display.poll();
  let _ = display.find_ui(ROOT, "count-1");
}

#[test]
fn exported_timers_follow_native_time_without_elapsed_wall_time() {
  let mut display = Display::connect(
    reactant::ApplicationEngine::for_export(|| application(TimerControls)),
    catalog(),
  );
  display.advance_frame();
  display.poll();
  display.advance(Duration::from_millis(1999));
  display.advance_frame();
  display.poll();
  let _ = display.find_ui(ROOT, "count-0");
  display.advance(Duration::from_millis(1));
  display.advance_frame();
  display.poll();
  let _ = display.find_ui(ROOT, "count-1");
  display.advance(Duration::from_secs(60));
  display.advance_frame();
  display.poll();
  let _ = display.find_ui(ROOT, "count-1");
}

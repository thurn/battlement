use std::{
  cell::RefCell,
  io,
  path::Path,
  rc::Rc,
  sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, Sender},
  },
  thread::{self, ThreadId},
  time::Duration,
};

use battlement::{Connect, ObjectId, ScreenSize, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{PersistenceBackend, PersistentState, host::ButtonHost, prelude::*};
use reactant_testing::Display;
use trox::ls;

const ROOT: ObjectId = object_id!("31db4a73-2bf4-49c9-b64c-f37a3a4c1fa5");
const TIMEOUT: Duration = Duration::from_secs(10);

struct Storage {
  bytes: Mutex<Option<Vec<u8>>>,
  started: Sender<(Option<Vec<u8>>, ThreadId)>,
  permits: Mutex<Receiver<()>>,
}

impl PersistenceBackend for Storage {
  fn load(&self, _: &Path) -> Result<Option<Vec<u8>>, String> {
    self.started.send((None, thread::current().id())).unwrap();
    self.permits.lock().unwrap().recv_timeout(TIMEOUT).unwrap();
    Ok(self.bytes.lock().unwrap().clone())
  }
  fn store(&self, _: &Path, bytes: &[u8]) -> Result<(), String> {
    self
      .started
      .send((Some(bytes.to_vec()), thread::current().id()))
      .unwrap();
    self.permits.lock().unwrap().recv_timeout(TIMEOUT).unwrap();
    *self.bytes.lock().unwrap() = Some(bytes.to_vec());
    Ok(())
  }
  fn remove(&self, _: &Path) -> Result<(), String> {
    unreachable!()
  }
}

#[derive(Clone)]
struct Fixture {
  backend: Arc<Storage>,
  captured: Rc<RefCell<Option<PersistentState<u32>>>>,
}
struct Save(Fixture);
struct Abandoned(Arc<Storage>);

impl Component for Abandoned {
  fn render(&self) -> impl Render {
    let _ = reactant::use_persistent_state_with::<u32>("match.json", self.0.clone());
    Err::<Label, _>(io::Error::other("discard this save-slot render"))
  }
}

impl Component for Save {
  fn render(&self) -> impl Render {
    let saved = reactant::use_persistent_state_with::<u32>("match.json", self.0.backend.clone());
    let label = format!("{:?}: {:?}", saved.status(), saved.value());
    *self.0.captured.borrow_mut() = Some(saved.clone());
    (
      Label::new(ls(label)).name("status"),
      ButtonHost::new(ls("Save"))
        .name("save")
        .on_click(move || saved.update_with(|value| value.unwrap_or(0) + 1)),
    )
  }
}

impl Component for Fixture {
  fn render(&self) -> impl Render {
    let (mounted, mount) = reactant::hooks::use_state(true);
    let (menu, increment) = reactant::hooks::use_state(0);
    (
      mounted.then(|| Save(self.clone())),
      ErrorBoundary::new(|_: &RenderError| Label::new(ls("fallback")))
        .child(Abandoned(self.backend.clone())),
      Label::new(ls(menu.to_string())).name("menu-count"),
      ButtonHost::new(ls("Menu"))
        .name("menu")
        .on_click(move || increment.set(menu + 1)),
      ButtonHost::new(ls("Mount"))
        .name("mount")
        .on_click(move || mount.set(!mounted)),
    )
  }
}

fn ready(display: &mut Display) {
  for _ in 0..20 {
    display.flush();
    if !display.with_engine(|engine| engine.has_ready_changes()) {
      return;
    }
  }
  panic!("component effects did not settle");
}
fn click(display: &mut Display, name: &str) {
  display.click_ui(display.find_ui(ROOT, name));
  self::ready(display);
}
fn captured(fixture: &Fixture) -> PersistentState<u32> {
  fixture.captured.borrow().as_ref().unwrap().clone()
}

#[test]
fn blocking_storage_never_blocks_menus_and_unmount_drains_the_latest_captured_write() {
  let (started, starts) = mpsc::channel();
  let (release, permits) = mpsc::channel();
  let fixture = Fixture {
    backend: Arc::new(Storage {
      bytes: Mutex::new(None),
      started,
      permits: Mutex::new(permits),
    }),
    captured: Rc::default(),
  };
  let component = fixture.clone();
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("storage");
  let mut display = Display::mount_with(
    move || {
      Application::new("storage")
        .child(component.clone())
        .document(|mut document| {
          document.root_id = ROOT;
          document
        })
    },
    assets,
    Connect::new("test", "test", ScreenSize::new(800, 600)).persistent_data_path("memory"),
  );
  self::ready(&mut display);
  let (load, worker) = starts.recv_timeout(TIMEOUT).unwrap();
  assert_eq!(load, None);
  assert_ne!(worker, thread::current().id());
  assert_eq!(
    display.ui_element(display.find_ui(ROOT, "status")).text(),
    Some("Loading: None")
  );
  self::click(&mut display, "menu");
  assert_eq!(
    display
      .ui_element(display.find_ui(ROOT, "menu-count"))
      .text(),
    Some("1")
  );
  release.send(()).unwrap();
  assert!(self::captured(&fixture).wait_for_idle(TIMEOUT));
  self::ready(&mut display);
  assert_eq!(
    display.ui_element(display.find_ui(ROOT, "status")).text(),
    Some("Absent: None")
  );
  self::click(&mut display, "save");
  let (first, worker) = starts.recv_timeout(TIMEOUT).unwrap();
  assert_eq!(first, Some(b"1".to_vec()));
  assert_ne!(worker, thread::current().id());
  self::click(&mut display, "save");
  self::click(&mut display, "save");
  self::click(&mut display, "menu");
  assert!(starts.try_recv().is_err());
  let old = self::captured(&fixture);
  assert_eq!(old.desired(), Some(&3));
  assert_eq!(old.value(), None);
  self::click(&mut display, "mount");
  old.update(99);
  release.send(()).unwrap();
  assert_eq!(starts.recv_timeout(TIMEOUT).unwrap().0, Some(b"3".to_vec()));
  release.send(()).unwrap();
  assert!(old.wait_for_idle(TIMEOUT));
  assert_eq!(*fixture.backend.bytes.lock().unwrap(), Some(b"3".to_vec()));
  self::click(&mut display, "mount");
  assert_eq!(starts.recv_timeout(TIMEOUT).unwrap().0, None);
  release.send(()).unwrap();
  assert!(self::captured(&fixture).wait_for_idle(TIMEOUT));
  self::ready(&mut display);
  assert_eq!(
    display.ui_element(display.find_ui(ROOT, "status")).text(),
    Some("Loaded: Some(3)")
  );
  assert_ne!(
    old.version().owner,
    self::captured(&fixture).version().owner
  );
  assert_eq!(display.presentation_time(), Duration::ZERO);
}

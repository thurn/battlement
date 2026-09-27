use std::{
  cell::RefCell,
  rc::Rc,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, Sender},
  },
  time::Duration,
};

use battlement::{ObjectId, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{Task, TaskState, host::ButtonHost, prelude::*};
use reactant_testing::Display;
use trox::ls;

const ROOT: ObjectId = object_id!("7334c8cf-0f2b-43c5-9f65-c0abf9a29a79");
const TIMEOUT: Duration = Duration::from_secs(10);

struct Probe {
  started: Sender<u32>,
  finished: Sender<u32>,
  permits: Mutex<Receiver<()>>,
  fail: AtomicBool,
}

#[derive(Clone)]
struct Fixture {
  probe: Arc<Probe>,
  task: Rc<RefCell<Option<Task<u32>>>>,
}

struct Work {
  fixture: Fixture,
  key: Option<u32>,
}

struct Finished(u32, Sender<u32>);
impl Drop for Finished {
  fn drop(&mut self) {
    let _ = self.1.send(self.0);
  }
}

impl Component for Work {
  fn render(&self) -> impl Render {
    let probe = Arc::clone(&self.fixture.probe);
    let key = self.key;
    let task = reactant::use_task(key, move |token| {
      let key = key.expect("enabled work");
      let _finished = Finished(key, probe.finished.clone());
      probe.started.send(key).unwrap();
      assert!(
        !probe.fail.swap(false, Ordering::SeqCst),
        "injected task failure"
      );
      probe.permits.lock().unwrap().recv_timeout(TIMEOUT).unwrap();
      token.checkpoint();
      key
    });
    let label = match task.state() {
      TaskState::Idle => "idle".to_owned(),
      TaskState::Pending => "pending".to_owned(),
      TaskState::Ready(value) => format!("ready {value}"),
      TaskState::Failed(_) => "failed".to_owned(),
    };
    *self.fixture.task.borrow_mut() = Some(task);
    Label::new(ls(label)).name("task")
  }
}

impl Component for Fixture {
  fn render(&self) -> impl Render {
    let (key, next) = reactant::hooks::use_state(0_u32);
    let (paused, pause) = reactant::hooks::use_state(false);
    let (mounted, mount) = reactant::hooks::use_state(true);
    let (count, menu) = reactant::hooks::use_state(0_u32);
    (
      mounted.then(|| Work {
        fixture: self.clone(),
        key: (!paused).then_some(key),
      }),
      Label::new(ls(count.to_string())).name("count"),
      ButtonHost::new(ls("Menu"))
        .name("menu")
        .on_click(move || menu.set(count + 1)),
      ButtonHost::new(ls("Next"))
        .name("next")
        .on_click(move || next.set(key + 1)),
      ButtonHost::new(ls("Pause"))
        .name("pause")
        .on_click(move || pause.set(!paused)),
      ButtonHost::new(ls("Mount"))
        .name("mount")
        .on_click(move || mount.set(!mounted)),
    )
  }
}

fn setup(fail: bool) -> (Display, Fixture, Receiver<u32>, Sender<()>, Receiver<u32>) {
  let (started, starts) = mpsc::channel();
  let (release, permits) = mpsc::channel();
  let (finished, finishes) = mpsc::channel();
  let fixture = Fixture {
    probe: Arc::new(Probe {
      started,
      finished,
      permits: Mutex::new(permits),
      fail: AtomicBool::new(fail),
    }),
    task: Rc::default(),
  };
  let component = fixture.clone();
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("tasks/scene");
  let mut display = Display::mount(
    move || {
      Application::new("tasks/scene")
        .child(component.clone())
        .document(|mut d| {
          d.root_id = ROOT;
          d
        })
    },
    assets,
  );
  display.flush();
  (display, fixture, starts, release, finishes)
}
fn task(fixture: &Fixture) -> Task<u32> {
  fixture.task.borrow().as_ref().unwrap().clone()
}
fn click(display: &mut Display, name: &str) {
  display.click_ui(display.find_ui(ROOT, name));
  display.flush();
}

#[test]
fn equal_keys_preserve_work_latest_replacement_wins_and_menus_stay_live() {
  let (mut display, fixture, starts, release, finishes) = self::setup(false);
  assert_eq!(starts.recv_timeout(TIMEOUT).unwrap(), 0);
  let old = self::task(&fixture);
  self::click(&mut display, "menu");
  assert_eq!(
    display.ui_element(display.find_ui(ROOT, "count")).text(),
    Some("1")
  );
  assert!(starts.try_recv().is_err());
  self::click(&mut display, "next");
  let superseded = self::task(&fixture);
  self::click(&mut display, "next");
  assert!(matches!(old.state(), TaskState::Idle));
  assert!(matches!(superseded.state(), TaskState::Idle));
  assert!(starts.try_recv().is_err());
  release.send(()).unwrap();
  assert_eq!(finishes.recv_timeout(TIMEOUT).unwrap(), 0);
  assert_eq!(starts.recv_timeout(TIMEOUT).unwrap(), 2);
  let current = self::task(&fixture);
  release.send(()).unwrap();
  assert!(reactant::testing::wait_for_task(&current, TIMEOUT));
  display.flush();
  assert_eq!(
    display.ui_element(display.find_ui(ROOT, "task")).text(),
    Some("ready 2")
  );
  assert!(matches!(current.state(), TaskState::Ready(value) if *value == 2));
  assert!(!old.retry());
  assert_eq!(display.presentation_time(), Duration::ZERO);
}

#[test]
fn disabled_and_unmounted_owners_invalidate_results_without_joining_workers() {
  let (mut display, fixture, starts, release, finishes) = self::setup(false);
  assert_eq!(starts.recv_timeout(TIMEOUT).unwrap(), 0);
  let old = self::task(&fixture);
  self::click(&mut display, "pause");
  assert!(matches!(old.state(), TaskState::Idle));
  assert!(matches!(self::task(&fixture).state(), TaskState::Idle));
  self::click(&mut display, "menu");
  release.send(()).unwrap();
  assert_eq!(finishes.recv_timeout(TIMEOUT).unwrap(), 0);
  display.flush();
  assert!(starts.try_recv().is_err());
  self::click(&mut display, "pause");
  assert_eq!(starts.recv_timeout(TIMEOUT).unwrap(), 0);
  let resumed = self::task(&fixture);
  self::click(&mut display, "mount");
  assert!(matches!(resumed.state(), TaskState::Idle));
  release.send(()).unwrap();
  assert_eq!(finishes.recv_timeout(TIMEOUT).unwrap(), 0);
  display.flush();
  assert!(matches!(resumed.state(), TaskState::Idle));
  assert!(!resumed.retry());
  assert_eq!(display.presentation_time(), Duration::ZERO);
}

#[test]
fn failure_is_reactive_and_explicit_retry_uses_the_current_owner_once() {
  let (mut display, fixture, starts, release, finishes) = self::setup(true);
  assert_eq!(starts.recv_timeout(TIMEOUT).unwrap(), 0);
  let failed = self::task(&fixture);
  assert!(reactant::testing::wait_for_task(&failed, TIMEOUT));
  display.flush();
  assert_eq!(
    display.ui_element(display.find_ui(ROOT, "task")).text(),
    Some("failed")
  );
  assert_eq!(finishes.recv_timeout(TIMEOUT).unwrap(), 0);
  assert!(
    matches!(failed.state(), TaskState::Failed(message) if message.contains("injected task failure"))
  );
  assert!(failed.retry());
  assert!(!failed.retry());
  display.flush();
  assert_eq!(starts.recv_timeout(TIMEOUT).unwrap(), 0);
  let replacement = self::task(&fixture);
  assert!(matches!(failed.state(), TaskState::Idle));
  release.send(()).unwrap();
  assert!(reactant::testing::wait_for_task(&replacement, TIMEOUT));
  display.flush();
  assert!(matches!(replacement.state(), TaskState::Ready(value) if *value == 0));
  assert!(!replacement.retry());
}

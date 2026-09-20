use std::time::Duration;

use battlement::{DragMode, ObjectId, ParentScene, Vector3, object_id};
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

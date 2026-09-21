use std::time::Duration;

use battlement_fake::assets::FakeAssetCatalog;
use reactant_core::{app::App, host::ButtonHost, prelude::*};
use reactant_testing::{Display, WorkerDisplay, temporal::Clock};
use trox::ls;

#[test]
fn public_ui_input_updates_a_visible_app_observation() {
  let app = App::with_model("app/content", 0_u32).root(|count| {
    View::new().child((
      Label::new(ls(format!("{count}"))).name("count"),
      ButtonHost::new(ls("Increment"))
        .name("increment")
        .on_click(|count: &mut u32| *count += 1),
    ))
  });
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("app/content");
  let mut display = Display::connect(app, assets);
  let increment = display.find_ui(root, "increment");

  display.click_ui(increment);

  let count = display.find_ui(root, "count");
  assert_eq!(display.ui_element(count).text(), Some("1"));
  Clock::advance(&mut display, Duration::from_millis(125));
  assert_eq!(display.frame(), 0);
  display.advance_frame();
  assert_eq!(display.presentation_time(), Duration::from_millis(125));
  assert_eq!(display.frame(), 1);
}

#[test]
fn exported_worker_waits_are_event_driven_bounded_and_time_independent() {
  let mut worker = WorkerDisplay::builder()
    .timeout(Duration::from_millis(500))
    .build()
    .unwrap();

  worker.enter_waiting_barrier();
  worker.wait_for_worker_started(1).unwrap();
  worker.wait_for_waiting_barrier().unwrap();
  assert_eq!(worker.display().presentation_time(), Duration::ZERO);
  assert_eq!(worker.display().frame(), 0);

  worker.reconnect();
  worker.wait_for_worker_stopped(1).unwrap();
  worker.wait_for_computation_barrier().unwrap();
  let error = worker.wait_for_worker_stopped(2).unwrap_err();
  assert_eq!(error.timeout(), Duration::from_millis(500));
  assert!(error.to_string().contains("2 worker task(s) to stop"));
  assert!(error.to_string().contains("last worker observation"));
  assert_eq!(error.last_observation().stopped, 1);
  assert_eq!(worker.display().presentation_time(), Duration::ZERO);
  assert_eq!(worker.display().frame(), 0);

  worker.reconnect();
  worker.release_computation();
  worker.wait_for_worker_stopped(3).unwrap();
}

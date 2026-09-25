use battlement::{
  Connect, ObjectId, ScreenSize,
  host_settings::{HostPlatform, HostSettings, SettingAvailability},
  object_id,
};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{Application, hooks, host::ButtonHost, prelude::*};
use reactant_core::component;
use reactant_testing::Display;

const ROOT: ObjectId = object_id!("e5fcebb6-8c38-4d0a-a672-4809284e05f6");

#[derive(PartialEq)]
struct Settings;

impl Component for Settings {
  fn render(&self) -> impl Render {
    let host = reactant::use_host_settings();
    let (preferred, set_preferred) = hooks::use_state(false);
    (
      ButtonHost::new(trox::ls("Save preference"))
        .name("save")
        .on_click(move || set_preferred.set(true)),
      View::new().name(format!(
        "{:?}-{}-{}-{}",
        host.display, host.keyboard_connected, host.controller_count, preferred
      )),
    )
  }
}

#[test]
fn touch_devices_monitor_failures_and_reconnect_update_context_without_resetting_preferences() {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("settings/scene");
  let mut connect = Connect::new("test", "test", ScreenSize::new(800, 600));
  let mut settings = HostSettings {
    platform: HostPlatform::Ios,
    ..HostSettings::default()
  };
  connect.host_settings = settings.clone();
  let mut display = Display::mount_with(
    || {
      Application::new("settings/scene")
        .child(component::memo(Settings))
        .document(|mut document| {
          document.root_id = ROOT;
          document
        })
    },
    assets,
    connect,
  );
  display.flush();
  let _ = display.find_ui(ROOT, "Unavailable-false-0-false");
  let save = display.find_ui(ROOT, "save");
  display.click_ui(save);
  display.flush();
  let _ = display.find_ui(ROOT, "Unavailable-false-0-true");
  settings.keyboard_connected = true;
  settings.controller_count = 1;
  display.set_host_settings(settings.clone());
  display.flush();
  let _ = display.find_ui(ROOT, "Unavailable-true-1-true");
  settings.platform = HostPlatform::MacOs;
  settings.display = SettingAvailability::Failed;
  settings.observation_error = Some("display disconnected".into());
  display.set_host_settings(settings.clone());
  display.flush();
  let _ = display.find_ui(ROOT, "Failed-true-1-true");
  settings.display = SettingAvailability::Available;
  settings.observation_error = None;
  settings.keyboard_connected = false;
  settings.controller_count = 0;
  display.set_host_settings(settings);
  display.flush();
  let _ = display.find_ui(ROOT, "Available-false-0-true");
  display.reconnect();
  display.flush();
  let _ = display.find_ui(ROOT, "Available-false-0-false");
}

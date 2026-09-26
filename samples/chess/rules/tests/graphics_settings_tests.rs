//! Graphics preferences cross the real menu, host transaction and persistence boundaries.
mod support;

use battlement::{
  CommandId, ControllerButton, PhysicalKey, ScreenSize,
  application::ApplicationState,
  host_settings::{
    DisplayConfiguration, DisplayMode, DisplayResolution, HostPlatform, HostSettings,
    HostSettingsResult, SettingAvailability,
  },
};
use chess_rules::settings::ChessSettings;
use reactant::PersistenceBackend;
use reactant_testing::MemoryPersistence;
use std::{path::Path, time::Duration};
use support::game::ChessTest;

fn resolution(width: u32, height: u32) -> DisplayResolution {
  DisplayResolution {
    width,
    height,
    refresh_numerator: 60,
    refresh_denominator: 1,
  }
}

fn desktop() -> HostSettings {
  HostSettings {
    platform: HostPlatform::MacOs,
    display: SettingAvailability::Available,
    display_modes: vec![DisplayMode::Windowed, DisplayMode::Borderless],
    resolutions: vec![resolution(1280, 720), resolution(1024, 768)],
    applied_display: Some(DisplayConfiguration {
      mode: DisplayMode::Windowed,
      resolution: resolution(1280, 720),
    }),
    window_bounds: Some(ScreenSize::new(1920, 1080)),
    frame_pacing: SettingAvailability::Available,
    frame_rates: vec![60, 120, 144, 240],
    vsync_available: true,
    keyboard_connected: true,
    controller_count: 1,
    ..HostSettings::default()
  }
}

fn open(game: &mut ChessTest) {
  game.display.set_host_settings(desktop());
  game.display.advance(Duration::ZERO);
  game.display.activate_accessible("SETTINGS");
  game.display.activate_accessible("Graphics");
}

fn saved(storage: &MemoryPersistence) -> Option<ChessSettings> {
  storage
    .load(Path::new("memory/chess-settings.json"))
    .unwrap()
    .map(|bytes| serde_json::from_slice(&bytes).unwrap())
}

fn preview(game: &mut ChessTest) {
  game.display.activate_accessible("Resolution 1280 × 720");
  game.display.activate_accessible("1024 × 768");
  game.display.expect_button("Keep");
}

#[test]
fn confirmed_display_pair_is_atomic_and_cancel_preserves_it() {
  let storage = MemoryPersistence::empty();
  let mut game = ChessTest::persisted(storage.clone());
  open(&mut game);
  preview(&mut game);
  assert!(saved(&storage).is_none_or(|value| value.resolution.is_none()));
  let mut stale = desktop();
  stale.applied_display = Some(DisplayConfiguration {
    mode: DisplayMode::Windowed,
    resolution: resolution(1024, 768),
  });
  stale.last_result = Some(HostSettingsResult {
    request_id: CommandId::new_v4(),
    error: None,
  });
  game.display.set_host_settings(stale);
  game.display.advance(Duration::ZERO);
  game.display.expect_button("Keep");
  assert!(saved(&storage).is_none_or(|value| value.resolution.is_none()));
  game.display.activate_accessible("Keep");
  game.display.expect_button("Resolution 1024 × 768");
  let confirmed = saved(&storage).unwrap();
  assert_eq!(confirmed.resolution, Some(resolution(1024, 768)));
  assert_eq!(confirmed.display_mode, DisplayMode::Windowed);
  game.display.activate_accessible("Display Mode Windowed");
  game.display.activate_accessible("Borderless");
  game.display.activate_accessible("Revert");
  game.display.expect_button("Display Mode Windowed");
  assert_eq!(saved(&storage), Some(confirmed));
}

#[test]
fn timeout_focus_loss_and_failed_confirmation_never_save_preview() {
  let storage = MemoryPersistence::empty();
  let mut game = ChessTest::persisted(storage.clone());
  open(&mut game);
  preview(&mut game);
  game.display.advance(Duration::from_secs(15));
  game.display.advance(Duration::ZERO);
  game.display.expect_button("Resolution 1280 × 720");
  preview(&mut game);
  game.display.send_key(PhysicalKey::Escape);
  game.display.expect_button("Resolution 1280 × 720");
  preview(&mut game);
  game.display.press_controller_button(ControllerButton::East);
  game.display.expect_button("Resolution 1280 × 720");
  preview(&mut game);
  game.display.fail_next_display_save();
  game.display.activate_accessible("Keep");
  game.display.expect_button("Resolution 1280 × 720");
  preview(&mut game);
  game.display.set_application_state(ApplicationState {
    focused: false,
    paused: false,
  });
  game.display.advance(Duration::ZERO);
  game.display.expect_button("Resolution 1280 × 720");
  assert!(saved(&storage).is_none_or(|value| value.resolution.is_none()));
}

#[test]
fn pacing_controls_follow_capabilities_and_retain_hidden_preferences() {
  let storage = MemoryPersistence::empty();
  let mut game = ChessTest::persisted(storage.clone());
  open(&mut game);
  assert!(!game.display.accessibility().nodes.iter().any(|node| {
    node
      .label
      .as_deref()
      .is_some_and(|name| name.starts_with("Max Framerate"))
  }));
  game.display.activate_accessible("VSync");
  game
    .display
    .expect_button("Max Framerate \u{2068}144\u{2069} FPS");
  game
    .display
    .activate_accessible("Max Framerate \u{2068}144\u{2069} FPS");
  game.display.activate_accessible("\u{2068}120\u{2069} FPS");
  game.display.activate_accessible("VSync");
  game.display.activate_accessible("VSync");
  game
    .display
    .expect_button("Max Framerate \u{2068}120\u{2069} FPS");
  game
    .display
    .activate_accessible("Max Framerate \u{2068}120\u{2069} FPS");
  let mut mobile = desktop();
  mobile.platform = HostPlatform::Ios;
  mobile.display = SettingAvailability::Unavailable;
  mobile.display_modes.clear();
  mobile.resolutions.clear();
  mobile.vsync_available = false;
  mobile.frame_rates = vec![30, 60];
  game.display.set_host_settings(mobile);
  game.display.advance(Duration::ZERO);
  game
    .display
    .expect_button("Max Framerate \u{2068}60\u{2069} FPS");
  assert_eq!(saved(&storage).unwrap().max_framerate, Some(120));
  assert!(
    !game
      .display
      .accessibility()
      .nodes
      .iter()
      .any(|node| node.label.as_deref() == Some("VSync"))
  );
  game.display.set_host_settings(desktop());
  game.display.advance(Duration::ZERO);
  game
    .display
    .expect_button("Max Framerate \u{2068}120\u{2069} FPS");
}

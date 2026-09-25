//! Saved bindings cross the physical host input, settings UI, and durable storage boundaries.
mod support;

use battlement::host_settings::{HostPlatform, HostSettings};
use battlement::{
  ControllerButton, ControllerDirection, ControllerNavigationPayload, ControllerNavigationSource,
  InputCaptureDevice, PhysicalKey,
};
use chess_rules::{
  PersistenceBackend, assets,
  settings::{
    ChessSettings,
    bindings::{Bindings, ControllerBinding},
  },
};
use cozy_chess::{Color, Piece, Square};
use reactant_testing::MemoryPersistence;
use std::path::Path;
use support::game::ChessTest;

const ACTIONS: [&str; 7] = [
  "Left",
  "Right",
  "Up",
  "Down",
  "Move Piece",
  "Pause",
  "Restart",
];
const KEYS: [PhysicalKey; 7] = [
  PhysicalKey::KeyA,
  PhysicalKey::KeyD,
  PhysicalKey::KeyW,
  PhysicalKey::KeyS,
  PhysicalKey::KeyF,
  PhysicalKey::KeyP,
  PhysicalKey::KeyT,
];

fn open_binding(game: &mut ChessTest, action: &str, family: &str) {
  game
    .display
    .activate_accessible(&format!("Change \u{2068}{action}\u{2069} {family} binding"));
}

fn settings(storage: &MemoryPersistence) -> ChessSettings {
  serde_json::from_slice(
    &storage
      .load(Path::new("memory/chess-settings.json"))
      .unwrap()
      .unwrap(),
  )
  .unwrap()
}

fn neutral(game: &mut ChessTest) {
  game.display.release_controller_navigation();
  for _ in 0..3 {
    game.display.advance_frame();
  }
}

fn has_label(game: &ChessTest, label: &str) -> bool {
  game
    .display
    .accessibility()
    .nodes
    .iter()
    .any(|node| node.label.as_deref() == Some(label))
}

fn has_pause(game: &ChessTest) -> bool {
  game
    .display
    .images()
    .any(|(object, image)| object.active_in_hierarchy() && image.texture == assets::REFRESH_BUTTON)
}

#[test]
fn every_keyboard_binding_is_captured_persisted_and_used_after_restart() {
  let storage = MemoryPersistence::empty();
  let mut game = ChessTest::persisted(storage.clone());
  game.display.activate_accessible("SETTINGS");
  game.display.activate_accessible("Input");
  for (action, key) in ACTIONS.into_iter().zip(KEYS) {
    self::open_binding(&mut game, action, "keyboard");
    game.display.send_key(key);
    self::neutral(&mut game);
    assert!(!self::has_label(&game, "Waiting for keyboard input"));
  }
  assert_eq!(self::settings(&storage).keyboard.values(), KEYS);
  drop(game);
  let mut game = ChessTest::persisted(storage.clone());
  game.start();
  game.display.send_keys(&[
    PhysicalKey::KeyD,
    PhysicalKey::KeyA,
    PhysicalKey::KeyF,
    PhysicalKey::KeyW,
    PhysicalKey::KeyW,
    PhysicalKey::KeyS,
    PhysicalKey::KeyW,
    PhysicalKey::KeyF,
  ]);
  game.expect_empty(Square::E2);
  game.expect_piece(Square::E4, Color::White, Piece::Pawn);
  game.display.send_key(PhysicalKey::KeyR);
  game.expect_piece(Square::E4, Color::White, Piece::Pawn);
  game.display.send_key(PhysicalKey::Escape);
  assert!(!self::has_pause(&game));
  game.display.send_key(PhysicalKey::KeyP);
  assert!(self::has_pause(&game));
  game.display.send_key(PhysicalKey::Escape);
  assert!(!self::has_pause(&game));
  game.display.send_key(PhysicalKey::KeyT);
  game.expect_piece(Square::E2, Color::White, Piece::Pawn);
  game.expect_empty(Square::E4);
  assert_eq!(self::settings(&storage).keyboard.values(), KEYS);
}

#[test]
fn old_movement_and_submit_aliases_do_not_act_after_remapping() {
  for old in [
    PhysicalKey::ArrowLeft,
    PhysicalKey::ArrowRight,
    PhysicalKey::ArrowUp,
    PhysicalKey::ArrowDown,
    PhysicalKey::Space,
    PhysicalKey::Enter,
    PhysicalKey::NumpadEnter,
  ] {
    let preferences = ChessSettings {
      keyboard: Bindings::from_values(KEYS),
      ..ChessSettings::default()
    };
    let storage = MemoryPersistence::with_file(
      "memory/chess-settings.json",
      &serde_json::to_vec(&preferences).unwrap(),
    );
    let mut game = ChessTest::persisted(storage);
    game.start();
    if game.display.world().global_keys().contains(&old) {
      game.display.send_key(old);
    }
    game.display.send_keys(&[
      PhysicalKey::KeyF,
      PhysicalKey::KeyW,
      PhysicalKey::KeyW,
      PhysicalKey::KeyF,
    ]);
    game.expect_empty(Square::E2);
    game.expect_piece(Square::E4, Color::White, Piece::Pawn);
  }
}

#[test]
fn conflicts_reset_and_cancel_leave_saved_bindings_unchanged_and_capture_rearms() {
  let storage = MemoryPersistence::empty();
  let mut game = ChessTest::persisted(storage.clone());
  game.display.activate_accessible("SETTINGS");
  game.display.activate_accessible("Input");
  self::open_binding(&mut game, "Left", "keyboard");
  game.display.send_key(PhysicalKey::ArrowRight);
  game
    .display
    .semantic_node("Already used by \u{2068}Right\u{2069}");
  self::neutral(&mut game);
  game.display.send_key(PhysicalKey::KeyA);
  self::neutral(&mut game);
  assert_eq!(self::settings(&storage).keyboard.left, PhysicalKey::KeyA);
  self::open_binding(&mut game, "Right", "keyboard");
  game.display.send_key(PhysicalKey::ArrowLeft);
  self::neutral(&mut game);
  let before = self::settings(&storage);
  self::open_binding(&mut game, "Left", "keyboard");
  game.display.activate_accessible("Reset");
  game
    .display
    .semantic_node("Already used by \u{2068}Right\u{2069}");
  assert_eq!(self::settings(&storage), before);
  game.display.send_key(PhysicalKey::Escape);
  self::neutral(&mut game);
  assert!(!self::has_label(&game, "Waiting for keyboard input"));
  self::open_binding(&mut game, "Pause", "keyboard");
  game.display.activate_accessible("Cancel");
  assert_eq!(self::settings(&storage), before);
  self::open_binding(&mut game, "Pause", "keyboard");
  game.display.send_key(PhysicalKey::Escape);
  self::neutral(&mut game);
  assert!(!self::has_label(&game, "Waiting for keyboard input"));
  assert_eq!(self::settings(&storage), before);
}

#[test]
fn controller_capture_remaps_all_actions_and_keeps_stick_navigation_fixed() {
  let storage = MemoryPersistence::empty();
  let mut game = ChessTest::persisted(storage.clone());
  game.display.set_host_settings(HostSettings {
    platform: HostPlatform::MacOs,
    controller_count: 1,
    ..HostSettings::default()
  });
  game.display.activate_accessible("SETTINGS");
  game.display.activate_accessible("Input");
  for (action, button) in [
    ("Left", ControllerButton::LeftShoulder),
    ("Right", ControllerButton::RightShoulder),
    ("Up", ControllerButton::West),
    ("Down", ControllerButton::Select),
  ] {
    self::open_binding(&mut game, action, "controller");
    game.display.press_controller_button(button);
    self::neutral(&mut game);
  }
  for (action, direction) in [
    ("Move Piece", ControllerDirection::Up),
    ("Pause", ControllerDirection::Down),
  ] {
    self::open_binding(&mut game, action, "controller");
    game.display.controller_navigate(0, direction);
    game.display.settle();
    self::neutral(&mut game);
  }
  self::open_binding(&mut game, "Restart", "controller");
  game
    .display
    .press_controller_button(ControllerButton::South);
  self::neutral(&mut game);
  assert_eq!(
    self::settings(&storage).controller.values(),
    [
      ControllerBinding::LeftShoulder,
      ControllerBinding::RightShoulder,
      ControllerBinding::West,
      ControllerBinding::Select,
      ControllerBinding::DpadUp,
      ControllerBinding::DpadDown,
      ControllerBinding::South
    ]
  );
  game.display.activate_accessible("RETURN");
  game.start();
  game
    .display
    .press_controller_button(ControllerButton::RightShoulder);
  game
    .display
    .press_controller_button(ControllerButton::LeftShoulder);
  game.display.navigate_controller(&[ControllerDirection::Up]);
  self::neutral(&mut game);
  game.display.press_controller_button(ControllerButton::West);
  game.display.press_controller_button(ControllerButton::West);
  game
    .display
    .press_controller_button(ControllerButton::Select);
  game
    .display
    .controller_navigation(ControllerNavigationPayload {
      controller_id: 0,
      direction: ControllerDirection::Up,
      source: ControllerNavigationSource::LeftStick,
      repeat: false,
    });
  game.display.settle();
  self::neutral(&mut game);
  game.display.navigate_controller(&[ControllerDirection::Up]);
  self::neutral(&mut game);
  game.expect_piece(Square::E4, Color::White, Piece::Pawn);
  assert!(
    !game
      .display
      .world()
      .controller_input()
      .unwrap()
      .buttons
      .contains(&ControllerButton::North)
  );
  game.expect_piece(Square::E4, Color::White, Piece::Pawn);
  assert!(
    !game
      .display
      .world()
      .controller_input()
      .unwrap()
      .buttons
      .contains(&ControllerButton::Start)
  );
  assert!(!self::has_pause(&game));
  game
    .display
    .navigate_controller(&[ControllerDirection::Down]);
  assert!(self::has_pause(&game));
  self::neutral(&mut game);
  game.display.press_controller_button(ControllerButton::East);
  assert!(!self::has_pause(&game));
  game
    .display
    .press_controller_button(ControllerButton::South);
  game.expect_piece(Square::E2, Color::White, Piece::Pawn);
  assert_eq!(
    self::settings(&storage).controller.restart,
    ControllerBinding::South
  );
}

#[test]
fn device_observations_reveal_columns_and_disconnect_cancels_capture() {
  let mut game = ChessTest::title();
  game.display.set_host_settings(HostSettings {
    platform: HostPlatform::Web,
    ..HostSettings::default()
  });
  game.display.activate_accessible("SETTINGS");
  game.display.activate_accessible("Text Size 100%");
  game.display.activate_accessible("200%");
  assert!(!self::has_label(&game, "Input"));
  game.display.set_host_settings(HostSettings {
    platform: HostPlatform::Web,
    keyboard_connected: true,
    ..HostSettings::default()
  });
  game.display.settle();
  assert!(self::has_label(&game, "Input"));
  game.display.activate_accessible("Input");
  game
    .display
    .expect_button("Change \u{2068}Left\u{2069} keyboard binding");
  assert!(!self::has_label(
    &game,
    "Change \u{2068}Left\u{2069} controller binding"
  ));
  game.display.set_host_settings(HostSettings {
    platform: HostPlatform::Ios,
    controller_count: 1,
    ..HostSettings::default()
  });
  game.display.settle();
  assert!(!self::has_label(
    &game,
    "Change \u{2068}Left\u{2069} keyboard binding"
  ));
  self::open_binding(&mut game, "Left", "controller");
  game
    .display
    .disconnect_input_device(InputCaptureDevice::Controller);
  game.display.settle();
  assert!(!self::has_label(&game, "Waiting for controller input"));
  assert!(!self::has_label(&game, "Input"));
  game.display.set_host_settings(HostSettings {
    platform: HostPlatform::Windows,
    ..HostSettings::default()
  });
  game.display.settle();
  assert!(self::has_label(&game, "Input"));
}

#[test]
fn escape_cancels_selection_before_pause_and_remapped_pause_stays_exclusive() {
  for pause in [PhysicalKey::Escape, PhysicalKey::KeyP] {
    let mut preferences = ChessSettings::default();
    preferences.keyboard.pause = pause;
    let storage = MemoryPersistence::with_file(
      "memory/chess-settings.json",
      &serde_json::to_vec(&preferences).unwrap(),
    );
    let mut game = ChessTest::persisted(storage);
    game.start();
    game.display.send_key(PhysicalKey::Space);
    assert!(self::has_label(&game, "Move to \u{2068}e4\u{2069}"));
    game.display.send_key(PhysicalKey::Escape);
    assert!(!self::has_label(&game, "Move to \u{2068}e4\u{2069}"));
    assert!(!self::has_pause(&game));
    game.display.send_key(PhysicalKey::Escape);
    assert_eq!(self::has_pause(&game), pause == PhysicalKey::Escape);
  }
}

#[test]
fn held_opener_and_focus_loss_do_not_bind_or_leave_capture_stuck() {
  let storage = MemoryPersistence::empty();
  let mut game = ChessTest::persisted(storage.clone());
  game.display.activate_accessible("SETTINGS");
  game.display.activate_accessible("Input");
  game.display.key_down(PhysicalKey::KeyR);
  self::open_binding(&mut game, "Left", "keyboard");
  game.display.key_down(PhysicalKey::KeyR);
  game.display.settle();
  game.display.semantic_node("Waiting for keyboard input");
  game
    .display
    .set_application_state(battlement::application::ApplicationState {
      focused: false,
      paused: false,
    });
  game.display.settle();
  assert!(!self::has_label(&game, "Waiting for keyboard input"));
  game.display.key_up(PhysicalKey::KeyR);
  game
    .display
    .set_application_state(battlement::application::ApplicationState {
      focused: true,
      paused: false,
    });
  self::neutral(&mut game);
  self::open_binding(&mut game, "Left", "keyboard");
  game.display.send_key(PhysicalKey::KeyA);
  self::neutral(&mut game);
  assert_eq!(self::settings(&storage).keyboard.left, PhysicalKey::KeyA);
  game.display.activate_accessible("RETURN");
  game.start();
  game.display.send_keys(&[
    PhysicalKey::KeyA,
    PhysicalKey::Space,
    PhysicalKey::ArrowUp,
    PhysicalKey::ArrowUp,
    PhysicalKey::Space,
  ]);
  game.expect_piece(Square::D4, Color::White, Piece::Pawn);
}

#[test]
fn controller_conflicting_reset_and_east_cancel_preserve_maps() {
  let storage = MemoryPersistence::empty();
  let mut game = ChessTest::persisted(storage.clone());
  game.display.set_host_settings(HostSettings {
    platform: HostPlatform::MacOs,
    controller_count: 1,
    ..HostSettings::default()
  });
  game.display.activate_accessible("SETTINGS");
  game.display.activate_accessible("Input");
  self::open_binding(&mut game, "Move Piece", "controller");
  game.display.press_controller_button(ControllerButton::West);
  self::neutral(&mut game);
  self::open_binding(&mut game, "Restart", "controller");
  game
    .display
    .press_controller_button(ControllerButton::South);
  self::neutral(&mut game);
  let before = self::settings(&storage);
  self::open_binding(&mut game, "Move Piece", "controller");
  game.display.activate_accessible("Reset");
  game
    .display
    .semantic_node("Already used by \u{2068}Restart\u{2069}");
  assert_eq!(self::settings(&storage), before);
  game
    .display
    .press_controller_button(ControllerButton::North);
  self::neutral(&mut game);
  assert_eq!(
    self::settings(&storage).controller.move_piece,
    ControllerBinding::North
  );
  self::open_binding(&mut game, "Pause", "controller");
  game.display.press_controller_button(ControllerButton::East);
  self::neutral(&mut game);
  assert!(!self::has_label(&game, "Waiting for controller input"));
  assert_eq!(
    self::settings(&storage).controller.pause,
    ControllerBinding::Start
  );
}

use std::{
  fs,
  path::PathBuf,
  time::{Duration, Instant},
};

use battlement::{
  CommandBody, Connect, ControllerButton, PanelPoint, PhysicalKey, Quaternion, ScreenSize, Vector3,
};
use battlement_fake::assets::{FakeAssetCatalog, FakePrefab};
use battlement_fake::time::ManualClock;
use battlement_rules::{
  MUSIC_TRACKS, PIECE_PREFABS, PIECE_SPAWN_SEQUENCE_DURATION_MS, REACTANT_CHESS_ROOT_ID,
  ReactantChessApp,
  assets::{self, effects, sfx},
  audio::SOUND_EFFECTS,
  visual_state::VisualState,
};
use cozy_chess::{Board, Square};
use reactant::GameStatus;
use reactant_testing::Display;

#[test]
fn default_factory_exports_the_complete_reactant_app() {
  let mut display = Display::connect(
    battlement_rules::create_engine().expect("chess engine should initialize"),
    catalog(),
  );
  initial_ready(&mut display);
  assert!(!display.with_engine(|app| app.accepted_state()).started());
  assert_eq!(
    display.with_engine(|app| app.visual_state()),
    VisualState::Title
  );
  let _play = display.find_ui(REACTANT_CHESS_ROOT_ID, "play-chess");
}

#[test]
fn positioned_compatibility_factory_preserves_the_title_screen_contract() {
  let fen = "8/8/8/8/8/5kq1/8/7K b - - 0 1";
  let mut display = Display::connect(
    battlement_rules::create_engine_with_position(fen, Duration::ZERO)
      .expect("valid chess test position"),
    catalog(),
  );
  initial_ready(&mut display);
  assert!(!display.with_engine(|app| app.accepted_state()).started());
  assert_eq!(
    display.with_engine(|app| app.accepted_state()).board(),
    &fen.parse::<Board>().expect("valid chess test position")
  );
  let play = display.find_ui(REACTANT_CHESS_ROOT_ID, "play-chess");
  display.click_ui(play);
  ready(&mut display);
  assert!(display.with_engine(|app| app.accepted_state()).started());
}

#[test]
fn opening_uses_accessible_play_and_shared_spawn_checkpoints() {
  let mut display = title();
  assert!(display.global_keys().contains(&PhysicalKey::Enter));
  let controller = display.controller_input().expect("controller input");
  assert!(controller.buttons.contains(&ControllerButton::South));
  assert_eq!(controller.stick_dead_zone, Some(0.35));
  assert_eq!(controller.repeat_delay_ms, Some(275));
  assert_eq!(controller.repeat_interval_ms, Some(125));

  let play = display.find_ui(REACTANT_CHESS_ROOT_ID, "play-chess");
  display.click_ui(play);
  ready(&mut display);
  assert!(display.with_engine(|app| app.accepted_state()).started());
  assert!(
    display
      .audio_occurrences()
      .iter()
      .any(|occurrence| occurrence.address == sfx::ACCEPT)
  );
  assert!(
    display
      .audio_occurrences()
      .iter()
      .any(|occurrence| occurrence.address == MUSIC_TRACKS[0] && occurrence.looping)
  );

  display.advance_time(Duration::from_millis(79));
  assert!(display.particle_occurrences().is_empty());
  display.advance_time(Duration::from_millis(1));
  assert_eq!(display.particle_occurrences().len(), 4);
  display.advance_time(Duration::from_millis(PIECE_SPAWN_SEQUENCE_DURATION_MS - 80));
  assert_eq!(display.particle_occurrences().len(), 32);
  assert_eq!(
    display.presentation_time(),
    Duration::from_millis(PIECE_SPAWN_SEQUENCE_DURATION_MS)
  );
}

#[test]
fn pause_menu_accessible_reset_confirms_without_replaying_spawn_beats() {
  let mut display = title();
  let play = display.find_ui(REACTANT_CHESS_ROOT_ID, "play-chess");
  display.click_ui(play);
  ready(&mut display);
  display.settle();
  let spawn_count = display.particle_occurrences().len();

  display.key_down(PhysicalKey::Escape);
  display.key_up(PhysicalKey::Escape);
  for _ in 0..4 {
    display.poll();
  }
  let reset = display.find_ui(REACTANT_CHESS_ROOT_ID, "new-game");
  display.click_ui(reset);
  for _ in 0..4 {
    display.poll();
  }
  let reset = display.find_ui(REACTANT_CHESS_ROOT_ID, "new-game");
  display.click_ui(reset);
  ready(&mut display);

  assert_eq!(
    display
      .with_engine(|app| app.accepted_state())
      .visual_state(),
    VisualState::Refreshed
  );
  display.advance_time(Duration::from_secs(2));
  assert_eq!(display.particle_occurrences().len(), spawn_count);
  assert!(
    display
      .audio_occurrences()
      .iter()
      .any(|occurrence| occurrence.address == sfx::SCENE_TRANSITION)
  );
}

#[test]
fn music_uses_the_app_clock_and_crossfades_in_playlist_order() {
  let clock = ManualClock::new(Instant::now());
  let engine_clock = clock.clone();
  let engine =
    ReactantChessApp::with_think_time_and_clock(Duration::ZERO, move || engine_clock.now());
  let mut display = Display::connect(engine, catalog());
  initial_ready(&mut display);
  let play = display.find_ui(REACTANT_CHESS_ROOT_ID, "play-chess");
  display.click_ui(play);
  ready(&mut display);
  display.poll();

  clock.advance(Duration::from_secs(119));
  display.poll();
  assert_eq!(played_music(&display), vec![(MUSIC_TRACKS[0].as_str(), 0)]);
  clock.advance(Duration::from_secs(1));
  for _ in 0..8 {
    display.poll();
  }
  assert_eq!(
    played_music(&display),
    vec![
      (MUSIC_TRACKS[0].as_str(), 0),
      (MUSIC_TRACKS[1].as_str(), 5_000)
    ]
  );
  assert!(display.commands().iter().any(|entry| {
    matches!(&entry.command.body, CommandBody::AudioStop(stop) if stop.fade_out_ms == 5_000)
  }));

  display.settle();
  display.key_down(PhysicalKey::Escape);
  display.key_up(PhysicalKey::Escape);
  for _ in 0..4 {
    display.poll();
  }
  let reset = display.find_ui(REACTANT_CHESS_ROOT_ID, "new-game");
  display.click_ui(reset);
  for _ in 0..4 {
    display.poll();
  }
  let confirm = display.find_ui(REACTANT_CHESS_ROOT_ID, "new-game");
  display.click_ui(confirm);
  ready(&mut display);

  clock.advance(Duration::from_secs(119));
  display.poll();
  assert_eq!(played_music(&display).len(), 2);
  clock.advance(Duration::from_secs(1));
  for _ in 0..8 {
    display.poll();
  }
  assert_eq!(
    played_music(&display).last(),
    Some(&(MUSIC_TRACKS[2].as_str(), 5_000))
  );

  display.with_engine(|app| app.restart());
  ready(&mut display);
  assert_eq!(
    played_music(&display).last(),
    Some(&(MUSIC_TRACKS[0].as_str(), 5_000))
  );
  let after_restart = played_music(&display).len();
  clock.advance(Duration::from_secs(119));
  display.poll();
  assert_eq!(played_music(&display).len(), after_restart);
  clock.advance(Duration::from_secs(1));
  for _ in 0..8 {
    display.poll();
  }
  assert_eq!(
    played_music(&display).last(),
    Some(&(MUSIC_TRACKS[1].as_str(), 5_000))
  );
}

#[test]
fn keyboard_controller_click_and_fake_drag_share_the_app_owned_input_path() {
  let mut keyboard = title();
  keyboard.key_down(PhysicalKey::Enter);
  keyboard.key_up(PhysicalKey::Enter);
  ready(&mut keyboard);
  assert!(keyboard.with_engine(|app| app.accepted_state()).started());

  let mut controller = title();
  controller.controller_button_down(0, ControllerButton::South);
  controller.controller_button_up(0, ControllerButton::South);
  ready(&mut controller);
  assert!(controller.with_engine(|app| app.accepted_state()).started());

  let mut drag = position(&Board::default().to_string(), Duration::ZERO);
  let pawn = piece(&mut drag, Square::E2);
  assert!(matches!(
    drag.object(pawn).expect("piece host exists").kind(),
    battlement::GameObjectKind::BoxHitRegion { .. }
  ));
  assert_eq!(drag.world_point(pawn, Vector3::ZERO), square(Square::E2));
  let pointer = drag
    .object(pawn)
    .unwrap()
    .world_pointer_settings()
    .expect("piece world pointer settings");
  assert!(pointer.capture_on_press);
  assert!(!drag.object(pawn).unwrap().pointer_events().is_empty());
  let from = panel(Square::E2);
  let to = panel(Square::E4);
  drag.pointer_down(7, from);
  assert_eq!(drag.pointer_capture(7), Some(pawn));
  drag.poll();
  drag.poll();
  let _ = drag.find_ui(REACTANT_CHESS_ROOT_ID, "selection.legal-targets");
  drag.pointer_move(7, to, true);
  drag.pointer_up(7, to);
  ready(&mut drag);
  let accepted = drag.with_engine(|app| app.accepted_state());
  assert!(accepted.board().piece_on(Square::E2).is_none());
  assert_eq!(accepted.board().side_to_move(), cozy_chess::Color::White);
}

#[test]
fn fake_pointer_click_keeps_the_selected_white_source() {
  let mut display = position("4k3/8/8/4p3/3B4/8/8/4K3 w - - 0 1", Duration::ZERO);
  let bishop = piece(&mut display, Square::D4);
  display.activate(bishop);
  display.poll();
  display.poll();

  let victim = piece(&mut display, Square::E5);
  display.pointer_down(8, panel(Square::E5));
  assert_eq!(display.pointer_capture(8), Some(victim));
  display.pointer_up(8, panel(Square::E5));
  ready(&mut display);

  let accepted = display.with_engine(|app| app.accepted_state());
  assert!(accepted.board().piece_on(Square::D4).is_none());
  assert_eq!(
    accepted.board().piece_on(Square::E5),
    Some(cozy_chess::Piece::Bishop)
  );
}

#[test]
fn computer_checkmate_reaches_the_terminal_state_through_polling() {
  let mut display = position("8/8/8/8/8/5kq1/8/7K b - - 0 1", Duration::ZERO);
  ready(&mut display);
  let accepted = display.with_engine(|app| app.accepted_state());
  assert_eq!(accepted.board().status(), cozy_chess::GameStatus::Won);
  assert_eq!(accepted.visual_state(), VisualState::ComputerWin);
}

#[test]
fn capture_and_castle_register_move_owned_sound_timing() {
  let mut capture = position("4k3/8/8/4p3/3B4/8/8/4K3 w - - 0 1", Duration::ZERO);
  let bishop = piece(&mut capture, Square::D4);
  move_by_activation(&mut capture, Square::D4, Square::E5);
  ready(&mut capture);
  assert!(!capture.audio_occurrences().iter().any(|occurrence| {
    [sfx::ATTACK_A, sfx::ATTACK_B, sfx::ATTACK_C, sfx::ATTACK_D].contains(&occurrence.address)
  }));
  capture.advance_time(Duration::from_millis(299));
  assert_ne!(
    capture.object(bishop).unwrap().local_transform().position,
    square(Square::E5)
  );
  assert!(!capture.audio_occurrences().iter().any(|occurrence| {
    [sfx::ATTACK_A, sfx::ATTACK_B, sfx::ATTACK_C, sfx::ATTACK_D].contains(&occurrence.address)
  }));
  capture.advance_time(Duration::from_millis(1));
  assert_eq!(
    capture.object(bishop).unwrap().local_transform().position,
    square(Square::E5)
  );
  assert!(capture.audio_occurrences().iter().any(|occurrence| {
    [sfx::ATTACK_A, sfx::ATTACK_B, sfx::ATTACK_C, sfx::ATTACK_D].contains(&occurrence.address)
  }));
  assert_eq!(capture.particle_occurrences()[0].address, effects::CAPTURE);

  let mut knight = position("4k3/8/4p3/8/3N4/8/8/4K3 w - - 0 1", Duration::ZERO);
  move_by_activation(&mut knight, Square::D4, Square::E6);
  ready(&mut knight);
  knight.advance_time(Duration::from_millis(300));
  assert!(!knight.audio_occurrences().iter().any(|occurrence| {
    [sfx::ATTACK_A, sfx::ATTACK_B, sfx::ATTACK_C, sfx::ATTACK_D].contains(&occurrence.address)
  }));
  knight.advance_time(Duration::from_millis(20));
  assert!(knight.audio_occurrences().iter().any(|occurrence| {
    [sfx::ATTACK_A, sfx::ATTACK_B, sfx::ATTACK_C, sfx::ATTACK_D].contains(&occurrence.address)
  }));

  let mut castle = position("4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1", Duration::ZERO);
  move_by_activation(&mut castle, Square::E1, Square::G1);
  ready(&mut castle);
  assert!(
    castle
      .audio_occurrences()
      .iter()
      .any(|occurrence| occurrence.address == sfx::POWERUP_A)
  );
}

#[test]
fn en_passant_and_promotion_update_the_visible_piece_tree() {
  let mut en_passant = position("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1", Duration::ZERO);
  let pawn = piece(&mut en_passant, Square::E5);
  let victim = piece(&mut en_passant, Square::D5);
  move_by_activation(&mut en_passant, Square::E5, Square::D6);
  wait_for_visual(&mut en_passant, VisualState::EnPassant);
  en_passant.advance_time(Duration::from_millis(300));
  assert!(en_passant.object(victim).is_none());
  assert_eq!(
    en_passant.object(pawn).unwrap().local_transform().position,
    square(Square::D6)
  );
  let accepted = en_passant.with_engine(|app| app.accepted_state());
  assert!(accepted.piece(Square::D5).is_none());
  assert_eq!(accepted.visual_state(), VisualState::EnPassant);
  assert!(matches!(
    en_passant.particle_occurrences()[0].location,
    battlement::ParticleSpawnLocation::WorldPosition(position) if position == square(Square::D5)
  ));

  let mut promotion = position("1r2k3/P7/8/8/8/8/8/4K3 w - - 0 1", Duration::ZERO);
  let identity = promotion
    .with_engine(|app| app.accepted_state())
    .piece(Square::A7)
    .unwrap()
    .id;
  let pawn = piece(&mut promotion, Square::A7);
  let victim = piece(&mut promotion, Square::B8);
  move_by_activation(&mut promotion, Square::A7, Square::B8);
  wait_for_visual(&mut promotion, VisualState::Promotion);
  promotion.advance_time(Duration::from_millis(300));
  assert!(promotion.object(victim).is_none());
  assert!(promotion.object(pawn).is_some());
  let accepted = promotion.with_engine(|app| app.accepted_state());
  assert_eq!(accepted.piece(Square::B8).unwrap().id, identity);
  assert_eq!(
    accepted.piece(Square::B8).unwrap().kind,
    cozy_chess::Piece::Queen
  );
  assert!(promotion.objects().any(|object| {
    object.parent_id() == Some(pawn)
      && matches!(
        object.kind(),
        battlement::GameObjectKind::Prefab { address, .. }
          if address == &assets::white::QUEEN
      )
  }));
  assert_eq!(accepted.visual_state(), VisualState::Promotion);
}

#[test]
fn restart_replaces_busy_rules_and_required_presentation_without_stale_results() {
  let mut display = title();
  let play = display.find_ui(REACTANT_CHESS_ROOT_ID, "play-chess");
  display.click_ui(play);
  ready(&mut display);
  display.with_engine(|app| app.restart());
  ready(&mut display);
  assert_eq!(
    display
      .with_engine(|app| app.accepted_state())
      .visual_state(),
    VisualState::Restarted
  );
  display.advance_time(Duration::from_millis(80));
  assert_eq!(display.particle_occurrences().len(), 4);
  display.settle();
  display.poll();

  let pawn = piece(&mut display, Square::E2);
  display.activate(pawn);
  display.poll();
  display.poll();
  let target = highlight(&display, Square::E4);
  display.activate(target);
  assert_eq!(display.with_engine(|app| app.status()), GameStatus::Busy);
  display.with_engine(|app| app.restart());
  ready(&mut display);
  assert_eq!(
    display.with_engine(|app| app.accepted_state()).board(),
    &Board::default()
  );
  display.advance_time(Duration::from_secs(2));
  for _ in 0..8 {
    display.poll();
  }
  assert_eq!(
    display.with_engine(|app| app.accepted_state()).board(),
    &Board::default()
  );
  display.settle();
  display.poll();
  let pawn = piece(&mut display, Square::D2);
  display.activate(pawn);
  display.poll();
  display.poll();
  assert!(
    display
      .ui_element(display.find_ui(REACTANT_CHESS_ROOT_ID, "selection.legal-targets"))
      .text()
      .is_some()
  );
}

#[test]
fn accepted_state_drives_save_reload_and_survives_save_failure() {
  let directory = temporary("save");
  let connect = connection().persistent_data_path(directory.to_string_lossy());
  let mut display = Display::connect_with(ReactantChessApp::new(), catalog(), connect.clone());
  initial_ready(&mut display);
  let play = display.find_ui(REACTANT_CHESS_ROOT_ID, "play-chess");
  display.click_ui(play);
  ready(&mut display);
  assert!(directory.join("chess-game.json").is_file());

  let mut restored = Display::connect_with(ReactantChessApp::new(), catalog(), connect);
  initial_ready(&mut restored);
  assert!(restored.with_engine(|app| app.accepted_state()).started());
  fs::remove_dir_all(&directory).expect("temporary save directory cleanup");

  let blocked = temporary("blocked");
  fs::write(&blocked, b"not a directory").expect("temporary blocked path");
  let mut failed = Display::connect_with(
    ReactantChessApp::new(),
    catalog(),
    connection().persistent_data_path(blocked.to_string_lossy()),
  );
  initial_ready(&mut failed);
  let play = failed.find_ui(REACTANT_CHESS_ROOT_ID, "play-chess");
  failed.click_ui(play);
  ready(&mut failed);
  assert!(failed.with_engine(|app| app.accepted_state()).started());
  assert!(failed.with_engine(|app| app.persistence_error().is_some()));
  fs::remove_file(blocked).expect("temporary blocked path cleanup");
}

#[test]
fn player_move_is_saved_before_ai_and_a_black_turn_resumes_the_reply() {
  let directory = temporary("save-before-ai");
  let connect = connection().persistent_data_path(directory.to_string_lossy());
  let mut display = Display::connect_with(
    ReactantChessApp::with_think_time(Duration::from_secs(1)),
    catalog(),
    connect.clone(),
  );
  initial_ready(&mut display);
  let play = display.find_ui(REACTANT_CHESS_ROOT_ID, "play-chess");
  display.click_ui(play);
  ready(&mut display);
  for key in [
    PhysicalKey::Enter,
    PhysicalKey::ArrowUp,
    PhysicalKey::ArrowUp,
    PhysicalKey::Enter,
  ] {
    display.key_down(key);
    display.key_up(key);
    display.poll();
  }

  for _ in 0..128 {
    display.poll();
    let accepted = display.with_engine(|app| app.accepted_state());
    if accepted.board().side_to_move() == cozy_chess::Color::Black {
      break;
    }
    let _ = display.with_engine(|app| app.wait_for_output(Duration::from_millis(50)));
  }
  let accepted = display.with_engine(|app| app.accepted_state());
  assert_eq!(accepted.board().side_to_move(), cozy_chess::Color::Black);
  assert!(accepted.board().piece_on(Square::E2).is_none());
  assert!(directory.join("chess-game.json").is_file());
  drop(display);

  let mut restored = Display::connect_with(
    ReactantChessApp::with_think_time(Duration::ZERO),
    catalog(),
    connect,
  );
  initial_ready(&mut restored);
  assert_eq!(
    restored
      .with_engine(|app| app.accepted_state())
      .board()
      .side_to_move(),
    cozy_chess::Color::White
  );
  fs::remove_dir_all(directory).expect("temporary save directory cleanup");
}

#[test]
fn diagnostics_metadata_is_emitted_only_when_the_module_is_selected() {
  let mut connect = connection();
  connect.modules.push("battlement.diagnostics".to_owned());
  let mut display = Display::connect_with(ReactantChessApp::new(), catalog(), connect);
  initial_ready(&mut display);
  for _ in 0..4 {
    display.poll();
  }
  let metadata = display
    .commands()
    .iter()
    .filter_map(|entry| match &entry.command.body {
      CommandBody::Diagnostics(battlement_cloud::diagnostics::DiagnosticsCommand::SetMetadata(
        value,
      )) => Some((value.key.as_str(), value.value.as_deref())),
      _ => None,
    })
    .collect::<Vec<_>>();
  assert!(metadata.contains(&("sample.name", Some("chess"))));
  assert!(metadata.contains(&("chess.opponent", Some("computer"))));
  assert!(metadata.contains(&("chess.game_status", Some("ongoing"))));
  assert!(metadata.contains(&("chess.game_origin", Some("new"))));
}

fn title() -> Display<ReactantChessApp> {
  let mut display = Display::connect(ReactantChessApp::with_think_time(Duration::ZERO), catalog());
  initial_ready(&mut display);
  display
}

fn position(fen: &str, think_time: Duration) -> Display<ReactantChessApp> {
  let mut display = Display::connect(
    ReactantChessApp::with_position(fen, think_time).expect("valid chess test position"),
    catalog(),
  );
  initial_ready(&mut display);
  display
}

fn initial_ready(display: &mut Display<ReactantChessApp>) {
  ready(display);
}

fn ready(display: &mut Display<ReactantChessApp>) {
  for _ in 0..128 {
    display.poll();
    if display.with_engine(|app| app.status()) == GameStatus::Ready {
      display.poll();
      return;
    }
    let _ = display.with_engine(|app| app.wait_for_output(Duration::from_millis(100)));
  }
  panic!("Reactant chess output did not reach ready");
}

fn wait_for_visual(display: &mut Display<ReactantChessApp>, expected: VisualState) {
  for _ in 0..128 {
    display.poll();
    if display
      .with_engine(|app| app.accepted_state())
      .visual_state()
      == expected
    {
      return;
    }
    let _ = display.with_engine(|app| app.wait_for_output(Duration::from_millis(100)));
  }
  panic!("Reactant chess did not reach {expected:?}");
}

fn move_by_activation(display: &mut Display<ReactantChessApp>, from: Square, to: Square) {
  let piece = piece(display, from);
  display.activate(piece);
  for _ in 0..8 {
    display.poll();
  }
  let target = highlight(display, to);
  display.activate(target);
}

fn piece(display: &mut Display<ReactantChessApp>, square: Square) -> battlement::ObjectId {
  let identity = display
    .with_engine(|app| app.accepted_state())
    .piece(square)
    .expect("piece exists")
    .id;
  display
    .with_engine(|app| app.native_piece(identity))
    .expect("piece has native host")
}

fn highlight(display: &Display<ReactantChessApp>, square: Square) -> battlement::ObjectId {
  let expected = self::square(square);
  display
    .objects()
    .find(|object| {
      object.active_self()
        && object.local_transform().position.x == expected.x
        && object.local_transform().position.z == expected.z
        && matches!(object.kind(), battlement::GameObjectKind::Plane { .. })
    })
    .expect("active legal highlight exists")
    .id()
}

fn square(square: Square) -> Vector3 {
  Vector3::new(
    square.file() as u8 as f64 - 3.5,
    0.0,
    square.rank() as u8 as f64 - 3.5,
  )
}

fn panel(square: Square) -> PanelPoint {
  let camera = Vector3::new(0.0, 8.0, -3.75);
  let world = self::square(square);
  let offset = Vector3::new(world.x - camera.x, world.y - camera.y, world.z - camera.z);
  let rotation = Quaternion::new(0.58184814, -0.001219943, 0.0008727778, 0.813296);
  let local = rotate(
    Quaternion::new(-rotation.x, -rotation.y, -rotation.z, rotation.w),
    offset,
  );
  let width = 1_920.0;
  let height = 1_080.0;
  let tangent = (std::f64::consts::PI / 6.0).tan();
  let x = local.x / local.z / (width / height * tangent);
  let y = local.y / local.z / tangent;
  PanelPoint::new((x + 1.0) * width / 2.0, (1.0 - y) * height / 2.0)
}

fn rotate(rotation: Quaternion, value: Vector3) -> Vector3 {
  let dot = rotation.x * value.x + rotation.y * value.y + rotation.z * value.z;
  let length = rotation.x * rotation.x + rotation.y * rotation.y + rotation.z * rotation.z;
  Vector3::new(
    2.0 * dot * rotation.x
      + (rotation.w * rotation.w - length) * value.x
      + 2.0 * rotation.w * (rotation.y * value.z - rotation.z * value.y),
    2.0 * dot * rotation.y
      + (rotation.w * rotation.w - length) * value.y
      + 2.0 * rotation.w * (rotation.z * value.x - rotation.x * value.z),
    2.0 * dot * rotation.z
      + (rotation.w * rotation.w - length) * value.z
      + 2.0 * rotation.w * (rotation.x * value.y - rotation.y * value.x),
  )
}

fn connection() -> Connect {
  Connect::new("test", "test", ScreenSize::new(1_920, 1_080))
}

fn played_music(display: &Display<ReactantChessApp>) -> Vec<(&str, u64)> {
  display
    .commands()
    .iter()
    .filter_map(|entry| match &entry.command.body {
      CommandBody::AudioPlay(play) if play.address.as_str().starts_with("music/") => {
        Some((play.address.as_str(), play.fade_in_ms))
      }
      _ => None,
    })
    .collect()
}

fn temporary(label: &str) -> PathBuf {
  std::env::temp_dir().join(format!(
    "battlement-reactant-chess-{label}-{}",
    battlement::ObjectId::new_v4()
  ))
}

fn catalog() -> FakeAssetCatalog {
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene(assets::CONTENT);
  for address in PIECE_PREFABS {
    catalog.add_prefab(
      address,
      FakePrefab::new()
        .with_material_slots(1)
        .with_pointer_collider(),
    );
  }
  for address in MUSIC_TRACKS {
    catalog.add_audio_clip(address);
  }
  for address in SOUND_EFFECTS {
    catalog.add_audio_clip(address);
  }
  catalog.add_texture(assets::PLAY_BUTTON);
  catalog.add_texture(assets::REFRESH_BUTTON);
  catalog.add_material(assets::LEGAL_SQUARE);
  catalog.add_prefab(
    effects::PIECE_SELECTED,
    FakePrefab::new().with_particle_systems(),
  );
  catalog.add_particle_effect(effects::PIECE_SPAWN);
  catalog.add_particle_effect(effects::CAPTURE);
  catalog
}

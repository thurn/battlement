mod support;

use std::time::Duration;

use battlement::{CommandBody, PhysicalKey};
use battlement_rules::{ChessGame, contract};
use reactant_testing::GameActionResult;

use crate::support::{
  DEFAULT, MemoryPersistence, TIMEOUT, assert_state, click_play, client, client_with_modules,
  piece_at, play_move, press_key,
};

#[test]
fn accepted_move_is_observable_after_an_opaque_engine_reload() {
  let persistence = MemoryPersistence::empty();
  let mut first = client(persistence.clone(), Duration::from_secs(1));
  click_play(&mut first);
  first.settle();
  assert_eq!(
    first.game_action::<ChessGame>(TIMEOUT, |display| {
      for key in [
        PhysicalKey::Enter,
        PhysicalKey::ArrowUp,
        PhysicalKey::ArrowUp,
        PhysicalKey::Enter,
      ] {
        press_key(display, key);
      }
    }),
    GameActionResult::Completed
  );
  assert_state(&first, contract::marker::PLAYER_MOVE);
  drop(first);

  let restored = client(persistence, Duration::from_secs(1));
  assert!(piece_at(&restored, 'e', 2).is_none());
  assert!(piece_at(&restored, 'e', 4).is_some());
}

#[test]
fn persistence_failures_leave_a_valid_interactive_unity_state() {
  let failed_load = MemoryPersistence::with_save(DEFAULT);
  failed_load.fail_load();
  let mut load = client(failed_load, Duration::ZERO);
  click_play(&mut load);
  assert!(piece_at(&load, 'e', 2).is_some());

  let failed_store = MemoryPersistence::empty();
  failed_store.fail_store();
  let mut store = client(failed_store, Duration::ZERO);
  click_play(&mut store);
  assert!(piece_at(&store, 'e', 2).is_some());

  let failed_remove = MemoryPersistence::with_save(DEFAULT);
  failed_remove.fail_remove();
  let mut remove = client(failed_remove, Duration::ZERO);
  press_key(&mut remove, PhysicalKey::Escape);
  assert_state(&remove, contract::marker::PAUSED);
  let new_game = remove.find_ui(contract::ROOT_ID, "new-game");
  remove.click_ui(new_game);
  let confirm = remove.find_ui(contract::ROOT_ID, "new-game");
  remove.click_ui(confirm);
  assert_eq!(
    remove.settle_game::<ChessGame>(TIMEOUT),
    GameActionResult::Completed
  );
  assert_state(&remove, contract::marker::REFRESHED);
  assert!(piece_at(&remove, 'e', 2).is_some());
}

#[test]
fn diagnostics_metadata_exists_only_for_the_selected_host_module() {
  let absent = client(MemoryPersistence::with_save(DEFAULT), Duration::ZERO);
  assert!(absent.diagnostics().metadata().is_empty());

  let enabled = client_with_modules(
    MemoryPersistence::with_save(DEFAULT),
    Duration::ZERO,
    &["battlement.diagnostics"],
  );
  assert_eq!(
    enabled
      .diagnostics()
      .metadata()
      .get("sample.name")
      .map(String::as_str),
    Some("chess")
  );
  assert_eq!(
    enabled
      .diagnostics()
      .metadata()
      .get("chess.opponent")
      .map(String::as_str),
    Some("computer")
  );
  assert_eq!(
    enabled
      .diagnostics()
      .metadata()
      .get("chess.game_origin")
      .map(String::as_str),
    Some("saved")
  );
}

#[test]
fn music_crossfades_and_reset_does_not_replay_completed_opening_beats() {
  let mut display = client(MemoryPersistence::empty(), Duration::ZERO);
  click_play(&mut display);
  display.settle();
  let spawn_count = display.particle_occurrences().len();

  display.until_timer(|display| {
    display
      .audio_occurrences()
      .iter()
      .filter(|effect| contract::MUSIC_TRACKS.contains(&effect.address))
      .count()
      == 2
  });
  let played = display
    .audio_occurrences()
    .iter()
    .filter(|effect| contract::MUSIC_TRACKS.contains(&effect.address))
    .collect::<Vec<_>>();
  assert_eq!(played.len(), 2);
  assert_eq!(played[0].address, contract::MUSIC_TRACKS[0]);
  assert_eq!(played[1].address, contract::MUSIC_TRACKS[1]);
  assert!(display.commands().iter().any(|entry| matches!(
    &entry.command.body,
    CommandBody::AudioPlay(play)
      if play.address == contract::MUSIC_TRACKS[1] && play.fade_in_ms == 5_000
  )));
  assert!(display.commands().iter().any(|entry| matches!(
    &entry.command.body,
    CommandBody::AudioStop(stop) if stop.fade_out_ms == 5_000
  )));

  press_key(&mut display, PhysicalKey::Escape);
  assert_state(&display, contract::marker::PAUSED);
  let new_game = display.find_ui(contract::ROOT_ID, "new-game");
  display.click_ui(new_game);
  let confirm = display.find_ui(contract::ROOT_ID, "new-game");
  display.click_ui(confirm);
  assert_eq!(
    display.settle_game::<ChessGame>(TIMEOUT),
    GameActionResult::Completed
  );
  assert_state(&display, contract::marker::REFRESHED);
  display.settle();
  assert_eq!(display.particle_occurrences().len(), spawn_count);
  assert!(
    display
      .audio_occurrences()
      .iter()
      .any(|effect| effect.address == contract::RESET_SOUND)
  );
}

#[test]
fn restart_discards_pending_rules_work_and_restores_the_visible_board() {
  let mut display = client(MemoryPersistence::with_save(DEFAULT), Duration::ZERO);
  play_move(&mut display, ('e', 2), ('e', 4));
  for key in [
    PhysicalKey::ControlLeft,
    PhysicalKey::ShiftLeft,
    PhysicalKey::KeyR,
  ] {
    display.key_down(key);
  }
  for key in [
    PhysicalKey::KeyR,
    PhysicalKey::ShiftLeft,
    PhysicalKey::ControlLeft,
  ] {
    display.key_up(key);
  }
  assert_eq!(
    display.settle_game::<ChessGame>(TIMEOUT),
    GameActionResult::Completed
  );
  assert_state(&display, contract::marker::RESTARTED);
  assert!(piece_at(&display, 'e', 2).is_some());
  assert!(piece_at(&display, 'e', 4).is_none());
}

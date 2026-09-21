mod support;

use std::time::Duration;

use battlement::{CommandBody, PhysicalKey};
use battlement_rules::contract;

use crate::support::{
  DEFAULT, MemoryPersistence, click_play, client, client_with_modules, find_ui, move_piece,
  piece_at, wait_for_marker,
};

#[test]
fn accepted_move_is_observable_after_an_opaque_engine_reload() {
  let persistence = MemoryPersistence::empty();
  let mut first = client(persistence.clone(), Duration::from_secs(1));
  wait_for_marker(&mut first, contract::marker::TITLE);
  click_play(&mut first);
  first.settle();
  for key in [
    PhysicalKey::Enter,
    PhysicalKey::ArrowUp,
    PhysicalKey::ArrowUp,
    PhysicalKey::Enter,
  ] {
    first.key_down(key);
    first.key_up(key);
  }
  wait_for_marker(&mut first, contract::marker::PLAYER_MOVE);
  drop(first);

  let mut restored = client(persistence, Duration::from_secs(1));
  wait_for_marker(&mut restored, contract::marker::RESUMED);
  assert!(piece_at(&restored, 'e', 2).is_none());
  assert!(piece_at(&restored, 'e', 4).is_some());
}

#[test]
fn persistence_failures_leave_a_valid_interactive_unity_state() {
  let failed_load = MemoryPersistence::with_save(DEFAULT);
  failed_load.fail_load();
  let mut load_client = client(failed_load, Duration::ZERO);
  wait_for_marker(&mut load_client, contract::marker::TITLE);
  click_play(&mut load_client);
  assert!(piece_at(&load_client, 'e', 2).is_some());

  let failed_store = MemoryPersistence::empty();
  failed_store.fail_store();
  let mut store_client = client(failed_store, Duration::ZERO);
  wait_for_marker(&mut store_client, contract::marker::TITLE);
  click_play(&mut store_client);
  assert!(piece_at(&store_client, 'e', 2).is_some());

  let failed_remove = MemoryPersistence::with_save(DEFAULT);
  failed_remove.fail_remove();
  let mut remove_client = client(failed_remove, Duration::ZERO);
  wait_for_marker(&mut remove_client, contract::marker::RESUMED);
  remove_client.key_down(PhysicalKey::Escape);
  remove_client.key_up(PhysicalKey::Escape);
  wait_for_marker(&mut remove_client, contract::marker::PAUSED);
  let new_game = find_ui(&remove_client, "new-game").expect("new-game action");
  remove_client.ui().click(new_game);
  let confirm = find_ui(&remove_client, "new-game").expect("confirmation action");
  remove_client.ui().click(confirm);
  wait_for_marker(&mut remove_client, contract::marker::REFRESHED);
  assert!(piece_at(&remove_client, 'e', 2).is_some());
}

#[test]
fn diagnostics_metadata_exists_only_for_the_selected_host_module() {
  let mut absent = client(MemoryPersistence::with_save(DEFAULT), Duration::ZERO);
  wait_for_marker(&mut absent, contract::marker::RESUMED);
  assert!(absent.diagnostics().metadata().is_empty());

  let mut enabled = client_with_modules(
    MemoryPersistence::with_save(DEFAULT),
    Duration::ZERO,
    &["battlement.diagnostics"],
  );
  wait_for_marker(&mut enabled, contract::marker::RESUMED);
  for _ in 0..4 {
    enabled.poll();
  }
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
  let mut client = client(MemoryPersistence::empty(), Duration::ZERO);
  wait_for_marker(&mut client, contract::marker::TITLE);
  click_play(&mut client);
  client.settle();
  let spawn_count = client.particle_occurrences().len();

  client.advance_time(Duration::from_secs(120));
  for _ in 0..8 {
    client.poll();
  }
  let played = client
    .audio_occurrences()
    .iter()
    .filter(|effect| contract::MUSIC_TRACKS.contains(&effect.address))
    .collect::<Vec<_>>();
  assert_eq!(played.len(), 2);
  assert_eq!(played[0].address, contract::MUSIC_TRACKS[0]);
  assert_eq!(played[1].address, contract::MUSIC_TRACKS[1]);
  assert!(client.commands().iter().any(|entry| {
    matches!(
      &entry.command.body,
      CommandBody::AudioPlay(play)
        if play.address == contract::MUSIC_TRACKS[1] && play.fade_in_ms == 5_000
    )
  }));
  assert!(client.commands().iter().any(|entry| {
    matches!(&entry.command.body, CommandBody::AudioStop(stop) if stop.fade_out_ms == 5_000)
  }));

  client.key_down(PhysicalKey::Escape);
  client.key_up(PhysicalKey::Escape);
  wait_for_marker(&mut client, contract::marker::PAUSED);
  let new_game = find_ui(&client, "new-game").expect("new-game action");
  client.ui().click(new_game);
  let confirm = find_ui(&client, "new-game").expect("confirmation action");
  client.ui().click(confirm);
  wait_for_marker(&mut client, contract::marker::REFRESHED);
  client.advance_time(Duration::from_secs(2));
  assert_eq!(client.particle_occurrences().len(), spawn_count);
  assert!(
    client
      .audio_occurrences()
      .iter()
      .any(|effect| { effect.address == contract::RESET_SOUND })
  );
}

#[test]
fn restart_discards_pending_rules_work_and_restores_the_visible_board() {
  let mut client = client(MemoryPersistence::with_save(DEFAULT), Duration::ZERO);
  wait_for_marker(&mut client, contract::marker::RESUMED);
  move_piece(&mut client, ('e', 2), ('e', 4));
  for key in [
    PhysicalKey::ControlLeft,
    PhysicalKey::ShiftLeft,
    PhysicalKey::KeyR,
  ] {
    client.key_down(key);
  }
  for key in [
    PhysicalKey::KeyR,
    PhysicalKey::ShiftLeft,
    PhysicalKey::ControlLeft,
  ] {
    client.key_up(key);
  }
  wait_for_marker(&mut client, contract::marker::RESTARTED);
  assert!(piece_at(&client, 'e', 2).is_some());
  assert!(piece_at(&client, 'e', 4).is_none());
}

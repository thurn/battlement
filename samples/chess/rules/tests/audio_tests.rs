//! Shared audio preferences exercised through application input and host observations.
mod support;

use std::time::Duration;

use battlement::{
  AudioBus, CommandBody, PhysicalKey, UiAccessibilityAction, UiAccessibilityActionEvent, UiEvent,
  UiEventBody, application::ApplicationState,
};
use chess_rules::{
  assets,
  settings::{ChessSettings, Language},
};
use reactant_testing::MemoryPersistence;
use support::game::ChessTest;

#[test]
fn hydrated_french_preferences_apply_before_the_first_music_sample() {
  for master_volume in [0, 35] {
    let preferences = ChessSettings {
      language: Language::French,
      master_volume,
      music_volume: 40,
      ..ChessSettings::default()
    };
    let game = ChessTest::persisted(MemoryPersistence::with_file(
      "memory/chess-settings.json",
      &serde_json::to_vec(&preferences).unwrap(),
    ));
    let music: Vec<_> = game
      .display
      .audio_occurrences()
      .iter()
      .filter(|audio| audio.looping)
      .collect();
    assert_eq!(music.len(), 1);
    assert!(
      (music[0].mix_gain - f64::from(master_volume) * 0.004).abs() < 0.0001,
      "master={master_volume}, first={}, current={:?}",
      music[0].mix_gain,
      game.display.world().audio_mix()
    );
  }
}

#[test]
fn sliders_and_shortcuts_update_active_and_future_audio_without_restarting_music() {
  let mut game = ChessTest::persisted(MemoryPersistence::empty());
  let menu = game
    .display
    .audio_occurrences()
    .iter()
    .find(|audio| audio.looping)
    .unwrap()
    .clone();
  assert_eq!(menu.bus, AudioBus::Music);
  assert!((menu.mix_gain - 0.52).abs() < 0.0001);
  game.display.activate_accessible("SETTINGS");
  game.display.activate_accessible("Sound");
  self::increment(&mut game, "Master Volume");
  self::increment(&mut game, "Music Volume");
  self::increment(&mut game, "Effects Volume");
  let mix = game.display.world().audio_mix();
  assert!(mix.master > 0.8 && mix.music > 0.65 && mix.effects > 0.75);
  assert!(
    (game.display.audio(menu.command_id).unwrap().output_volume() - mix.master * mix.music).abs()
      < 0.0001
  );
  assert_eq!(
    game
      .display
      .audio_occurrences()
      .iter()
      .filter(|audio| audio.looping)
      .count(),
    1
  );
  game.display.activate_accessible("RETURN");
  game.start();
  let track = game
    .display
    .audio_occurrences()
    .iter()
    .rev()
    .find(|audio| audio.looping)
    .unwrap()
    .clone();
  assert_eq!(track.bus, AudioBus::Music);
  assert_ne!(track.command_id, menu.command_id);
  assert!(
    (game
      .display
      .audio(track.command_id)
      .unwrap()
      .output_volume()
      - mix.master * mix.music)
      .abs()
      < 0.0001
  );
  assert!(
    game
      .display
      .audio_occurrences()
      .iter()
      .filter(|audio| !audio.looping)
      .all(|audio| audio.bus == AudioBus::Effects)
  );
  let count = game
    .display
    .audio_occurrences()
    .iter()
    .filter(|audio| audio.looping)
    .count();
  game.display.send_key(PhysicalKey::Minus);
  let quieter = game.display.world().audio_mix();
  assert!((quieter.master - (mix.master - 0.1)).abs() < 0.0001);
  assert_eq!(quieter.music, mix.music);
  assert_eq!(
    game
      .display
      .audio_occurrences()
      .iter()
      .filter(|audio| audio.looping)
      .count(),
    count
  );
  game.show_menu();
  game.display.activate_accessible("Mute background music");
  assert_eq!(game.display.world().audio_mix().music, 0.0);
  assert_eq!(game.display.world().audio_mix().effects, mix.effects);
  game.start();
  assert_eq!(game.display.world().audio_mix().music, 0.0);
}

#[test]
fn saved_gameplay_bindings_take_priority_over_volume_shortcuts() {
  let mut preferences = ChessSettings::default();
  preferences.keyboard.left = PhysicalKey::Minus;
  let mut game = ChessTest::persisted(MemoryPersistence::with_file(
    "memory/chess-settings.json",
    &serde_json::to_vec(&preferences).unwrap(),
  ));
  game.start();
  game.display.send_key(PhysicalKey::Minus);
  assert_eq!(game.display.world().audio_mix().master, 0.8);
  game.display.send_key(PhysicalKey::Equal);
  assert_eq!(game.display.world().audio_mix().master, 0.9);
}

#[test]
fn background_policy_covers_focus_and_pause_without_replaying_or_changing_preferences() {
  let mut game = ChessTest::title();
  game.display.set_application_state(ApplicationState {
    focused: false,
    paused: false,
  });
  game.display.settle();
  assert!(!game.display.world().audio_mix().muted);
  game.display.activate_accessible("SETTINGS");
  game.display.activate_accessible("Sound");
  game.display.activate_accessible("Mute in Background");
  assert!(game.display.world().audio_mix().muted);
  game
    .display
    .set_application_state(ApplicationState::default());
  game.display.settle();
  assert!(!game.display.world().audio_mix().muted);
  game.display.activate_accessible("RETURN");
  game.start();
  let command_count = game.display.commands().len();
  let count = game.display.audio_occurrences().len();
  let mix = game.display.world().audio_mix();
  for state in [
    ApplicationState {
      focused: true,
      paused: true,
    },
    ApplicationState {
      focused: false,
      paused: false,
    },
  ] {
    game.display.set_application_state(state);
    game.display.settle();
    assert!(game.display.world().audio_mix().muted);
    for audio in game
      .display
      .audio_occurrences()
      .iter()
      .filter(|audio| audio.looping)
    {
      if let Some(active) = game.display.audio(audio.command_id) {
        assert_eq!(active.output_volume(), 0.0);
      }
    }
    game
      .display
      .set_application_state(ApplicationState::default());
    game.display.settle();
    assert_eq!(game.display.world().audio_mix(), mix);
    assert_eq!(game.display.audio_occurrences().len(), count);
    assert!(
      !game.display.commands()[command_count..]
        .iter()
        .any(|entry| matches!(
          entry.command.body,
          CommandBody::AudioPause(_) | CommandBody::AudioResume(_)
        )),
      "chess background mute must keep its playhead moving"
    );
  }
}

#[test]
fn playlist_restart_and_menu_transitions_keep_one_owned_music_lifecycle() {
  let mut game = ChessTest::title();
  game.start();
  let first = game
    .display
    .audio_occurrences()
    .iter()
    .rev()
    .find(|a| a.looping)
    .unwrap()
    .command_id;
  assert!(game.display.commands().iter().any(|entry| matches!(
    &entry.command.body, CommandBody::AudioPlay(play)
      if play.address == assets::music::CRITICAL && play.fade_in_ms == 0
  )));
  game.advance(Duration::from_secs(121));
  game
    .display
    .expect_crossfade(assets::music::SWITCH_WITH_ME, Duration::from_secs(5));
  let second = game
    .display
    .audio_occurrences()
    .iter()
    .rev()
    .find(|a| a.looping)
    .unwrap()
    .command_id;
  assert_ne!(first, second);
  game.restart();
  game.display.settle();
  game
    .display
    .expect_crossfade(assets::music::CRITICAL, Duration::from_secs(5));
  game.show_menu();
  for id in [first, second] {
    assert!(
      game.display.audio(id).is_none(),
      "retiring gameplay track survived menu"
    );
  }
  for _ in 0..2 {
    game.start();
    let before = game
      .display
      .audio_occurrences()
      .iter()
      .filter(|a| a.looping)
      .count();
    game.display.send_key(PhysicalKey::Minus);
    assert_eq!(
      before,
      game
        .display
        .audio_occurrences()
        .iter()
        .filter(|a| a.looping)
        .count()
    );
    game.show_menu();
  }
  let active = game
    .display
    .audio_occurrences()
    .iter()
    .filter(|a| a.looping && game.display.audio(a.command_id).is_some())
    .count();
  assert_eq!(active, 1, "only the menu track remains active");
}

fn increment(game: &mut ChessTest, label: &str) {
  game.display.deliver_ui_event(UiEvent {
    target_id: game.display.semantic_node(label).object_id,
    cancelable: true,
    default_prevented: false,
    body: UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
      backend_generation: 0,
      action: UiAccessibilityAction::Increment,
    }),
  });
  game.display.settle();
}

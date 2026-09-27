use std::time::Duration;

use battlement::{AudioBus, AudioClipAddress, CommandBody, application::ApplicationState};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{
  Application,
  audio::{self, AudioSettings, AudioTrack},
  hooks,
  prelude::*,
};
use reactant_testing::Display;
use trox::ls;

struct Fixture;
struct Playback {
  key: u32,
  alternate: bool,
  settings: AudioSettings,
}

impl Component for Playback {
  fn render(&self) -> impl Render {
    audio::use_audio(
      self.key,
      Some(AudioTrack {
        address: AudioClipAddress::from_static(if self.alternate { "audio/b" } else { "audio/a" }),
        bus: AudioBus::Music,
        looping: true,
      }),
      self.settings,
    );
  }
}

impl Component for Fixture {
  fn render(&self) -> impl Render {
    let (key, set_key) = hooks::use_state(0_u32);
    let (alternate, set_alternate) = hooks::use_state(false);
    let (mounted, set_mounted) = hooks::use_state(true);
    let (settings, set_settings) = hooks::use_state(AudioSettings {
      crossfade: Duration::from_millis(400),
      ..AudioSettings::default()
    });
    Stack::new().child((
      Button::new(ls("Occurrence")).on_press(set_key.update_callback(|v| v + 1)),
      Button::new(ls("Track")).on_press(set_alternate.update_callback(|v| !v)),
      Button::new(ls("Mount")).on_press(set_mounted.update_callback(|v| !v)),
      Button::new(ls("Gain"))
        .on_press(set_settings.update_callback(|v| AudioSettings { gain: 0.3, ..v })),
      Button::new(ls("Enable")).on_press(set_settings.update_callback(|v| AudioSettings {
        enabled: !v.enabled,
        ..v
      })),
      Button::new(ls("Pause")).on_press(set_settings.update_callback(|v| AudioSettings {
        paused: !v.paused,
        ..v
      })),
      mounted.then_some(Playback {
        key,
        alternate,
        settings,
      }),
    ))
  }
}

#[test]
fn volume_pause_and_background_preserve_playback_while_replacement_crossfades() {
  let mut display = Display::mount(
    || Application::new("audio/scene").child(Fixture),
    self::catalog(),
  );
  display.flush();
  assert_eq!(display.audio_occurrences().len(), 1);
  let id = display.audio_occurrences()[0].command_id;
  display.activate_accessible("Gain");
  display.flush();
  assert_eq!(display.audio_occurrences().len(), 1);
  assert_eq!(display.audio(id).unwrap().output_volume(), 0.3);
  display.activate_accessible("Pause");
  display.flush();
  assert!(
    display
      .commands()
      .iter()
      .any(|e| matches!(e.command.body, CommandBody::AudioPause(_)))
  );
  display.activate_accessible("Pause");
  display.set_application_state(ApplicationState {
    focused: false,
    paused: true,
  });
  display.flush();
  display.set_application_state(ApplicationState::default());
  display.flush();
  assert_eq!(display.audio_occurrences().len(), 1);
  assert!(
    display
      .commands()
      .iter()
      .any(|e| matches!(e.command.body, CommandBody::AudioResume(_)))
  );
  display.activate_accessible("Track");
  display.flush();
  display.expect_crossfade(
    AudioClipAddress::from_static("audio/b"),
    Duration::from_millis(400),
  );
  assert_eq!(display.audio_occurrences().len(), 2);
  display.activate_accessible("Occurrence");
  display.flush();
  assert_eq!(display.audio_occurrences().len(), 3);
  display.activate_accessible("Enable");
  display.flush();
  display.activate_accessible("Mount");
  display.flush();
  let stops = display
    .commands()
    .iter()
    .filter(|e| matches!(e.command.body, CommandBody::AudioStop(_)))
    .count();
  assert!(stops >= 3);
  display.activate_accessible("Enable");
  display.activate_accessible("Mount");
  display.flush();
  assert_eq!(display.audio_occurrences().len(), 4);
  display.activate_accessible("Mount");
  display.flush();
  assert!(
    display
      .commands()
      .iter()
      .filter(|e| matches!(e.command.body, CommandBody::AudioStop(_)))
      .count()
      > stops
  );
}

fn catalog() -> FakeAssetCatalog {
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene("audio/scene");
  catalog.add_audio_clip("audio/a");
  catalog.add_audio_clip("audio/b");
  catalog
}

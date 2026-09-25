//! Shared audio settings independent of individual playback lifetimes.

use battlement::{AudioMix, Command, CommandBody};

/// Applies master, category, and mute gains to active and future sounds.
pub fn set_mix(mix: AudioMix) -> Command {
  assert!(
    [mix.master, mix.music, mix.effects]
      .iter()
      .all(|gain| (0.0..=1.0).contains(gain)),
    "audio mixer gains must be between zero and one"
  );
  Command::new_v4(CommandBody::AudioSetMix(mix)).nonblocking()
}

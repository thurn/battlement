//! Shared audio settings independent of individual playback lifetimes.

use battlement::{AudioMix, Command, CommandBody};

use crate::{
  app_context,
  component::Component,
  hooks,
  render::{Children, Render},
};

/// Applies the initial mixer state before mounting playback-producing children.
pub struct AudioMixProvider {
  /// Current host gains for all active and future playback.
  pub mix: AudioMix,
  /// Playback consumers mounted after the initial mix has been sent.
  pub children: Children,
}

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

impl Component for AudioMixProvider {
  fn render(&self) -> impl Render {
    let app = app_context::use_app();
    let mix = self.mix;
    let (ready, set_ready) = hooks::use_state(false);
    hooks::use_effect(
      move || {
        app.send(self::set_mix(mix));
        set_ready.set(true);
      },
      mix,
    );
    ready.then(|| self.children.render())
  }
}

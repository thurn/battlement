use battlement::AudioMix;
use reactant::{hooks, prelude::*};
use serde::{Deserialize, Serialize};
use trox::ls;

use crate::match_ui;

#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct Preferences {
  pub larger_text: bool,
  pub reduced_motion: bool,
  pub quick_animation: bool,
}

pub(crate) struct Settings {
  pub preferences: Preferences,
  pub set_preferences: hooks::StateSetter<Preferences>,
  pub mix: AudioMix,
  pub set_mix: hooks::StateSetter<AudioMix>,
}

impl Preferences {
  pub fn font(self) -> f32 {
    if self.larger_text { 24.0 } else { 20.0 }
  }

  pub fn duration_scale(self) -> f64 {
    if self.quick_animation { 0.5 } else { 1.0 }
  }
}

impl Component for Settings {
  fn render(&self) -> impl Render {
    let music = self.set_mix.clone();
    let effects = self.set_mix.clone();
    let font = self.preferences.font();
    (
      Text::new(ls(
        "Sound levels change immediately. Your cards and score stay the same.",
      ))
      .style(match_ui::text_style(font)),
      Slider::new(ls("Music volume"), self.mix.music, 0.0, 1.0, 0.05)
        .value_text(ls(format!("{} percent", (self.mix.music * 100.0).round())))
        .style(Style::new().height(48.px()).font_size(font.px()))
        .on_change(move |value| {
          music.update(move |mix| AudioMix {
            music: value,
            ..mix
          })
        }),
      Slider::new(ls("Effects volume"), self.mix.effects, 0.0, 1.0, 0.05)
        .value_text(ls(format!(
          "{} percent",
          (self.mix.effects * 100.0).round()
        )))
        .style(Style::new().height(48.px()).font_size(font.px()))
        .on_change(move |value| {
          effects.update(move |mix| AudioMix {
            effects: value,
            ..mix
          })
        }),
      Button::new(ls(if self.preferences.reduced_motion {
        "Reduced motion: on"
      } else {
        "Reduced motion: automatic"
      }))
      .style(match_ui::button_style(font))
      .on_press(self.set_preferences.update_callback(|p| Preferences {
        reduced_motion: !p.reduced_motion,
        ..p
      })),
      Button::new(ls(if self.preferences.larger_text {
        "Text size: larger"
      } else {
        "Text size: standard"
      }))
      .style(match_ui::button_style(font))
      .on_press(self.set_preferences.update_callback(|p| Preferences {
        larger_text: !p.larger_text,
        ..p
      })),
      Button::new(ls(if self.preferences.quick_animation {
        "Animation pace: quick"
      } else {
        "Animation pace: relaxed"
      }))
      .style(match_ui::button_style(font))
      .on_press(self.set_preferences.update_callback(|p| Preferences {
        quick_animation: !p.quick_animation,
        ..p
      })),
    )
  }
}

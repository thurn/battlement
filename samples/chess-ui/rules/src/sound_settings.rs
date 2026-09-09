//! Controlled Sound settings composition backed by the shared music provider.

use battlement::Style;
use battlement_reactant::{control_behavior, prelude::*};
use trox::tx;

use crate::{font_scale, toggle_control::ToggleControl, volume_control::VolumeControl};

/// Source-ordered Sound settings controls.
#[builder]
pub struct SoundSettings {
  #[builder(required)]
  master_volume: u32,
  #[builder(required)]
  music_volume: u32,
  #[builder(required)]
  effects_volume: u32,
  #[builder(required)]
  mute_in_background: bool,
  #[builder(required)]
  on_master_volume_change: EventCallback<u32>,
  #[builder(required)]
  on_music_volume_change: EventCallback<u32>,
  #[builder(required)]
  on_effects_volume_change: EventCallback<u32>,
  #[builder(required)]
  on_mute_in_background_change: EventCallback<bool>,
}

impl Component for SoundSettings {
  fn render(&self) -> impl Render {
    let selected_scale = font_scale::use_font_scale();
    View::new()
      .name("sound-settings")
      .style(Style::new().height(971.0 * selected_scale.factor()))
      .child((
        VolumeControl::new()
          .label(tx("Master Volume", "Sound master-volume setting label."))
          .value(self.master_volume)
          .on_change(self.on_master_volume_change.clone())
          .first(true),
        VolumeControl::new()
          .label(tx("Music Volume", "Sound music-volume setting label."))
          .value(self.music_volume)
          .on_change(self.on_music_volume_change.clone()),
        VolumeControl::new()
          .label(tx("Effects Volume", "Sound effects-volume setting label."))
          .value(self.effects_volume)
          .on_change(self.on_effects_volume_change.clone()),
        ToggleControl::new()
          .label(
            control_behavior::name_source_text(tx(
              "Mute in\nBackground",
              "Two-line background-mute setting label.",
            ))
            .style(Style::new().height(112.24 * selected_scale.factor())),
          )
          .aria_label(tx(
            "Mute in Background",
            "Sound background-mute checkbox accessibility label.",
          ))
          .row_height(self::multiline_row_height(selected_scale))
          .checked(self.mute_in_background)
          .on_change(self.on_mute_in_background_change.clone()),
      ))
  }
}

fn multiline_row_height(scale: font_scale::FontScale) -> f32 {
  match scale {
    font_scale::FontScale::Percent100 => 159.0,
    font_scale::FontScale::Percent150 => 227.0,
    font_scale::FontScale::Percent200 => 211.0,
  }
}

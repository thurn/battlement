//! Generated settings surround with live, padded content.

use battlement::{ImageScaleMode, Overflow, PickingMode, Position, Style};
use reactant::prelude::*;

use crate::settings;

use crate::menu::{
  assets,
  font_scale::{self, FontScale},
};

/// Source-sized settings panel with 18/24/32-pixel content padding.
#[builder]
pub struct SettingsPanel {
  #[builder(required, into)]
  children: Children,
}

pub fn content_height(scale: FontScale, feedback: bool) -> f32 {
  if scale.factor() > 1.0 {
    if feedback { 470.0 } else { 680.0 }
  } else if feedback {
    791.0
  } else {
    971.0
  }
}

impl Component for SettingsPanel {
  fn render(&self) -> impl Render {
    let settings = settings::use_settings();
    let height = self::content_height(
      font_scale::use_font_scale(),
      settings.failed || settings.pending,
    ) + 50.0;
    View::new()
      .name("settings-panel")
      .style(
        Style::new()
          .position(Position::Relative)
          .width(887)
          .height(height)
          .overflow(Overflow::Hidden),
      )
      .child((
        assets::SETTINGS_PANEL_FRAME
          .image()
          .name("settings-panel-background")
          .picking_mode(PickingMode::Ignore)
          .scale_mode(ImageScaleMode::ScaleToFit)
          .style(
            Style::new()
              .position(Position::Absolute)
              .inset(0)
              .width(887)
              .height(height),
          ),
        View::new()
          .name("settings-panel-content")
          .style(
            Style::new()
              .position(Position::Relative)
              .full_size()
              .padding_top(18)
              .padding_right(24)
              .padding_bottom(32)
              .padding_left(24),
          )
          .child(self.children.render()),
      ))
  }
}

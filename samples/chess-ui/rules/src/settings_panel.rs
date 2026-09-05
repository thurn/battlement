//! Generated settings surround with live, padded content.

use battlement::{ImageScaleMode, Overflow, PickingMode, Position, Style};
use battlement_reactant::prelude::*;

use crate::assets;

/// Source-sized settings panel with 18/24/32-pixel content padding.
#[builder]
pub struct SettingsPanel {
  #[builder(required, into)]
  children: Children,
}

impl Component for SettingsPanel {
  fn render(&self) -> impl Render {
    View::new()
      .name("settings-panel")
      .style(
        Style::new()
          .position(Position::Relative)
          .width(887)
          .height(1021)
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
              .height(1021),
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

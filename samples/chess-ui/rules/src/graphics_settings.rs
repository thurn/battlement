//! Controlled graphics settings composed from shared arcade controls.

use battlement::{Position, Style};
use battlement_reactant::{control_behavior, portal::PortalTarget, prelude::*};
use trox::tx;

use crate::{select_control::SelectControl, toggle_control::ToggleControl};

/// The source Graphics panel; its parent owns every accepted value.
#[builder]
pub struct GraphicsSettings {
  #[builder(required)]
  resolution: String,
  #[builder(required)]
  max_framerate: String,
  #[builder(required)]
  display_mode: String,
  #[builder(required)]
  screenshake: bool,
  #[builder(required)]
  vsync: bool,
  #[builder(required)]
  overlay: PortalTarget,
  #[builder(required)]
  on_resolution_change: EventCallback<String>,
  #[builder(required)]
  on_max_framerate_change: EventCallback<String>,
  #[builder(required)]
  on_display_mode_change: EventCallback<String>,
  #[builder(required)]
  on_screenshake_change: EventCallback<bool>,
  #[builder(required)]
  on_vsync_change: EventCallback<bool>,
}

impl Component for GraphicsSettings {
  fn render(&self) -> impl Render {
    View::new()
      .name("graphics-settings")
      .style(Style::new().position(Position::Relative).height(971))
      .child((
        SelectControl::new()
          .label(control_behavior::name_source_text(tx(
            "Resolution",
            "Graphics resolution setting label.",
          )))
          .value(self.resolution.clone())
          .options(
            ["1920 × 1080", "2560 × 1440", "3840 × 2160"]
              .map(String::from)
              .to_vec(),
          )
          .overlay(self.overlay.clone())
          .on_change(self.on_resolution_change.clone())
          .first(true),
        SelectControl::new()
          .label(control_behavior::name_source_text(tx(
            "Max Framerate",
            "Graphics frame-rate setting label.",
          )))
          .value(self.max_framerate.clone())
          .options(
            ["60 FPS", "120 FPS", "144 FPS", "240 FPS"]
              .map(String::from)
              .to_vec(),
          )
          .overlay(self.overlay.clone())
          .on_change(self.on_max_framerate_change.clone()),
        SelectControl::new()
          .label(control_behavior::name_source_text(tx(
            "Display Mode",
            "Graphics display-mode setting label.",
          )))
          .value(self.display_mode.clone())
          .options(
            ["Borderless", "Fullscreen", "Windowed"]
              .map(String::from)
              .to_vec(),
          )
          .overlay(self.overlay.clone())
          .on_change(self.on_display_mode_change.clone()),
        ToggleControl::new()
          .label(control_behavior::name_source_text(tx(
            "Screenshake",
            "Graphics screenshake setting label.",
          )))
          .checked(self.screenshake)
          .on_change(self.on_screenshake_change.clone()),
        ToggleControl::new()
          .label(control_behavior::name_source_text(tx(
            "VSync",
            "Graphics vertical-sync setting label.",
          )))
          .checked(self.vsync)
          .on_change(self.on_vsync_change.clone()),
      ))
  }
}

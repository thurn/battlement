//! Main menu and settings for the chess application.

mod action_button;
mod action_skin;
mod arcade_attract_mode;
mod arcade_exit_sequence;
mod arcade_frame_pulse;
mod arcade_menu_transition;
mod arcade_modal;
mod arcade_route_transition;
mod arcade_screen_router;
mod arcade_tab_transition;
mod assets;
mod background_music;
mod caret;
mod check_mark;
mod concept_frame;
mod control_effects;
mod dropdown_motion;
mod erase_control;
mod erase_dialog;
mod font_scale;
mod frame_styles;
mod graphics_settings;
mod header_artwork;
mod input_binding_icons;
mod input_labels;
mod input_settings;
mod main_menu;
mod music_heartbeat;
mod music_playback_indicator;
mod portrait_viewport;
mod privacy_policy;
mod return_button;
mod screen_frame;
mod screen_header;
mod select_control;
mod select_navigation;
mod select_option;
mod select_popover;
mod setting_row;
mod settings_panel;
mod settings_screen;
mod settings_tabs;
mod sound_settings;
mod tabs_navigation;
mod tabs_skin;
mod toggle_control;
mod use_interaction;
mod volume_control;
mod volume_input;
mod volume_skin;

use battlement::{Color, LengthUnits, Overflow, Style};
use reactant::{host::Stack, overlay::OverlayHost, prelude::*};

use crate::menu::{
  arcade_route_transition::ArcadeRouteTransition, arcade_screen_router::ArcadeScreenRouter,
  background_music::BackgroundMusicProvider, portrait_viewport::PortraitViewport,
};

/// Presents the main menu and settings over the chess world.
pub struct ChessMenu {
  pub active: bool,
  pub on_play: EventCallback<()>,
}

impl Component for ChessMenu {
  fn render(&self) -> impl Render {
    let overlay = reactant::use_portal_target();
    BackgroundMusicProvider::new()
      .autoplay(true)
      .active(self.active)
      .children(ArcadeRouteTransition::new().children(self.active.then(|| {
        Stack::new()
          .style(
            Style::new()
              .width(100.pct())
              .height(100.pct())
              .overflow(Overflow::Hidden)
              .background_color(Color::BLACK),
          )
          .child(
            PortraitViewport::new().child(
              ArcadeScreenRouter::new()
                .overlay(overlay.clone())
                .on_play(self.on_play.clone()),
            ),
          )
          .child(OverlayHost::new(overlay))
      })))
  }
}

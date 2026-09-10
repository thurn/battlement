//! Native entry point for the complete Chess UI mockup.

use battlement::{Color, LengthUnits, Overflow, Style, UiDocument};
use battlement_reactant::{app::App, host::Stack, overlay::OverlayHost};

use crate::{
  arcade_route_transition::ArcadeRouteTransition, arcade_screen_router::ArcadeScreenRouter,
  background_music::BackgroundMusicProvider, font_scale::FontScaleProvider,
  portrait_viewport::PortraitViewport,
};

/// Creates the complete Chess UI mockup.
pub fn create_engine() -> App {
  let mut app = App::new("chess-ui/content");
  let overlay = app.create_portal_target();
  app
    .ui(
      Stack::new()
        .style(Style::new().width(100.pct()).height(100.pct()))
        .child(
          BackgroundMusicProvider::new().autoplay(true).children(
            FontScaleProvider::new().children(
              PortraitViewport::new().child(
                ArcadeRouteTransition::new()
                  .children(ArcadeScreenRouter::new().overlay(overlay.clone())),
              ),
            ),
          ),
        )
        .child(OverlayHost::new(overlay)),
    )
    .background(Color::BLACK)
    .document(self::document)
    .reset_on_reconnect()
}

fn document(document: UiDocument) -> UiDocument {
  document.style(
    Style::new()
      .full_size()
      .overflow(Overflow::Hidden)
      .background_color(Color::BLACK),
  )
}

battlement_native::export_deterministic_engine!(
  self::create_engine,
  clock = virtualized,
  randomness = seeded,
  external_state = isolated,
  persistent_state = reset,
  input = semantic,
  visible_output = protocol_owned,
);

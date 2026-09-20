//! Native entry point for the complete Chess UI mockup.

use battlement::{Color, LengthUnits, Overflow, Style, UiDocument};
use reactant::{
  Application,
  host::Stack,
  overlay::OverlayHost,
  prelude::{Component, Render},
};

use crate::{
  arcade_route_transition::ArcadeRouteTransition, arcade_screen_router::ArcadeScreenRouter,
  background_music::BackgroundMusicProvider, font_scale::FontScaleProvider,
  portrait_viewport::PortraitViewport,
};

struct ChessUiRoot;

/// Creates the complete Chess UI mockup.
pub fn application() -> Application {
  Application::new("chess-ui/content")
    .child(ChessUiRoot)
    .background(Color::BLACK)
    .document(self::document)
}

impl Component for ChessUiRoot {
  fn render(&self) -> impl Render {
    let overlay = reactant::use_portal_target();
    Stack::new()
      .style(Style::new().width(100.pct()).height(100.pct()))
      .child(BackgroundMusicProvider::new().autoplay(true).children(
        FontScaleProvider::new().children(PortraitViewport::new().child(
          ArcadeRouteTransition::new().children(ArcadeScreenRouter::new().overlay(overlay.clone())),
        )),
      ))
      .child(OverlayHost::new(overlay))
  }
}

fn document(document: UiDocument) -> UiDocument {
  document.style(
    Style::new()
      .full_size()
      .overflow(Overflow::Hidden)
      .background_color(Color::BLACK),
  )
}

reactant::export_application!(application);

//! Native entry point and application configuration for the gallery.

use battlement::{LengthUnits, Style};
use battlement_reactant::{app::App, host::Stack, overlay::OverlayHost};

use crate::{pages, review_surface::ReviewSurface, review_theme};

/// Creates the Chess UI gallery.
pub fn create_engine() -> App {
  let mut app = App::new("chess-ui/content");
  let overlay = app.create_portal_target();
  app
    .ui(
      Stack::new()
        .style(Style::new().width(100.pct()).height(100.pct()))
        .child(pages::gallery(overlay.clone()))
        .child(OverlayHost::new(overlay)),
    )
    .background(review_theme::BACKGROUND)
    .document(ReviewSurface::document)
    .reset_on_reconnect()
}

battlement_native::export_engine!(self::create_engine);

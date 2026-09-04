//! Native entry point and application configuration for the gallery.

use battlement_reactant::app::App;

use crate::{pages, review_surface::ReviewSurface, review_theme};

/// Creates the Chess UI gallery.
pub fn create_engine() -> App {
  App::new("chess-ui/content")
    .ui(pages::gallery())
    .background(review_theme::BACKGROUND)
    .document(ReviewSurface::document)
    .reset_on_reconnect()
}

battlement_native::export_engine!(self::create_engine);

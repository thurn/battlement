use battlement_reactant::app::App;

use crate::{gallery::Gallery, review_surface::ReviewSurface, review_theme};

/// Creates the Chess UI gallery.
pub fn create_engine() -> App {
  App::new("chess-ui/content", Gallery)
    .background(review_theme::BACKGROUND)
    .document(ReviewSurface::document)
    .reset_on_reconnect()
}

battlement_native::export_engine!(self::create_engine);

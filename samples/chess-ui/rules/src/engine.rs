//! Native entry point and application configuration for the gallery.

use battlement_reactant::app::App;
use trox::Bundle;

use crate::{pages, review_surface::ReviewSurface, review_theme};

/// Creates the Chess UI gallery.
pub fn create_engine() -> App {
  let source = Bundle::from_canonical_json(include_str!("../../localization/en-US.trox.json"))
    .expect("valid embedded English trox bundle");

  App::new("chess-ui/content")
    .source_bundle(source)
    .ui(pages::gallery())
    .background(review_theme::BACKGROUND)
    .document(ReviewSurface::document)
    .reset_on_reconnect()
}

battlement_native::export_engine!(self::create_engine);

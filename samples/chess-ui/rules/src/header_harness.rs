use trox::tx;

use crate::{
  review_button::ReviewButton,
  screen_header::{HeaderVariant, ScreenHeader},
};
use battlement::{Color, Position, Style};
use battlement_reactant::{hooks, prelude::*};

/// Selects independent heading specimens on a source-sized canvas.
#[builder]
pub struct HeaderHarness;

impl Component for HeaderHarness {
  fn render(&self) -> impl Render {
    let (variant, set_variant) = hooks::use_state(HeaderVariant::Game);
    View::new()
      .style(Style::new().position(Position::Absolute).inset(0))
      .child((
        View::new()
          .name("header-specimen")
          .style(
            Style::new()
              .position(Position::Absolute)
              .left(0)
              .top(0)
              .width(1024)
              .height(480)
              .background_color(Color::BLACK),
          )
          .child(ScreenHeader::new().variant(variant)),
        View::new()
          .style(
            Style::new()
              .position(Position::Absolute)
              .left(64)
              .top(530)
              .width(500),
          )
          .child((
            ReviewButton::new()
              .label(tx("Show game heading", "Heading specimen selector."))
              .on_press(set_variant.callback().map_input(|_| HeaderVariant::Game)),
            ReviewButton::new()
              .label(tx("Show settings heading", "Heading specimen selector."))
              .on_press(
                set_variant
                  .callback()
                  .map_input(|_| HeaderVariant::Settings),
              ),
          )),
      ))
  }
}

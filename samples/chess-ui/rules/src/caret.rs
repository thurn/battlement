//! A decorative chevron whose orientation follows a selector’s open state.

use battlement::{Color, Length, Position, Rotate, Style, Translate};
use battlement_reactant::prelude::builder;
use battlement_reactant::{
  component::Component,
  host::View,
  paint::{PaintFill, PaintStyle},
  render::Render,
};

use crate::font_scale::{self, FontScaleRole};

/// The decorative direction indicator on a select trigger.
#[builder]
pub struct Caret {
  /// Points upward while the selector is open.
  is_open: bool,
}

impl Component for Caret {
  fn render(&self) -> impl Render {
    let font_scale = font_scale::use_font_scale();
    View::decorative()
      .name("select-caret")
      .style(
        Style::new()
          .position(Position::Absolute)
          .top(Length::Percent(50.0))
          .right(45.0 * font_scale.dynamic(FontScaleRole::Control))
          .width(30.0 * font_scale.dynamic(FontScaleRole::Control))
          .height(18.0 * font_scale.dynamic(FontScaleRole::Control))
          .translate(Translate::two_dimensional(
            Length::Px(0.0),
            Length::Percent(-50.0),
          ))
          .rotate(Rotate::degrees(if self.is_open { 180.0 } else { 0.0 })),
      )
      .paint(
        PaintStyle::new()
          .background(PaintFill::Color(Color::hex(0xf4f5fa)))
          .clip_polygon([
            [Length::percent(0.0), Length::percent(0.0)],
            [Length::percent(100.0), Length::percent(0.0)],
            [Length::percent(50.0), Length::percent(100.0)],
          ]),
      )
  }
}

impl Default for Caret {
  fn default() -> Self {
    Self::new()
  }
}

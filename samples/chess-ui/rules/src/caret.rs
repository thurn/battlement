//! A decorative chevron whose orientation follows a selector’s open state.

use battlement::{Color, Length, Position, Rotate, Style, Translate};
use battlement_reactant::prelude::{StyleTarget, builder};
use battlement_reactant::{
  component::Component,
  host::View,
  motion_config,
  paint::{PaintFill, PaintStyle},
  render::Render,
};

use crate::{
  dropdown_motion,
  font_scale::{self, FontScaleRole},
};

/// The decorative direction indicator on a select trigger.
#[builder]
pub struct Caret {
  /// Points upward while the selector is open.
  is_open: bool,
}

impl Component for Caret {
  fn render(&self) -> impl Render {
    let font_scale = font_scale::use_font_scale();
    let reduced_motion = motion_config::use_reduced_motion();
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
          .rotate(Rotate::degrees(if reduced_motion {
            if self.is_open { 180.0 } else { 0.0 }
          } else {
            0.0
          })),
      )
      .initial(false)
      .animate(if reduced_motion {
        StyleTarget::new()
      } else {
        StyleTarget::new().rotate(if self.is_open { 180.0 } else { 0.0 })
      })
      .transition(dropdown_motion::caret_transition(reduced_motion))
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

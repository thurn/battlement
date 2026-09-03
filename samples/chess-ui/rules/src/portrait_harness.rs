use battlement::{Align, Color, Justify, LengthUnits, Style, TextAnchor};
use battlement_reactant::{
  accessibility,
  component::Component,
  host::{Label, View},
  render::Render,
  semantics,
};

use crate::portrait_viewport::PortraitViewport;

/// Marks the corners of a fixed canvas to demonstrate fitting.
pub struct PortraitHarness;

impl Component for PortraitHarness {
  fn render(&self) -> impl Render {
    View::new()
      .style(Style::new().flex_grow(1).min_height(0).margin_top(48))
      .child(
        PortraitViewport::new().child(
          View::new()
            .name("portrait-specimen")
            .style(
              Style::new()
                .width(100.pct())
                .height(100.pct())
                .border_width(2)
                .border_color(Color::rgb(0.38, 0.94, 0.90))
                .padding(32)
                .justify_content(Justify::SpaceBetween)
                .align_items(Align::Stretch),
            )
            .child((
              Label::new("TOP LEFT").style(
                Style::new()
                  .font_size(32)
                  .unity_text_align(TextAnchor::UpperLeft),
              ),
              Label::new("1024 × 1536")
                .semantic(accessibility::use_static_text(semantics::text(
                  "Portrait canvas, 1024 by 1536 logical pixels",
                )))
                .style(
                  Style::new()
                    .font_size(64)
                    .unity_text_align(TextAnchor::MiddleCenter),
                ),
              Label::new("BOTTOM RIGHT").style(
                Style::new()
                  .font_size(32)
                  .unity_text_align(TextAnchor::LowerRight),
              ),
            )),
        ),
      )
  }
}

use trox::tx;

use crate::portrait_viewport::PortraitViewport;
use battlement::{Align, Color, Justify, LengthUnits, Style, TextAnchor};
use battlement_reactant::prelude::builder;
use battlement_reactant::{
  component::Component,
  control_behavior,
  host::{Label, View},
  render::Render,
};

/// Marks the corners of a fixed canvas to demonstrate fitting.
#[builder]
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
              Label::new(tx(
                "TOP LEFT",
                "User-facing product copy in the Chess UI sample.",
              ))
              .style(
                Style::new()
                  .font_size(32)
                  .unity_text_align(TextAnchor::UpperLeft),
              ),
              Label::new(tx(
                "1024 × 1536",
                "User-facing product copy in the Chess UI sample.",
              ))
              .semantic(control_behavior::static_text_props(tx(
                "Portrait canvas, 1024 by 1536 logical pixels",
                "User-facing product copy in the Chess UI sample.",
              )))
              .style(
                Style::new()
                  .font_size(64)
                  .unity_text_align(TextAnchor::MiddleCenter),
              ),
              Label::new(tx(
                "BOTTOM RIGHT",
                "User-facing product copy in the Chess UI sample.",
              ))
              .style(
                Style::new()
                  .font_size(32)
                  .unity_text_align(TextAnchor::LowerRight),
              ),
            )),
        ),
      )
  }
}

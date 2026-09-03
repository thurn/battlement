use crate::{portrait_viewport::PortraitViewport, screen_frame::ScreenFrame};
use battlement::{Justify, LengthUnits, Style, TextAnchor};
use battlement_reactant::prelude::builder;
use battlement_reactant::{
  accessibility,
  component::Component,
  host::{Label, View},
  render::Render,
  semantics,
};

/// Displays the arcade border around a centered content label.
#[builder]
pub struct FrameHarness;

impl Component for FrameHarness {
  fn render(&self) -> impl Render {
    View::new()
      .style(Style::new().flex_grow(1).min_height(0).margin_top(48))
      .child(
        PortraitViewport::new().child(
          ScreenFrame::new().children(
            View::new()
              .style(
                Style::new()
                  .width(100.pct())
                  .height(100.pct())
                  .justify_content(Justify::Center),
              )
              .child(
                Label::new("ARCADE FRAME")
                  .semantic(accessibility::use_static_text(semantics::text(
                    "Arcade frame content",
                  )))
                  .style(
                    Style::new()
                      .font_size(72)
                      .unity_text_align(TextAnchor::MiddleCenter),
                  ),
              ),
          ),
        ),
      )
  }
}

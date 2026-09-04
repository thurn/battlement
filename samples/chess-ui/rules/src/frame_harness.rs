use trox::tx;

use crate::{portrait_viewport::PortraitViewport, screen_frame::ScreenFrame};
use battlement::{Justify, LengthUnits, Style, TextAnchor};
use battlement_reactant::prelude::builder;
use battlement_reactant::{
  component::Component,
  control_behavior,
  host::{Label, View},
  render::Render,
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
                Label::new(tx(
                  "ARCADE FRAME",
                  "User-facing product copy in the Chess UI sample.",
                ))
                .semantic(control_behavior::static_text_props(tx(
                  "Arcade frame content",
                  "User-facing product copy in the Chess UI sample.",
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

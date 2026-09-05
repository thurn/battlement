//! Interactive display-mode selector specimen.

use trox::tx;

use crate::select_control::SelectControl;
use battlement::{Color, Style};
use battlement_reactant::{control_behavior, hooks, portal::PortalTarget, prelude::*};

/// Owns the accepted display mode for the pointer popover review page.
#[builder]
pub struct SelectPopoverHarness {
  #[builder(required)]
  overlay: PortalTarget,
}

impl Component for SelectPopoverHarness {
  fn render(&self) -> impl Render {
    let (value, set_value) = hooks::use_state(String::from("Borderless"));
    View::new()
      .name("select-popover-specimen")
      .style(
        Style::new()
          .width(839)
          .height(480)
          .margin_top(48)
          .background_color(Color::rgb(0.01, 0.035, 0.08)),
      )
      .child(
        SelectControl::new()
          .label(control_behavior::name_source_text(tx(
            "Display Mode",
            "Display mode selector interface label.",
          )))
          .value(value)
          .options(vec![
            String::from("Borderless"),
            String::from("Fullscreen"),
            String::from("Windowed"),
          ])
          .overlay(self.overlay.clone())
          .on_change(set_value)
          .first(true),
      )
  }
}

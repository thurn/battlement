use trox::tx;

use crate::{review_button::ReviewButton, select_control::SelectControl};
use battlement::{Color, Style};
use battlement_reactant::{control_behavior, hooks, prelude::*};

/// Owns selected values and demonstrates parent-driven updates.
#[builder]
pub struct SelectHarness;

impl Component for SelectHarness {
  fn render(&self) -> impl Render {
    let (high_resolution, set_high_resolution) = hooks::use_state(false);
    View::new()
      .name("select-specimen")
      .style(
        Style::new()
          .width(839)
          .margin_top(48)
          .background_color(Color::rgb(0.01, 0.035, 0.08)),
      )
      .child((
        SelectControl::new()
          .label(self::label(tx(
            "Resolution",
            "User-facing product copy in the Chess UI sample.",
          )))
          .value(if high_resolution {
            "2560 × 1440"
          } else {
            "1920 × 1080"
          })
          .first(true),
        SelectControl::new()
          .label(self::label(tx(
            "Display Mode",
            "User-facing product copy in the Chess UI sample.",
          )))
          .value("Borderless")
          .row_height(190.0)
          .offset_y(-8.0),
        control_behavior::static_label(tx(
          "Selection changes: 0",
          "User-facing product copy in the Chess UI sample.",
        ))
        .style(
          Style::new()
            .font_size(28)
            .color(Color::rgb(0.75, 0.86, 0.97))
            .margin_top(30),
        ),
        ReviewButton::new()
          .label(tx(
            "Change resolution from parent",
            "User-facing product copy in the Chess UI sample.",
          ))
          .on_press(move || set_high_resolution.update(|value| !value)),
      ))
  }
}

fn label(value: LocalizedString) -> TextElement {
  control_behavior::name_source_text(value)
}

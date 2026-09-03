use crate::{review_button::ReviewButton, select_control::SelectControl};
use battlement::{Color, Style};
use battlement_reactant::{accessibility, hooks, prelude::*};

/// Owns selected values and demonstrates parent-driven updates.
#[builder]
pub struct SelectHarness;

impl Component for SelectHarness {
  fn render(&self) -> impl Render {
    let (high_resolution, set_high_resolution) = hooks::use_state(false);
    let value = if high_resolution {
      "2560 × 1440"
    } else {
      "1920 × 1080"
    };
    let external = set_high_resolution.clone();
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
          .label(self::label("Resolution"))
          .value(value)
          .first(true),
        SelectControl::new()
          .label(self::label("Display Mode"))
          .value("Borderless")
          .row_height(190.0)
          .offset_y(-8.0),
        accessibility::static_label("Selection changes: 0").style(
          Style::new()
            .font_size(28)
            .color(Color::rgb(0.75, 0.86, 0.97))
            .margin_top(30),
        ),
        ReviewButton::new()
          .label("Change resolution from parent")
          .on_press(move || external.update(|value| !value)),
      ))
  }
}

fn label(value: &'static str) -> TextElement {
  accessibility::name_source_text(value)
}

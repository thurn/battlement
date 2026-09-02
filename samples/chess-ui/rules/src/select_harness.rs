use battlement::{Color, Style};
use battlement_reactant::prelude::*;

use crate::{
  review_button::{ReviewButton, ReviewButtonKind},
  select_control::SelectControl,
};

struct SelectHarness;

pub(crate) fn render() -> Node {
  Node::new(SelectHarness)
}

impl Component for SelectHarness {
  fn render(&self) -> impl Render {
    let (high_resolution, set_high_resolution) = use_state(false);
    let (changes, set_changes) = use_state(0_u32);
    let value = if high_resolution {
      "2560 × 1440"
    } else {
      "1920 × 1080"
    };
    let external = set_high_resolution.clone();
    let update = use_button(ButtonOptions {
      name: text("Change resolution from parent"),
      is_disabled: false,
      on_press: move || external.update(|value| !value),
    });
    View::new()
      .name("select-specimen")
      .style(
        Style::new()
          .width(839)
          .margin_top(48)
          .background_color(Color::rgb(0.01, 0.035, 0.08)),
      )
      .child((
        SelectControl::new(
          self::label("Resolution"),
          ["1920 × 1080", "2560 × 1440", "3840 × 2160"],
          value,
          move |value| {
            set_high_resolution.set(value == "2560 × 1440");
            set_changes.update(|count| count + 1);
          },
        )
        .first(true),
        SelectControl::new(
          self::label("Display Mode"),
          ["Borderless", "Fullscreen", "Windowed"],
          "Borderless",
          |_| {},
        )
        .row_height(190.0)
        .offset_y(-8.0),
        Label::new(format!("Selection changes: {changes}"))
          .semantic(use_static_text(text(format!(
            "Selection changes: {changes}"
          ))))
          .style(
            Style::new()
              .font_size(28)
              .color(Color::rgb(0.75, 0.86, 0.97))
              .margin_top(30),
          ),
        ReviewButton::new(
          Button::new("Change resolution from parent")
            .semantic(update.semantic)
            .focus_props(update.focus)
            .interaction_props(update.interaction),
          ReviewButtonKind::Action,
        ),
      ))
  }
}

fn label(value: &'static str) -> TextElement {
  TextElement::new(value)
    .semantic(use_static_text(text(value)).visibility(SemanticVisibility::NameSourceOnly))
}

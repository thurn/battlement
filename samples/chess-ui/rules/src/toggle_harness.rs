use battlement::{Color, Style};
use battlement_reactant::prelude::*;

use crate::{
  engine::Game,
  review_button::{ReviewButton, ReviewButtonKind},
  toggle_control::ToggleControl,
};

struct ToggleHarness;

pub(crate) fn render() -> Node {
  Node::new(ToggleHarness)
}

impl Component for ToggleHarness {
  fn render(&self) -> impl Render {
    let (checked, set_checked) = use_state(false);
    let (changes, set_changes) = use_state(0_u32);
    let (screenshake, set_screenshake) = use_state(true);
    let external = set_checked.clone();
    let toggle = use_button(ButtonOptions {
      name: text("Change VSync from parent"),
      is_disabled: false,
      on_press: move |_: &mut Game| external.update(|value| !value),
    });
    View::new()
      .name("toggle-specimen")
      .style(
        Style::new()
          .width(839)
          .margin_top(48)
          .background_color(Color::rgb(0.01, 0.035, 0.08)),
      )
      .child((
        ToggleControl::new(self::label("VSync"), checked, move |value| {
          set_checked.set(value);
          set_changes.update(|count| count + 1);
        }),
        ToggleControl::new(self::label("Screenshake"), screenshake, move |value| {
          set_screenshake.set(value)
        })
        .aria_label("Screen shake")
        .row_height(190.0)
        .offset_y(-8.0),
        Label::new(format!("VSync changes: {changes}"))
          .semantic(use_static_text(text(format!("VSync changes: {changes}"))))
          .style(
            Style::new()
              .font_size(28)
              .color(Color::rgb(0.75, 0.86, 0.97))
              .margin_top(30),
          ),
        ReviewButton::new(
          Button::new("Change VSync from parent")
            .semantic(toggle.semantic)
            .focus_props(toggle.focus)
            .interaction_props(toggle.interaction),
          ReviewButtonKind::Action,
        ),
      ))
  }
}

fn label(value: &'static str) -> TextElement {
  TextElement::new(value)
    .semantic(use_static_text(text(value)).visibility(SemanticVisibility::NameSourceOnly))
}

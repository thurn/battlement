use crate::{review_button::ReviewButton, toggle_control::ToggleControl};
use battlement::{Color, Style};
use battlement_reactant::{accessibility, hooks, prelude::*};

/// Owns checkbox values and counts proposals accepted from the controls.
#[builder]
pub struct ToggleHarness;

impl Component for ToggleHarness {
  fn render(&self) -> impl Render {
    let (checked, set_checked) = hooks::use_state(false);
    let (changes, set_changes) = hooks::use_state(0_u32);
    let (screenshake, set_screenshake) = hooks::use_state(true);
    View::new()
      .name("toggle-specimen")
      .style(
        Style::new()
          .width(839)
          .margin_top(48)
          .background_color(Color::rgb(0.01, 0.035, 0.08)),
      )
      .child((
        ToggleControl::new()
          .label(self::label("VSync"))
          .checked(checked)
          .on_change({
            let set_checked = set_checked.clone();
            move |checked| {
              set_checked.set(checked);
              set_changes.update(|count| count + 1);
            }
          }),
        ToggleControl::new()
          .label(self::label("Screenshake"))
          .checked(screenshake)
          .on_change(set_screenshake)
          .aria_label("Screen shake")
          .row_height(190.0)
          .offset_y(-8.0),
        accessibility::static_label(format!("VSync changes: {changes}")).style(
          Style::new()
            .font_size(28)
            .color(Color::rgb(0.75, 0.86, 0.97))
            .margin_top(30),
        ),
        ReviewButton::new()
          .label("Change VSync from parent")
          .on_press(move || set_checked.update(|value| !value)),
      ))
  }
}

fn label(value: &'static str) -> TextElement {
  accessibility::name_source_text(value)
}

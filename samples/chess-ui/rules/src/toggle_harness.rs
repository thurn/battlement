use trox::{tx, tx_args, txa};

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
          .label(self::label(tx(
            "VSync",
            "User-facing product copy in the Chess UI sample.",
          )))
          .checked(checked)
          .on_change(
            set_checked.callback().then(
              set_changes
                .update_callback(|count| count + 1)
                .map_input(|_: bool| ()),
            ),
          ),
        ToggleControl::new()
          .label(self::label(tx(
            "Screenshake",
            "User-facing product copy in the Chess UI sample.",
          )))
          .checked(screenshake)
          .on_change(set_screenshake)
          .aria_label(tx(
            "Screen shake",
            "User-facing product copy in the Chess UI sample.",
          ))
          .row_height(190.0)
          .offset_y(-8.0),
        accessibility::static_label(txa(
          "VSync changes: {changes}",
          tx_args![changes],
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
            "Change VSync from parent",
            "User-facing product copy in the Chess UI sample.",
          ))
          .on_press(move || set_checked.update(|value| !value)),
      ))
  }
}

fn label(value: LocalizedString) -> TextElement {
  accessibility::name_source_text(value)
}

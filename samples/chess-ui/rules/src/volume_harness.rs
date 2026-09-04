use trox::{tx, tx_args, txa};

use crate::{review_button::ReviewButton, volume_control::VolumeControl};
use battlement::{Color, Style};
use battlement_reactant::prelude::builder;
use battlement_reactant::{accessibility, component::Component, hooks, host::View, render::Render};

/// Owns slider percentages and demonstrates accepted and rejected changes.
#[builder]
pub struct VolumeHarness;

impl Component for VolumeHarness {
  fn render(&self) -> impl Render {
    let (value, set_value) = hooks::use_state(80_u32);
    let (changes, set_changes) = hooks::use_state(0_u32);
    View::new()
      .name("volume-specimen")
      .style(
        Style::new()
          .width(839)
          .margin_top(48)
          .background_color(Color::rgb(0.01, 0.035, 0.08)),
      )
      .child((
        VolumeControl::new()
          .label(tx(
            "Master Volume",
            "User-facing product copy in the Chess UI sample.",
          ))
          .value(value)
          .on_change(
            set_value.callback().then(
              set_changes
                .update_callback(|count| count + 1)
                .map_input(|_: u32| ()),
            ),
          )
          .first(true),
        VolumeControl::new()
          .label(tx(
            "Minimum",
            "User-facing product copy in the Chess UI sample.",
          ))
          .value(0)
          .on_change(|_| {}),
        VolumeControl::new()
          .label(tx(
            "Maximum",
            "User-facing product copy in the Chess UI sample.",
          ))
          .value(100)
          .on_change(|_| {}),
        accessibility::static_label(txa(
          "Volume changes: {changes}",
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
            "Change volume from parent",
            "User-facing product copy in the Chess UI sample.",
          ))
          .on_press(move || {
            set_value.update(|value| if value == 25 { 80 } else { 25 });
          }),
      ))
  }
}

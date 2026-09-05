//! Resettable owner for the interactive volume specimen.

use trox::{tx, tx_args, txa};

use crate::volume_control::VolumeControl;
use battlement::{Color, Style};
use battlement_reactant::{control_behavior, hooks, prelude::*};

/// Owns the accepted Master Volume value for input demonstrations.
#[builder]
pub struct VolumeInputHarness;

impl Component for VolumeInputHarness {
  fn render(&self) -> impl Render {
    let (value, set_value) = hooks::use_state(80_u32);
    let (changes, set_changes) = hooks::use_state(0_u32);
    View::new()
      .name("volume-input-specimen")
      .style(
        Style::new()
          .width(839)
          .margin_top(48)
          .background_color(Color::rgb(0.01, 0.035, 0.08)),
      )
      .child((
        VolumeControl::new()
          .label(tx("Master Volume", "Volume control interface label."))
          .value(value)
          .on_change(
            set_value.callback().then(
              set_changes
                .update_callback(|count| count + 1)
                .map_input(|_: u32| ()),
            ),
          )
          .first(true),
        control_behavior::static_label(txa(
          "Volume changes: {changes}",
          tx_args![changes],
          "Volume control dynamic value label.",
        ))
        .style(
          Style::new()
            .font_size(28)
            .color(Color::rgb(0.75, 0.86, 0.97))
            .margin_top(30),
        ),
      ))
  }
}

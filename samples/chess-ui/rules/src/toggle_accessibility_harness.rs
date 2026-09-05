use trox::{tx, tx_args, txa};

use crate::toggle_control::ToggleControl;
use battlement::{Color, Style};
use battlement_reactant::{control_behavior, hooks, prelude::*};

/// Demonstrates an explicitly named checkbox with assistive-only context.
#[builder]
pub struct ToggleAccessibilityHarness;

impl Component for ToggleAccessibilityHarness {
  fn render(&self) -> impl Render {
    let (checked, set_checked) = hooks::use_state(true);
    let (changes, set_changes) = hooks::use_state(0_u32);
    View::new()
      .name("toggle-accessibility-specimen")
      .style(
        Style::new()
          .width(839)
          .margin_top(48)
          .background_color(Color::rgb(0.01, 0.035, 0.08)),
      )
      .child((
        ToggleControl::new()
          .label(control_behavior::name_source_text(tx(
            "Upload Crash Reports",
            "Crash report checkbox interface label.",
          )))
          .aria_label(tx(
            "Upload Crash Reports",
            "Crash report checkbox accessibility label.",
          ))
          .accessibility_description(tx(
            "We upload crash reports to Unity Diagnostics.",
            "Crash report checkbox accessibility description.",
          ))
          .checked(checked)
          .on_change(
            set_checked.callback().then(
              set_changes
                .update_callback(|count| count + 1)
                .map_input(|_: bool| ()),
            ),
          ),
        control_behavior::static_label(txa(
          "Upload Crash Reports: {state} · Changes: {changes}",
          tx_args![state => if checked { "On" } else { "Off" }, changes],
          "Crash report checkbox demonstration status.",
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

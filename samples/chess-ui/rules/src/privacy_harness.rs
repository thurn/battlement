use trox::{ls, tx};

use crate::{privacy_policy::PrivacyPolicyHelp, toggle_control::ToggleControl};
use battlement::{Color, Style};
use battlement_reactant::{control_behavior, hooks, portal::PortalTarget, prelude::*};

/// Owns the crash-report help specimen and observable link request.
#[builder]
pub struct PrivacyHarness {
  #[builder(required)]
  overlay: PortalTarget,
}

impl Component for PrivacyHarness {
  fn render(&self) -> impl Render {
    let (checked, set_checked) = hooks::use_state(true);
    let (open, set_open) = hooks::use_state(false);
    let (requests, set_requests) = hooks::use_state(0_u32);
    View::new()
      .name("privacy-specimen")
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
          .checked(checked)
          .with_info(true)
          .on_info_click(set_open.callback().map_input(|_| true))
          .on_change(set_checked),
        control_behavior::static_label(ls(format!(
          "Crash reports: {} · Dialog: {} · Privacy requests: {requests}",
          if checked { "On" } else { "Off" },
          if open { "Open" } else { "Closed" },
        )))
        .style(
          Style::new()
            .font_size(28)
            .color(Color::rgb(0.75, 0.86, 0.97))
            .margin_top(30),
        ),
        PrivacyPolicyHelp::new()
          .open(open)
          .on_open_url(
            set_requests
              .update_callback(|count| count + 1)
              .map_input(|_: String| ()),
          )
          .on_close(set_open.callback().map_input(|_| false))
          .overlay(self.overlay.clone()),
      ))
  }
}

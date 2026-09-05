use trox::{ls, tx};

use crate::{arcade_modal::ArcadeModal, review_button::ReviewButton, select_control::VALUE_FONT};
use battlement::{Color, Style, TextAnchor, WhiteSpace};
use battlement_reactant::{control_behavior, hooks, portal::PortalTarget, prelude::*};

/// Owns the erase-confirmation specimen and its observable outcome.
#[builder]
pub struct ArcadeModalHarness {
  #[builder(required)]
  overlay: PortalTarget,
}

impl Component for ArcadeModalHarness {
  fn render(&self) -> impl Render {
    let (open, set_open) = hooks::use_state(false);
    let (confirmations, set_confirmations) = hooks::use_state(0_u32);
    View::new().style(Style::new().margin_top(48)).child((
      ReviewButton::new()
        .label(tx(
          "Open erase confirmation",
          "Erase confirmation demonstration action.",
        ))
        .name("erase-modal-opener")
        .on_press(set_open.callback().map_input(|_| true)),
      control_behavior::static_label(ls(format!(
        "Dialog: {}",
        if open { "Open" } else { "Closed" }
      )))
      .style(
        Style::new()
          .font_size(28)
          .color(Color::rgb(0.75, 0.86, 0.97)),
      ),
      control_behavior::static_label(ls(format!("Erase confirmations: {confirmations}"))).style(
        Style::new()
          .font_size(28)
          .color(Color::rgb(0.75, 0.86, 0.97)),
      ),
      ArcadeModal::new()
        .open(open)
        .title(tx("Erase Saved Data?", "Erase confirmation title."))
        .children(
          control_behavior::static_label(tx(
            "All saved data will be permanently erased. This cannot be undone.",
            "Erase confirmation warning.",
          ))
          .style(
            Style::new()
              .width(620)
              .font_size(47)
              .white_space(WhiteSpace::Normal)
              .unity_font_definition(VALUE_FONT)
              .unity_text_align(TextAnchor::MiddleCenter),
          ),
        )
        .confirm_label(tx("Erase", "Erase confirmation action."))
        .cancel_label(tx("Cancel", "Erase confirmation cancellation."))
        .danger(true)
        .reduce_motion(false)
        .on_confirm(
          set_confirmations
            .update_callback(|count| count + 1)
            .then(set_open.callback().map_input(|_| false)),
        )
        .on_close(set_open.callback().map_input(|_| false))
        .overlay(self.overlay.clone()),
    ))
  }
}

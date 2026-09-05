//! Crash-report help content and its privacy-policy request boundary.

use trox::tx;

use crate::{arcade_modal::ArcadeModal, select_control::VALUE_FONT};
use battlement::{Align, Color, Style, TextAnchor, TextShadow, WhiteSpace};
use battlement_reactant::{control_behavior, portal::PortalTarget, prelude::*};

/// Unity's game-player privacy policy requested by the source interface.
pub const PRIVACY_POLICY_URL: &str =
  "https://unity.com/legal/game-player-and-app-user-privacy-policy";

/// A titleless crash-report help dialog with a host-owned URL request.
#[builder]
pub struct PrivacyPolicyHelp {
  #[builder(required)]
  open: bool,
  #[builder(required)]
  on_open_url: EventCallback<String>,
  #[builder(required)]
  on_close: EventCallback<()>,
  #[builder(required)]
  overlay: PortalTarget,
}

impl Component for PrivacyPolicyHelp {
  fn render(&self) -> impl Render {
    ArcadeModal::new()
      .open(self.open)
      .aria_label(tx(
        "Crash report upload information",
        "Crash report help dialog accessibility label.",
      ))
      .children(
        View::new()
          .style(Style::new().align_items(Align::Center))
          .child((
            control_behavior::static_label(tx(
              "We upload crash reports to Unity Diagnostics.",
              "Crash report help message.",
            ))
            .style(
              Style::new()
                .width(620)
                .font_size(47)
                .white_space(WhiteSpace::Normal)
                .unity_font_definition(VALUE_FONT)
                .unity_text_align(TextAnchor::MiddleCenter),
            ),
            Link::new(tx("Privacy Policy", "Privacy policy link label."))
              .host_name("privacy-policy-link")
              .style(self::privacy_link_style())
              .on_press(
                self
                  .on_open_url
                  .clone()
                  .map_input(|_| PRIVACY_POLICY_URL.to_owned()),
              ),
          )),
      )
      .reduce_motion(false)
      .on_confirm(self.on_close.clone())
      .on_close(self.on_close.clone())
      .overlay(self.overlay.clone())
  }
}

fn privacy_link_style() -> Style {
  Style::new()
    .margin_top(34)
    .padding(0)
    .padding_bottom(7)
    .border_width(0)
    .border_bottom_width(2)
    .border_bottom_color(Color::rgba8(255, 88, 210, 204))
    .background_color(Color::TRANSPARENT)
    .color(Color::hex(0x70efff))
    .font_size(42)
    .unity_font_definition(VALUE_FONT)
    .unity_text_align(TextAnchor::MiddleCenter)
    .text_shadow(TextShadow::new(
      0.0,
      0.0,
      12.0,
      Color::rgba8(55, 210, 255, 166),
    ))
}

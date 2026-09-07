//! Source-sized Input settings composition and reset controls.

use battlement::{Align, Color, FlexDirection, Style, TextAnchor};
use battlement_reactant::{hooks, portal::PortalTarget, prelude::*};
use trox::ls;

use crate::{
  font_scale::{self, FontScale},
  input_settings::InputSettings,
  settings_panel::SettingsPanel,
};

/// Presents the complete Input panel with its real modal portal.
#[builder]
pub struct InputSettingsCompositionHarness {
  #[builder(required)]
  overlay: PortalTarget,
}

impl Component for InputSettingsCompositionHarness {
  fn render(&self) -> impl Render {
    let (font_scale, set_font_scale) = hooks::use_state(FontScale::Percent100);
    let (generation, set_generation) = hooks::use_state(0_u32);
    View::new()
      .name("input-settings-composition-harness")
      .style(Style::new().width(887).margin_top(8))
      .child((
        Flex::new()
          .direction(FlexDirection::Row)
          .gap(10.0)
          .style(Style::new().min_height(64).align_items(Align::Center))
          .child((
            self::button(
              font_scale.label(),
              "input-composition-text-size",
              set_font_scale.update_callback(|scale| match scale {
                FontScale::Percent100 => FontScale::Percent200,
                _ => FontScale::Percent100,
              }),
            ),
            self::button(
              "RESET",
              "input-composition-reset",
              set_font_scale
                .callback()
                .map_input(|_| FontScale::Percent100)
                .then(set_generation.update_callback(|value| value.wrapping_add(1))),
            ),
          )),
        font_scale::provider(
          font_scale,
          SettingsPanel::new().children(
            InputSettings::new()
              .overlay(self.overlay.clone())
              .full_panel(true)
              .key((generation, font_scale.label())),
          ),
        ),
      ))
  }
}

fn button(label: &'static str, name: &'static str, on_press: EventCallback<()>) -> impl Render {
  Button::new(ls(label))
    .host_name(name)
    .on_press(on_press)
    .style(
      Style::new()
        .width(160)
        .height(54)
        .padding((8, 12))
        .border_width(1)
        .border_color(Color::hex(0x31455d))
        .border_radius(6)
        .background_color(Color::hex(0x101a28))
        .color(Color::hex(0xd4e4f1))
        .font_size(24)
        .unity_text_align(TextAnchor::MiddleCenter),
    )
}

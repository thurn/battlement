use trox::{ls, tx};

use crate::setting_row::SettingRow;
use battlement::{Color, Style, TextAnchor};
use battlement_reactant::prelude::{builder, use_control_label};
use battlement_reactant::{
  component::Component,
  control_behavior,
  host::{ButtonHost, Label, TextElement, View},
  render::Render,
};

/// Compares settings row heights and visible label associations.
#[builder]
pub struct SettingRowHarness;

impl Component for SettingRowHarness {
  fn render(&self) -> impl Render {
    let (label, control) =
      use_control_label().bind_with(|name| control_behavior::button(name, None, false, || {}));
    View::new()
      .name("setting-row-specimen")
      .style(
        Style::new()
          .width(839)
          .margin_top(48)
          .background_color(Color::rgb(0.01, 0.035, 0.08)),
      )
      .child((
        SettingRow::new()
          .label(self::label("Resolution"))
          .children(self::value("1920 × 1080"))
          .first(true),
        SettingRow::new()
          .label(self::label("Max Framerate"))
          .children(self::value("144 FPS")),
        SettingRow::new()
          .label(control_behavior::name_source_text(tx(
            "Display Mode",
            "Setting row interface label.",
          )))
          .children(
            ButtonHost::new(tx("Borderless", "Setting row interface label."))
              .associated_control(control)
              .style(
                Style::new()
                  .font_size(40)
                  .color(Color::rgb(0.75, 0.86, 0.97))
                  .unity_text_align(TextAnchor::MiddleCenter),
              ),
          )
          .associated_label(label)
          .row_height(190.0),
      ))
  }
}

fn value(text: &'static str) -> Label {
  control_behavior::static_label(ls(text)).style(
    Style::new()
      .font_size(40)
      .color(Color::rgb(0.75, 0.86, 0.97))
      .unity_text_align(TextAnchor::MiddleCenter),
  )
}

fn label(text: &'static str) -> TextElement {
  control_behavior::static_text(ls(text))
}

use crate::setting_row::SettingRow;
use battlement::{Color, Style, TextAnchor};
use battlement_reactant::prelude::{builder, use_control_label};
use battlement_reactant::{
  accessibility::{self, ButtonOptions},
  component::Component,
  host::{Button, Label, TextElement, View},
  render::Render,
};

/// Compares settings row heights and visible label associations.
#[builder]
pub struct SettingRowHarness;

impl Component for SettingRowHarness {
  fn render(&self) -> impl Render {
    let label = use_control_label();
    let control = accessibility::use_button(ButtonOptions {
      name: label.name(),
      description: None,
      is_disabled: false,
      on_press: || {},
    });
    let (label, control) = label.bind(control);
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
          .label(accessibility::name_source_text("Display Mode"))
          .children(
            Button::new("Borderless").associated_control(control).style(
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
  accessibility::static_label(text).style(
    Style::new()
      .font_size(40)
      .color(Color::rgb(0.75, 0.86, 0.97))
      .unity_text_align(TextAnchor::MiddleCenter),
  )
}

fn label(text: &'static str) -> TextElement {
  accessibility::static_text(text)
}

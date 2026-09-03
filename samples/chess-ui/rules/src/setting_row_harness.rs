use crate::setting_row::SettingRow;
use battlement::{Color, SemanticRole, Style, TextAnchor};
use battlement_reactant::label_binding;
use battlement_reactant::prelude::builder;
use battlement_reactant::{
  accessibility,
  component::Component,
  host::{Label, TextElement, View},
  render::Render,
  semantics::{self, SemanticProps},
};

/// Compares settings row heights and visible label associations.
#[builder]
pub struct SettingRowHarness;

impl Component for SettingRowHarness {
  fn render(&self) -> impl Render {
    let label = label_binding::use_label();
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
          .label(self::label("Display Mode"))
          .children(
            View::new()
              .semantic(SemanticProps::new(SemanticRole::Group).name(label.name()))
              .child(self::value("Borderless")),
          )
          .label_binding(label)
          .row_height(190.0),
      ))
  }
}

fn value(text: &'static str) -> Label {
  Label::new(text)
    .semantic(accessibility::use_static_text(semantics::text(text)))
    .style(
      Style::new()
        .font_size(40)
        .color(Color::rgb(0.75, 0.86, 0.97))
        .unity_text_align(TextAnchor::MiddleCenter),
    )
}

fn label(text: &'static str) -> TextElement {
  TextElement::new(text).semantic(accessibility::use_static_text(semantics::text(text)))
}

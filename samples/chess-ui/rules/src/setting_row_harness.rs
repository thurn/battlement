use battlement::{Color, SemanticRole, Style, TextAnchor};
use battlement_reactant::{
  accessibility,
  component::Component,
  element_ref,
  host::{Label, TextElement, View},
  render::{Node, Render},
  semantics::{self, AccessibleName, SemanticProps},
};

use crate::setting_row::SettingRow;

struct SettingRowHarness;

pub(crate) fn render() -> Node {
  Node::new(SettingRowHarness)
}

impl Component for SettingRowHarness {
  fn render(&self) -> impl Render {
    let label = element_ref::use_element_ref();
    View::new()
      .name("setting-row-specimen")
      .style(
        Style::new()
          .width(839)
          .margin_top(48)
          .background_color(Color::rgb(0.01, 0.035, 0.08)),
      )
      .child((
        SettingRow::new(self::label("Resolution"), self::value("1920 × 1080")).first(true),
        SettingRow::new(self::label("Max Framerate"), self::value("144 FPS")),
        SettingRow::new(
          self::label("Display Mode"),
          View::new()
            .semantic(
              SemanticProps::new(SemanticRole::Group)
                .name(AccessibleName::LabelledBy(vec![label.clone()])),
            )
            .child(self::value("Borderless")),
        )
        .label_id(label)
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

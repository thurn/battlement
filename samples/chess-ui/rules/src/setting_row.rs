use battlement::{
  Align, Color, FlexDirection, GridTrack, Length, LengthUnits, Position, Scale, SemanticRole,
  Style, TransformOrigin, UiFontAddress,
};
use battlement_reactant::{
  component::Component,
  element_ref::ElementRef,
  host::{Grid, View},
  render::{Node, Render},
  semantics::{AccessibleName, SemanticProps, SemanticVisibility},
};

const LABEL_FONT_SIZE: f32 = 61.0;

/// Default minimum height of a settings row in portrait design pixels.
pub const SETTINGS_ROW_HEIGHT: f32 = 159.0;
/// Native TextCore face for the display labels.
pub const DISPLAY_FONT: UiFontAddress = UiFontAddress::from_static("chess-ui/fonts/display");

/// A display label and its content in fixed and flexible columns.
pub struct SettingRow {
  pub label: Node,
  pub label_id: Option<ElementRef>,
  pub children: Node,
  pub first: bool,
  pub row_height: Option<f32>,
}

impl SettingRow {
  /// Creates a row with required label and content.
  pub fn new(label: impl Render, children: impl Render) -> Self {
    Self {
      label: Node::new(label),
      label_id: None,
      children: Node::new(children),
      first: false,
      row_height: None,
    }
  }

  pub fn first(mut self, value: bool) -> Self {
    self.first = value;
    self
  }

  /// Exposes the label host for semantic name references.
  pub fn label_id(mut self, value: ElementRef) -> Self {
    self.label_id = Some(value);
    self
  }

  pub fn row_height(mut self, value: f32) -> Self {
    self.row_height = Some(value);
    self
  }
}

impl Component for SettingRow {
  fn render(&self) -> impl Render {
    let mut label = View::new()
      .name("setting-row-label")
      .style(
        Style::new()
          .position(Position::Relative)
          .flex_direction(FlexDirection::Row)
          .align_items(Align::Center)
          .min_width(0)
          .height(100.pct())
          .padding_left(18)
          .color(Color::rgb(245.0 / 255.0, 245.0 / 255.0, 248.0 / 255.0))
          .unity_font_definition(DISPLAY_FONT)
          .font_size(LABEL_FONT_SIZE)
          .letter_spacing(1.3)
          .scale(Scale::new(1.045, 1.0))
          .transform_origin(TransformOrigin::two_dimensional(
            Length::Px(0.0),
            Length::Percent(50.0),
          )),
      )
      .child(self.label.clone());
    if let Some(reference) = &self.label_id {
      label = label.element_ref(reference.clone()).semantic(
        SemanticProps::new(SemanticRole::StaticText)
          .name(AccessibleName::Contents)
          .visibility(SemanticVisibility::NameSourceOnly),
      );
    }
    Grid::new()
      .name("setting-row")
      .columns([GridTrack::px(422.0), GridTrack::fr(1.0)])
      .align_items(Align::Center)
      .style(
        Style::new()
          .min_height(self.row_height.unwrap_or(SETTINGS_ROW_HEIGHT))
          .border_top_width(if self.first { 0.0 } else { 2.0 })
          .border_top_color(Color::rgba(43.0 / 255.0, 74.0 / 255.0, 123.0 / 255.0, 0.25)),
      )
      .child((label, self.children.clone()))
  }
}

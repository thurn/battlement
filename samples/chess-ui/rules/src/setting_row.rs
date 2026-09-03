//! Two-column settings layout and optional visible-label associations.

use battlement::{
  Align, Color, FlexDirection, GridTrack, Length, LengthUnits, Position, Scale, Style,
  TransformOrigin, UiFontAddress,
};
use battlement_reactant::prelude::builder;
use battlement_reactant::{
  component::Component,
  host::{Grid, View},
  label_binding::LabelBinding,
  render::Render,
};
use std::rc::Rc;

const LABEL_FONT_SIZE: f32 = 61.0;

/// Default minimum height of a settings row in portrait design pixels.
pub const SETTINGS_ROW_HEIGHT: f32 = 159.0;

/// Native TextCore face for the display labels.
pub const DISPLAY_FONT: UiFontAddress = UiFontAddress::from_static("chess-ui/fonts/display");

/// A display label and its content in fixed and flexible columns.
#[builder]
pub struct SettingRow<L, R> {
  #[builder(required, into)]
  label: Rc<L>,
  /// Associates the visible label with the control that uses this binding.
  label_binding: Option<LabelBinding>,
  #[builder(required, into)]
  children: Rc<R>,
  /// Omits the separator above the first row.
  first: bool,
  /// Sets the minimum row height in portrait design pixels.
  row_height: Option<f32>,
}

impl<L: Render, R: Render> Component for SettingRow<L, R> {
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
    if let Some(binding) = &self.label_binding {
      label = label
        .element_ref(binding.reference())
        .semantic(binding.semantic());
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

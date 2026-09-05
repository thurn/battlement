//! Two-column settings layout and optional visible-label associations.

use battlement::{
  Align, Color, FlexDirection, GridTrack, Length, LengthOrAuto, Position, Scale, Style,
  TransformOrigin, UiFontAddress,
};
use battlement_reactant::prelude::{Child, Children, builder};
use battlement_reactant::{
  component::Component,
  host::{Grid, View},
  label_binding::AssociatedLabel,
  render::Render,
};

use crate::font_scale;

const LABEL_FONT_SIZE: f32 = 61.0;

/// Default minimum height of a settings row in portrait design pixels.
pub const SETTINGS_ROW_HEIGHT: f32 = 159.0;

/// Native TextCore face for the display labels.
pub const DISPLAY_FONT: UiFontAddress = UiFontAddress::from_static("chess-ui/fonts/display");

/// A display label and its content in fixed and flexible columns.
#[builder]
pub struct SettingRow {
  #[builder(required, into)]
  label: Child,
  /// Associates the visible label with its control behavior.
  associated_label: Option<AssociatedLabel>,
  #[builder(required, into)]
  children: Children,
  /// Omits the separator above the first row.
  first: bool,
  /// Sets the minimum row height in portrait design pixels.
  row_height: Option<f32>,
}

impl Component for SettingRow {
  fn render(&self) -> impl Render {
    let font_scale = font_scale::use_font_scale();
    Grid::new()
      .name("setting-row")
      .columns(if font_scale.factor() > 1.0 {
        vec![GridTrack::fr(1.0)]
      } else {
        vec![GridTrack::px(422.0), GridTrack::fr(1.0)]
      })
      .rows(if font_scale.factor() > 1.0 {
        vec![GridTrack::auto(), GridTrack::auto()]
      } else {
        Vec::new()
      })
      .gap(if font_scale.factor() > 1.0 { 18.0 } else { 0.0 })
      .align_items(Align::Center)
      .style(
        Style::new()
          .min_height(self.row_height.unwrap_or(SETTINGS_ROW_HEIGHT) * font_scale.factor())
          .padding(if font_scale.factor() > 1.0 {
            (24, 18, 28, 18)
          } else {
            (0, 0, 0, 0)
          })
          .border_top_width(if self.first { 0.0 } else { 2.0 })
          .border_top_color(Color::rgb8(43, 74, 123).with_alpha(0.25)),
      )
      .child((
        View::new()
          .name("setting-row-label")
          .style(
            Style::new()
              .position(Position::Relative)
              .flex_direction(FlexDirection::Row)
              .align_items(Align::Center)
              .min_width(0)
              .height(if font_scale.factor() > 1.0 {
                LengthOrAuto::Auto
              } else {
                LengthOrAuto::Percent(100.0)
              })
              .padding_left(if font_scale.factor() > 1.0 { 0 } else { 18 })
              .color(Color::rgb8(245, 245, 248))
              .unity_font_definition(DISPLAY_FONT)
              .font_size(LABEL_FONT_SIZE * font_scale.factor())
              .letter_spacing(1.3)
              .scale(Scale::new(
                if font_scale.factor() > 1.0 {
                  1.0
                } else {
                  1.045
                },
                1.0,
              ))
              .transform_origin(TransformOrigin::two_dimensional(
                Length::Px(0.0),
                Length::Percent(50.0),
              )),
          )
          .associated_label(self.associated_label.clone())
          .child(self.label.render()),
        self.children.render(),
      ))
  }
}

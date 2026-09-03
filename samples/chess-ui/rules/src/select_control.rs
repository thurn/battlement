//! A closed selector specimen with a visible value and combined accessible name.

use battlement_reactant::label_binding;
use std::rc::Rc;

use battlement::{
  Align, Color, FlexDirection, Length, LengthUnits, MotionLength, Position, Style, TextAnchor,
  Translate, UiFontAddress,
};
use battlement_reactant::{
  accessibility, accessibility_popup,
  paint::{PaintFill, PaintStyle},
  prelude::*,
};

use crate::{caret::Caret, clipped_inset::ClippedInset, frame_styles, setting_row::SettingRow};

/// Native TextCore face for selected control values.
pub const VALUE_FONT: UiFontAddress = UiFontAddress::from_static("chess-ui/fonts/control");

/// A closed settings selector showing its parent-owned value and decorative caret.
///
/// This specimen demonstrates trigger layout and accessible naming. It does not
/// offer a menu or propose new values; the parent supplies the displayed value.
pub struct SelectControl<R> {
  label: Rc<R>,
  first: bool,
  offset_y: f32,
  row_height: Option<f32>,
  value: String,
}

impl<R: Render> SelectControl<R> {
  /// Creates a closed selector with visible label content and a selected value.
  pub fn new(label: R, value: impl Into<String>) -> Self {
    Self {
      label: Rc::new(label),
      first: false,
      offset_y: 0.0,
      row_height: None,
      value: value.into(),
    }
  }

  /// Omits the separator above the first row.
  pub fn first(mut self, value: bool) -> Self {
    self.first = value;
    self
  }
  /// Offsets the control vertically without moving its row label.
  pub fn offset_y(mut self, value: f32) -> Self {
    self.offset_y = value;
    self
  }
  /// Sets the minimum row height in portrait design pixels.
  pub fn row_height(mut self, value: f32) -> Self {
    self.row_height = Some(value);
    self
  }
}

impl<R: Render> Component for SelectControl<R> {
  fn render(&self) -> impl Render {
    let label = label_binding::use_label();
    let value_label = label_binding::use_label();
    let trigger = accessibility_popup::use_popup_button(PopupButtonOptions {
      name: label.name_with(&value_label),
      popup: PopupKind::ListBox,
      expanded: false,
      is_disabled: false,
      on_press: || {},
    });
    let control = View::new()
      .name("select-control")
      .style(
        Style::new()
          .position(Position::Relative)
          .width(396)
          .height(106)
          .flex_shrink(0.0)
          .align_items(Align::Center)
          .translate(Translate::two_dimensional(
            Length::Px(0.0),
            Length::Px(self.offset_y),
          )),
      )
      .child(
        View::new()
          .name("select-frame")
          .style(
            Style::new()
              .position(Position::Relative)
              .width(396)
              .height(106),
          )
          .child(
            Button::new("")
              .name("select-trigger")
              .semantic(trigger.semantic)
              .focus_props(trigger.focus)
              .interaction_props(trigger.interaction)
              .style(
                Style::new()
                  .position(Position::Relative)
                  .width(100.pct())
                  .height(100.pct())
                  .flex_direction(FlexDirection::Row)
                  .align_items(Align::Center)
                  .margin(0)
                  .padding_top(0)
                  .padding_bottom(0)
                  .padding_left(39)
                  .padding_right(74)
                  .border_width(0)
                  .background_color(Color::rgba(0.0, 0.0, 0.0, 0.0))
                  .color(Color::rgb(245.0 / 255.0, 246.0 / 255.0, 251.0 / 255.0))
                  .unity_font_definition(VALUE_FONT)
                  .font_size(60)
                  .unity_text_align(TextAnchor::MiddleLeft),
              )
              .paint(
                PaintStyle::new()
                  .background(PaintFill::Color(frame_styles::color(0x5df5ff)))
                  .clip_polygon(self::clip(10.0)),
              )
              .child((
                ClippedInset::new(PaintFill::Color(frame_styles::color(0x020611)))
                  .inset(3.0)
                  .clip_path(self::clip(7.0)),
                TextElement::new(self.value.clone())
                  .name("select-value")
                  .element_ref(value_label.reference())
                  .semantic(
                    accessibility::use_static_text(text(self.value.clone()))
                      .visibility(SemanticVisibility::NameSourceOnly),
                  ),
                Caret::new().open(false),
              )),
          ),
      );
    let mut row = SettingRow::new(self.label.clone(), control)
      .label_binding(label)
      .first(self.first);
    if let Some(height) = self.row_height {
      row = row.row_height(height);
    }
    row
  }
}

fn clip(cut: f32) -> Vec<[MotionLength; 2]> {
  let near = MotionLength::px(cut);
  let far = MotionLength::calc(-cut, 100.0);
  let zero = MotionLength::px(0.0);
  let full = MotionLength::percent(100.0);
  vec![
    [near, zero],
    [far, zero],
    [full, near],
    [full, far],
    [far, full],
    [near, full],
    [zero, far],
    [zero, near],
  ]
}

//! A closed selector specimen with a visible value and combined accessible name.

use trox::{assert_localized, tx};

use crate::{caret::Caret, setting_row::SettingRow};
use battlement::{
  Align, Color, FlexDirection, Length, LengthUnits, Position, Style, TextAnchor, Translate,
  UiFontAddress,
};
use battlement_reactant::{
  accessibility, accessibility_popup,
  paint::{PaintLayer, PaintStyle},
  prelude::*,
};

/// Native TextCore face for selected control values.
pub const VALUE_FONT: UiFontAddress = UiFontAddress::from_static("chess-ui/fonts/control");

/// A closed settings selector showing its parent-owned value and decorative caret.
///
/// This specimen demonstrates trigger layout and accessible naming. It does not
/// offer a menu or propose new values; the parent supplies the displayed value.
#[builder]
pub struct SelectControl {
  #[builder(required, into)]
  label: Child,
  /// Omits the separator above the first row.
  first: bool,
  /// Offsets the control vertically without moving its row label.
  offset_y: f32,
  /// Sets the minimum row height in portrait design pixels.
  row_height: Option<f32>,
  #[builder(required)]
  value: String,
}

impl Component for SelectControl {
  fn render(&self) -> impl Render {
    let label = use_control_label();
    let value_label = use_label();
    let trigger = accessibility_popup::use_popup_button(
      PopupButtonOptions::new()
        .name(label.name_with(&value_label))
        .popup(PopupKind::ListBox)
        .expanded(false)
        .on_press(|| {}),
    );
    let (label, trigger) = label.bind(trigger);
    SettingRow::new()
      .label(self.label.render())
      .children(
        View::new()
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
                Button::new(tx("", "User-facing product copy in the Chess UI sample."))
                  .name("select-trigger")
                  .associated_control(trigger)
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
                      .background_color(Color::TRANSPARENT)
                      .color(Color::rgb8(245, 246, 251))
                      .unity_font_definition(VALUE_FONT)
                      .font_size(60)
                      .unity_text_align(TextAnchor::MiddleLeft),
                  )
                  .paint(
                    PaintStyle::fill(Color::hex(0x5df5ff))
                      .clip_polygon(self::clip(10.0))
                      .layer(
                        PaintLayer::new(Color::hex(0x020611))
                          .bounds_inset(3.0)
                          .clip_polygon(self::clip(7.0)),
                      ),
                  )
                  .child((
                    accessibility::name_source_text(assert_localized(self.value.clone()))
                      .name("select-value")
                      .element_ref(value_label.reference()),
                    Caret::new().is_open(false),
                  )),
              ),
          ),
      )
      .associated_label(label)
      .first(self.first)
      .row_height(self.row_height)
  }
}

fn clip(cut: f32) -> Vec<[Length; 2]> {
  let near = Length::px(cut);
  let far = Length::calc(-cut, 100.0);
  let zero = Length::px(0.0);
  let full = Length::percent(100.0);
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

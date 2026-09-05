//! A closed selector specimen with a visible value and combined accessible name.

use trox::{ls, tx};

use crate::{caret::Caret, setting_row::SettingRow, use_interaction};
use battlement::{
  Align, Color, FlexDirection, Gradient, Length, LengthUnits, MotionProperty, Position, Style,
  TextAnchor, Translate, UiFontAddress,
};
use battlement_reactant::{
  control_behavior,
  host::ButtonHost,
  motion::{Easing, MotionTarget, StyleTarget, Transition},
  paint::{PaintLayer, PaintStyle},
  prelude::*,
  prelude::{PaintDropShadow, PaintFilterList},
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
    let interaction = use_interaction::use_interaction();
    let label = use_control_label();
    let value_label = use_label();
    let (label, trigger) = label.bind_with(|name| {
      let SemanticName::LabelledBy(references) = name else {
        panic!("control labels must resolve through labelled-by references");
      };
      control_behavior::button(
        SemanticName::LabelledBy(
          references
            .into_iter()
            .chain([value_label.reference()])
            .collect(),
        ),
        None,
        false,
        || {},
      )
      .map_semantic(|mut semantic| {
        semantic.state.popup = Some(PopupKind::ListBox);
        semantic.state.expanded = Some(false);
        semantic
      })
    });
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
                interaction
                  .button(
                    ButtonHost::new(tx("", "Resolution selector interface label."))
                      .name("select-trigger")
                      .associated_control(trigger),
                  )
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
                    PaintStyle::new()
                      .background(self::border(false))
                      .paint_filter(self::filter(false))
                      .clip_polygon(self::clip(10.0))
                      .layer(
                        PaintLayer::new(Color::hex(0x020611))
                          .bounds_inset(3.0)
                          .clip_polygon(self::clip(7.0)),
                      ),
                  )
                  .initial(false)
                  .animate(self::target(interaction.state))
                  .child((
                    control_behavior::name_source_text(ls(self.value.clone()))
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

fn border(highlighted: bool) -> Gradient {
  if highlighted {
    Gradient::linear(16.0)
      .stop(0.0, Color::hex(0xb5ffff))
      .stop(0.48, Color::hex(0xd3ddff))
      .stop(1.0, Color::hex(0xff75dc))
  } else {
    Gradient::linear(16.0)
      .stop(0.0, Color::hex(0x5df5ff))
      .stop(0.48, Color::hex(0xa5cbff))
      .stop(1.0, Color::hex(0xff4bc9))
  }
}

fn filter(highlighted: bool) -> PaintFilterList {
  PaintFilterList::default()
    .brightness(if highlighted { 1.12 } else { 1.0 })
    .drop_shadow(PaintDropShadow::new(
      0.0,
      0.0,
      if highlighted { 13.0 } else { 6.0 },
      0.0,
      if highlighted {
        Color::hex(0x53e2ff).with_alpha(0.78)
      } else {
        Color::hex(0x2a67ff).with_alpha(0.38)
      },
    ))
}

fn target(state: use_interaction::InteractionState) -> MotionTarget {
  MotionTarget::new(
    StyleTarget::new()
      .background_gradient(self::border(state.hovered))
      .paint_filter(self::filter(state.hovered))
      .scale(if state.pressed && !state.reduced_motion {
        0.965
      } else {
        1.0
      }),
  )
  .transition(
    Transition::tween()
      .duration_secs(0.14)
      .ease(Easing::Ease)
      .property(
        MotionProperty::Scale,
        Transition::tween()
          .duration_secs(0.09)
          .ease(Easing::CubicBezier([0.2, 0.8, 0.2, 1.0])),
      ),
  )
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

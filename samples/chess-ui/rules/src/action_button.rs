//! Arcade actions with composed labels and parent-owned callbacks.

use crate::action_skin;
use battlement::{
  Align, Color, FlexDirection, Length, LengthUnits, Position, Style, TextAnchor, Translate,
  UiFontAddress, WhiteSpace,
};
use battlement_reactant::prelude::{Children, EventCallback, builder};
use battlement_reactant::{
  component::Component,
  components::Button,
  host::View,
  paint::{PaintLayer, PaintStyle},
  render::Render,
  semantics::SemanticName,
};

/// Native TextCore face for action labels.
pub const ACTION_FONT: UiFontAddress = UiFontAddress::from_static("chess-ui/fonts/action");

/// A parent-sized arcade button with arbitrary non-interactive label content.
#[builder]
pub struct ActionButton {
  #[builder(required, into)]
  children: Children,
  /// Disables activation while retaining the control’s place in the layout.
  disabled: bool,
  /// Caps the label size relative to its authored arcade typography.
  max_text_scale: Option<f32>,
  /// Handles an accepted button activation.
  #[builder(default = EventCallback::noop())]
  on_press: EventCallback<()>,
}

impl ActionButton {
  fn label_style(&self) -> Style {
    let scale = 1.0_f32.min(self.max_text_scale.unwrap_or(f32::INFINITY));
    Style::new()
      .position(Position::Relative)
      .flex_direction(FlexDirection::Row)
      .align_items(Align::Center)
      .height(81.9 * scale)
      .padding_right(10.92 * scale)
      .color(Color::rgb(0.97, 1.0, 1.0))
      .unity_font_definition(ACTION_FONT)
      .font_size(91.0 * scale)
      .white_space(WhiteSpace::NoWrap)
      .letter_spacing(-2)
      .unity_text_align(TextAnchor::MiddleCenter)
      .translate(Translate::two_dimensional(
        Length::Px(0.0),
        Length::Px(-1.0),
      ))
  }
}

impl Component for ActionButton {
  fn render(&self) -> impl Render {
    View::new()
      .style(
        Style::new()
          .position(Position::Relative)
          .width(100.pct())
          .height(100.pct()),
      )
      .child(
        Button::content(
          View::new()
            .name("action-label")
            .style(self.label_style())
            .child(self.children.render()),
        )
        .semantic_name(SemanticName::Contents)
        .host_name("action-button")
        .disabled(self.disabled)
        .on_press(self.on_press.clone())
        .style(
          Style::new()
            .position(Position::Relative)
            .full_size()
            .margin(0)
            .padding(0)
            .border_width(0)
            .center_content()
            .background_color(Color::TRANSPARENT),
        )
        .disabled_style(
          Style::new()
            .opacity(1.0)
            .background_color(Color::TRANSPARENT),
        )
        .paint(
          PaintStyle::new()
            .background(action_skin::border())
            .clip_polygon(action_skin::clip(18.0, 17.0))
            .layer(
              PaintLayer::new(action_skin::INTERIOR)
                .bounds_inset(6.0)
                .clip_polygon(action_skin::clip(14.0, 13.0)),
            ),
        ),
      )
  }
}

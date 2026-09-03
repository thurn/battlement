use std::rc::Rc;

use battlement::{
  Align, Color, FlexDirection, Justify, Length, LengthUnits, Position, Style, TextAnchor,
  Translate, UiFontAddress, WhiteSpace,
};
use battlement_reactant::{
  accessibility::{self, ButtonOptions},
  component::Component,
  host::{Button, View},
  paint::{PaintFill, PaintStyle},
  render::{Node, Render},
  semantics::AccessibleName,
};

use crate::{action_skin, clipped_inset::ClippedInset};

/// Native TextCore face for action labels.
pub const ACTION_FONT: UiFontAddress = UiFontAddress::from_static("chess-ui/fonts/action");

/// A parent-sized arcade button with arbitrary non-interactive label content.
pub struct ActionButton {
  pub children: Node,
  pub disabled: bool,
  pub max_text_scale: Option<f32>,
  pub on_click: Option<Rc<dyn Fn()>>,
}

impl ActionButton {
  /// Creates a button named by its children's static-text semantics.
  pub fn new(children: impl Render) -> Self {
    Self {
      children: Node::new(children),
      disabled: false,
      max_text_scale: None,
      on_click: None,
    }
  }

  pub fn disabled(mut self, value: bool) -> Self {
    self.disabled = value;
    self
  }

  pub fn max_text_scale(mut self, value: f32) -> Self {
    self.max_text_scale = Some(value);
    self
  }

  pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
    self.on_click = Some(Rc::new(callback));
    self
  }
}

impl Component for ActionButton {
  fn render(&self) -> impl Render {
    let on_click = self.on_click.clone();
    let button = accessibility::use_button(ButtonOptions {
      name: AccessibleName::Contents,
      is_disabled: self.disabled,
      on_press: move || {
        if let Some(callback) = &on_click {
          callback();
        }
      },
    });
    let text_scale = 1.0_f32.min(self.max_text_scale.unwrap_or(f32::INFINITY));
    View::new()
      .style(
        Style::new()
          .position(Position::Relative)
          .width(100.pct())
          .height(100.pct()),
      )
      .child(
        Button::new("")
          .name("action-button")
          .semantic(button.semantic)
          .focus_props(button.focus)
          .interaction_props(button.interaction)
          .style(
            Style::new()
              .position(Position::Relative)
              .width(100.pct())
              .height(100.pct())
              .margin(0)
              .padding(0)
              .border_width(0)
              .align_items(Align::Center)
              .justify_content(Justify::Center)
              .background_color(Color::rgba(0.0, 0.0, 0.0, 0.0)),
          )
          .paint(
            PaintStyle::new()
              .background(action_skin::border())
              .clip_polygon(action_skin::clip(18.0, 17.0)),
          )
          .child((
            ClippedInset {
              inset: 6.0,
              clip_path: action_skin::clip(14.0, 13.0),
              background: PaintFill::Color(action_skin::INTERIOR),
              box_shadow: None,
            },
            View::new()
              .name("action-label")
              .style(
                Style::new()
                  .position(Position::Relative)
                  .flex_direction(FlexDirection::Row)
                  .align_items(Align::Center)
                  .height(81.9 * text_scale)
                  .padding_right(10.92 * text_scale)
                  .color(Color::rgb(0.97, 1.0, 1.0))
                  .unity_font_definition(ACTION_FONT)
                  .font_size(91.0 * text_scale)
                  .white_space(WhiteSpace::NoWrap)
                  .letter_spacing(-2)
                  .unity_text_align(TextAnchor::MiddleCenter)
                  .translate(Translate::two_dimensional(
                    Length::Px(0.0),
                    Length::Px(-1.0),
                  )),
              )
              .child(self.children.clone()),
          )),
      )
  }
}

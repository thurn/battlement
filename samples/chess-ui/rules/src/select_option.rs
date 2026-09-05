//! One focus-managed option in the custom selector.

use trox::ls;

use crate::{check_mark::CheckMark, select_control::VALUE_FONT};
use battlement::{
  Align, Color, FlexDirection, Gradient, Justify, PickingMode, Position, Shadow, Style, TextAnchor,
};
use battlement_reactant::{element_ref, hooks, paint::PaintStyle, prelude::*};

/// Renders one controlled option and focuses it while it is active.
#[builder]
pub(crate) struct SelectOption {
  active: bool,
  #[builder(required)]
  control_scale: f32,
  #[builder(required)]
  font_scale: f32,
  index: usize,
  #[builder(required)]
  label: String,
  #[builder(required)]
  on_press: EventCallback<()>,
  selected: bool,
}

impl Component for SelectOption {
  fn render(&self) -> impl Render {
    let reference = element_ref::use_element_ref();
    hooks::use_effect(
      {
        let reference = reference.clone();
        let active = self.active;
        move || {
          if active {
            reference.focus();
          }
        }
      },
      self.active,
    );
    ListBoxOption::new(ls(self.label.clone()), self.selected)
      .host_name(format!("select-option-{}", self.label.to_ascii_lowercase()))
      .element_ref(reference)
      .key(self.index)
      .style(self::style(self.font_scale, self.control_scale))
      .paint(self::paint(self.active))
      .hover_style(Style::new().background_color(Color::rgba8(11, 113, 207, 128)))
      .on_press(self.on_press.clone())
      .child(
        View::decorative()
          .name("select-option-mark")
          .picking_mode(PickingMode::Ignore)
          .style(
            Style::new()
              .position(Position::Absolute)
              .right(20)
              .top(16)
              .width(48)
              .height(44)
              .flex_shrink(0.0),
          )
          .child(self.selected.then(|| CheckMark::new().scale(0.62))),
      )
  }
}

fn style(font_scale: f32, control_scale: f32) -> Style {
  Style::new()
    .position(Position::Relative)
    .width(100.pct())
    .height(76.0 * control_scale)
    .min_height(76.0 * control_scale)
    .flex_direction(FlexDirection::Row)
    .align_items(Align::Center)
    .justify_content(Justify::SpaceBetween)
    .padding_top(6.0 * control_scale)
    .padding_bottom(6.0 * control_scale)
    .padding_left(25.0 * control_scale)
    .padding_right(20.0 * control_scale)
    .border_width(0)
    .background_color(Color::TRANSPARENT)
    .color(Color::hex(0xd9e1f2))
    .unity_font_definition(VALUE_FONT)
    .font_size(47.0 * font_scale)
    .unity_text_align(TextAnchor::MiddleLeft)
}

fn paint(active: bool) -> PaintStyle {
  PaintStyle::new()
    .background(if active {
      Gradient::linear(90.0)
        .stop(0.0, Color::rgba8(255, 238, 0, 82))
        .stop(1.0, Color::rgba8(255, 167, 0, 36))
    } else {
      Gradient::linear(90.0)
        .stop(0.0, Color::TRANSPARENT)
        .stop(1.0, Color::TRANSPARENT)
    })
    .box_shadow(active.then(|| Shadow::inset(0.0, 0.0, 0.0, 3.0, Color::hex(0xfff400))))
}

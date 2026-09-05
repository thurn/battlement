//! Procedural keyboard and controller glyphs for input binding rows.

use battlement::{Color, Gradient, Position, Rotate, Shadow, Style, TextAnchor};
use battlement_reactant::{paint::PaintStyle, prelude::*};
use trox::ls;

use crate::{font_scale, setting_row::DISPLAY_FONT};

/// Direction represented by a keyboard arrow or D-pad highlight.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputDirection {
  Left,
  Right,
  Up,
  Down,
}

impl InputDirection {
  fn angle(self) -> f32 {
    match self {
      Self::Left => 180.0,
      Self::Right => 0.0,
      Self::Up => -90.0,
      Self::Down => 90.0,
    }
  }
}

/// Bold directional arrow used inside a keyboard keycap.
#[builder]
pub struct KeyboardArrow {
  #[builder(required)]
  direction: InputDirection,
}

impl Component for KeyboardArrow {
  fn render(&self) -> impl Render {
    let font_scale = font_scale::use_font_scale();
    View::decorative()
      .name(format!("keyboard-arrow-{}", self.direction.slug()))
      .style(
        Style::new()
          .position(Position::Relative)
          .width(65.0 * font_scale.dynamic(font_scale::FontScaleRole::Control))
          .height(65.0 * font_scale.dynamic(font_scale::FontScaleRole::Control))
          .rotate(Rotate::degrees(self.direction.angle())),
      )
      .child(self::arrow_parts(
        font_scale.dynamic(font_scale::FontScaleRole::Control),
      ))
  }
}

/// Five-cell D-pad with one highlighted direction.
#[builder]
pub struct DPadIcon {
  #[builder(required)]
  direction: InputDirection,
}

impl Component for DPadIcon {
  fn render(&self) -> impl Render {
    let font_scale = font_scale::use_font_scale();
    View::decorative()
      .name(format!("d-pad-{}", self.direction.slug()))
      .style(
        Style::new()
          .position(Position::Relative)
          .width(87.0 * font_scale.dynamic(font_scale::FontScaleRole::Control))
          .height(87.0 * font_scale.dynamic(font_scale::FontScaleRole::Control)),
      )
      .child(self::d_pad_cells(
        self.direction,
        font_scale.dynamic(font_scale::FontScaleRole::Control),
      ))
  }
}

/// Source-colored circular controller face button.
#[builder]
pub struct ControllerButtonIcon {
  #[builder(required)]
  label: ControllerLabel,
}

impl Component for ControllerButtonIcon {
  fn render(&self) -> impl Render {
    let font_scale = font_scale::use_font_scale();
    View::decorative()
      .name(format!("controller-button-{}", self.label.slug()))
      .style(self::controller_style(
        self.label,
        font_scale.dynamic(font_scale::FontScaleRole::Control),
      ))
      .paint(self::controller_paint(self.label))
      .child(
        Label::new(ls(self.label.visible())).style(self::controller_label_style(
          self.label,
          font_scale.dynamic(font_scale::FontScaleRole::Control),
        )),
      )
  }
}

/// Fixed controller button labels used by the source table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerLabel {
  A,
  Menu,
  Y,
}

impl ControllerLabel {
  fn visible(self) -> &'static str {
    match self {
      Self::A => "A",
      Self::Menu => "≡",
      Self::Y => "Y",
    }
  }

  fn slug(self) -> &'static str {
    match self {
      Self::A => "a",
      Self::Menu => "menu",
      Self::Y => "y",
    }
  }
}

impl InputDirection {
  fn slug(self) -> &'static str {
    match self {
      Self::Left => "left",
      Self::Right => "right",
      Self::Up => "up",
      Self::Down => "down",
    }
  }
}

fn arrow_parts(scale: f32) -> impl Render {
  (
    self::arrow_bar(10.0, 28.5, 43.0, 8.0, 0.0, scale),
    self::arrow_bar(34.0, 19.0, 27.0, 8.0, 45.0, scale),
    self::arrow_bar(34.0, 38.0, 27.0, 8.0, -45.0, scale),
  )
}

fn arrow_bar(left: f32, top: f32, width: f32, height: f32, angle: f32, scale: f32) -> View {
  View::decorative()
    .style(
      Style::new()
        .position(Position::Absolute)
        .left(left * scale)
        .top(top * scale)
        .width(width * scale)
        .height(height * scale)
        .border_radius(4.0 * scale)
        .background_color(Color::hex(0xf6f6fa))
        .rotate(Rotate::degrees(angle)),
    )
    .paint(PaintStyle::new().box_shadow([
      Shadow::outer(2.0 * scale, 4.0 * scale, 0.0, 0.0, Color::hex(0x19284a)),
      Shadow::outer(0.0, 4.0 * scale, 5.0 * scale, 0.0, Color::BLACK),
    ]))
}

fn d_pad_cells(direction: InputDirection, scale: f32) -> impl Render {
  [
    (InputDirection::Up, 29.0, 0.0),
    (InputDirection::Left, 0.0, 29.0),
    (InputDirection::Right, 58.0, 29.0),
    (InputDirection::Down, 29.0, 58.0),
  ]
  .map(|(cell, left, top)| self::d_pad_cell(cell == direction, left, top, scale))
  .into_iter()
  .chain(std::iter::once(self::d_pad_cell(false, 29.0, 29.0, scale)))
  .collect::<Vec<_>>()
}

fn d_pad_cell(active: bool, left: f32, top: f32, scale: f32) -> View {
  View::decorative()
    .style(
      Style::new()
        .position(Position::Absolute)
        .left(left * scale)
        .top(top * scale)
        .width(29.0 * scale)
        .height(29.0 * scale)
        .border_width(2.0 * scale)
        .border_color(Color::hex(if active { 0xa8ffff } else { 0x78808c }))
        .border_radius(5.0 * scale),
    )
    .paint(
      PaintStyle::new()
        .background(if active {
          Gradient::linear(145.0)
            .stop(0.0, Color::hex(0x40f7ff))
            .stop(1.0, Color::hex(0x05bfd8))
        } else {
          Gradient::linear(145.0)
            .stop(0.0, Color::hex(0x202a36))
            .stop(1.0, Color::hex(0x080d15))
        })
        .box_shadow(if active {
          vec![
            Shadow::inset(0.0, 0.0, 7.0 * scale, 0.0, Color::rgba8(255, 255, 255, 166)),
            Shadow::outer(0.0, 0.0, 8.0 * scale, 0.0, Color::hex(0x13ddff)),
          ]
        } else {
          vec![
            Shadow::inset(0.0, 0.0, 7.0 * scale, 0.0, Color::BLACK),
            Shadow::outer(0.0, 0.0, 0.0, 2.0 * scale, Color::rgba8(0, 0, 0, 191)),
          ]
        }),
    )
}

fn controller_style(label: ControllerLabel, scale: f32) -> Style {
  Style::new()
    .width(78.0 * scale)
    .height(78.0 * scale)
    .min_width(78.0 * scale)
    .min_height(78.0 * scale)
    .flex_shrink(0.0)
    .border_width(3.0 * scale)
    .border_color(self::controller_border(label))
    .border_radius(39.0 * scale)
}

fn controller_label_style(label: ControllerLabel, scale: f32) -> Style {
  Style::new()
    .full_size()
    .color(Color::hex(0xf8f8f5))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(if label == ControllerLabel::Menu {
      54.0 * scale
    } else {
      57.0 * scale
    })
    .unity_text_align(TextAnchor::MiddleCenter)
}

fn controller_paint(label: ControllerLabel) -> PaintStyle {
  let (center, edge, glow) = match label {
    ControllerLabel::A => (0x65bd14, 0x237000, 0x72e71c),
    ControllerLabel::Y => (0xffca15, 0xc27a00, 0xffb000),
    ControllerLabel::Menu => (0x34373b, 0x121416, 0x08090b),
  };
  PaintStyle::new()
    .background(
      Gradient::radial([0.5, 0.5], [0.5, 0.5])
        .stop(0.0, Color::hex(center))
        .stop(0.55, Color::hex(center))
        .stop(0.58, Color::hex(edge))
        .stop(1.0, Color::hex(edge)),
    )
    .box_shadow([
      Shadow::inset(0.0, 0.0, 0.0, 5.0, Color::rgba8(0, 0, 0, 71)),
      Shadow::outer(0.0, 0.0, 13.0, 0.0, Color::hex(glow)),
      Shadow::outer(0.0, 5.0, 6.0, 0.0, Color::BLACK),
    ])
}

fn controller_border(label: ControllerLabel) -> Color {
  Color::hex(match label {
    ControllerLabel::A => 0xa7ff35,
    ControllerLabel::Y => 0xfff5a6,
    ControllerLabel::Menu => 0x777b80,
  })
}

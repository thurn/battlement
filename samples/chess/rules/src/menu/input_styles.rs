//! Shared binding table and capture presentation.
use crate::menu::{action_skin, input_binding_icons::InputDirection, setting_row::DISPLAY_FONT};
use battlement::{
  Align, Color, Gradient, PhysicalKey, Position, Shadow, Style, TextAnchor, WhiteSpace,
};
use reactant::{
  paint::{PaintLayer, PaintStyle},
  prelude::*,
};

pub fn key_name(key: PhysicalKey) -> String {
  match key {
    PhysicalKey::Escape => "Esc".to_owned(),
    PhysicalKey::Space => "Space".to_owned(),
    PhysicalKey::ArrowLeft => "Left arrow".to_owned(),
    PhysicalKey::ArrowRight => "Right arrow".to_owned(),
    PhysicalKey::ArrowUp => "Up arrow".to_owned(),
    PhysicalKey::ArrowDown => "Down arrow".to_owned(),
    _ => {
      let name = format!("{key:?}");
      name
        .strip_prefix("Key")
        .or_else(|| name.strip_prefix("Digit"))
        .unwrap_or(&name)
        .to_owned()
    }
  }
}

pub fn key_direction(key: PhysicalKey) -> Option<InputDirection> {
  match key {
    PhysicalKey::ArrowLeft => Some(InputDirection::Left),
    PhysicalKey::ArrowRight => Some(InputDirection::Right),
    PhysicalKey::ArrowUp => Some(InputDirection::Up),
    PhysicalKey::ArrowDown => Some(InputDirection::Down),
    _ => None,
  }
}

pub fn keycap_style(compact: bool, font_scale: f32, control_scale: f32) -> Style {
  Style::new()
    .position(Position::Relative)
    .width(if compact { 120.0 } else { 205.0 } * control_scale)
    .height(75.0 * control_scale)
    .margin(0)
    .padding(3.0 * control_scale)
    .border_width(0)
    .align_self(Align::Center)
    .center_content()
    .color(Color::hex(0xf6f6fa))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(49.0 * font_scale)
}

pub fn keycap_paint() -> PaintStyle {
  PaintStyle::new()
    .background(
      Gradient::linear(110.0)
        .stop(0.0, Color::hex(0x55f1ff))
        .stop(0.54, Color::hex(0x7ba3ff))
        .stop(1.0, Color::hex(0xff48c6)),
    )
    .paint_filter(PaintFilterList::default().drop_shadow(PaintDropShadow::new(
      0.0,
      0.0,
      7.0,
      0.0,
      Color::rgba8(42, 103, 255, 117),
    )))
    .clip_polygon(action_skin::clip(10.0, 10.0))
    .layer(
      PaintLayer::new(
        Gradient::linear(180.0)
          .stop(0.0, Color::hex(0x050b1c))
          .stop(1.0, Color::hex(0x020611)),
      )
      .bounds_inset(3.0)
      .box_shadow([Shadow::inset(0.0, 0.0, 22.0, 0.0, Color::BLACK)])
      .clip_polygon(action_skin::clip(7.0, 7.0)),
    )
}

pub fn keycap_label_style(keyboard: PhysicalKey, font_scale: f32, control_scale: f32) -> Style {
  let value = self::key_name(keyboard);
  Style::new()
    .position(Position::Relative)
    .full_size()
    .color(Color::hex(0xf6f6fa))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(
      if value.len() > 2 { 49.0 } else { 60.0 }
        * if value.len() > 2 {
          control_scale
        } else {
          font_scale
        },
    )
    .letter_spacing(if value.len() > 2 { 1.0 } else { 0.0 })
    .unity_text_align(TextAnchor::MiddleCenter)
}

pub fn heading_style(font_scale: f32) -> Style {
  Style::new()
    .color(Color::rgb8(244, 245, 250))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(47.0 * font_scale)
    .white_space(WhiteSpace::Normal)
    .letter_spacing(1.2)
    .unity_text_align(TextAnchor::MiddleCenter)
}

pub fn action_style(action: &str, font_scale: f32, control_scale: f32) -> Style {
  Style::new()
    .padding_left(18)
    .color(Color::rgb8(245, 245, 248))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(
      54.0
        * if action.len() >= 7 {
          control_scale
        } else {
          font_scale
        },
    )
    .letter_spacing(1.3)
    .white_space(WhiteSpace::Normal)
    .unity_text_align(TextAnchor::MiddleLeft)
}

pub fn capture_prompt_style(scale: f32) -> Style {
  Style::new()
    .color(Color::rgb8(246, 246, 250))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(46.0 * scale)
    .white_space(WhiteSpace::Normal)
    .unity_text_align(TextAnchor::MiddleCenter)
}

pub fn waiting_marker_style(scale: f32) -> Style {
  Style::new()
    .width(100)
    .height(82.0 * scale)
    .margin_top(18)
    .padding(0)
    .border_width(0)
    .background_color(Color::TRANSPARENT)
    .color(Color::hex(0x5cecff))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(62.0 * scale)
    .unity_text_align(TextAnchor::MiddleCenter)
}

pub fn status_style(scale: f32) -> Style {
  Style::new()
    .margin_top(12)
    .color(Color::hex(0xff5ca8))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(36.0 * scale)
    .white_space(WhiteSpace::Normal)
    .unity_text_align(TextAnchor::MiddleCenter)
}

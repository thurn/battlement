use battlement::{
  Align, Color, Display, FlexDirection, Justify, Length, LengthUnits, MotionColor, Overflow,
  Position, Scale, Style, TextAnchor, TransformOrigin, WhiteSpace,
};

use battlement_reactant::motion::MotionStyle;

const TEXT: Color = Color::rgb(0.91, 0.94, 0.96);
const MUTED: Color = Color::rgb(0.55, 0.62, 0.68);
const ACCENT: Color = Color::rgb(0.38, 0.94, 0.90);

pub(crate) fn root() -> Style {
  Style::new()
    .width(100.pct())
    .height(100.pct())
    .overflow(Overflow::Hidden)
    .background_color(Color::rgb(0.035, 0.047, 0.067))
    .color(TEXT)
}

pub(crate) fn gallery() -> Style {
  self::root().flex_direction(FlexDirection::Row).padding(24)
}

pub(crate) fn navigation_column() -> Style {
  Style::new()
    .width(320)
    .margin_right(24)
    .height(100.pct())
    .flex_shrink(0)
}
pub(crate) fn brand() -> Style {
  Style::new().font_size(28).color(ACCENT).margin_bottom(6)
}
pub(crate) fn navigation_caption() -> Style {
  Style::new().font_size(14).color(MUTED).margin_bottom(24)
}
pub(crate) fn navigation_scroll() -> Style {
  Style::new().flex_grow(1).min_height(0)
}
pub(crate) fn navigation_content() -> Style {
  Style::new().padding_right(12)
}
pub(crate) fn navigation_item(selected: bool) -> Style {
  Style::new()
    .min_height(48)
    .padding((12, 14))
    .margin((0, 0, 6, 0))
    .font_size(16)
    .white_space(WhiteSpace::Normal)
    .unity_text_align(TextAnchor::MiddleLeft)
    .border_radius(6)
    .border_width(1)
    .border_color(if selected {
      ACCENT
    } else {
      Color::rgb(0.15, 0.19, 0.24)
    })
    .background_color(if selected {
      Color::rgb(0.10, 0.24, 0.27)
    } else {
      Color::rgb(0.065, 0.086, 0.12)
    })
    .color(if selected { ACCENT } else { TEXT })
    .flex_shrink(0)
}
pub(crate) fn stage_area() -> Style {
  Style::new()
    .flex_grow(1)
    .min_width(0)
    .height(100.pct())
    .align_items(Align::Center)
    .justify_content(Justify::Center)
}
pub(crate) fn stage_bounds(scale: f32) -> Style {
  Style::new()
    .width(1024.0 * scale)
    .height(1536.0 * scale)
    .flex_shrink(0)
}
pub(crate) fn stage(scale: f32) -> Style {
  Style::new()
    .position(Position::Absolute)
    .left(0)
    .top(0)
    .width(1024)
    .height(1536)
    .scale(Scale::uniform(scale))
    .transform_origin(TransformOrigin::two_dimensional(
      Length::Px(0.0),
      Length::Px(0.0),
    ))
    .overflow(Overflow::Hidden)
    .background_color(Color::rgb(0.065, 0.086, 0.12))
}
pub(crate) fn page() -> Style {
  Style::new().width(100.pct()).height(100.pct()).padding(64)
}
pub(crate) fn eyebrow() -> Style {
  Style::new().font_size(24).color(ACCENT).margin_bottom(28)
}
pub(crate) fn heading() -> Style {
  Style::new()
    .font_size(64)
    .white_space(WhiteSpace::Normal)
    .color(TEXT)
    .margin_bottom(24)
}
pub(crate) fn description() -> Style {
  Style::new()
    .font_size(28)
    .white_space(WhiteSpace::Normal)
    .color(MUTED)
    .margin_bottom(32)
}
pub(crate) fn demonstration() -> Style {
  Style::new()
    .margin_top(64)
    .padding(40)
    .border_width(1)
    .border_color(Color::rgb(0.19, 0.26, 0.33))
    .border_radius(12)
}
pub(crate) fn demonstration_title() -> Style {
  Style::new().font_size(36).color(TEXT).margin_bottom(24)
}
pub(crate) fn action() -> Style {
  self::navigation_item(true)
    .font_size(28)
    .min_height(72)
    .margin_top(16)
}

pub(crate) fn scrollbar() -> Style {
  Style::new()
    .width(10)
    .background_color(Color::rgb(0.035, 0.047, 0.067))
}
pub(crate) fn scroll_button() -> Style {
  Style::new().display(Display::None)
}
pub(crate) fn scroll_track() -> Style {
  Style::new().background_color(Color::rgb(0.065, 0.086, 0.12))
}
pub(crate) fn scroll_thumb() -> Style {
  Style::new()
    .background_color(MUTED)
    .border_width(0)
    .border_radius(5)
}
pub(crate) fn focus_visible() -> MotionStyle {
  MotionStyle::new().background_color(MotionColor::new(0.18, 0.37, 0.38, 1.0))
}

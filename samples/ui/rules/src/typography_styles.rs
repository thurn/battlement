use battlement::{
  Color, FontStyle, LengthUnits, Style, TextAnchor, TextAutoSize, TextGenerator, TextOverflow,
  TextOverflowPosition, TextShadow, WhiteSpace,
};

use crate::{
  asset_catalog::ui::assets,
  design_system::{ACCENT, BACKGROUND, CYAN, PRIMARY_TEXT, SPECIMEN_BACKGROUND},
};

pub(crate) fn matrix() -> Style {
  Style::new()
    .flex_direction(battlement::FlexDirection::Row)
    .flex_wrap(battlement::FlexWrap::Wrap)
    .width(100.0_f32.pct())
    .background_color(BACKGROUND)
}

pub(crate) fn card() -> Style {
  Style::new()
    .width(48_f32.pct())
    .height(90)
    .margin(4)
    .padding(10)
    .flex_direction(battlement::FlexDirection::Row)
    .align_items(battlement::Align::Center)
    .background_color(SPECIMEN_BACKGROUND)
    .border_radius(10)
    .overflow(battlement::Overflow::Hidden)
}

pub(crate) fn caption() -> Style {
  Style::new()
    .width(35_f32.pct())
    .font_size(24)
    .color(CYAN)
    .margin((0, 12, 0, 0))
}

fn value() -> Style {
  Style::new()
    .font_size(26)
    .color(PRIMARY_TEXT)
    .flex_grow(1)
    .overflow(battlement::Overflow::Hidden)
}

pub(crate) fn font_definition() -> Style {
  value()
    .unity_font_definition(assets::UI_FONT.clone())
    .unity_text_generator(TextGenerator::Standard)
}
pub(crate) fn weight() -> Style {
  value().unity_font_style_and_weight(FontStyle::BoldAndItalic)
}
pub(crate) fn alignment() -> Style {
  value()
    .height(42)
    .unity_text_align(TextAnchor::MiddleCenter)
}
pub(crate) fn auto_size() -> Style {
  value()
    .width(100.0_f32.pct())
    .height(42)
    .unity_text_auto_size(TextAutoSize::best_fit(24.0, 34.0))
}
pub(crate) fn outline_shadow() -> Style {
  value()
    .color(ACCENT)
    .unity_text_outline_color(BACKGROUND)
    .unity_text_outline_width(1)
    .text_shadow(TextShadow::new(
      3.0,
      3.0,
      2.0,
      Color::rgba(0.0, 0.0, 0.0, 0.8),
    ))
}
pub(crate) fn spacing() -> Style {
  value()
    .letter_spacing(1.5)
    .word_spacing(5)
    .unity_paragraph_spacing(4)
    .white_space(WhiteSpace::PreWrap)
}
pub(crate) fn elision(position: TextOverflowPosition) -> Style {
  value()
    .width(0)
    .white_space(WhiteSpace::NoWrap)
    .text_overflow(TextOverflow::Ellipsis)
    .unity_text_overflow_position(position)
}
pub(crate) fn rich() -> Style {
  value().unity_text_generator(TextGenerator::Standard)
}
pub(crate) fn selectable(white_space: WhiteSpace) -> Style {
  value()
    .white_space(white_space)
    .unity_text_generator(TextGenerator::Advanced)
    .background_color(BACKGROUND)
    .padding((4, 8))
}

use battlement::{Align, Color, FlexDirection, FlexWrap, Justify, LengthUnits, Style, WhiteSpace};

use crate::design_system;

pub(crate) fn intro() -> Style {
  Style::new()
    .font_size(24.0)
    .color(design_system::BODY_TEXT)
    .margin_bottom(4.0)
    .white_space(WhiteSpace::Normal)
}

pub(crate) fn title() -> Style {
  Style::new()
    .font_size(40.0)
    .color(design_system::PRIMARY_TEXT)
    .margin(4.0)
}

pub(crate) fn summary() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .align_items(Align::Center)
    .justify_content(Justify::SpaceBetween)
    .background_color(Color::rgb(0.05, 0.16, 0.12))
    .border_left_width(4.0)
    .border_left_color(design_system::ACCENT)
    .padding_left(20.0)
    .padding_right(20.0)
    .padding_top(6.0)
    .padding_bottom(6.0)
    .margin_bottom(6.0)
}

pub(crate) fn summary_text() -> Style {
  Style::new()
    .font_size(24.0)
    .color(Color::rgb(0.79, 1.0, 0.88))
}

pub(crate) fn grid() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .justify_content(Justify::SpaceBetween)
    .background_color(design_system::BACKGROUND)
}

pub(crate) fn card() -> Style {
  Style::new()
    .width(24.0_f32.pct())
    .min_height(124.0)
    .font_size(24.0)
    .color(Color::rgb(0.79, 1.0, 0.88))
    .white_space(WhiteSpace::Normal)
    .background_color(Color::rgb(0.05, 0.16, 0.12))
    .border_top_width(1.0)
    .border_top_color(Color::rgb(0.18, 0.27, 0.29))
    .padding_left(10.0)
    .padding_right(10.0)
    .padding_top(8.0)
    .padding_bottom(8.0)
    .margin_bottom(8.0)
}

pub(crate) fn back_button() -> Style {
  Style::new()
    .width(260.0)
    .height(36.0)
    .font_size(24.0)
    .color(Color::rgb(0.79, 1.0, 0.88))
    .background_color(Color::rgb(0.05, 0.16, 0.12))
    .margin_bottom(8.0)
}

pub(crate) fn detail_intro() -> Style {
  Style::new()
    .font_size(24.0)
    .color(design_system::ACCENT)
    .margin_bottom(8.0)
}

pub(crate) fn ledger() -> Style {
  Style::new()
    .height(500.0)
    .background_color(Color::rgb(0.04, 0.06, 0.09))
    .padding(8.0)
}

pub(crate) fn ledger_row() -> Style {
  Style::new()
    .min_height(38.0)
    .font_size(24.0)
    .color(design_system::PRIMARY_TEXT)
    .background_color(Color::rgb(0.07, 0.09, 0.13))
    .border_bottom_width(1.0)
    .border_bottom_color(Color::rgb(0.15, 0.22, 0.24))
    .padding_left(10.0)
    .padding_top(4.0)
}

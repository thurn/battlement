use battlement::{
    Align, AspectRatio, FlexDirection, FlexWrap, InlineKeyword, Justify, LengthOrAuto, LengthUnits,
    Position, Style,
};

use crate::design_system::{NAVIGATION_ITEM_BACKGROUND, PRIMARY_TEXT, SPECIMEN_BACKGROUND};

pub(crate) fn playground() -> Style {
    Style::new()
        .align_content(Align::FlexStart)
        .align_items(Align::Stretch)
        .aspect_ratio(AspectRatio::new(16.0, 7.0))
        .flex_direction(FlexDirection::Row)
        .flex_wrap(FlexWrap::Wrap)
        .justify_content(Justify::SpaceEvenly)
        .width(100.pct())
        .height(360)
        .min_height(360)
        .max_height(480)
        .padding((20, 28))
        .margin((LengthOrAuto::Px(12.0), LengthOrAuto::Auto))
        .position(Position::Relative)
        .background_color(SPECIMEN_BACKGROUND)
}

pub(crate) fn column_playground() -> Style {
    Style::new()
        .align_content(Align::FlexEnd)
        .align_items(Align::Center)
        .flex_direction(FlexDirection::ColumnReverse)
        .flex_wrap(FlexWrap::NoWrap)
        .justify_content(Justify::Center)
        .width(78.pct())
        .height(460)
        .min_height(360)
        .max_height(480)
        .padding((24, 36, 28, 30))
        .margin((LengthOrAuto::Px(12.0), LengthOrAuto::Auto))
        .position(Position::Relative)
}

pub(crate) fn item() -> Style {
    Style::new()
        .align_self(Align::Auto)
        .width(42.pct())
        .min_width(160)
        .max_width(320)
        .height(110)
        .flex_basis(LengthOrAuto::Auto)
        .flex_grow(1)
        .flex_shrink(1)
        .position(Position::Relative)
        .top(InlineKeyword::Initial)
        .right(InlineKeyword::Initial)
        .padding((24, 18, 20))
        .margin((4, 8))
        .font_size(28.0)
        .color(PRIMARY_TEXT)
        .background_color(NAVIGATION_ITEM_BACKGROUND)
}

pub(crate) fn column_item() -> Style {
    Style::new()
        .align_self(Align::FlexStart)
        .width(64.pct())
        .min_width(160)
        .max_width(360)
        .height(90)
        .flex_basis(LengthOrAuto::Auto)
        .flex_grow(0)
        .flex_shrink(0)
        .position(Position::Relative)
        .top(InlineKeyword::Initial)
        .right(InlineKeyword::Initial)
        .padding((18, 26))
        .margin((5, 10, 7))
}

pub(crate) fn absolute_item() -> Style {
    Style::new()
        .align_self(Align::FlexEnd)
        .width(32.pct())
        .min_width(150)
        .max_width(280)
        .height(110)
        .flex_basis(LengthOrAuto::Auto)
        .flex_grow(0)
        .flex_shrink(0)
        .position(Position::Absolute)
        .top(24)
        .right(4.pct())
        .padding((20, 30, 24, 26))
        .margin(0)
}

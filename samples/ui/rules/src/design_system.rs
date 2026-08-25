use battlement::{
    Align, AspectRatio, Color, FlexDirection, FlexWrap, InlineKeyword, Justify, LengthOrAuto,
    LengthUnits, Position, Style,
};

const ACCENT: Color = Color::rgb(0.98, 0.72, 0.24);
const BACKGROUND: Color = Color::rgb(0.012, 0.025, 0.045);
const BODY_TEXT: Color = Color::rgb(0.86, 0.93, 0.95);
const BUTTON_BACKGROUND: Color = Color::rgb(0.08, 0.31, 0.38);
const BUTTON_TEXT: Color = Color::rgb(0.96, 0.99, 1.0);
const CYAN: Color = Color::rgb(0.32, 0.92, 0.96);
const NAVIGATION_BACKGROUND: Color = Color::rgb(0.025, 0.065, 0.085);
const NAVIGATION_ITEM_BACKGROUND: Color = Color::rgb(0.045, 0.12, 0.15);
const PRIMARY_TEXT: Color = Color::rgb(0.94, 0.98, 0.99);
const SPECIMEN_BACKGROUND: Color = Color::rgb(0.035, 0.09, 0.115);
const SUCCESS_BACKGROUND: Color = Color::rgb(0.04, 0.32, 0.18);
const MINIMUM_TEXT_SIZE: f32 = 24.0;

pub(crate) fn root() -> Style {
    Style::new()
        .background_color(BACKGROUND)
        .color(BODY_TEXT)
        .font_size(MINIMUM_TEXT_SIZE)
        .flex_direction(FlexDirection::Row)
        .padding(20.0)
}

pub(crate) fn navigation() -> Style {
    Style::new()
        .width(300.0)
        .background_color(NAVIGATION_BACKGROUND)
        .padding(24.0)
}

pub(crate) fn brand() -> Style {
    Style::new().color(CYAN).font_size(30.0).margin(10.0)
}

pub(crate) fn navigation_item(active: bool) -> Style {
    Style::new()
        .background_color(if active {
            ACCENT
        } else {
            NAVIGATION_ITEM_BACKGROUND
        })
        .color(if active { BACKGROUND } else { PRIMARY_TEXT })
        .font_size(MINIMUM_TEXT_SIZE)
        .padding(14.0)
        .margin(8.0)
}

pub(crate) fn canvas() -> Style {
    Style::new()
        .background_color(BACKGROUND)
        .flex_grow(1.0)
        .padding(36.0)
}

pub(crate) fn eyebrow() -> Style {
    Style::new()
        .font_size(MINIMUM_TEXT_SIZE)
        .color(ACCENT)
        .margin(4.0)
}

pub(crate) fn title() -> Style {
    Style::new().font_size(44.0).color(PRIMARY_TEXT).margin(8.0)
}

pub(crate) fn specimen() -> Style {
    Style::new()
        .background_color(SPECIMEN_BACKGROUND)
        .padding(28.0)
        .margin(18.0)
}

pub(crate) fn specimen_title() -> Style {
    Style::new().font_size(28.0).color(CYAN).margin(6.0)
}

pub(crate) fn component_value() -> Style {
    Style::new()
        .font_size(MINIMUM_TEXT_SIZE)
        .color(PRIMARY_TEXT)
        .margin(6.0)
}

pub(crate) fn command_button() -> Style {
    Style::new()
        .background_color(BUTTON_BACKGROUND)
        .color(BUTTON_TEXT)
        .padding(18.0)
        .margin(12.0)
        .font_size(MINIMUM_TEXT_SIZE)
}

pub(crate) fn success() -> Style {
    Style::new()
        .background_color(SUCCESS_BACKGROUND)
        .padding(22.0)
        .margin(12.0)
}

pub(crate) fn success_text() -> Style {
    Style::new().font_size(26.0).color(PRIMARY_TEXT)
}

pub(crate) fn hierarchy_explorer() -> Style {
    Style::new()
        .background_color(SPECIMEN_BACKGROUND)
        .padding(18.0)
        .margin(8.0)
}

pub(crate) fn hierarchy_branch() -> Style {
    Style::new()
        .background_color(NAVIGATION_ITEM_BACKGROUND)
        .padding(10.0)
        .margin(5.0)
}

pub(crate) fn hierarchy_item() -> Style {
    Style::new()
        .font_size(MINIMUM_TEXT_SIZE)
        .color(PRIMARY_TEXT)
        .margin(3.0)
}

pub(crate) fn asset_gallery() -> Style {
    Style::new()
        .flex_direction(FlexDirection::Row)
        .padding(8.0)
        .margin(8.0)
}

pub(crate) fn asset_card() -> Style {
    Style::new()
        .width(230.0)
        .height(220.0)
        .background_color(SPECIMEN_BACKGROUND)
        .padding(12.0)
        .margin(8.0)
}

pub(crate) fn gallery_image() -> Style {
    Style::new()
        .width(190.0)
        .height(140.0)
        .background_color(NAVIGATION_ITEM_BACKGROUND)
        .margin(4.0)
}

pub(crate) fn source_inspector() -> Style {
    Style::new()
        .flex_direction(FlexDirection::Row)
        .background_color(SPECIMEN_BACKGROUND)
        .padding(14.0)
        .margin(8.0)
}

pub(crate) fn switched_image() -> Style {
    Style::new().width(180.0).height(120.0).margin(8.0)
}

pub(crate) fn address_value() -> Style {
    Style::new()
        .font_size(MINIMUM_TEXT_SIZE)
        .color(ACCENT)
        .margin(14.0)
}

pub(crate) fn layout_playground() -> Style {
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

pub(crate) fn layout_playground_column() -> Style {
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

pub(crate) fn layout_item() -> Style {
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

pub(crate) fn layout_item_column() -> Style {
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

pub(crate) fn layout_item_absolute() -> Style {
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

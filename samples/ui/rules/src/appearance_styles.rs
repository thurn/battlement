use battlement::{
    Align, BackgroundSource, Display, FlexDirection, Justify, LengthUnits, Overflow,
    OverflowClipBox, Position, SliceType, SpriteAddress, Style, Visibility,
};

use crate::design_system::{
    ACCENT, BACKGROUND, BUTTON_BACKGROUND, CYAN, NAVIGATION_ITEM_BACKGROUND, PRIMARY_TEXT,
    SPECIMEN_BACKGROUND,
};

pub(crate) fn matrix() -> Style {
    Style::new()
        .flex_direction(FlexDirection::Row)
        .justify_content(Justify::SpaceEvenly)
        .width(100.pct())
        .background_color(SPECIMEN_BACKGROUND)
        .border_color(NAVIGATION_ITEM_BACKGROUND)
        .border_radius(18)
        .border_width(2)
        .padding(12)
        .margin((8, 0))
}

pub(crate) fn square() -> Style {
    card()
        .background_color(NAVIGATION_ITEM_BACKGROUND)
        .border_color(CYAN)
        .border_width((2, 4, 6, 8))
}

pub(crate) fn rounded() -> Style {
    card()
        .background_color(SPECIMEN_BACKGROUND)
        .border_color((ACCENT, CYAN))
        .border_radius((10, 28, 44, 18))
        .border_width(4)
}

pub(crate) fn sliced(sprite: SpriteAddress) -> Style {
    card()
        .background_color(NAVIGATION_ITEM_BACKGROUND)
        .background_image(BackgroundSource::Sprite(sprite))
        .border_color(ACCENT)
        .border_radius(18)
        .border_width(3)
        .unity_background_image_tint_color(CYAN)
        .unity_slice_bottom(24)
        .unity_slice_left(24)
        .unity_slice_right(24)
        .unity_slice_scale(1)
        .unity_slice_top(24)
        .unity_slice_type(SliceType::Tiled)
}

pub(crate) fn opacity_card() -> Style {
    card()
        .background_color(NAVIGATION_ITEM_BACKGROUND)
        .border_color(CYAN)
        .border_radius(22)
        .border_width(3)
}

pub(crate) fn faded() -> Style {
    Style::new()
        .width(72.pct())
        .height(54)
        .background_color(CYAN)
        .border_radius(16)
        .opacity(0.42)
}

pub(crate) fn clipped() -> Style {
    card()
        .background_color(SPECIMEN_BACKGROUND)
        .border_color(CYAN)
        .border_radius(24)
        .border_width(3)
        .overflow(Overflow::Hidden)
        .padding(22)
        .unity_overflow_clip_box(OverflowClipBox::ContentBox)
}

pub(crate) fn overflow_content() -> Style {
    Style::new()
        .position(Position::Absolute)
        .left(55.pct())
        .top(20)
        .width(70.pct())
        .height(86)
        .background_color(ACCENT)
        .border_radius(18)
}

pub(crate) fn label() -> Style {
    Style::new().font_size(28.0).color(PRIMARY_TEXT).margin(8)
}

pub(crate) fn overlay_label() -> Style {
    label()
        .background_color(BACKGROUND)
        .border_color(ACCENT)
        .border_radius(10)
        .border_width(2)
        .padding((8, 16))
}

pub(crate) fn visibility_slot() -> Style {
    Style::new()
        .width(260)
        .height(130)
        .background_color(NAVIGATION_ITEM_BACKGROUND)
        .border_color(BUTTON_BACKGROUND)
        .border_radius(14)
        .border_width(2)
        .margin((4, 12))
}

pub(crate) fn hidden() -> Style {
    visibility_block().visibility(Visibility::Hidden)
}

pub(crate) fn visible() -> Style {
    visibility_block().visibility(Visibility::Visible)
}

pub(crate) fn removed() -> Style {
    visibility_block().display(Display::None)
}

pub(crate) fn present() -> Style {
    visibility_block().display(Display::Flex)
}

fn card() -> Style {
    Style::new()
        .align_items(Align::Center)
        .justify_content(Justify::Center)
        .width(29.pct())
        .height(140)
        .margin(8)
}

fn visibility_block() -> Style {
    Style::new()
        .width(220)
        .height(54)
        .background_color(CYAN)
        .border_color(ACCENT)
        .border_radius(14)
        .border_width(3)
        .margin(6)
}

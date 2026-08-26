use battlement::{FlexDirection, Style};

use crate::design_system::{
    ACCENT, BACKGROUND, MINIMUM_TEXT_SIZE, NAVIGATION_ITEM_BACKGROUND, SPECIMEN_BACKGROUND,
};

pub(crate) fn gallery() -> Style {
    Style::new()
        .flex_direction(FlexDirection::Row)
        .background_color(BACKGROUND)
        .padding(8.0)
        .margin(8.0)
}

pub(crate) fn card() -> Style {
    Style::new()
        .width(230.0)
        .height(220.0)
        .background_color(SPECIMEN_BACKGROUND)
        .padding(12.0)
        .margin(8.0)
}

pub(crate) fn image() -> Style {
    Style::new()
        .width(190.0)
        .height(140.0)
        .background_color(NAVIGATION_ITEM_BACKGROUND)
        .margin(4.0)
}

pub(crate) fn inspector() -> Style {
    Style::new()
        .flex_direction(FlexDirection::Row)
        .background_color(SPECIMEN_BACKGROUND)
        .padding(14.0)
        .margin(8.0)
}

pub(crate) fn switched_image() -> Style {
    Style::new().width(180.0).height(120.0).margin(8.0)
}

pub(crate) fn address() -> Style {
    Style::new()
        .font_size(MINIMUM_TEXT_SIZE)
        .color(ACCENT)
        .margin(14.0)
}

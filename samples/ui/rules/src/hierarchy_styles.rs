use battlement::Style;

use crate::design_system::{
  MINIMUM_TEXT_SIZE, NAVIGATION_ITEM_BACKGROUND, PRIMARY_TEXT, SPECIMEN_BACKGROUND,
};

pub(crate) fn explorer() -> Style {
  Style::new()
    .background_color(SPECIMEN_BACKGROUND)
    .padding(18.0)
    .margin(8.0)
}

pub(crate) fn branch() -> Style {
  Style::new()
    .background_color(NAVIGATION_ITEM_BACKGROUND)
    .padding(10.0)
    .margin(5.0)
}

pub(crate) fn item() -> Style {
  Style::new()
    .font_size(MINIMUM_TEXT_SIZE)
    .color(PRIMARY_TEXT)
    .margin(3.0)
}

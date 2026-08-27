use battlement::Style;

use crate::design_system::{PRIMARY_TEXT, SUCCESS_BACKGROUND};

pub(crate) fn result() -> Style {
  Style::new()
    .background_color(SUCCESS_BACKGROUND)
    .padding(22.0)
    .margin(12.0)
}

pub(crate) fn result_text() -> Style {
  Style::new().font_size(26.0).color(PRIMARY_TEXT)
}

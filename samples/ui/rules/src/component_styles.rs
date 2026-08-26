use battlement::Style;

use crate::design_system::{MINIMUM_TEXT_SIZE, PRIMARY_TEXT};

pub(crate) fn value() -> Style {
    Style::new()
        .font_size(MINIMUM_TEXT_SIZE)
        .color(PRIMARY_TEXT)
        .margin(6.0)
}

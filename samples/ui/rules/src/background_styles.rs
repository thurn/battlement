use battlement::{
    Align, BackgroundPosition, BackgroundPositionKeyword, BackgroundRepeat, BackgroundRepeatMode,
    BackgroundSize, BackgroundSource, Color, Cursor, CursorHotspot, FlexDirection, Justify,
    LengthUnits, Style, TextureAddress,
};

use crate::design_system::{
    ACCENT, BACKGROUND, CYAN, NAVIGATION_ITEM_BACKGROUND, PRIMARY_TEXT, SPECIMEN_BACKGROUND,
};

pub(crate) fn gallery() -> Style {
    Style::new()
        .flex_direction(FlexDirection::Row)
        .justify_content(Justify::SpaceEvenly)
        .width(100.pct())
        .background_color(BACKGROUND)
        .margin((8, 0))
}

pub(crate) fn source_card(source: BackgroundSource, variant: usize) -> Style {
    base_card()
        .background_image(source)
        .background_position_x(match variant {
            0 => BackgroundPosition::new(BackgroundPositionKeyword::Left, 8),
            1 => BackgroundPosition::new(BackgroundPositionKeyword::Center, 10.pct()),
            2 => BackgroundPosition::new(BackgroundPositionKeyword::Right, 6),
            _ => BackgroundPosition::new(BackgroundPositionKeyword::Center, 0),
        })
        .background_position_y(match variant {
            0 => BackgroundPosition::new(BackgroundPositionKeyword::Top, 8.pct()),
            1 => BackgroundPosition::new(BackgroundPositionKeyword::Center, 0),
            2 => BackgroundPosition::new(BackgroundPositionKeyword::Bottom, 5),
            _ => BackgroundPosition::new(BackgroundPositionKeyword::Top, 12),
        })
        .background_repeat(match variant {
            0 => {
                BackgroundRepeat::new(BackgroundRepeatMode::Repeat, BackgroundRepeatMode::NoRepeat)
            }
            1 => BackgroundRepeat::new(BackgroundRepeatMode::Round, BackgroundRepeatMode::Space),
            2 => BackgroundRepeat::new(BackgroundRepeatMode::Space, BackgroundRepeatMode::Round),
            _ => {
                BackgroundRepeat::new(BackgroundRepeatMode::NoRepeat, BackgroundRepeatMode::Repeat)
            }
        })
        .background_size(match variant {
            0 => BackgroundSize::Auto,
            1 => BackgroundSize::Cover,
            2 => BackgroundSize::Contain,
            _ => BackgroundSize::axes(72.pct(), 54),
        })
        .unity_background_image_tint_color(match variant {
            0 => Color::rgb(0.7, 0.95, 1.0),
            1 => Color::rgb(1.0, 0.78, 0.38),
            2 => Color::rgb(0.55, 1.0, 0.78),
            _ => Color::rgb(0.72, 0.68, 1.0),
        })
}

pub(crate) fn interactive(source: BackgroundSource, cursor: TextureAddress) -> Style {
    source_card(source, 0).cursor(Cursor::texture(cursor, CursorHotspot::new(4.0, 4.0)))
}

pub(crate) fn adjusted(source: BackgroundSource) -> Style {
    source_card(source, 3)
        .background_position_x(BackgroundPosition::new(
            BackgroundPositionKeyword::Right,
            9.pct(),
        ))
        .background_position_y(BackgroundPosition::new(
            BackgroundPositionKeyword::Bottom,
            7,
        ))
        .background_repeat(BackgroundRepeat::new(
            BackgroundRepeatMode::Space,
            BackgroundRepeatMode::Round,
        ))
        .background_size(BackgroundSize::Contain)
        .unity_background_image_tint_color(ACCENT)
        .cursor(Cursor::Default)
}

pub(crate) fn label() -> Style {
    Style::new()
        .align_items(Align::Center)
        .background_color(BACKGROUND)
        .border_color(CYAN)
        .border_radius(10)
        .border_width(2)
        .color(PRIMARY_TEXT)
        .font_size(24.0)
        .padding((6, 10))
}

pub(crate) fn cursor_preview() -> Style {
    Style::new()
        .width(64)
        .height(64)
        .background_color(NAVIGATION_ITEM_BACKGROUND)
        .margin((4, 12))
}

fn base_card() -> Style {
    Style::new()
        .align_items(Align::Center)
        .justify_content(Justify::Center)
        .width(23.pct())
        .height(190)
        .background_color(SPECIMEN_BACKGROUND)
        .border_color(CYAN)
        .border_radius(16)
        .border_width(2)
        .margin(6)
}

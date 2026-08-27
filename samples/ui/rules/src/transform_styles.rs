use battlement::{
  Align, Color, EasingFunction, FilterFunction, FilterList, FlexDirection, Justify, LengthUnits,
  Rotate, Scale, Style, TransformOrigin, TransitionList, TransitionProperty, Translate,
};

use crate::design_system::{
  ACCENT, BACKGROUND, BUTTON_BACKGROUND, CYAN, NAVIGATION_ITEM_BACKGROUND, PRIMARY_TEXT,
  SPECIMEN_BACKGROUND,
};

pub(crate) fn row() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .justify_content(Justify::SpaceBetween)
    .width(100.pct())
    .background_color(BACKGROUND)
    .margin((6, 0))
}

pub(crate) fn origin_card() -> Style {
  Style::new()
    .align_items(Align::Center)
    .justify_content(Justify::Center)
    .width(31.pct())
    .height(134)
    .background_color(SPECIMEN_BACKGROUND)
    .border_color(CYAN)
    .border_radius(14)
    .border_width(2)
}

pub(crate) fn origin_mark(origin: TransformOrigin) -> Style {
  Style::new()
    .width(56)
    .height(56)
    .background_color(BUTTON_BACKGROUND)
    .border_radius(8)
    .rotate(Rotate::degrees(24.0))
    .transform_origin(origin)
}

pub(crate) fn label() -> Style {
  Style::new()
    .font_size(24.0)
    .color(PRIMARY_TEXT)
    .background_color(BACKGROUND)
    .margin((2, 6))
}

pub(crate) fn filter_slot() -> Style {
  Style::new()
    .align_items(Align::Center)
    .width(12.pct())
    .background_color(BACKGROUND)
}

pub(crate) fn filter_swatch(filter: FilterFunction) -> Style {
  Style::new()
    .width(54)
    .height(36)
    .background_color(Color::rgb(0.22, 0.72, 0.78))
    .border_radius(8)
    .filter(FilterList::new([filter]))
}

pub(crate) fn transition_stage() -> Style {
  Style::new()
    .align_items(Align::Center)
    .flex_direction(FlexDirection::Row)
    .justify_content(Justify::SpaceBetween)
    .height(128)
    .width(100.pct())
    .background_color(SPECIMEN_BACKGROUND)
    .border_radius(14)
    .padding((12, 24))
    .margin((8, 0))
}

pub(crate) fn transition_initial() -> Style {
  Style::new()
    .align_items(Align::Center)
    .justify_content(Justify::Center)
    .width(180)
    .height(72)
    .background_color(NAVIGATION_ITEM_BACKGROUND)
    .border_color(CYAN)
    .border_radius(12)
    .border_width(2)
    .rotate(Rotate::degrees(0.0))
    .scale(Scale::uniform(1.0))
    .translate(Translate::two_dimensional(0.px(), 0.px()))
    .transform_origin(TransformOrigin::two_dimensional(0.pct(), 100.pct()))
    .transition_property(TransitionList::new([
      TransitionProperty::Rotate,
      TransitionProperty::Scale,
      TransitionProperty::Translate,
    ]))
    .transition_duration(TransitionList::new([480.0.into()]))
    .transition_delay(TransitionList::new([0.0.into(), 40.0.into()]))
    .transition_timing_function(TransitionList::new([
      EasingFunction::EaseOutBack,
      EasingFunction::EaseInOutCubic,
    ]))
}

pub(crate) fn transition_settled() -> Style {
  Style::new()
    .background_color(ACCENT)
    .border_color(ACCENT)
    .rotate(Rotate::degrees(10.0))
    .scale(Scale::new(1.08, 1.08))
    .translate(Translate::two_dimensional(42.px(), 0.px()))
}

pub(crate) fn transition_status() -> Style {
  Style::new()
    .font_size(24.0)
    .color(PRIMARY_TEXT)
    .background_color(BACKGROUND)
    .padding((8, 14))
    .border_radius(8)
}

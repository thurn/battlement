use battlement::{
  Align, Color, EasingFunction, FlexDirection, Justify, LengthUnits, Rotate, Scale, Style,
  TransitionList, TransitionProperty,
};

use crate::design_system::{
  ACCENT, BACKGROUND, CYAN, NAVIGATION_ITEM_BACKGROUND, PRIMARY_TEXT, SPECIMEN_BACKGROUND,
};

pub(crate) fn intro() -> Style {
  Style::new()
    .color(Color::rgb(0.67, 0.78, 0.82))
    .font_size(18.0)
    .margin((2.0, 8.0, 10.0, 8.0))
    .max_width(1040.0)
    .white_space(battlement::WhiteSpace::Normal)
}

pub(crate) fn columns() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .flex_grow(1.0)
    .width(100.0_f32.pct())
}

pub(crate) fn card(accent: bool) -> Style {
  Style::new()
    .width(50.0_f32.pct())
    .background_color(SPECIMEN_BACKGROUND)
    .border_color(if accent {
      CYAN
    } else {
      Color::rgb(0.16, 0.3, 0.34)
    })
    .border_width(1.0)
    .border_radius(16.0)
    .padding(16.0)
    .margin(8.0)
}

pub(crate) fn caption() -> Style {
  Style::new()
    .color(ACCENT)
    .font_size(16.0)
    .margin((0.0, 0.0, 12.0, 0.0))
}

pub(crate) fn link_surface() -> Style {
  Style::new()
    .background_color(BACKGROUND)
    .color(PRIMARY_TEXT)
    .font_size(24.0)
    .padding((16.0, 18.0))
    .min_height(80.0)
    .border_radius(12.0)
    .margin((0.0, 0.0, 12.0, 0.0))
}

pub(crate) fn inspector() -> Style {
  Style::new()
    .background_color(NAVIGATION_ITEM_BACKGROUND)
    .color(PRIMARY_TEXT)
    .font_size(17.0)
    .min_height(150.0)
    .padding(14.0)
    .border_radius(10.0)
    .white_space(battlement::WhiteSpace::Normal)
}

pub(crate) fn legend() -> Style {
  Style::new()
    .color(Color::rgb(0.64, 0.75, 0.78))
    .font_size(14.0)
    .margin((8.0, 2.0, 0.0, 2.0))
    .white_space(battlement::WhiteSpace::Normal)
}

pub(crate) fn stage() -> Style {
  Style::new()
    .align_items(Align::Center)
    .justify_content(Justify::Center)
    .height(126.0)
    .background_color(BACKGROUND)
    .border_radius(12.0)
    .margin((0.0, 0.0, 12.0, 0.0))
}

pub(crate) fn target(settled: bool) -> Style {
  Style::new()
    .align_items(Align::Center)
    .justify_content(Justify::Center)
    .width(if settled { 224.0 } else { 168.0 })
    .height(if settled { 78.0 } else { 62.0 })
    .background_color(if settled {
      ACCENT
    } else {
      NAVIGATION_ITEM_BACKGROUND
    })
    .color(if settled { BACKGROUND } else { PRIMARY_TEXT })
    .border_color(if settled { ACCENT } else { CYAN })
    .border_width(2.0)
    .border_radius(12.0)
    .rotate(Rotate::degrees(if settled { 4.0 } else { 0.0 }))
    .scale(Scale::uniform(if settled { 1.04 } else { 1.0 }))
    .transition_property(TransitionList::new([
      TransitionProperty::Width,
      TransitionProperty::Height,
      TransitionProperty::Rotate,
      TransitionProperty::Scale,
      TransitionProperty::BackgroundColor,
    ]))
    .transition_duration(TransitionList::new([420.0.into()]))
    .transition_timing_function(TransitionList::new([EasingFunction::EaseInOutCubic]))
}

pub(crate) fn button() -> Style {
  Style::new()
    .background_color(Color::rgb(0.08, 0.31, 0.38))
    .color(PRIMARY_TEXT)
    .font_size(19.0)
    .padding((10.0, 16.0))
    .margin((12.0, 0.0, 0.0, 0.0))
}

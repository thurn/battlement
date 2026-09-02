use battlement::{Color, FlexDirection, FlexWrap, LengthUnits, Overflow, Style};

pub(crate) fn content() -> Style {
  Style::new().padding(28.0)
}

pub(crate) fn eyebrow() -> Style {
  Style::new()
    .font_size(11.0)
    .letter_spacing(1.5)
    .color(Color::rgba(0.38, 0.78, 0.98, 1.0))
}

pub(crate) fn title(large: bool) -> Style {
  Style::new()
    .font_size(if large { 42.0 } else { 34.0 })
    .color(Color::rgb(1.0, 1.0, 1.0))
}

pub(crate) fn toolbar() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
}

pub(crate) fn section() -> Style {
  Style::new()
    .margin_top(14.0)
    .padding(16.0)
    .background_color(Color::rgba(0.045, 0.07, 0.12, 1.0))
    .border_radius(10.0)
}

pub(crate) fn section_heading() -> Style {
  Style::new()
    .font_size(12.0)
    .letter_spacing(1.1)
    .margin_bottom(10.0)
    .color(Color::rgba(0.62, 0.72, 0.86, 1.0))
}

pub(crate) fn tab(active: bool) -> Style {
  Style::new()
    .height(44.0)
    .background_color(if active {
      Color::rgba(0.1, 0.48, 0.78, 1.0)
    } else {
      Color::rgba(0.08, 0.12, 0.19, 1.0)
    })
    .color(Color::rgb(1.0, 1.0, 1.0))
}

pub(crate) fn setting_label(large: bool) -> Style {
  Style::new()
    .font_size(if large { 18.0 } else { 13.0 })
    .color(Color::rgba(0.82, 0.88, 0.96, 1.0))
}

pub(crate) fn setting_value() -> Style {
  Style::new()
    .height(40.0)
    .background_color(Color::rgba(0.08, 0.13, 0.21, 1.0))
    .color(Color::rgb(1.0, 1.0, 1.0))
}

pub(crate) fn clipped_control() -> Style {
  Style::new()
    .height(72.0)
    .padding(12.0)
    .overflow(Overflow::Hidden)
    .background_color(Color::rgba(0.03, 0.05, 0.09, 1.0))
}

pub(crate) fn table() -> Style {
  Style::new().height(190.0)
}

pub(crate) fn table_content() -> Style {
  Style::new().padding_right(8.0)
}

pub(crate) fn table_header() -> Style {
  Style::new()
    .height(34.0)
    .padding(8.0)
    .background_color(Color::rgba(0.07, 0.42, 0.65, 1.0))
    .color(Color::rgb(1.0, 1.0, 1.0))
}

pub(crate) fn table_cell(alternate: bool) -> Style {
  Style::new()
    .height(32.0)
    .padding(7.0)
    .background_color(if alternate {
      Color::rgba(0.065, 0.09, 0.14, 1.0)
    } else {
      Color::rgba(0.045, 0.065, 0.1, 1.0)
    })
    .color(Color::rgba(0.78, 0.84, 0.92, 1.0))
}

pub(crate) fn layer_stage() -> Style {
  Style::new().height(130.0)
}

pub(crate) fn layer(color: Color) -> Style {
  Style::new()
    .padding(14.0)
    .background_color(color)
    .border_radius(9.0)
}

pub(crate) fn popover() -> Style {
  Style::new()
    .width(230.0)
    .padding(12.0)
    .background_color(Color::rgba(0.055, 0.09, 0.15, 0.99))
    .border_radius(9.0)
}

pub(crate) fn popover_action() -> Style {
  Style::new().height(38.0).margin_top(6.0)
}

pub(crate) fn modal() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .height(100.0_f32.pct())
    .background_color(Color::rgba(0.01, 0.02, 0.04, 0.82))
}

pub(crate) fn modal_overlay() -> Style {
  Style::new().background_color(Color::rgba(0.01, 0.02, 0.04, 0.82))
}

pub(crate) fn modal_card() -> Style {
  Style::new()
    .width(420.0)
    .height(132.0)
    .padding(24.0)
    .background_color(Color::rgba(0.06, 0.1, 0.17, 1.0))
    .border_radius(12.0)
}

pub(crate) fn modal_title() -> Style {
  Style::new()
    .font_size(24.0)
    .margin_bottom(12.0)
    .color(Color::rgb(1.0, 1.0, 1.0))
}

pub(crate) fn status() -> Style {
  Style::new()
    .margin_top(12.0)
    .font_size(11.0)
    .color(Color::rgba(0.5, 0.75, 0.82, 1.0))
}

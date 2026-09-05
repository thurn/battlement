//! Scrollable input binding table used by the settings screens.

use battlement::{
  AccessibilityScrollAxis, AccessibilityScrollDirection, Align, Color, GridTrack,
  ScrollerVisibility, Sticky, Style, TextAnchor, Vector,
};
use battlement_reactant::{component::Component, hooks, prelude::*};
use trox::ls;

use crate::setting_row::DISPLAY_FONT;

const INPUT_WIDTH: f32 = 839.0;
const HEADER_HEIGHT: f32 = 100.0;
const ROW_HEIGHT: f32 = 159.0;
const SCROLL_OFFSET: f32 = 470.0;

const DEFAULT_BINDINGS: [(&str, &str, &str); 7] = [
  ("Left", "Left arrow", "D-pad left"),
  ("Right", "Right arrow", "D-pad right"),
  ("Up", "Up arrow", "D-pad up"),
  ("Down", "Down arrow", "D-pad down"),
  ("Move Piece", "Space", "A"),
  ("Pause", "Esc", "menu"),
  ("Restart", "R", "Y"),
];

/// Displays the default keyboard and controller bindings in a sticky-header table.
#[builder]
pub struct InputSettings;

impl Component for InputSettings {
  fn render(&self) -> impl Render {
    let (scrolled, set_scrolled) = hooks::use_state(false);
    ScrollArea::new(
      Some(ls("Input bindings")),
      AccessibilityScrollAxis::Vertical,
      !scrolled,
      scrolled,
    )
    .on_scroll(move |direction| {
      set_scrolled.set(direction == AccessibilityScrollDirection::Forward)
    })
    .host_name("input-bindings-scroll")
    .configure_host(|host| {
      host
        .scroll_offset(Vector::new(0.0, if scrolled { SCROLL_OFFSET } else { 0.0 }))
        .horizontal_scroller_visibility(ScrollerVisibility::Hidden)
        .vertical_scroller_visibility(ScrollerVisibility::Auto)
        .content_container_style(Style::new().align_items(Align::Center))
    })
    .style(
      Style::new()
        .width(INPUT_WIDTH)
        .height(720)
        .margin_top(48)
        .background_color(Color::rgb8(4, 17, 38)),
    )
    .child(
      Table::new(ls("Input bindings"))
        .style(Style::new().width(INPUT_WIDTH))
        .child((self::header(), DEFAULT_BINDINGS.map(self::binding_row))),
    )
  }
}

fn header() -> TableRow {
  TableRow::new()
    .host_name("input-bindings-header")
    .configure_host(|host| host.sticky(Sticky::top(0.0).order(4)))
    .style(
      Style::new()
        .width(INPUT_WIDTH)
        .height(HEADER_HEIGHT)
        .background_color(Color::rgb8(4, 17, 38))
        .border_bottom_width(2)
        .border_bottom_color(Color::rgb8(43, 74, 123).with_alpha(0.3)),
    )
    .child(
      Grid::new()
        .columns([
          GridTrack::px(310.0),
          GridTrack::px(310.0),
          GridTrack::fr(1.0),
        ])
        .align_items(Align::Center)
        .style(Style::new().full_size())
        .child([
          ColumnHeader::new(ls("Action")).style(self::heading_style()),
          ColumnHeader::new(ls("Keyboard")).style(self::heading_style()),
          ColumnHeader::new(ls("Controller")).style(self::heading_style()),
        ]),
    )
}

fn binding_row((action, keyboard, controller): (&str, &str, &str)) -> TableRow {
  TableRow::new()
    .host_name(format!(
      "input-binding-{}",
      action.to_ascii_lowercase().replace(' ', "-")
    ))
    .style(
      Style::new()
        .width(INPUT_WIDTH)
        .height(ROW_HEIGHT)
        .border_bottom_width(2)
        .border_bottom_color(Color::rgb8(43, 74, 123).with_alpha(0.25)),
    )
    .child(
      Grid::new()
        .columns([
          GridTrack::px(310.0),
          GridTrack::px(310.0),
          GridTrack::fr(1.0),
        ])
        .align_items(Align::Center)
        .style(Style::new().full_size())
        .child((
          RowHeader::new(ls(action)).style(self::action_style()),
          TableCell::new(ls(keyboard)).style(self::binding_style()),
          TableCell::new(ls(controller)).style(self::binding_style()),
        )),
    )
}

fn heading_style() -> Style {
  Style::new()
    .color(Color::rgb8(244, 245, 250))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(47)
    .letter_spacing(1.2)
    .unity_text_align(TextAnchor::MiddleCenter)
}

fn action_style() -> Style {
  Style::new()
    .padding_left(18)
    .color(Color::rgb8(245, 245, 248))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(54)
    .letter_spacing(1.3)
    .unity_text_align(TextAnchor::MiddleLeft)
}

fn binding_style() -> Style {
  Style::new()
    .color(Color::rgb8(246, 246, 250))
    .unity_font_definition(DISPLAY_FONT)
    .font_size(45)
    .letter_spacing(1.0)
    .unity_text_align(TextAnchor::MiddleCenter)
}

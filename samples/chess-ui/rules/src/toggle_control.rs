//! A controlled checkbox whose label activates and focuses its input.

use crate::{check_mark::CheckMark, setting_row::SettingRow};
use battlement::{
  Align, Color, Justify, Length, PickingMode, Position, Style, TextAnchor, Translate,
};
use battlement_reactant::{accessibility, prelude::*};
use std::rc::Rc;

/// A controlled checkbox with an associated, clickable settings label.
#[builder]
pub struct ToggleControl<R> {
  #[builder(required, into)]
  label: Rc<R>,
  /// Omits the top separator when this is the first row.
  first: bool,
  /// Offsets the control vertically without moving its row label.
  offset_y: f32,
  /// Sets the minimum row height in portrait design pixels.
  row_height: Option<f32>,
  #[builder(required)]
  checked: bool,
  /// Overrides the accessible name when the visible wording needs clarification.
  aria_label: Option<String>,
  /// Shows crash-report context next to the visible label.
  with_info: bool,
  /// Handles activation of the optional crash-report information badge.
  #[builder(default = EventCallback::noop())]
  on_info_click: EventCallback<()>,
  #[builder(required)]
  on_change: EventCallback<bool>,
}

impl<R: Render> Component for ToggleControl<R> {
  fn render(&self) -> impl Render {
    let label = use_control_label();
    let checkbox = accessibility::use_checkbox(ToggleOptions {
      name: self
        .aria_label
        .as_ref()
        .filter(|label| !label.is_empty())
        .map_or_else(|| label.name(), |name| AccessibleName::text(name.clone())),
      description: self
        .with_info
        .then(|| AccessibleDescription::text("We upload crash reports to Unity Diagnostics.")),
      checked: self.checked,
      is_disabled: false,
      on_change: self.on_change.clone(),
    });
    let (label, checkbox) = label.bind(checkbox);
    View::new()
      .name("toggle-control-label")
      .style(Style::new().height(self.row_height))
      .child(
        SettingRow::new()
          .label((
            self.label.clone(),
            self
              .with_info
              .then(|| InfoBadge::new().on_click(self.on_info_click.clone())),
          ))
          .children(
            View::new()
              .name("toggle-control-box")
              .style(
                Style::new()
                  .position(Position::Relative)
                  .align_items(Align::Center)
                  .width(77)
                  .height(77)
                  .margin_left(8)
                  .translate(Translate::two_dimensional(
                    Length::Px(0.0),
                    Length::Px(self.offset_y),
                  )),
              )
              .child((
                View::new()
                  .name("toggle-control-surface")
                  .picking_mode(PickingMode::Ignore)
                  .style(
                    Style::new()
                      .width(77)
                      .height(77)
                      .border_width(4)
                      .border_radius(11)
                      .border_color(Color::rgb8(75, 163, 255))
                      .background_color(Color::rgb8(2, 9, 26)),
                  )
                  .child(self.checked.then_some(CheckMark::new())),
                Button::new("")
                  .name("toggle-control-input")
                  .associated_control(checkbox)
                  .style(
                    Style::new()
                      .position(Position::Absolute)
                      .left(0)
                      .top(0)
                      .width(77)
                      .height(77)
                      .margin(0)
                      .padding(0)
                      .border_width(0)
                      .background_color(Color::TRANSPARENT),
                  ),
              )),
          )
          .associated_label(label)
          .first(self.first)
          .row_height(self.row_height),
      )
  }
}

#[builder]
struct InfoBadge {
  #[builder(required)]
  on_click: EventCallback<()>,
}

impl Component for InfoBadge {
  fn render(&self) -> impl Render {
    Button::new("i")
      .name("toggle-info")
      .behavior(accessibility::use_button(ButtonOptions {
        name: AccessibleName::text("About crash report uploads"),
        description: None,
        is_disabled: false,
        on_press: self.on_click.clone(),
      }))
      .style(
        Style::new()
          .position(Position::Absolute)
          .left(205)
          .bottom(37)
          .width(38)
          .height(38)
          .padding(0)
          .border_width(2)
          .border_color(Color::rgb8(85, 184, 255))
          .border_radius(19)
          .background_color(Color::TRANSPARENT)
          .color(Color::rgb8(188, 244, 255))
          .font_size(27)
          .unity_text_align(TextAnchor::MiddleCenter)
          .align_items(Align::Center)
          .justify_content(Justify::Center),
      )
  }
}

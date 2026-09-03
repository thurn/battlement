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
  on_info_click: Option<Rc<dyn Fn()>>,
  #[builder(required)]
  on_change: Rc<dyn Fn(bool)>,
}

impl<R: Render> Component for ToggleControl<R> {
  fn render(&self) -> impl Render {
    let label = use_control_label();
    let aria_label = self.aria_label.as_ref().filter(|label| !label.is_empty());
    let on_change = Rc::clone(&self.on_change);
    let checkbox = accessibility::use_checkbox(ToggleOptions {
      name: aria_label.map_or_else(|| label.name(), |name| AccessibleName::text(name.clone())),
      description: self
        .with_info
        .then(|| AccessibleDescription::text("We upload crash reports to Unity Diagnostics.")),
      checked: self.checked,
      is_disabled: false,
      on_change: move |checked| on_change(checked),
    });
    let (label, checkbox) = label.bind(checkbox);
    let control = View::new()
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
              .border_color(Color::rgb(75.0 / 255.0, 163.0 / 255.0, 1.0))
              .background_color(Color::rgb(2.0 / 255.0, 9.0 / 255.0, 26.0 / 255.0)),
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
              .background_color(Color::rgba(0.0, 0.0, 0.0, 0.0)),
          ),
      ));
    let mut row = SettingRow::new()
      .label((
        self.label.clone(),
        self
          .with_info
          .then(|| InfoBadge::new().on_click_optional(self.on_info_click.clone())),
      ))
      .children(control)
      .associated_label(label)
      .first(self.first);
    if let Some(height) = self.row_height {
      row = row.row_height(height);
    }
    let mut label = View::new().name("toggle-control-label").child(row);
    if let Some(height) = self.row_height {
      label = label.style(Style::new().height(height));
    }
    label
  }
}

#[builder]
struct InfoBadge {
  on_click: Option<Rc<dyn Fn()>>,
}

impl Component for InfoBadge {
  fn render(&self) -> impl Render {
    let on_click = self.on_click.clone();
    let button = accessibility::use_button(ButtonOptions {
      name: AccessibleName::text("About crash report uploads"),
      description: None,
      is_disabled: false,
      on_press: move || {
        if let Some(on_click) = &on_click {
          on_click();
        }
      },
    });
    Button::new("i").name("toggle-info").behavior(button).style(
      Style::new()
        .position(Position::Absolute)
        .left(205)
        .bottom(37)
        .width(38)
        .height(38)
        .padding(0)
        .border_width(2)
        .border_color(Color::rgb(85.0 / 255.0, 184.0 / 255.0, 1.0))
        .border_radius(19)
        .background_color(Color::rgba(0.0, 0.0, 0.0, 0.0))
        .color(Color::rgb(188.0 / 255.0, 244.0 / 255.0, 1.0))
        .font_size(27)
        .unity_text_align(TextAnchor::MiddleCenter)
        .align_items(Align::Center)
        .justify_content(Justify::Center),
    )
  }
}

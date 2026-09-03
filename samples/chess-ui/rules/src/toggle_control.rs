//! A controlled checkbox whose label activates and focuses its input.

use crate::{check_mark::CheckMark, setting_row::SettingRow};
use battlement::{Align, Color, Length, PickingMode, Position, Style, Translate};
use battlement_reactant::label_binding;
use battlement_reactant::{accessibility, element_ref, prelude::*};
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
  #[builder(required)]
  on_change: Rc<dyn Fn(bool)>,
}

impl<R: Render> Component for ToggleControl<R> {
  fn render(&self) -> impl Render {
    let label = label_binding::use_label();
    let input = element_ref::use_element_ref();
    let aria_label = self.aria_label.as_ref().filter(|label| !label.is_empty());
    let on_change = Rc::clone(&self.on_change);
    let checkbox = accessibility::use_checkbox(ToggleOptions {
      name: aria_label.map_or_else(|| label.name(), |name| AccessibleName::text(name.clone())),
      checked: self.checked,
      is_disabled: false,
      on_change: move |checked| on_change(checked),
    });
    let label_interaction = checkbox.label_interaction(&input);
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
          .element_ref(input.clone())
          .semantic(checkbox.semantic)
          .focus_props(checkbox.focus)
          .interaction_props(checkbox.interaction)
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
    let mut row = SettingRow::<R, _>::new()
      .label(self.label.clone())
      .children(control)
      .label_binding(label)
      .first(self.first);
    if let Some(height) = self.row_height {
      row = row.row_height(height);
    }
    let mut label = View::new()
      .name("toggle-control-label")
      .interaction_props(label_interaction)
      .child(row);
    if let Some(height) = self.row_height {
      label = label.style(Style::new().height(height));
    }
    label
  }
}

use std::rc::Rc;

use battlement::{Align, Color, Length, PickingMode, Position, Style, Translate};
use battlement_reactant::{accessibility, element_ref, prelude::*};

use crate::{check_mark::CheckMark, engine::Game, setting_row::SettingRow};

/// A controlled checkbox with an associated, clickable settings label.
pub struct ToggleControl {
  pub label: Node,
  pub first: Option<bool>,
  pub offset_y: f32,
  pub row_height: Option<f32>,
  pub checked: bool,
  pub aria_label: Option<String>,
  pub on_change: Rc<dyn Fn(bool)>,
  pub with_info: bool,
  pub on_info_click: Option<Rc<dyn Fn()>>,
}

impl ToggleControl {
  /// Creates a checkbox whose parent accepts or rejects each proposed value.
  pub fn new(label: impl Render, checked: bool, on_change: impl Fn(bool) + 'static) -> Self {
    Self {
      label: Node::new(label),
      first: None,
      offset_y: 0.0,
      row_height: None,
      checked,
      aria_label: None,
      on_change: Rc::new(on_change),
      with_info: false,
      on_info_click: None,
    }
  }

  pub fn aria_label(mut self, value: impl Into<String>) -> Self {
    self.aria_label = Some(value.into());
    self
  }

  pub fn offset_y(mut self, value: f32) -> Self {
    self.offset_y = value;
    self
  }

  pub fn row_height(mut self, value: f32) -> Self {
    self.row_height = Some(value);
    self
  }
}

impl Component for ToggleControl {
  fn render(&self) -> impl Render {
    let label_id = element_ref::use_element_ref();
    let input = element_ref::use_element_ref();
    let aria_label = self.aria_label.as_ref().filter(|label| !label.is_empty());
    let on_change = Rc::clone(&self.on_change);
    let checkbox = accessibility::use_checkbox(ToggleOptions {
      name: aria_label.map_or_else(
        || AccessibleName::LabelledBy(label_id.clone()),
        |name| AccessibleName::text(name.clone()),
      ),
      checked: self.checked,
      is_disabled: false,
      on_change: move |_: &mut Game, checked| on_change(checked),
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
          .child(self.checked.then_some(CheckMark::default())),
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
    let mut row = SettingRow::new(self.label.clone(), control).label_id(label_id);
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

use std::rc::Rc;

use battlement::{
  Align, Color, FlexDirection, PickingMode, Position, Style, TextAnchor, WhiteSpace,
};
use battlement_reactant::{
  accessibility::{self, SliderOptions},
  component::Component,
  element_ref,
  host::{Flex, TextElement, View},
  render::Render,
  semantics::{self, AccessibleName, SemanticVisibility},
};

use crate::{
  setting_row::{self, SettingRow},
  volume_skin,
};

/// A labelled volume slider whose parent owns its integer percentage.
pub struct VolumeControl {
  pub label: String,
  pub value: u32,
  pub on_change: Rc<dyn Fn(u32)>,
  pub first: bool,
}

impl VolumeControl {
  pub fn new(label: impl Into<String>, value: u32, on_change: impl Fn(u32) + 'static) -> Self {
    Self {
      label: label.into(),
      value,
      on_change: Rc::new(on_change),
      first: false,
    }
  }

  pub fn first(mut self, value: bool) -> Self {
    self.first = value;
    self
  }
}

impl Component for VolumeControl {
  fn render(&self) -> impl Render {
    let label_id = element_ref::use_element_ref();
    let on_change = Rc::clone(&self.on_change);
    let slider = accessibility::use_slider(SliderOptions {
      name: AccessibleName::LabelledBy(vec![label_id.clone()]),
      value: f64::from(self.value),
      minimum: 0.0,
      maximum: 100.0,
      step: 5.0,
      value_text: Some(semantics::text(format!("{} percent", self.value))),
      is_disabled: false,
      on_change: move |value| on_change(value as u32),
    });
    SettingRow::new(
      TextElement::new(self.label.clone()).semantic(
        accessibility::use_static_text(semantics::text(self.label.clone()))
          .visibility(SemanticVisibility::NameSourceOnly),
      ),
      Flex::new()
        .direction(FlexDirection::Row)
        .gap(18.0)
        .name("volume-control")
        .style(
          Style::new()
            .position(Position::Relative)
            .width(398)
            .height(82)
            .flex_shrink(0)
            .align_items(Align::Center),
        )
        .child((
          View::new()
            .name("volume-track-area")
            .style(
              Style::new()
                .position(Position::Relative)
                .width(284)
                .height(64)
                .flex_shrink(0),
            )
            .child((
              volume_skin::track(self.value),
              volume_skin::ticks(),
              volume_skin::thumb(self.value),
              View::new()
                .name("volume-input")
                .semantic(slider.semantic)
                .focus_props(slider.focus)
                .interaction_props(slider.interaction)
                .style(
                  Style::new()
                    .position(Position::Absolute)
                    .left(-42)
                    .top(-34)
                    .width(368)
                    .height(132),
                ),
            )),
          TextElement::new(format!("{}%", self.value))
            .name("volume-value")
            .picking_mode(PickingMode::Ignore)
            .style(
              Style::new()
                .width(96)
                .height(55)
                .flex_shrink(0)
                .color(Color::rgb(245.0 / 255.0, 245.0 / 255.0, 248.0 / 255.0))
                .unity_font_definition(setting_row::DISPLAY_FONT)
                .font_size(55)
                .white_space(WhiteSpace::NoWrap)
                .letter_spacing(1)
                .unity_text_align(TextAnchor::MiddleLeft),
            ),
        )),
    )
    .label_id(label_id)
    .first(self.first)
  }
}

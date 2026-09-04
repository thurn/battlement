//! A controlled integer-percentage slider with a painted track and value.

use crate::{
  setting_row::{self, SettingRow},
  volume_skin::{VolumeThumb, VolumeTicks, VolumeTrack},
};
use battlement::{
  Align, Color, FlexDirection, PickingMode, Position, Style, TextAnchor, WhiteSpace,
};
use battlement_reactant::prelude::builder;
use battlement_reactant::{
  accessibility::{self, SliderOptions},
  component::Component,
  host::{Flex, TextElement, View},
  label_binding::use_control_label,
  render::Render,
  semantics,
};
use std::rc::Rc;

/// A labelled volume slider whose parent owns its integer percentage.
#[builder]
pub struct VolumeControl {
  #[builder(required)]
  label: String,
  #[builder(required)]
  value: u32,
  #[builder(required)]
  on_change: Rc<dyn Fn(u32)>,
  /// Omits the separator above the first row.
  first: bool,
}

impl Component for VolumeControl {
  fn render(&self) -> impl Render {
    let label = use_control_label();
    let on_change = Rc::clone(&self.on_change);
    let slider = accessibility::use_slider(SliderOptions {
      name: label.name(),
      description: None,
      value: f64::from(self.value),
      minimum: 0.0,
      maximum: 100.0,
      step: 5.0,
      value_text: Some(semantics::text(format!("{} percent", self.value))),
      is_disabled: false,
      on_change: move |value| on_change(value as u32),
    });
    let (label, slider) = label.bind(slider);
    SettingRow::new()
      .label(accessibility::name_source_text(self.label.clone()))
      .children(
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
                VolumeTrack::new().value(self.value),
                VolumeTicks::new(),
                VolumeThumb::new().value(self.value),
                View::new()
                  .name("volume-input")
                  .associated_control(slider)
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
                  .color(Color::rgb8(245, 245, 248))
                  .unity_font_definition(setting_row::DISPLAY_FONT)
                  .font_size(55)
                  .white_space(WhiteSpace::NoWrap)
                  .letter_spacing(1)
                  .unity_text_align(TextAnchor::MiddleLeft),
              ),
          )),
      )
      .associated_label(label)
      .first(self.first)
  }
}

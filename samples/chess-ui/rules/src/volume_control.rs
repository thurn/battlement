//! A controlled integer-percentage slider with a painted track and value.

use trox::{LocalizedString, ls, tx_args, txa};

use crate::{
  setting_row::{self, SettingRow},
  volume_skin::{VolumeThumb, VolumeTicks, VolumeTrack},
};
use battlement::{
  Align, Color, FlexDirection, PickingMode, Position, Style, TextAnchor, WhiteSpace,
};
use battlement_reactant::prelude::{EventCallback, builder, use_control_label};
use battlement_reactant::{
  component::Component,
  control_behavior,
  host::{Flex, SliderHost, TextElement, View},
  render::Render,
  semantics::SemanticRange,
};

/// A labelled volume slider whose parent owns its integer percentage.
#[builder]
pub struct VolumeControl {
  #[builder(required)]
  label: LocalizedString,
  #[builder(required)]
  value: u32,
  #[builder(required)]
  on_change: EventCallback<u32>,
  /// Omits the separator above the first row.
  first: bool,
}

impl Component for VolumeControl {
  fn render(&self) -> impl Render {
    let (label, slider) = use_control_label().bind_with(|name| {
      control_behavior::slider(
        name,
        None,
        SemanticRange {
          current: f64::from(self.value),
          minimum: 0.0,
          maximum: 100.0,
          text: Some(txa(
            "{volume_percent} percent",
            tx_args![volume_percent => self.value],
            "Volume control dynamic value label.",
          )),
        },
        5.0,
        false,
        self.on_change.clone().map_input(|value: f64| value as u32),
      )
    });
    SettingRow::new()
      .label(control_behavior::name_source_text(self.label.clone()))
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
                SliderHost::new()
                  .label(ls(""))
                  .low_value(0.0)
                  .high_value(100.0)
                  .value(self.value as f32)
                  .name("volume-input")
                  .associated_control(slider)
                  .on_change_value(self.on_change.clone().map_input(|value: f32| value as u32))
                  .style(
                    Style::new()
                      .position(Position::Absolute)
                      .left(-42)
                      .top(-34)
                      .width(368)
                      .height(132),
                  ),
              )),
            TextElement::new(txa(
              "{volume_percent}%",
              tx_args![volume_percent => self.value],
              "Volume control dynamic value label.",
            ))
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

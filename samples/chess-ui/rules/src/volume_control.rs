//! A controlled integer-percentage slider with a painted track and value.

use trox::{LocalizedString, tx_args, txa};

use crate::{
  control_effects, font_scale,
  setting_row::{self, SettingRow},
  use_interaction, volume_input,
  volume_skin::{VolumeThumb, VolumeTicks, VolumeTrack},
};
use battlement::{
  Align, Color, FlexDirection, Length, PickingMode, Position, Scale, Style, TextAnchor,
  TransformOrigin, WhiteSpace,
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
    let interaction = use_interaction::use_interaction();
    let font_scale = font_scale::use_font_scale();
    let burst = control_effects::use_slider_burst(self.on_change.clone());
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
        burst.on_change.clone().map_input(|value: f64| value as u32),
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
              .align_items(Align::Center)
              .scale(Scale::uniform(1.0 + (font_scale.factor() - 1.0) * 0.35))
              .transform_origin(TransformOrigin::two_dimensional(
                Length::Px(0.0),
                Length::Percent(50.0),
              )),
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
                VolumeTrack::new()
                  .value(self.value)
                  .interaction(interaction.state),
                VolumeTicks::new(),
                VolumeThumb::new()
                  .value(self.value)
                  .interaction(interaction.state),
                View::decorative()
                  .name("volume-release-effect")
                  .style(
                    Style::new()
                      .position(Position::Absolute)
                      .left(self.value as f32 * 2.84 - 21.5)
                      .top(0)
                      .width(43)
                      .height(64),
                  )
                  .after_all(control_effects::slider_burst(
                    burst.generation,
                    control_effects::EffectPlayback {
                      reduced_motion: interaction.state.reduced_motion,
                      ..Default::default()
                    },
                  )),
                {
                  let slider_reference = slider.reference();
                  interaction.slider_with_release(
                    SliderHost::new()
                      .low_value(0.0)
                      .high_value(100.0)
                      .value(self.value as f32)
                      .name("volume-input")
                      .associated_control(slider)
                      .on_key_down_event_callback(burst.on_change.clone().filter_map_input({
                        let value = self.value;
                        move |event| volume_input::key_down(event, value)
                      }))
                      .on_navigation_move_event_callback(burst.on_change.clone().filter_map_input(
                        {
                          let value = self.value;
                          move |event| volume_input::navigation_move(event, value)
                        },
                      )),
                    slider_reference,
                    burst.on_pointer_begin.clone(),
                    burst.on_pointer_release.clone(),
                    burst.on_pointer_cancel.clone(),
                  )
                }
                .on_change_value(
                  burst
                    .on_change
                    .clone()
                    .map_input(volume_input::pointer_value),
                )
                .style(
                  Style::new()
                    .position(Position::Absolute)
                    .left(-42)
                    .top(-34)
                    .width(368)
                    .height(132)
                    .opacity(0.0),
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
                .font_size(55.0 * font_scale.factor() / (1.0 + (font_scale.factor() - 1.0) * 0.35))
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

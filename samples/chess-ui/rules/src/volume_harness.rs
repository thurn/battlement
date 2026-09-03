use crate::{review_button::ReviewButton, volume_control::VolumeControl};
use battlement::{Color, Style};
use battlement_reactant::prelude::builder;
use battlement_reactant::{accessibility, component::Component, hooks, host::View, render::Render};

/// Owns slider percentages and demonstrates accepted and rejected changes.
#[builder]
pub struct VolumeHarness;

impl Component for VolumeHarness {
  fn render(&self) -> impl Render {
    let (value, set_value) = hooks::use_state(80_u32);
    let (changes, set_changes) = hooks::use_state(0_u32);
    let external = set_value.clone();
    View::new()
      .name("volume-specimen")
      .style(
        Style::new()
          .width(839)
          .margin_top(48)
          .background_color(Color::rgb(0.01, 0.035, 0.08)),
      )
      .child((
        VolumeControl::new()
          .label("Master Volume")
          .value(value)
          .on_change(move |value| {
            set_value.set(value);
            set_changes.update(|count| count + 1);
          })
          .first(true),
        VolumeControl::new()
          .label("Minimum")
          .value(0)
          .on_change(|_| {}),
        VolumeControl::new()
          .label("Maximum")
          .value(100)
          .on_change(|_| {}),
        accessibility::static_label(format!("Volume changes: {changes}")).style(
          Style::new()
            .font_size(28)
            .color(Color::rgb(0.75, 0.86, 0.97))
            .margin_top(30),
        ),
        ReviewButton::new()
          .label("Change volume from parent")
          .on_press(move || external.update(|value| if value == 25 { 80 } else { 25 })),
      ))
  }
}

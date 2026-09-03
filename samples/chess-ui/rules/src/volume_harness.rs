use battlement::{Color, Style};
use battlement_reactant::{
  accessibility,
  component::Component,
  hooks,
  host::{Label, View},
  render::Render,
  semantics,
};

use crate::{review_button::ReviewButton, volume_control::VolumeControl};

/// Owns slider percentages and demonstrates accepted and rejected changes.
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
        VolumeControl::new("Master Volume", value, move |value| {
          set_value.set(value);
          set_changes.update(|count| count + 1);
        })
        .first(true),
        VolumeControl::new("Minimum", 0, |_| {}),
        VolumeControl::new("Maximum", 100, |_| {}),
        Label::new(format!("Volume changes: {changes}"))
          .semantic(accessibility::use_static_text(semantics::text(format!(
            "Volume changes: {changes}"
          ))))
          .style(
            Style::new()
              .font_size(28)
              .color(Color::rgb(0.75, 0.86, 0.97))
              .margin_top(30),
          ),
        ReviewButton::new("Change volume from parent")
          .on_press(move || external.update(|value| if value == 25 { 80 } else { 25 })),
      ))
  }
}

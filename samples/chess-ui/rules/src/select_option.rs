//! One focus-managed option in the custom selector.

use trox::ls;

use crate::{
  check_mark::CheckMark, control_effects, dropdown_motion, select_control::VALUE_FONT,
  use_interaction,
};
use battlement::{
  Align, Color, FlexDirection, Gradient, Justify, PickingMode, Position, Shadow, Style, TextAnchor,
};
use battlement_reactant::{
  element_ref, hooks,
  paint::{PaintLayer, PaintStyle},
  prelude::*,
};

/// Renders one controlled option and focuses it while it is active.
#[builder]
pub(crate) struct SelectOption {
  active: bool,
  #[builder(required)]
  control_scale: f32,
  #[builder(required)]
  font_scale: f32,
  focus_generation: u32,
  index: usize,
  #[builder(required)]
  label: String,
  #[builder(required)]
  on_press: EventCallback<()>,
  selected: bool,
}

impl Component for SelectOption {
  fn render(&self) -> impl Render {
    let reference = element_ref::use_element_ref();
    let interaction = use_interaction::use_interaction();
    let (selection_flash, set_selection_flash) = hooks::use_state(0_u32);
    let (burst_generation, on_press) = control_effects::use_burst_callback(
      set_selection_flash
        .update_callback(|generation| generation.wrapping_add(1))
        .then(self.on_press.clone()),
    );
    hooks::use_effect(
      {
        let reference = reference.clone();
        let active = self.active;
        move || {
          if active {
            reference.focus();
          }
        }
      },
      (self.active, self.focus_generation),
    );
    ListBoxOption::new(ls(self.label.clone()), self.selected)
      .host_name(format!("select-option-{}", self.label.to_ascii_lowercase()))
      .element_ref(reference)
      .key(self.index)
      .style(self::style(self.font_scale, self.control_scale))
      .initial(dropdown_motion::option_initial(
        interaction.state.reduced_motion,
      ))
      .animate(dropdown_motion::option_visible())
      .exit(dropdown_motion::option_exit(
        interaction.state.reduced_motion,
      ))
      .transition(dropdown_motion::option_transition(
        interaction.state.reduced_motion,
        self.index,
      ))
      .paint(self::paint(self.active))
      .hover_style(Style::new().background_color(Color::rgba8(11, 113, 207, 128)))
      .configure_host(|host| {
        interaction
          .button(host)
          .before_all(control_effects::button_burst(
            burst_generation,
            true,
            control_effects::EffectPlayback {
              reduced_motion: interaction.state.reduced_motion,
              ..Default::default()
            },
          ))
      })
      .on_press(on_press)
      .child((
        (selection_flash > 0).then(|| {
          View::decorative()
            .name("select-option-flash")
            .key(("selection-flash", selection_flash))
            .picking_mode(PickingMode::Ignore)
            .style(
              Style::new()
                .position(Position::Absolute)
                .left(3)
                .right(3)
                .top(3)
                .bottom(3)
                .border_width(2)
                .border_color(Color::hex(0x66f6ff)),
            )
            .paint(
              PaintStyle::new()
                .box_shadow(Some(Shadow::inset(
                  0.0,
                  0.0,
                  12.0,
                  0.0,
                  Color::rgba8(47, 143, 255, 140),
                )))
                .layer(
                  PaintLayer::new(Color::TRANSPARENT).box_shadow(Some(Shadow::outer(
                    0.0,
                    0.0,
                    10.0,
                    0.0,
                    Color::hex(0xff50d1),
                  ))),
                ),
            )
            .initial(dropdown_motion::flash_initial(
              interaction.state.reduced_motion,
            ))
            .animate(dropdown_motion::flash_target(
              interaction.state.reduced_motion,
            ))
        }),
        View::decorative()
          .name("select-option-mark")
          .picking_mode(PickingMode::Ignore)
          .style(
            Style::new()
              .position(Position::Absolute)
              .right(20)
              .top(16)
              .width(48)
              .height(44)
              .flex_shrink(0.0),
          )
          .child(self.selected.then(|| CheckMark::new().scale(0.62))),
      ))
  }
}

fn style(font_scale: f32, control_scale: f32) -> Style {
  Style::new()
    .position(Position::Relative)
    .width(100.pct())
    .height(76.0 * control_scale)
    .min_height(76.0 * control_scale)
    .flex_direction(FlexDirection::Row)
    .align_items(Align::Center)
    .justify_content(Justify::SpaceBetween)
    .padding_top(6.0 * control_scale)
    .padding_bottom(6.0 * control_scale)
    .padding_left(25.0 * control_scale)
    .padding_right(20.0 * control_scale)
    .border_width(0)
    .background_color(Color::TRANSPARENT)
    .color(Color::hex(0xd9e1f2))
    .unity_font_definition(VALUE_FONT)
    .font_size(47.0 * font_scale)
    .unity_text_align(TextAnchor::MiddleLeft)
}

fn paint(active: bool) -> PaintStyle {
  PaintStyle::new()
    .background(if active {
      Gradient::linear(90.0)
        .stop(0.0, Color::rgba8(255, 238, 0, 82))
        .stop(1.0, Color::rgba8(255, 167, 0, 36))
    } else {
      Gradient::linear(90.0)
        .stop(0.0, Color::TRANSPARENT)
        .stop(1.0, Color::TRANSPARENT)
    })
    .box_shadow(active.then(|| Shadow::inset(0.0, 0.0, 0.0, 3.0, Color::hex(0xfff400))))
}

//! The source-painted destructive action row used by Gameplay settings.

use battlement::{
  Color, Gradient, Length, MotionProperty, Position, Shadow, Style, TextAnchor, Translate,
};
use battlement_reactant::{
  control_behavior,
  host::ButtonHost,
  motion::{Easing, MotionTarget, StyleTarget, Transition},
  paint::{PaintLayer, PaintStyle},
  prelude::{PaintDropShadow, PaintFilterList, *},
};
use trox::{ls, tx};

use crate::{action_skin, control_effects, font_scale, setting_row::SettingRow, use_interaction};

/// A red arcade action whose visible row label supplies its accessible name.
#[builder]
pub struct EraseControl {
  #[builder(default = EventCallback::noop())]
  on_click: EventCallback<()>,
}

impl Component for EraseControl {
  fn render(&self) -> impl Render {
    let interaction = use_interaction::use_interaction();
    let scale = font_scale::use_font_scale();
    let (burst_generation, on_click) = control_effects::use_burst_callback(self.on_click.clone());
    let (label, button) = use_control_label()
      .bind_with(move |name| control_behavior::button(name, None, false, on_click.clone()));
    SettingRow::new()
      .label(control_behavior::name_source_text(tx(
        "Erase Saved Data",
        "Saved-data action row label.",
      )))
      .associated_label(label)
      .children(
        View::new()
          .name("erase-control")
          .style(
            Style::new()
              .position(Position::Relative)
              .width(362.0 * (1.0 + (scale.factor() - 1.0) * 0.25))
              .height(114.0 * (1.0 + (scale.factor() - 1.0) * 0.35))
              .margin_left(21)
              .translate(Translate::two_dimensional(
                Length::Px(0.0),
                Length::Px(-8.0),
              )),
          )
          .child(
            interaction
              .button(
                ButtonHost::new(ls(""))
                  .name("erase-control-button")
                  .associated_control(button)
                  .child(
                    Text::new(ls("ERASE")).style(
                      Style::new()
                        .full_size()
                        .color(Color::hex(0xff3553))
                        .unity_font_definition(crate::action_button::ACTION_FONT)
                        .font_size(67.0 * scale.dynamic(font_scale::FontScaleRole::Control))
                        .unity_text_align(TextAnchor::MiddleCenter),
                    ),
                  ),
              )
              .style(
                Style::new()
                  .position(Position::Relative)
                  .full_size()
                  .margin(0)
                  .padding(0)
                  .border_width(0)
                  .background_color(Color::TRANSPARENT)
                  .color(Color::hex(0xff3553)),
              )
              .paint(self::paint())
              .initial(false)
              .animate(self::target(interaction.state))
              .before_all(control_effects::button_burst(
                burst_generation,
                true,
                control_effects::EffectPlayback {
                  reduced_motion: interaction.state.reduced_motion,
                  ..Default::default()
                },
              )),
          ),
      )
  }
}

fn paint() -> PaintStyle {
  PaintStyle::new()
    .background(
      Gradient::linear(20.0)
        .stop(0.0, Color::hex(0xff355e))
        .stop(0.55, Color::hex(0xff204f))
        .stop(1.0, Color::hex(0xff75a1)),
    )
    .paint_filter(filter(1.0, 9.0))
    .clip_polygon(action_skin::clip(18.0, 17.0))
    .layer(
      PaintLayer::new(
        Gradient::radial([0.5, 0.45], [0.5, 0.5])
          .stop(0.0, Color::hex(0x200511))
          .stop(0.67, Color::hex(0x07030c))
          .stop(1.0, Color::hex(0x020208)),
      )
      .bounds_inset(4.0)
      .clip_polygon(action_skin::clip(14.0, 13.0))
      .box_shadow(Some(Shadow::inset(0.0, 0.0, 22.0, 0.0, Color::BLACK))),
    )
}

fn target(state: use_interaction::InteractionState) -> MotionTarget {
  MotionTarget::new(
    StyleTarget::new()
      .background_gradient(if state.focus_visible {
        use_interaction::focus_gradient(110.0)
      } else if state.hovered {
        Gradient::linear(20.0)
          .stop(0.0, Color::hex(0xff657f))
          .stop(0.55, Color::hex(0xff204f))
          .stop(1.0, Color::hex(0xff75a1))
      } else {
        Gradient::linear(20.0)
          .stop(0.0, Color::hex(0xff355e))
          .stop(0.55, Color::hex(0xff204f))
          .stop(1.0, Color::hex(0xff75a1))
      })
      .paint_filter(if state.focus_visible {
        use_interaction::focus_filter()
      } else {
        self::filter(
          if state.hovered { 1.16 } else { 1.0 },
          if state.hovered { 15.0 } else { 9.0 },
        )
      })
      .scale(if state.pressed && !state.reduced_motion {
        0.96
      } else {
        1.0
      }),
  )
  .transition(
    Transition::tween()
      .duration_secs(0.14)
      .ease(Easing::Ease)
      .property(
        MotionProperty::Scale,
        Transition::tween()
          .duration_secs(0.09)
          .ease(Easing::CubicBezier([0.2, 0.8, 0.2, 1.0])),
      ),
  )
}

fn filter(brightness: f32, blur: f32) -> PaintFilterList {
  PaintFilterList::default()
    .brightness(brightness)
    .drop_shadow(PaintDropShadow::new(
      0.0,
      0.0,
      blur,
      0.0,
      Color::hex(0xff144e).with_alpha(0.65),
    ))
}

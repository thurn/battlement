use trox::{assert_localized, tx};

use crate::{Game, design_system};
use battlement::{
  Align, AudioClipAddress, Color, FlexDirection, FlexWrap, Length, LengthUnits, Overflow,
  ScrollViewMode, ScrollerVisibility, Shadow, Style, WhiteSpace, object_id,
};
use battlement_reactant::prelude::*;
use std::time::Duration;

const AUDIO_PLAYBACK_ID: battlement::ObjectId = object_id!("72100000-0000-4000-8000-000000000001");

const AUDIO_CLIP: AudioClipAddress = AudioClipAddress::from_static("reactant/assets/clock-pulse");

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ComposedEffectsState {
  dropdown_open: bool,
  selected: usize,
  modal_open: bool,
  route: usize,
  burst: u32,
  checked: bool,
  slider: usize,
  reduced_motion: ReducedMotion,
  reconnects: u32,
}

impl Default for ComposedEffectsState {
  fn default() -> Self {
    Self {
      dropdown_open: true,
      selected: 0,
      modal_open: true,
      route: 0,
      burst: 0,
      checked: false,
      slider: 1,
      reduced_motion: ReducedMotion::User,
      reconnects: 0,
    }
  }
}

#[builder]
pub(crate) struct ComposedEffects {
  pub(crate) state: ComposedEffectsState,
  pub(crate) compact: bool,
}

impl Component for ComposedEffects {
  fn render(&self) -> impl Render {
    let audio = AudioPlayback::new(AUDIO_PLAYBACK_ID);
    let audio_time = use_motion_time(MotionTimeSource::Audio(audio));
    let audio_scale = use_transform(
      audio_time,
      InputRange::new([
        Duration::ZERO,
        Duration::from_millis(250),
        Duration::from_millis(500),
      ]),
      OutputRange::new([0.82, 1.22, 0.82]),
    );
    MotionConfig::new(
      ScrollView::new()
        .name("composed-effects-canvas")
        .mode(ScrollViewMode::Vertical)
        .horizontal_scroller_visibility(ScrollerVisibility::Hidden)
        .vertical_scroller_visibility(ScrollerVisibility::Auto)
        .style(design_system::canvas(self.compact).padding(0.0))
        .content_container_style(content())
        .child(
          Label::new(tx(
            "PUBLIC MOTION COMPOSITION",
            "User-facing product copy in the Reactant sample.",
          ))
          .style(eyebrow()),
        )
        .child(
          Label::new(tx(
            "Composed Effects",
            "User-facing product copy in the Reactant sample.",
          ))
          .name("page-title")
          .style(title()),
        )
        .child(
          Label::new(assert_localized(format!(
            "{} · ROUTE {} · BURST {} · RECONNECTS {}",
            reduced_name(self.state.reduced_motion),
            self.state.route + 1,
            self.state.burst,
            self.state.reconnects,
          )))
          .name("composed-status")
          .style(status()),
        )
        .child(controls(audio))
        .child(
          View::new()
            .name("composed-gallery")
            .style(gallery())
            .child(dropdown(&self.state))
            .child(modal(&self.state))
            .child(routes(&self.state))
            .child(interactions(&self.state))
            .child(ambient(audio_scale, self.state.burst)),
        ),
    )
    .transition(
      Transition::tween()
        .duration_secs(0.24)
        .ease(Easing::EaseOut),
    )
    .reduced_motion(self.state.reduced_motion)
  }
}

fn controls(audio: AudioPlayback) -> View {
  let app = use_app();
  View::new()
    .style(control_row())
    .child(action("DROPDOWN", "composed-dropdown", |game| {
      game.composed_effects.dropdown_open = !game.composed_effects.dropdown_open;
    }))
    .child(action("MODAL", "composed-modal", |game| {
      game.composed_effects.modal_open = !game.composed_effects.modal_open;
    }))
    .child(action("ROUTE", "composed-route", |game| {
      game.composed_effects.route = (game.composed_effects.route + 1) % 3;
    }))
    .child(action("BURST", "composed-burst", |game| {
      game.composed_effects.burst = game.composed_effects.burst.wrapping_add(1);
    }))
    .child(action("REDUCED", "composed-reduced", |game| {
      game.composed_effects.reduced_motion = match game.composed_effects.reduced_motion {
        ReducedMotion::User => ReducedMotion::Always,
        ReducedMotion::Always => ReducedMotion::Never,
        ReducedMotion::Never => ReducedMotion::User,
      };
    }))
    .child(action("RECONNECT", "composed-reconnect", {
      let app = app.clone();
      move |game| {
        game.composed_effects.reconnects = game.composed_effects.reconnects.wrapping_add(1);
        app.refresh_snapshot();
      }
    }))
    .child(action("AUDIO", "composed-audio", {
      let app = app.clone();
      move |_game| {
        app.send(audio.play_command(AUDIO_CLIP, AudioPlaybackOptions::new().looping(true)));
      }
    }))
}

fn dropdown(state: &ComposedEffectsState) -> View {
  let options = ["GENERAL", "AUDIO", "CONTROLS"];
  specimen(
    "composed-dropdown-specimen",
    "DROPDOWN",
    "stagger · flash · retained exit",
  )
  .child(
    Button::new(assert_localized(options[state.selected]))
      .style(probe())
      .hover_style(
        StyleTarget::new()
          .scale(1.03)
          .filter(MotionFilterList::default().contrast(1.15)),
      )
      .active_style(StyleTarget::new().scale(0.97))
      .style_transition(StyleTransition::new().all(Transition::tween().duration_secs(0.12))),
  )
  .child(AnimatePresence::new().child(state.dropdown_open.then(|| {
    Node::new(
      View::new()
        .key("dropdown-menu")
        .style(menu())
        .initial(StyleTarget::new().y(-12.0).opacity(0.0).scale_y(0.86))
        .animate(StyleTarget::new().y(0.0).opacity(1.0).scale_y(1.0))
        .exit(StyleTarget::new().y(-8.0).opacity(0.0).scale_y(0.92))
        .child(
          options
            .into_iter()
            .enumerate()
            .map(|(index, label)| {
              Button::new(assert_localized(label))
                .key(label)
                .name(format!("composed-option-{index}"))
                .style(option())
                .initial(StyleTarget::new().x(-18.0).opacity(0.0))
                .animate(StyleTarget::new().x(0.0).opacity(1.0))
                .transition(
                  Transition::tween()
                    .duration_secs(0.18)
                    .delay_secs(index as f64 * 0.055),
                )
                .on_click(move |game: &mut Game| {
                  game.composed_effects.selected = index;
                  game.composed_effects.burst = game.composed_effects.burst.wrapping_add(1);
                })
            })
            .collect::<Vec<_>>(),
        )
        .child(
          View::new()
            .key(("selection-flash", state.selected, state.burst))
            .style(flash())
            .initial(StyleTarget::new().opacity(0.9).scale(0.5))
            .animate(StyleTarget::new().opacity(0.0).scale(1.3)),
        ),
    )
  })))
}

fn modal(state: &ComposedEffectsState) -> View {
  specimen(
    "composed-modal-specimen",
    "MODAL",
    "backdrop · filter mix · shine",
  )
  .child(
    AnimatePresence::new()
      .mode(PresenceMode::Wait)
      .child(state.modal_open.then(|| {
        Node::new(
          View::new()
            .key("settings-modal")
            .style(backdrop())
            .initial(StyleTarget::new().opacity(0.0))
            .animate(StyleTarget::new().opacity(1.0))
            .exit(StyleTarget::new().opacity(0.0))
            .child(
              View::new()
                .style(modal_panel())
                .initial(
                  StyleTarget::new()
                    .y(28.0)
                    .scale(0.88)
                    .filter(MotionFilterList::default().blur(8.0).contrast(0.6)),
                )
                .animate(
                  StyleTarget::new()
                    .y(0.0)
                    .scale(1.0)
                    .filter(MotionFilterList::default().blur(0.0).contrast(1.0)),
                )
                .exit(StyleTarget::new().y(18.0).scale(0.92).opacity(0.0))
                .after(
                  Decoration::new()
                    .key("modal-shine")
                    .style(shine())
                    .animation(loop_animation(
                      StyleTarget::new().x(-150.0).skew_x(-18.0).opacity(0.0),
                      StyleTarget::new().x(150.0).skew_x(-18.0).opacity(0.7),
                      1.8,
                    )),
                )
                .child(
                  Label::new(tx(
                    "SETTINGS READY",
                    "User-facing product copy in the Reactant sample.",
                  ))
                  .style(probe_label()),
                ),
            ),
        )
      })),
  )
}

fn routes(state: &ComposedEffectsState) -> View {
  specimen(
    "composed-routes-specimen",
    "ROUTES",
    "direction · PopLayout · beam / scan",
  )
  .style(specimen_style().overflow(Overflow::Hidden))
  .child(
    LayoutGroup::new("composed-tabs").child(
      View::new().style(row()).child(
        (0..3)
          .map(|index| {
            View::new()
              .key(index)
              .style(tab())
              .child(Label::new(assert_localized(format!("0{}", index + 1))))
              .child((index == state.route).then(|| {
                View::new()
                  .layout_id("composed-active-tab")
                  .layout(Layout::Both)
                  .style(indicator())
              }))
          })
          .collect::<Vec<_>>(),
      ),
    ),
  )
  .child(
    AnimatePresence::new()
      .mode(PresenceMode::PopLayout)
      .child(Node::new(
        View::new()
          .key(state.route)
          .layout(Layout::Both)
          .style(route_panel())
          .initial(StyleTarget::new().x(70.0).opacity(0.0).clip_inset([
            Length::px(0.0),
            Length::percent(100.0),
            Length::px(0.0),
            Length::px(0.0),
          ]))
          .animate(
            StyleTarget::new()
              .x(0.0)
              .opacity(1.0)
              .clip_inset([Length::px(0.0); 4]),
          )
          .exit(StyleTarget::new().x(-70.0).opacity(0.0))
          .after(
            Decoration::new()
              .key(("route-beam", state.route))
              .style(beam())
              .animation(loop_animation(
                StyleTarget::new().x(-160.0).opacity(0.1),
                StyleTarget::new().x(160.0).opacity(0.9),
                1.35,
              )),
          )
          .child(Label::new(assert_localized(format!(
            "ROUTE {} ACTIVE",
            state.route + 1
          )))),
      )),
  )
}

fn interactions(state: &ComposedEffectsState) -> View {
  let particles = (0..5).map(|index| {
    Decoration::new()
      .key((state.burst, index))
      .style(particle(index))
      .animation(
        Animation::new(Keyframes::new([
          StyleTarget::new().scale(0.4).opacity(0.9),
          StyleTarget::new().scale(1.8).opacity(0.0),
        ]))
        .duration_secs(0.55)
        .delay_secs(index as f64 * 0.035)
        .fill(AnimationFill::Forwards)
        .animation_key((state.burst, index)),
      )
  });
  specimen(
    "composed-interactions-specimen",
    "CONTROL BURSTS",
    "button · checkbox · slider",
  )
  .child(
    Button::new(if state.checked {
      tx(
        "[x] ENABLED",
        "User-facing product copy in the Reactant sample.",
      )
    } else {
      tx(
        "[ ] ENABLE",
        "User-facing product copy in the Reactant sample.",
      )
    })
    .name("composed-checkbox")
    .style(probe())
    .hover_style(
      StyleTarget::new()
        .scale(1.04)
        .background_color(Color::rgba(0.18, 0.72, 0.76, 1.0))
        .box_shadow([Shadow {
          x: 0.0,
          y: 5.0,
          blur: 0.0,
          spread: 1.0,
          color: Color::rgba(0.0, 0.9, 1.0, 0.4),
          inset: false,
        }]),
    )
    .active_style(StyleTarget::new().scale(0.94).rotate(-1.5))
    .focus_style(StyleTarget::new().color(Color::rgba(1.0, 0.72, 0.2, 1.0)))
    .style_transition(
      StyleTransition::new().all(
        Transition::tween()
          .duration_secs(0.14)
          .ease(Easing::EaseOut),
      ),
    )
    .after_all(particles)
    .on_click(|game: &mut Game| {
      game.composed_effects.checked = !game.composed_effects.checked;
      game.composed_effects.burst = game.composed_effects.burst.wrapping_add(1);
    }),
  )
  .child(
    Button::new(assert_localized(format!("SLIDER  {}%", state.slider * 25)))
      .name("composed-slider")
      .style(slider())
      .on_click(|game: &mut Game| {
        game.composed_effects.slider = (game.composed_effects.slider + 1) % 5;
        game.composed_effects.burst = game.composed_effects.burst.wrapping_add(1);
      }),
  )
}

fn ambient(
  audio_scale: battlement_reactant::motion_value::MotionValue<f32>,
  generation: u32,
) -> View {
  specimen(
    "composed-ambient-specimen",
    "AMBIENT + AUDIO",
    "grid · particle · comet · binding",
  )
  .style(specimen_style().overflow(Overflow::Hidden))
  .child(
    View::new()
      .style(ambient_stage())
      .child(ambient_probe("grid", 2.8, -0.4))
      .child(ambient_probe("particle", 1.7, -0.9))
      .child(ambient_probe("comet", 3.4, -1.8))
      .child(ambient_probe("binding", 2.2, -1.2))
      .child(
        View::new()
          .key(("audio", generation))
          .name("composed-audio-pulse")
          .style(audio_probe())
          .animate(StyleTarget::new().scale_value(audio_scale)),
      ),
  )
}

fn ambient_probe(name: &'static str, duration: f64, delay: f64) -> View {
  View::new()
    .name(format!("composed-{name}"))
    .style(dot())
    .animation(
      loop_animation(
        StyleTarget::new().x(-55.0).opacity(0.22).scale(0.65),
        StyleTarget::new().x(55.0).opacity(1.0).scale(1.1),
        duration,
      )
      .delay_secs(delay),
    )
}

fn loop_animation(from: StyleTarget, to: StyleTarget, duration: f64) -> Animation {
  Animation::new(Keyframes::new([from, to]))
    .duration_secs(duration)
    .iterations(AnimationIterations::Forever)
    .direction(AnimationDirection::Alternate)
    .fill(AnimationFill::Both)
}

fn specimen(name: &'static str, heading: &'static str, detail: &'static str) -> View {
  View::new()
    .name(name)
    .style(specimen_style())
    .child(Label::new(assert_localized(heading)).style(specimen_title()))
    .child(Label::new(assert_localized(detail)).style(specimen_detail()))
}

fn action(
  text: &'static str,
  name: &'static str,
  callback: impl Fn(&mut Game) + 'static,
) -> Button {
  Button::new(assert_localized(text))
    .name(name)
    .style(action_style())
    .on_click(callback)
}

fn reduced_name(value: ReducedMotion) -> &'static str {
  match value {
    ReducedMotion::User => "SYSTEM MOTION",
    ReducedMotion::Always => "REDUCED MOTION",
    ReducedMotion::Never => "FULL MOTION",
  }
}

fn content() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .padding(28.0)
    .align_items(Align::FlexStart)
}

fn eyebrow() -> Style {
  Style::new()
    .font_size(20.0)
    .color(Color::rgb(0.98, 0.4, 0.16))
}

fn title() -> Style {
  Style::new()
    .font_size(40.0)
    .color(Color::rgb(0.94, 0.98, 0.99))
    .margin((6, 0, 12, 0))
}

fn status() -> Style {
  Style::new()
    .font_size(18.0)
    .color(Color::rgb(0.68, 0.76, 0.78))
}

fn control_row() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .margin((10, 0))
}

fn action_style() -> Style {
  Style::new()
    .height(40.0)
    .min_width(92.0)
    .background_color(Color::rgb(0.035, 0.09, 0.115))
    .color(Color::rgb(0.94, 0.98, 0.99))
    .border_color(Color::rgb(0.32, 0.92, 0.96))
    .border_width(1.0)
    .font_size(14.0)
    .margin((0, 7, 7, 0))
}

fn gallery() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
}

fn specimen_style() -> Style {
  Style::new()
    .width(310.0)
    .min_height(250.0)
    .padding(12.0)
    .margin((6, 8, 6, 0))
    .background_color(Color::rgb(0.025, 0.06, 0.08))
    .border_color(Color::rgb(0.15, 0.28, 0.32))
    .border_width(1.0)
}

fn specimen_title() -> Style {
  Style::new()
    .font_size(18.0)
    .color(Color::rgb(0.94, 0.98, 0.99))
}

fn specimen_detail() -> Style {
  Style::new()
    .font_size(13.0)
    .white_space(WhiteSpace::Normal)
    .color(Color::rgb(0.68, 0.76, 0.78))
    .margin((6, 0))
}

fn probe() -> Style {
  Style::new()
    .width(190.0)
    .height(42.0)
    .margin((8, 0))
    .background_color(Color::rgb(0.08, 0.39, 0.44))
    .border_color(Color::rgb(0.22, 0.86, 0.9))
    .border_width(1.0)
    .border_radius(8.0)
    .color(Color::rgb(0.95, 0.99, 1.0))
}

fn probe_label() -> Style {
  Style::new()
    .color(Color::rgb(0.95, 0.99, 1.0))
    .font_size(16.0)
}

fn menu() -> Style {
  Style::new()
    .width(210.0)
    .padding(8.0)
    .background_color(Color::rgb(0.03, 0.1, 0.13))
    .overflow(Overflow::Hidden)
}

fn option() -> Style {
  Style::new()
    .height(34.0)
    .margin((2, 0))
    .background_color(Color::rgb(0.05, 0.16, 0.19))
    .color(Color::rgb(0.9, 0.98, 1.0))
}

fn flash() -> Style {
  Style::new()
    .position(battlement::Position::Absolute)
    .width(190.0)
    .height(36.0)
    .border_color(Color::rgb(1.0, 0.66, 0.18))
    .border_width(2.0)
    .border_radius(8.0)
}

fn backdrop() -> Style {
  Style::new()
    .width(270.0)
    .height(150.0)
    .padding(18.0)
    .background_color(Color::rgba(0.0, 0.0, 0.0, 0.55))
    .align_items(Align::Center)
    .overflow(Overflow::Hidden)
}

fn modal_panel() -> Style {
  Style::new()
    .width(210.0)
    .height(100.0)
    .padding(18.0)
    .background_color(Color::rgb(0.07, 0.18, 0.24))
    .border_color(Color::rgb(0.28, 0.9, 0.94))
    .border_width(1.0)
    .overflow(Overflow::Hidden)
}

fn shine() -> Style {
  Style::new()
    .width(52.0)
    .background_color(Color::rgba(0.65, 0.96, 1.0, 0.26))
}

fn row() -> Style {
  Style::new().flex_direction(FlexDirection::Row)
}

fn tab() -> Style {
  Style::new()
    .width(72.0)
    .height(36.0)
    .margin((0, 5, 5, 0))
    .align_items(Align::Center)
    .background_color(Color::rgb(0.04, 0.12, 0.15))
}

fn indicator() -> Style {
  Style::new()
    .position(battlement::Position::Absolute)
    .height(3.0)
    .left(0.0)
    .right(0.0)
    .bottom(0.0)
    .background_color(Color::rgb(0.98, 0.48, 0.18))
}

fn route_panel() -> Style {
  Style::new()
    .width(250.0)
    .height(80.0)
    .padding(20.0)
    .background_color(Color::rgb(0.08, 0.26, 0.3))
    .color(Color::rgb(0.95, 0.99, 1.0))
    .overflow(Overflow::Hidden)
}

fn beam() -> Style {
  Style::new()
    .width(28.0)
    .background_color(Color::rgba(0.25, 0.95, 1.0, 0.42))
}

fn particle(index: usize) -> Style {
  Style::new()
    .width(12.0 + index as f32 * 3.0)
    .height(12.0 + index as f32 * 3.0)
    .border_radius(30.0)
    .border_color(Color::rgb(1.0, 0.58, 0.18))
    .border_width(1.0)
}

fn slider() -> Style {
  probe()
    .width(250.0)
    .background_color(Color::rgb(0.18, 0.12, 0.34))
}

fn ambient_stage() -> Style {
  Style::new()
    .width(270.0)
    .height(150.0)
    .padding(10.0)
    .overflow(Overflow::Hidden)
    .align_items(Align::Center)
}

fn dot() -> Style {
  Style::new()
    .width(30.0)
    .height(18.0)
    .border_radius(12.0)
    .background_color(Color::rgb(0.58, 0.25, 0.94))
    .margin((2, 0))
}

fn audio_probe() -> Style {
  Style::new()
    .width(42.0)
    .height(42.0)
    .border_radius(21.0)
    .background_color(Color::rgb(1.0, 0.46, 0.16))
    .margin((5, 0))
}

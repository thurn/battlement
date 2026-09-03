use crate::{Game, MOTION_MATERIAL, MOTION_TEXTURE, design_system};
use battlement::{
  Align, Color, FlexDirection, FlexWrap, LengthUnits, MotionColor, MotionFilter, MotionGradient,
  MotionGradientStop, MotionLength, MotionShadow, Style,
};
use battlement_reactant::prelude::*;

const CHECKPOINTS: [f64; 5] = [0.0, 0.18, 0.5, 0.99, 1.0];

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StylesDecorationsState {
  checkpoint: usize,
  burst: u32,
  paused: bool,
}

impl StylesDecorationsState {
  fn advance(&mut self) {
    self.checkpoint = (self.checkpoint + 1) % CHECKPOINTS.len();
  }

  fn burst(&mut self) {
    self.burst = self.burst.wrapping_add(1);
  }

  fn reset(&mut self) {
    *self = Self::default();
  }
}

#[builder]
pub(crate) struct StylesDecorations {
  pub(crate) state: StylesDecorationsState,
  pub(crate) compact: bool,
}

impl Component for StylesDecorations {
  fn render(&self) -> impl Render {
    let elapsed = CHECKPOINTS[self.state.checkpoint];
    ScrollView::new()
      .name("styles-decorations-canvas")
      .style(design_system::canvas(self.compact).padding(0.0))
      .content_container_style(content())
      .child(Label::new("CSS-STYLE MOTION").style(eyebrow()))
      .child(
        Label::new("Styles & Decorations")
          .name("page-title")
          .style(title()),
      )
      .child(
        Label::new(format!(
          "CHECKPOINT {:>4.0}%  ·  BURST GENERATION {}  ·  {}",
          elapsed * 100.0,
          self.state.burst,
          if self.state.paused {
            "paused"
          } else {
            "running"
          },
        ))
        .name("styles-status")
        .style(status()),
      )
      .child(controls())
      .child(
        View::new()
          .name("styles-gallery")
          .style(gallery())
          .child(pseudo_specimen())
          .child(fill_specimen(elapsed, self.state.paused))
          .child(loop_specimen())
          .child(burst_specimen(self.state.burst, elapsed))
          .child(advanced_specimen(elapsed)),
      )
  }
}

fn controls() -> View {
  View::new()
    .style(control_row())
    .child(action("CHECKPOINT", "styles-checkpoint", |game| {
      game.styles_decorations.advance()
    }))
    .child(action("BURST", "styles-burst", |game| {
      game.styles_decorations.burst()
    }))
    .child(action("PAUSE", "styles-pause", |game| {
      game.styles_decorations.paused = !game.styles_decorations.paused;
    }))
    .child(action("RESET", "styles-reset", |game| {
      game.styles_decorations.reset()
    }))
    .child(action("TARGETS", "styles-targets", |game| {
      game.screen = crate::Screen::TargetsTimelines;
    }))
    .child(action("VARIANTS", "styles-variants", |game| {
      game.screen = crate::Screen::VariantsOrchestration;
    }))
}

fn pseudo_specimen() -> View {
  specimen(
    "styles-pseudo",
    "PSEUDO PRECEDENCE",
    "hover → focus → active → disabled",
  )
  .child(
    Button::new("HOLD / FOCUS / PRESS")
      .name("styles-pseudo-target")
      .focusable(true)
      .style(probe().background_color(Color::rgb(0.05, 0.20, 0.23)))
      .hover_style(
        MotionStyle::new()
          .background_color(motion_color(0.10, 0.46, 0.50))
          .scale(1.03),
      )
      .focus_style(MotionStyle::new().background_color(motion_color(0.20, 0.37, 0.84)))
      .active_style(
        MotionStyle::new()
          .background_color(motion_color(0.92, 0.31, 0.17))
          .scale(0.96),
      )
      .disabled_style(MotionStyle::new().opacity(0.35))
      .style_transition(
        StyleTransition::new()
          .property(
            StyleProperty::BackgroundColor,
            Transition::tween()
              .duration_secs(0.18)
              .ease(Easing::EaseOut),
          )
          .property(
            StyleProperty::Scale,
            Transition::tween()
              .duration_secs(0.09)
              .ease(Easing::EaseOut),
          )
          .property(
            StyleProperty::Opacity,
            Transition::tween().duration_secs(0.14),
          ),
      ),
  )
}

fn fill_specimen(elapsed: f64, paused: bool) -> View {
  specimen(
    "styles-finite-fill",
    "FINITE FILL",
    "both fill · alternate · exact endpoint",
  )
  .child(
    View::new()
      .name("styles-fill-probe")
      .style(probe())
      .animation(
        Animation::new(Keyframes::new([
          MotionStyle::new().x(-38.0).opacity(0.28),
          MotionStyle::new().x(38.0).opacity(1.0),
        ]))
        .duration_secs(1.0)
        .delay_secs(-elapsed)
        .fill(AnimationFill::Both)
        .play_state(if paused {
          AnimationPlayState::Paused
        } else {
          AnimationPlayState::Running
        })
        .diagnostic_name("styles-finite-fill"),
      ),
  )
}

fn loop_specimen() -> View {
  specimen(
    "styles-independent-loops",
    "INDEPENDENT LOOPS",
    "negative phase offsets never synchronize",
  )
  .child(
    View::new()
      .style(loop_row())
      .child(loop_probe("styles-loop-a", -0.1, 1.7))
      .child(loop_probe("styles-loop-b", -0.7, 2.3))
      .child(loop_probe("styles-loop-c", -1.4, 3.1)),
  )
}

fn loop_probe(name: &'static str, delay: f64, duration: f64) -> View {
  View::new().name(name).style(dot()).animation(
    Animation::new(Keyframes::new([
      MotionStyle::new().scale(0.72).opacity(0.35),
      MotionStyle::new().scale(1.18).opacity(1.0),
    ]))
    .duration_secs(duration)
    .delay_secs(delay)
    .iterations(AnimationIterations::Forever)
    .direction(AnimationDirection::Alternate)
    .fill(AnimationFill::Both),
  )
}

fn burst_specimen(generation: u32, elapsed: f64) -> View {
  let particles = (0..6).map(|index| {
    Decoration::new()
      .key((generation, index))
      .position(DecorationPosition::Fill)
      .style(
        Style::new()
          .border_color(Color::rgb(0.98, 0.58, 0.18))
          .border_width(1.0)
          .opacity(0.28),
      )
      .animation(
        Animation::new(Keyframes::new([
          MotionStyle::new().scale(0.5).opacity(0.9),
          MotionStyle::new().scale(1.35).opacity(0.0),
        ]))
        .duration_secs(0.7)
        .delay_secs(-(elapsed + f64::from(index) * 0.03))
        .fill(AnimationFill::Forwards)
        .animation_key((generation, index)),
      )
  });
  specimen(
    "styles-keyed-burst",
    "KEYED BURST",
    "restart replaces one generation · chrome ignores picking",
  )
  .child(
    Button::new("BURST SAFE INPUT")
      .name("styles-burst-target")
      .style(probe())
      .after_all(particles),
  )
}

fn advanced_specimen(elapsed: f64) -> View {
  let gradient = MotionGradient::Linear {
    angle: 35.0,
    stops: vec![
      MotionGradientStop {
        color: motion_color(0.10, 0.85, 0.88),
        position: 0.0,
      },
      MotionGradientStop {
        color: motion_color(0.58, 0.22, 0.92),
        position: 1.0,
      },
    ],
  };
  let shadow = MotionShadow {
    x: 0.0,
    y: 6.0,
    blur: 18.0,
    spread: 2.0,
    color: MotionColor::new(0.0, 0.8, 0.9, 0.5),
    inset: false,
  };
  specimen(
    "styles-advanced-paint",
    "ADVANCED PAINT",
    "filter · clip · polygon · gradient · shadow · mask · texture",
  )
  .style(advanced_specimen_style())
  .child(
    View::new()
      .style(advanced_row())
      .child(paint_probe(
        "FILTER / RECT CLIP",
        "styles-filter-clip",
        Animation::new(Keyframes::new([
          MotionStyle::new()
            .filter([MotionFilter::Blur(0.0), MotionFilter::Contrast(0.8)])
            .clip_inset([MotionLength::px(0.0); 4]),
          MotionStyle::new()
            .filter([MotionFilter::Blur(4.0), MotionFilter::Contrast(1.3)])
            .clip_inset([MotionLength::px(7.0); 4]),
        ])),
        elapsed,
      ))
      .child(paint_probe(
        "QUAD / POLYGON",
        "styles-quad-polygon",
        Animation::new(Keyframes::new([
          MotionStyle::new()
            .rotate_x(-18.0)
            .skew_x(-8.0)
            .clip_polygon([
              [MotionLength::percent(8.0), MotionLength::percent(0.0)],
              [MotionLength::percent(100.0), MotionLength::percent(12.0)],
              [MotionLength::percent(92.0), MotionLength::percent(100.0)],
              [MotionLength::percent(0.0), MotionLength::percent(88.0)],
            ]),
          MotionStyle::new().rotate_x(18.0).skew_x(8.0).clip_polygon([
            [MotionLength::percent(0.0), MotionLength::percent(12.0)],
            [MotionLength::percent(92.0), MotionLength::percent(0.0)],
            [MotionLength::percent(100.0), MotionLength::percent(88.0)],
            [MotionLength::percent(8.0), MotionLength::percent(100.0)],
          ]),
        ])),
        elapsed,
      ))
      .child(paint_probe(
        "GRADIENT / SHADOW",
        "styles-gradient-shadow",
        Animation::new(Keyframes::new([
          MotionStyle::new()
            .box_shadow([shadow])
            .background_gradient(gradient.clone()),
          MotionStyle::new()
            .box_shadow([MotionShadow {
              blur: 28.0,
              spread: 5.0,
              ..shadow
            }])
            .background_gradient(gradient),
        ])),
        elapsed,
      ))
      .child(paint_probe(
        "MASK / TEXTURE / SHADER",
        "styles-prepared-paint",
        Animation::new(Keyframes::new([
          MotionStyle::new()
            .prepared_texture(MOTION_TEXTURE)
            .mask(MOTION_TEXTURE)
            .shader_material(MOTION_MATERIAL),
          MotionStyle::new()
            .prepared_texture(MOTION_TEXTURE)
            .mask(MOTION_TEXTURE)
            .shader_material(MOTION_MATERIAL),
        ])),
        elapsed,
      )),
  )
}

fn paint_probe(
  label: &'static str,
  name: &'static str,
  animation: Animation,
  elapsed: f64,
) -> View {
  View::new()
    .style(paint_cell())
    .child(Label::new(label).style(paint_label()))
    .child(
      View::new()
        .name(name)
        .style(probe().width(210.0))
        .animation(
          animation
            .duration_secs(1.0)
            .delay_secs(-elapsed)
            .fill(AnimationFill::Both),
        ),
    )
}

fn specimen(name: &'static str, heading: &'static str, detail: &'static str) -> View {
  View::new()
    .name(name)
    .style(specimen_style())
    .child(Label::new(heading).style(specimen_title()))
    .child(Label::new(detail).style(specimen_detail()))
}

fn action(
  text: &'static str,
  name: &'static str,
  callback: impl Fn(&mut Game) + 'static,
) -> Button {
  Button::new(text)
    .name(name)
    .style(action_style())
    .on_click(callback)
}

fn motion_color(r: f32, g: f32, b: f32) -> MotionColor {
  MotionColor::new(r, g, b, 1.0)
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
    .min_width(94.0)
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
    .min_height(164.0)
    .padding(12.0)
    .margin((6, 8, 6, 0))
    .background_color(Color::rgb(0.025, 0.06, 0.08))
    .border_color(Color::rgb(0.15, 0.28, 0.32))
    .border_width(1.0)
}

fn advanced_specimen_style() -> Style {
  specimen_style().width(636.0).min_height(330.0)
}

fn advanced_row() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
}

fn paint_cell() -> Style {
  Style::new().width(285.0).margin((6, 8, 0, 0))
}

fn paint_label() -> Style {
  Style::new()
    .font_size(12.0)
    .color(Color::rgb(0.68, 0.76, 0.78))
}

fn specimen_title() -> Style {
  Style::new()
    .font_size(18.0)
    .color(Color::rgb(0.94, 0.98, 0.99))
}

fn specimen_detail() -> Style {
  Style::new()
    .font_size(13.0)
    .white_space(battlement::WhiteSpace::Normal)
    .color(Color::rgb(0.68, 0.76, 0.78))
    .margin((6, 0))
}

fn probe() -> Style {
  Style::new()
    .width(190.0)
    .height(46.0)
    .margin((12, 0))
    .background_color(Color::rgb(0.13, 0.78, 0.88))
    .border_radius(8.0)
    .color(Color::rgb(0.95, 0.99, 1.0))
}

fn loop_row() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .align_items(Align::Center)
}

fn dot() -> Style {
  Style::new()
    .width(42.0)
    .height(42.0)
    .margin((14, 12, 0, 0))
    .border_radius(21.0)
    .background_color(Color::rgb(0.65, 0.28, 0.95))
}

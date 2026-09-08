//! Animated comets tracing the arcade frame perimeter.

use crate::frame_styles;
use battlement::{Color, Gradient, Length, LengthUnits, Position, Style};
use battlement_reactant::{
  component::Component,
  host::View,
  paint::{PaintBlendMode, PaintClipPath, PaintFillRule, PaintStyle},
  prelude::{
    Animation, AnimationIterations, Easing, EffectGroup, Keyframes, PaintDropShadow,
    PaintFilterList, StyleTarget, builder,
  },
  render::Render,
};

const FRAME_WIDTH: f32 = 982.0;
const FRAME_HEIGHT: f32 = 1404.0;
const PULSE_THICKNESS: f32 = 15.0;
const SETTINGS_BOTTOM_HEIGHT: f32 = 75.0;
const SETTINGS_SIDE_WIDTH: f32 = 297.0;
const LAP_TIMES: [f64; 9] = [0.0, 0.24, 0.25, 0.49, 0.5, 0.74, 0.75, 0.99, 1.0];

/// Application screen that determines frame-specific pulse geometry.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ArcadeScreen {
  Main,
  Settings,
}

/// Two glowing comets that travel around the source arcade-frame path.
#[builder]
pub struct ArcadeFramePulse {
  #[builder(required)]
  active_screen: ArcadeScreen,
  #[builder(required)]
  reduce_motion: bool,
}

impl Component for ArcadeFramePulse {
  fn render(&self) -> impl Render {
    let beams = (
      self::comet(
        0,
        270.0,
        76.0,
        self::large_comet_paint(),
        self.reduce_motion,
      ),
      self::comet(1, 86.0, 30.0, self::small_comet_paint(), self.reduce_motion),
    );
    let mut content = EffectGroup::new()
      .style(Style::new().absolute_fill())
      .child(beams);
    if self.active_screen == ArcadeScreen::Settings {
      content = content.clip_path(self::settings_clip());
    }
    EffectGroup::new()
      .name("arcade-frame-pulse")
      .style(
        Style::new()
          .position(Position::Absolute)
          .top(frame_styles::OUTER_INSET)
          .left(frame_styles::OUTER_INSET)
          .width(FRAME_WIDTH)
          .height(FRAME_HEIGHT)
          .opacity(if self.reduce_motion { 0.28 } else { 1.0 }),
      )
      .clip_path(self::frame_ring())
      .blend_mode(PaintBlendMode::Screen)
      .child(content)
  }
}

fn comet(index: usize, width: f32, height: f32, paint: PaintStyle, reduce_motion: bool) -> View {
  let beam = View::decorative()
    .name(format!("arcade-frame-comet-{index}"))
    .style(
      Style::new()
        .position(Position::Absolute)
        .left(0)
        .top(0)
        .width(width)
        .height(height)
        .margin_left(-width / 2.0)
        .margin_top(-height / 2.0)
        .border_radius(50.pct()),
    )
    .paint(paint);
  if reduce_motion {
    beam
  } else {
    beam.animation(
      Animation::new(self::lap())
        .duration_secs(6.5)
        .ease(Easing::Linear)
        .iterations(AnimationIterations::Forever)
        .diagnostic_name("arcade-border-comet-lap"),
    )
  }
}

fn frame_ring() -> PaintClipPath {
  let outer = frame_styles::clip();
  let inner = outer.iter().map(|point| {
    point.map(|length| {
      let [px, percent] = length.components();
      Length::calc(px + PULSE_THICKNESS * (1.0 - percent / 50.0), percent)
    })
  });
  PaintClipPath::new(PaintFillRule::EvenOdd)
    .contour(inner)
    .contour(outer)
}

fn settings_clip() -> PaintClipPath {
  let top = FRAME_HEIGHT - SETTINGS_BOTTOM_HEIGHT;
  PaintClipPath::new(PaintFillRule::NonZero)
    .contour(self::rectangle(0.0, 0.0, FRAME_WIDTH, top))
    .contour(self::rectangle(0.0, top, SETTINGS_SIDE_WIDTH, FRAME_HEIGHT))
    .contour(self::rectangle(
      FRAME_WIDTH - SETTINGS_SIDE_WIDTH,
      top,
      FRAME_WIDTH,
      FRAME_HEIGHT,
    ))
}

fn rectangle(left: f32, top: f32, right: f32, bottom: f32) -> [[Length; 2]; 4] {
  [[left, top], [right, top], [right, bottom], [left, bottom]].map(|point| point.map(Length::px))
}

fn lap() -> Keyframes<StyleTarget> {
  Keyframes::new([
    self::lap_frame(0.0, 0.0, 0.0),
    self::lap_frame(FRAME_WIDTH, 0.0, 0.0),
    self::lap_frame(FRAME_WIDTH, 0.0, 90.0),
    self::lap_frame(FRAME_WIDTH, FRAME_HEIGHT, 90.0),
    self::lap_frame(FRAME_WIDTH, FRAME_HEIGHT, 180.0),
    self::lap_frame(0.0, FRAME_HEIGHT, 180.0),
    self::lap_frame(0.0, FRAME_HEIGHT, 270.0),
    self::lap_frame(0.0, 0.0, 270.0),
    self::lap_frame(0.0, 0.0, 360.0),
  ])
  .times(LAP_TIMES)
}

fn lap_frame(x: f32, y: f32, rotate: f32) -> StyleTarget {
  StyleTarget::new().x(x).y(y).rotate(rotate)
}

fn large_comet_paint() -> PaintStyle {
  PaintStyle::new()
    .background(
      Gradient::radial([0.5, 0.5], [std::f32::consts::FRAC_1_SQRT_2; 2])
        .stop(0.0, Color::rgba8(255, 255, 255, 242))
        .stop(0.07, Color::rgba8(255, 255, 255, 242))
        .stop(0.24, Color::rgba8(69, 225, 255, 235))
        .stop(0.49, Color::rgba8(48, 138, 255, 153))
        .stop(0.68, Color::rgba8(255, 61, 205, 87))
        .stop(0.78, Color::TRANSPARENT),
    )
    .paint_filter(
      PaintFilterList::default()
        .brightness(2.0)
        .drop_shadow(PaintDropShadow::new(
          0.0,
          0.0,
          11.0,
          0.0,
          Color::WHITE.with_alpha(0.95),
        ))
        .drop_shadow(PaintDropShadow::new(
          0.0,
          0.0,
          25.0,
          0.0,
          Color::hex(0x47d3ff),
        ))
        .drop_shadow(PaintDropShadow::new(
          0.0,
          0.0,
          34.0,
          0.0,
          Color::hex(0xff47cf).with_alpha(0.92),
        )),
    )
}

fn small_comet_paint() -> PaintStyle {
  PaintStyle::new()
    .background(
      Gradient::radial([0.5, 0.5], [std::f32::consts::FRAC_1_SQRT_2; 2])
        .stop(0.0, Color::WHITE)
        .stop(0.20, Color::WHITE)
        .stop(0.42, Color::hex(0xbdf5ff))
        .stop(0.64, Color::hex(0xffb5ec))
        .stop(0.76, Color::TRANSPARENT),
    )
    .paint_filter(
      PaintFilterList::default()
        .brightness(2.8)
        .drop_shadow(PaintDropShadow::new(0.0, 0.0, 7.0, 0.0, Color::WHITE))
        .drop_shadow(PaintDropShadow::new(
          0.0,
          0.0,
          15.0,
          0.0,
          Color::hex(0x77e6ff),
        )),
    )
}

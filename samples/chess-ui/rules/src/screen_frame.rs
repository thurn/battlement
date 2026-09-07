//! An arcade frame and content canvas at the portrait design size.

use crate::{
  concept_frame::ConceptFrame,
  frame_styles,
  portrait_viewport::{PORTRAIT_DESIGN_HEIGHT, PORTRAIT_DESIGN_WIDTH},
};
use battlement::{Color, Length, Overflow, Position, Style, TransformOrigin};
use battlement_reactant::prelude::{
  Children, Easing, Keyframes, MotionFilter, MotionFilterList, StyleTarget, Transition, builder,
};
use battlement_reactant::{
  component::Component,
  host::View,
  paint::{PaintFill, PaintStyle},
  render::Render,
};

/// Fixed portrait frame surrounding application content.
#[builder]
pub struct ScreenFrame {
  #[builder(required, into)]
  children: Children,
  exit_active: bool,
  reduce_motion: bool,
}

impl Component for ScreenFrame {
  fn render(&self) -> impl Render {
    self::frame(self)
  }
}

fn frame(component: &ScreenFrame) -> View {
  let frame_surface = View::decorative()
    .name("exit-frame-surface")
    .style(
      Style::new()
        .absolute_fill()
        .transform_origin(TransformOrigin::two_dimensional(
          Length::Percent(50.0),
          Length::Percent(47.07),
        )),
    )
    .child((
      View::decorative()
        .name("frame-interior")
        .style(
          Style::new()
            .position(Position::Absolute)
            .top(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
            .left(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
            .right(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
            .bottom(frame_styles::OUTER_BOTTOM + frame_styles::BORDER_THICKNESS),
        )
        .paint(
          PaintStyle::new()
            .clip_polygon(frame_styles::clip())
            .background(PaintFill::Gradient(frame_styles::interior())),
        ),
      ConceptFrame::new(),
    ));
  let frame_surface = if component.exit_active {
    frame_surface
      .animate(self::exit_target(component.reduce_motion, true))
      .transition(self::exit_transition(component.reduce_motion))
  } else {
    frame_surface.initial(false)
  };
  View::new()
    .name("screen-frame")
    .style(
      Style::new()
        .position(Position::Relative)
        .width(PORTRAIT_DESIGN_WIDTH)
        .height(PORTRAIT_DESIGN_HEIGHT)
        .overflow(Overflow::Hidden)
        .color(Color::rgb8(247, 248, 255))
        .background_color(Color::BLACK),
    )
    .child((frame_surface, component.children.render()))
}

pub(crate) fn exit_target(reduce_motion: bool, frame: bool) -> StyleTarget {
  if reduce_motion {
    return StyleTarget::new().opacity(0.0);
  }
  let (x, opacity, scale_x, scale_y, contrast) = if frame {
    (
      [0.0, 4.0, -4.0, 0.0, 0.0],
      [1.0, 1.0, 1.0, 0.94, 0.0],
      [1.0, 1.01, 0.992, 1.018, 0.015],
      [1.0, 0.996, 1.006, 0.045, 0.002],
      [1.0, 2.55, 1.15, 3.8, 1.0],
    )
  } else {
    (
      [0.0, -5.0, 4.0, 0.0, 0.0],
      [1.0, 1.0, 1.0, 0.96, 0.0],
      [1.0, 1.008, 0.992, 1.025, 0.02],
      [1.0, 0.994, 1.008, 0.035, 0.002],
      [1.0, 2.3, 1.18, 3.5, 1.0],
    )
  };
  StyleTarget::new()
    .filter_keyframes(
      Keyframes::new([
        self::exit_filter(contrast[0], 0.0),
        self::exit_filter(contrast[1], 0.0),
        self::exit_filter(contrast[2], 0.0),
        self::exit_filter(contrast[3], 0.75),
        self::exit_filter(contrast[4], 1.0),
      ])
      .times([0.0, 0.14, 0.38, 0.73, 1.0]),
    )
    .opacity_keyframes(Keyframes::new(opacity).times([0.0, 0.14, 0.38, 0.73, 1.0]))
    .scale_x_keyframes(Keyframes::new(scale_x).times([0.0, 0.14, 0.38, 0.73, 1.0]))
    .scale_y_keyframes(Keyframes::new(scale_y).times([0.0, 0.14, 0.38, 0.73, 1.0]))
    .x_keyframes(Keyframes::new(x).times([0.0, 0.14, 0.38, 0.73, 1.0]))
}

fn exit_filter(contrast: f32, grayscale: f32) -> MotionFilterList {
  MotionFilterList::new([
    MotionFilter::Contrast(contrast),
    MotionFilter::Grayscale(grayscale),
  ])
}

pub(crate) fn exit_transition(reduce_motion: bool) -> Transition {
  Transition::tween()
    .duration_secs(if reduce_motion { 0.08 } else { 0.62 })
    .ease(Easing::CubicBezier([0.65, 0.0, 0.35, 1.0]))
}

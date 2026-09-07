//! Synchronized arcade-screen collapse and terminal black stage.

use crate::{assets, frame_styles, screen_frame, screen_frame::ScreenFrame};
use battlement::{
  Color, Gradient, ImageScaleMode, Length, LengthUnits, Overflow, PickingMode, Position,
  SemanticRole, Shadow, Style, TransformOrigin,
};
use battlement_reactant::{hooks, paint::PaintStyle, prelude::*, semantics::SemanticVisibility};
use trox::ls;

/// Duration shared by the menu content and frame collapse.
pub const ARCADE_EXIT_DURATION_SECS: f64 = 0.62;

/// Owns the intact, exiting, and terminal-black states of an arcade screen.
#[builder]
pub struct ArcadeExitStage {
  #[builder(required)]
  active: bool,
  #[builder(required)]
  reduce_motion: bool,
  #[builder(required, into)]
  children: Children,
}

impl Component for ArcadeExitStage {
  fn render(&self) -> impl Render {
    let (dismissed, set_dismissed) = hooks::use_state(false);
    hooks::use_effect(
      {
        let set_dismissed = set_dismissed.clone();
        let active = self.active;
        move || {
          if !active {
            set_dismissed.set(false);
          }
        }
      },
      self.active,
    );
    self::stage(self, dismissed, set_dismissed)
  }
}

/// Decorative flash, beam, converging lines, and central collapse overlay.
#[builder]
pub struct ArcadeExitSequence {
  #[builder(required)]
  active: bool,
  #[builder(required)]
  reduce_motion: bool,
}

impl Component for ArcadeExitSequence {
  fn render(&self) -> impl Render {
    (self.active && !self.reduce_motion).then(self::overlay)
  }
}

fn stage(component: &ArcadeExitStage, dismissed: bool, set_dismissed: StateSetter<bool>) -> View {
  let motion_active = component.active && !dismissed;
  View::new()
    .name("arcade-exit-stage")
    .style(
      Style::new()
        .position(Position::Relative)
        .width(crate::portrait_viewport::PORTRAIT_DESIGN_WIDTH)
        .height(crate::portrait_viewport::PORTRAIT_DESIGN_HEIGHT)
        .overflow(Overflow::Hidden)
        .background_color(Color::BLACK),
    )
    .child((
      ScreenFrame::new()
        .exit_active(motion_active)
        .reduce_motion(component.reduce_motion)
        .children((
          self::content_surface(
            component.children.clone(),
            motion_active,
            component.active,
            component.reduce_motion,
            set_dismissed,
          ),
          ArcadeExitSequence::new()
            .active(motion_active)
            .reduce_motion(component.reduce_motion),
        )),
      dismissed.then(self::black_stage),
    ))
}

fn content_surface(
  children: Children,
  motion_active: bool,
  hidden: bool,
  reduce_motion: bool,
  set_dismissed: StateSetter<bool>,
) -> View {
  let surface = View::new()
    .name("arcade-exit-content-surface")
    .inert(hidden)
    .style(self::interior_style().overflow(Overflow::Hidden))
    .child(children.render());
  let surface = if hidden {
    surface.semantic(SemanticProps::new(SemanticRole::Group).visibility(SemanticVisibility::Hidden))
  } else {
    surface
  };
  if motion_active {
    let target = if reduce_motion {
      StyleTarget::new().opacity(0.0)
    } else {
      self::content_exit_target()
    };
    surface
      .animate(target)
      .transition(screen_frame::exit_transition(reduce_motion))
      .on_animation_complete::<()>(move |_| set_dismissed.set(true))
  } else {
    surface.initial(false)
  }
}

fn content_exit_target() -> StyleTarget {
  screen_frame::exit_target(false, false).clip_inset_keyframes(
    Keyframes::new([
      self::content_clip(0.0, 0.0, 0.0, 0.0),
      self::content_clip(0.0, 0.0, 0.0, 0.0),
      self::content_clip(0.0, 0.0, 0.0, 0.0),
      self::content_clip(46.62, 0.0, 52.48, 0.0),
      self::content_clip(47.02, 49.5, 52.88, 49.5),
    ])
    .times([0.0, 0.14, 0.38, 0.73, 1.0]),
  )
}

fn content_clip(top: f32, right: f32, bottom: f32, left: f32) -> [Length; 4] {
  [top, right, bottom, left].map(Length::percent)
}

fn overlay() -> View {
  View::decorative()
    .name("arcade-exit-sequence")
    .style(self::interior_style().overflow(Overflow::Hidden))
    .child((
      self::flash(),
      self::expanding_beam(),
      self::converging_line(true),
      self::converging_line(false),
      self::central_collapse(),
    ))
}

fn flash() -> impl Render {
  assets::EXIT_FLASH
    .image()
    .name("arcade-exit-flash")
    .picking_mode(PickingMode::Ignore)
    .scale_mode(ImageScaleMode::StretchToFill)
    .style(Style::new().position(Position::Absolute).inset(0))
    .initial(StyleTarget::new().opacity(0.0))
    .animate(
      StyleTarget::new()
        .opacity_keyframes(Keyframes::new([0.0, 0.7, 0.25, 0.0]).times([0.0, 0.25, 0.65, 1.0])),
    )
    .transition(
      Transition::tween()
        .duration_secs(0.36)
        .ease(Easing::EaseOut),
    )
}

fn expanding_beam() -> View {
  View::decorative()
    .name("arcade-exit-expanding-beam")
    .style(
      Style::new()
        .position(Position::Absolute)
        .top(9.pct())
        .right(0)
        .bottom(9.pct())
        .left(0)
        .transform_origin(TransformOrigin::two_dimensional(
          Length::percent(50.0),
          Length::percent(50.0),
        )),
    )
    .paint(
      PaintStyle::new()
        .background(
          Gradient::linear(90.0)
            .stop(0.0, Color::TRANSPARENT)
            .stop(0.14, Color::rgba8(75, 205, 255, 38))
            .stop(0.5, Color::rgba8(232, 250, 255, 158))
            .stop(0.86, Color::rgba8(255, 70, 216, 41))
            .stop(1.0, Color::TRANSPARENT),
        )
        .paint_filter(PaintFilterList::default().brightness(1.4)),
    )
    .initial(StyleTarget::new().opacity(0.0).scale_y(0.2))
    .animate(
      StyleTarget::new()
        .opacity_keyframes(Keyframes::new([0.0, 0.9, 0.0]).times([0.0, 0.34, 1.0]))
        .scale_y_keyframes(Keyframes::new([0.2, 1.0, 0.08]).times([0.0, 0.34, 1.0])),
    )
    .transition(
      Transition::tween()
        .duration_secs(0.43)
        .ease(Easing::CubicBezier([0.2, 0.8, 0.2, 1.0])),
    )
}

fn converging_line(top: bool) -> View {
  let name = if top {
    "arcade-exit-top-line"
  } else {
    "arcade-exit-bottom-line"
  };
  let paint = if top {
    self::line_paint(Color::hex(0x59d8ff), Color::hex(0xff5bd7))
  } else {
    self::line_paint(Color::hex(0xff5bd7), Color::hex(0x59d8ff))
  };
  let initial = if top {
    StyleTarget::new().top(Length::percent(7.0)).opacity(0.0)
  } else {
    StyleTarget::new().bottom(Length::percent(7.0)).opacity(0.0)
  };
  let positions = Keyframes::new([
    Length::percent(7.0),
    Length::percent(50.0),
    Length::percent(50.0),
  ])
  .times([0.0, 0.72, 1.0]);
  let animate = if top {
    StyleTarget::new().top_keyframes(positions)
  } else {
    StyleTarget::new().bottom_keyframes(positions)
  }
  .opacity_keyframes(Keyframes::new([0.0, 0.78, 0.0]).times([0.0, 0.72, 1.0]));
  View::decorative()
    .name(name)
    .style(
      Style::new()
        .position(Position::Absolute)
        .right(0)
        .left(0)
        .height(4),
    )
    .paint(paint)
    .initial(initial)
    .animate(animate)
    .transition(
      Transition::tween()
        .duration_secs(0.5)
        .ease(Easing::CubicBezier([0.7, 0.0, 0.3, 1.0])),
    )
}

fn central_collapse() -> View {
  View::decorative()
    .name("arcade-exit-central-collapse")
    .style(
      Style::new()
        .position(Position::Absolute)
        .top(50.pct())
        .right(0)
        .left(0)
        .height(5)
        .margin_top(-2)
        .transform_origin(TransformOrigin::two_dimensional(
          Length::percent(50.0),
          Length::percent(50.0),
        )),
    )
    .paint(
      PaintStyle::new()
        .background(
          Gradient::linear(90.0)
            .stop(0.0, Color::TRANSPARENT)
            .stop(0.12, Color::hex(0x58d7ff))
            .stop(0.43, Color::WHITE)
            .stop(0.57, Color::WHITE)
            .stop(0.88, Color::hex(0xff56d5))
            .stop(1.0, Color::TRANSPARENT),
        )
        .box_shadow([
          Shadow::outer(0.0, 0.0, 7.0, 0.0, Color::WHITE),
          Shadow::outer(0.0, 0.0, 18.0, 0.0, Color::hex(0x59daff).with_alpha(0.95)),
          Shadow::outer(0.0, 0.0, 46.0, 0.0, Color::hex(0x9b5dff).with_alpha(0.9)),
          Shadow::outer(0.0, 0.0, 74.0, 0.0, Color::hex(0xff46d2).with_alpha(0.46)),
        ]),
    )
    .initial(StyleTarget::new().opacity(0.0).scale_x(0.08).scale_y(0.5))
    .animate(
      StyleTarget::new()
        .opacity_keyframes(
          Keyframes::new([0.0, 0.0, 1.0, 0.92, 0.0]).times([0.0, 0.52, 0.72, 0.87, 1.0]),
        )
        .scale_x_keyframes(
          Keyframes::new([0.08, 0.08, 1.0, 0.32, 0.01]).times([0.0, 0.52, 0.72, 0.87, 1.0]),
        )
        .scale_y_keyframes(
          Keyframes::new([0.5, 0.5, 1.9, 0.5, 0.1]).times([0.0, 0.52, 0.72, 0.87, 1.0]),
        ),
    )
    .transition(
      Transition::tween()
        .duration_secs(ARCADE_EXIT_DURATION_SECS)
        .ease(Easing::EaseOut),
    )
}

fn line_paint(start: Color, end: Color) -> PaintStyle {
  PaintStyle::new()
    .background(
      Gradient::linear(90.0)
        .stop(0.0, Color::TRANSPARENT)
        .stop(0.18, start)
        .stop(0.5, Color::WHITE)
        .stop(0.82, end)
        .stop(1.0, Color::TRANSPARENT),
    )
    .box_shadow([
      Shadow::outer(0.0, 0.0, 10.0, 0.0, Color::WHITE),
      Shadow::outer(0.0, 0.0, 26.0, 0.0, start.with_alpha(0.9)),
      Shadow::outer(0.0, 0.0, 42.0, 0.0, end.with_alpha(0.5)),
    ])
}

fn interior_style() -> Style {
  Style::new()
    .position(Position::Absolute)
    .top(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
    .right(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
    .bottom(frame_styles::OUTER_BOTTOM + frame_styles::BORDER_THICKNESS)
    .left(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
}

fn black_stage() -> impl Render {
  Region::new(ls("Dismissed arcade stage"))
    .host_name("arcade-exit-black-stage")
    .style(Style::new().absolute_fill().background_color(Color::BLACK))
}

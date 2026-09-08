//! An arcade frame and content canvas at the portrait design size.

use crate::{
  arcade_frame_pulse::{ArcadeFramePulse, ArcadeScreen},
  concept_frame::ConceptFrame,
  frame_styles,
  portrait_viewport::{PORTRAIT_DESIGN_HEIGHT, PORTRAIT_DESIGN_WIDTH},
};
use battlement::{Color, Length, Overflow, Position, Style, TransformOrigin};
use battlement_reactant::prelude::{
  Children, Easing, Keyframes, StateSetter, StyleTarget, Transition, builder,
};
use battlement_reactant::{
  component::Component,
  components::Region,
  context::ContextProvider,
  element_behavior,
  focus::FocusProps,
  hooks,
  host::View,
  paint::{PaintFill, PaintStyle},
  render::Render,
};
use trox::ls;

/// Fixed portrait frame surrounding application content.
#[builder]
pub struct ScreenFrame {
  #[builder(required, into)]
  children: Children,
  exit_active: bool,
  reduce_motion: bool,
  frame_pulse: Option<ArcadeScreen>,
}

/// Adds descendant-triggered frame collapse to a normal [`ScreenFrame`].
#[builder]
pub struct ExitAwareScreenFrame {
  #[builder(required, into)]
  children: Children,
  reduce_motion: bool,
  frame_pulse: Option<ArcadeScreen>,
}

#[derive(Clone, PartialEq)]
pub(crate) struct ScreenFrameExitContext {
  request: StateSetter<Option<bool>>,
  terminal: StateSetter<bool>,
}

impl ScreenFrameExitContext {
  pub(crate) fn set(&self, active: bool, reduce_motion: bool) {
    self.request.set(active.then_some(reduce_motion));
    if !active {
      self.terminal.set(false);
    }
  }

  pub(crate) fn set_terminal(&self, terminal: bool) {
    self.terminal.set(terminal);
  }
}

pub(crate) fn use_screen_frame_exit() -> ScreenFrameExitContext {
  hooks::use_required_context::<ScreenFrameExitContext>()
}

impl Component for ScreenFrame {
  fn render(&self) -> impl Render {
    self::frame(self)
  }
}

impl Component for ExitAwareScreenFrame {
  fn render(&self) -> impl Render {
    let (requested_exit, set_requested_exit) = hooks::use_state(None::<bool>);
    let (terminal, set_terminal) = hooks::use_state(false);
    ContextProvider::new()
      .context(ScreenFrameExitContext {
        request: set_requested_exit,
        terminal: set_terminal,
      })
      .child((
        ScreenFrame::new()
          .exit_active(requested_exit.is_some())
          .reduce_motion(requested_exit.unwrap_or(self.reduce_motion))
          .frame_pulse(self.frame_pulse)
          .children(self.children.render()),
        terminal.then(TerminalBlackStage::new),
      ))
  }
}

#[builder]
struct TerminalBlackStage;

impl Component for TerminalBlackStage {
  fn render(&self) -> impl Render {
    let focus = element_behavior::use_focus_on_mount();
    Region::new(ls("Dismissed arcade stage"))
      .host_name("arcade-exit-black-stage")
      .configure_host(|host| {
        host
          .element_ref(focus)
          .focus_props(FocusProps::new().focusable(true).tab_index(-1))
      })
      .style(Style::new().absolute_fill().background_color(Color::BLACK))
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
    .child((
      frame_surface,
      component.frame_pulse.map(|active_screen| {
        ArcadeFramePulse::new()
          .active_screen(active_screen)
          .reduce_motion(component.reduce_motion)
      }),
      component.children.render(),
    ))
}

pub(crate) fn exit_target(reduce_motion: bool, frame: bool) -> StyleTarget {
  if reduce_motion {
    return StyleTarget::new().opacity(0.0);
  }
  let (x, opacity, scale_x, scale_y) = if frame {
    (
      [0.0, 4.0, -4.0, 0.0, 0.0],
      [1.0, 1.0, 1.0, 0.94, 0.0],
      [1.0, 1.01, 0.992, 1.018, 0.015],
      [1.0, 0.996, 1.006, 0.045, 0.002],
    )
  } else {
    (
      [0.0, -5.0, 4.0, 0.0, 0.0],
      [1.0, 1.0, 1.0, 0.96, 0.0],
      [1.0, 1.008, 0.992, 1.025, 0.02],
      [1.0, 0.994, 1.008, 0.035, 0.002],
    )
  };
  StyleTarget::new()
    .opacity_keyframes(Keyframes::new(opacity).times([0.0, 0.14, 0.38, 0.73, 1.0]))
    .scale_x_keyframes(Keyframes::new(scale_x).times([0.0, 0.14, 0.38, 0.73, 1.0]))
    .scale_y_keyframes(Keyframes::new(scale_y).times([0.0, 0.14, 0.38, 0.73, 1.0]))
    .x_keyframes(Keyframes::new(x).times([0.0, 0.14, 0.38, 0.73, 1.0]))
}

pub(crate) fn exit_transition(reduce_motion: bool) -> Transition {
  Transition::tween()
    .duration_secs(if reduce_motion { 0.08 } else { 0.62 })
    .ease(Easing::CubicBezier([0.65, 0.0, 0.35, 1.0]))
}

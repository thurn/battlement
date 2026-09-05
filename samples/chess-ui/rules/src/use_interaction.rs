//! Shared interaction-feedback state for arcade controls.

use battlement::{Color, Gradient};
use battlement_reactant::{
  element_ref::ElementRef,
  hooks,
  host::{ButtonHost, SliderHost, ToggleHost},
  prelude::{EventCallback, PaintDropShadow, PaintFilterList},
};

/// Interaction presentation shared by the arcade controls.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InteractionState {
  /// Whether the pointer is currently inside the control.
  pub hovered: bool,
  /// Whether a pointer press is currently held inside the control.
  pub pressed: bool,
  /// Whether keyboard or controller modality currently exposes focus.
  pub focus_visible: bool,
  /// Whether motion-sensitive press transforms should be suppressed.
  pub reduced_motion: bool,
}

/// Stable event callbacks paired with the current interaction state.
pub struct Interaction {
  /// Current render-time interaction presentation.
  pub state: InteractionState,
  enter: EventCallback<()>,
  leave: EventCallback<()>,
  press: EventCallback<()>,
  release: EventCallback<()>,
  set_focus_visible: hooks::StateSetter<bool>,
}

/// Observes pointer and focus-visible presentation for one mounted control.
///
/// Unity retains pointer ownership and decides whether an activation succeeds.
/// Leaving, cancellation, or capture loss only clears presentation state.
pub fn use_interaction() -> Interaction {
  let (hovered, set_hovered) = hooks::use_state(false);
  let (pressed, set_pressed) = hooks::use_state(false);
  let (focus_visible, set_focus_visible) = hooks::use_state(false);
  Interaction {
    state: InteractionState {
      hovered,
      pressed,
      focus_visible,
      reduced_motion: battlement_reactant::motion_config::use_reduced_motion(),
    },
    enter: set_hovered.callback().map_input(|()| true),
    leave: set_hovered
      .callback()
      .map_input(|()| false)
      .then(set_pressed.callback().map_input(|()| false)),
    press: set_pressed.callback().map_input(|()| true),
    release: set_pressed.callback().map_input(|()| false),
    set_focus_visible,
  }
}

/// Gold focus border used by keyboard and controller navigation.
pub fn focus_gradient(angle: f32) -> Gradient {
  Gradient::linear(angle)
    .stop(0.0, Color::hex(0xfffbd0))
    .stop(0.2, Color::hex(0xfff700))
    .stop(0.72, Color::hex(0xffbd00))
    .stop(1.0, Color::hex(0xfff56a))
}

/// White-and-gold focus glow used by keyboard and controller navigation.
pub fn focus_filter() -> PaintFilterList {
  PaintFilterList::default()
    .brightness(1.08)
    .drop_shadow(PaintDropShadow::new(0.0, 0.0, 3.0, 0.0, Color::WHITE))
    .drop_shadow(PaintDropShadow::new(
      0.0,
      0.0,
      13.0,
      0.0,
      Color::hex(0xffe000).with_alpha(0.94),
    ))
}

impl Interaction {
  /// Attaches the visual-state observation to a native button.
  #[must_use]
  pub fn button(&self, host: ButtonHost) -> ButtonHost {
    let focus_start = self.set_focus_visible.clone();
    let focus_end = self.set_focus_visible.clone();
    host
      .on_pointer_enter(self.enter.clone())
      .on_pointer_leave(self.leave.clone())
      .on_pointer_down(self.press.clone())
      .on_pointer_up(self.release.clone())
      .on_pointer_cancel(self.release.clone())
      .on_pointer_capture_out(self.release.clone())
      .on_focus_visible_start(move |_: &mut (), _| focus_start.set(true))
      .on_focus_visible_end(move |_: &mut (), _| focus_end.set(false))
  }

  /// Attaches the visual-state observation to a native checkbox.
  #[must_use]
  pub fn toggle(&self, host: ToggleHost) -> ToggleHost {
    let focus_start = self.set_focus_visible.clone();
    let focus_end = self.set_focus_visible.clone();
    host
      .on_pointer_enter(self.enter.clone())
      .on_pointer_leave(self.leave.clone())
      .on_pointer_down(self.press.clone())
      .on_pointer_up(self.release.clone())
      .on_pointer_cancel(self.release.clone())
      .on_pointer_capture_out(self.release.clone())
      .on_focus_visible_start(move |_: &mut (), _| focus_start.set(true))
      .on_focus_visible_end(move |_: &mut (), _| focus_end.set(false))
  }

  /// Attaches visual-state observation and pointer focus to a native slider.
  #[must_use]
  pub fn slider(&self, host: SliderHost, focus_target: ElementRef) -> SliderHost {
    let focus_start = self.set_focus_visible.clone();
    let focus_end = self.set_focus_visible.clone();
    let focus_on_press = EventCallback::new(move |()| focus_target.focus());
    host
      .on_pointer_enter(self.enter.clone())
      .on_pointer_leave(self.leave.clone())
      .on_pointer_down(self.press.clone().then(focus_on_press))
      .on_pointer_up(self.release.clone())
      .on_pointer_cancel(self.release.clone())
      .on_pointer_capture_out(self.release.clone())
      .on_focus_visible_start(move |_: &mut (), _| focus_start.set(true))
      .on_focus_visible_end(move |_: &mut (), _| focus_end.set(false))
  }

  /// Attaches slider visuals plus explicit capture lifecycle callbacks.
  #[must_use]
  pub fn slider_with_release(
    &self,
    host: SliderHost,
    focus_target: ElementRef,
    on_begin: EventCallback<()>,
    on_release: EventCallback<()>,
    on_cancel: EventCallback<()>,
  ) -> SliderHost {
    let focus_start = self.set_focus_visible.clone();
    let focus_end = self.set_focus_visible.clone();
    let focus_on_press = EventCallback::new(move |()| focus_target.focus());
    host
      .on_pointer_enter(self.enter.clone())
      .on_pointer_leave(self.leave.clone())
      .on_pointer_down(self.press.clone().then(focus_on_press).then(on_begin))
      .on_pointer_up(self.release.clone().then(on_release))
      .on_pointer_cancel(self.release.clone().then(on_cancel.clone()))
      .on_pointer_capture_out(self.release.clone().then(on_cancel))
      .on_focus_visible_start(move |_: &mut (), _| focus_start.set(true))
      .on_focus_visible_end(move |_: &mut (), _| focus_end.set(false))
  }
}

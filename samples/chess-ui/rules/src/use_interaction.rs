//! Shared pointer-feedback state for arcade controls.

use battlement_reactant::{
  hooks,
  host::{ButtonHost, SliderHost, ToggleHost},
  prelude::EventCallback,
};

/// Pointer presentation shared by the arcade controls.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InteractionState {
  /// Whether the pointer is currently inside the control.
  pub hovered: bool,
  /// Whether a pointer press is currently held inside the control.
  pub pressed: bool,
  /// Whether motion-sensitive press transforms should be suppressed.
  pub reduced_motion: bool,
}

/// Stable event callbacks paired with the current interaction state.
pub struct Interaction {
  /// Current render-time pointer presentation.
  pub state: InteractionState,
  enter: EventCallback<()>,
  leave: EventCallback<()>,
  press: EventCallback<()>,
  release: EventCallback<()>,
}

/// Observes hover and held-press presentation for one mounted control.
///
/// Unity retains pointer ownership and decides whether an activation succeeds.
/// Leaving, cancellation, or capture loss only clears presentation state.
pub fn use_interaction() -> Interaction {
  let (hovered, set_hovered) = hooks::use_state(false);
  let (pressed, set_pressed) = hooks::use_state(false);
  Interaction {
    state: InteractionState {
      hovered,
      pressed,
      reduced_motion: battlement_reactant::motion_config::use_reduced_motion(),
    },
    enter: set_hovered.callback().map_input(|()| true),
    leave: set_hovered
      .callback()
      .map_input(|()| false)
      .then(set_pressed.callback().map_input(|()| false)),
    press: set_pressed.callback().map_input(|()| true),
    release: set_pressed.callback().map_input(|()| false),
  }
}

impl Interaction {
  /// Attaches the visual-state observation to a native button.
  #[must_use]
  pub fn button(&self, host: ButtonHost) -> ButtonHost {
    host
      .on_pointer_enter(self.enter.clone())
      .on_pointer_leave(self.leave.clone())
      .on_pointer_down(self.press.clone())
      .on_pointer_up(self.release.clone())
      .on_pointer_cancel(self.release.clone())
      .on_pointer_capture_out(self.release.clone())
  }

  /// Attaches the visual-state observation to a native checkbox.
  #[must_use]
  pub fn toggle(&self, host: ToggleHost) -> ToggleHost {
    host
      .on_pointer_enter(self.enter.clone())
      .on_pointer_leave(self.leave.clone())
      .on_pointer_down(self.press.clone())
      .on_pointer_up(self.release.clone())
      .on_pointer_cancel(self.release.clone())
      .on_pointer_capture_out(self.release.clone())
  }

  /// Attaches the visual-state observation to a native slider.
  #[must_use]
  pub fn slider(&self, host: SliderHost) -> SliderHost {
    host
      .on_pointer_enter(self.enter.clone())
      .on_pointer_leave(self.leave.clone())
      .on_pointer_down(self.press.clone())
      .on_pointer_up(self.release.clone())
      .on_pointer_cancel(self.release.clone())
      .on_pointer_capture_out(self.release.clone())
  }
}

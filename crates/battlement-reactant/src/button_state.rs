//! Reusable button interaction state derived from native gesture recognition.

use battlement::MotionGestureEventKind;

use crate::{
  accessibility::{self, ButtonOptions},
  callback::IntoCallback,
  hooks::{self, StateSetter},
  motion::MotionProps,
  semantics::{AccessibleBehavior, AccessibleName},
};

/// Styling state for a button whose interaction affects composed children.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ButtonState {
  /// Whether the control is unavailable.
  pub disabled: bool,
  /// Whether a non-touch pointer is over the button.
  pub hovered: bool,
  /// Whether native tap recognition is active.
  pub pressed: bool,
  /// Whether exact focus is visibly indicated for keyboard or controller input.
  pub focus_visible: bool,
}

/// Returns unified button behavior with a reusable interaction-state snapshot.
///
/// `on_press` remains the single accepted-activation notification. Cancelled and
/// disabled taps update no application-owned press counter and do not call it.
/// Attach the complete value with `.behavior(...)`, or forward its `motion`
/// field together with its semantic, focus, and interaction fields.
pub fn use_button_state<G: 'static>(
  options: ButtonOptions<impl IntoCallback<(), G>, impl Into<AccessibleName>>,
) -> AccessibleBehavior<G, ButtonState> {
  let (state, setter) = hooks::use_state(ButtonState::default());
  let base = accessibility::use_button(options);
  let disabled = base.state.disabled;
  AccessibleBehavior {
    semantic: base.semantic,
    focus: base.focus,
    interaction: base.interaction,
    motion: self::interaction_motion(setter),
    state: ButtonState {
      disabled,
      pressed: !disabled && state.pressed,
      focus_visible: !disabled && state.focus_visible,
      ..state
    },
  }
}

fn interaction_motion(setter: StateSetter<ButtonState>) -> MotionProps {
  let hover_start = setter.clone();
  let hover_end = setter.clone();
  let tap_start = setter.clone();
  let tap_end = setter.clone();
  let tap_cancel = setter.clone();
  let focus_start = setter.clone();
  MotionProps::new()
    .gesture_brief(MotionGestureEventKind::HoverStart, move || {
      hover_start.update(|state| ButtonState {
        hovered: true,
        ..state
      });
    })
    .gesture_brief(MotionGestureEventKind::HoverEnd, move || {
      hover_end.update(|state| ButtonState {
        hovered: false,
        ..state
      });
    })
    .gesture_brief(MotionGestureEventKind::TapStart, move || {
      tap_start.update(|state| ButtonState {
        pressed: true,
        ..state
      });
    })
    .gesture_brief(MotionGestureEventKind::Tap, move || {
      tap_end.update(|state| ButtonState {
        pressed: false,
        ..state
      });
    })
    .gesture_brief(MotionGestureEventKind::TapCancel, move || {
      tap_cancel.update(|state| ButtonState {
        pressed: false,
        ..state
      });
    })
    .gesture_brief(MotionGestureEventKind::FocusVisibleStart, move || {
      focus_start.update(|state| ButtonState {
        focus_visible: true,
        ..state
      });
    })
    .gesture_brief(MotionGestureEventKind::FocusVisibleEnd, move || {
      setter.update(|state| ButtonState {
        focus_visible: false,
        ..state
      });
    })
}

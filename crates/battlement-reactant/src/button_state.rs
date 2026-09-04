//! Button interaction state derived from native gesture recognition.

use battlement::MotionGestureEventKind;

use crate::{
  accessibility::ButtonState,
  hooks::{self, StateSetter},
  motion::MotionProps,
};

pub(crate) fn use_interaction_state(disabled: bool) -> (ButtonState, MotionProps) {
  let (state, setter) = hooks::use_state(ButtonState::default());
  if disabled && state != ButtonState::default() {
    setter.set(ButtonState::default());
  }
  (state, self::interaction_motion(setter))
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

//! Keyboard, controller, and pointer value normalization for volume sliders.

use battlement::{KeyEvent, NavigationDirection, NavigationMoveEvent, PhysicalKey};
use battlement_reactant::event::ReactantEvent;

/// Rounds a native pointer proposal to the slider's integer domain.
pub(crate) fn pointer_value(value: f32) -> u32 {
  value.round().clamp(0.0, 100.0) as u32
}

/// Applies the source slider's keyboard steps and endpoints.
pub(crate) fn key_down(event: ReactantEvent<KeyEvent>, value: u32) -> Option<u32> {
  let next = match event.payload().physical_key {
    Some(PhysicalKey::ArrowDown | PhysicalKey::ArrowLeft) => Some(value.saturating_sub(5)),
    Some(PhysicalKey::ArrowRight | PhysicalKey::ArrowUp) => Some((value + 5).min(100)),
    Some(PhysicalKey::PageDown) => Some(value.saturating_sub(10)),
    Some(PhysicalKey::PageUp) => Some((value + 10).min(100)),
    Some(PhysicalKey::Home) => Some(0),
    Some(PhysicalKey::End) => Some(100),
    _ => None,
  };
  if let Some(next) = next {
    event.prevent_default();
    event.stop_propagation();
    (next != value).then_some(next)
  } else {
    None
  }
}

/// Applies a normalized directional controller step.
pub(crate) fn navigation_move(
  event: ReactantEvent<NavigationMoveEvent>,
  value: u32,
) -> Option<u32> {
  let next = match event.payload().direction {
    NavigationDirection::Down | NavigationDirection::Left => Some(value.saturating_sub(5)),
    NavigationDirection::Up | NavigationDirection::Right => Some((value + 5).min(100)),
    _ => None,
  };
  if let Some(next) = next {
    event.prevent_default();
    event.stop_propagation();
    (next != value).then_some(next)
  } else {
    None
  }
}

//! Keyboard and controller state transitions for the custom selector.

use battlement::{KeyEvent, NavigationDirection, NavigationMoveEvent, PhysicalKey};
use battlement_reactant::{event::ReactantEvent, hooks::StateSetter};

/// Returns the selected option index, defaulting to the first option.
pub(crate) fn selected_index(options: &[String], value: &str) -> usize {
  options
    .iter()
    .position(|option| option == value)
    .unwrap_or(0)
}

/// Opens or closes the list while preserving focus ownership.
pub(crate) fn toggle(
  open: bool,
  selected: usize,
  set_open: StateSetter<bool>,
  set_active: StateSetter<usize>,
  set_restore_focus: StateSetter<bool>,
) {
  if open {
    set_open.set(false);
    set_restore_focus.set(true);
  } else {
    set_restore_focus.set(false);
    set_active.set(selected);
    set_open.set(true);
  }
}

/// Opens the list from a directional trigger gesture.
pub(crate) fn trigger_key(
  event: ReactantEvent<KeyEvent>,
  selected: usize,
  set_open: StateSetter<bool>,
  set_active: StateSetter<usize>,
  set_restore_focus: StateSetter<bool>,
) {
  if matches!(
    event.payload().physical_key,
    Some(PhysicalKey::ArrowDown | PhysicalKey::ArrowUp)
  ) {
    set_restore_focus.set(false);
    set_active.set(selected);
    set_open.set(true);
    event.prevent_default();
    event.stop_propagation();
  }
}

/// Opens the list from an upward or downward controller move.
pub(crate) fn trigger_navigation(
  event: ReactantEvent<NavigationMoveEvent>,
  selected: usize,
  set_open: StateSetter<bool>,
  set_active: StateSetter<usize>,
  set_restore_focus: StateSetter<bool>,
) {
  if matches!(
    event.payload().direction,
    NavigationDirection::Up | NavigationDirection::Down
  ) {
    set_restore_focus.set(false);
    set_active.set(selected);
    set_open.set(true);
    event.prevent_default();
    event.stop_propagation();
  }
}

/// Moves the active option or dismisses the open list.
pub(crate) fn list_key(
  event: ReactantEvent<KeyEvent>,
  active: usize,
  options: &[String],
  set_active: StateSetter<usize>,
  set_open: StateSetter<bool>,
  set_restore_focus: StateSetter<bool>,
) {
  let next = match event.payload().physical_key {
    Some(PhysicalKey::ArrowDown) => Some((active + 1).min(options.len() - 1)),
    Some(PhysicalKey::ArrowUp) => Some(active.saturating_sub(1)),
    Some(PhysicalKey::Home) => Some(0),
    Some(PhysicalKey::End) => Some(options.len() - 1),
    Some(PhysicalKey::Escape) => {
      self::dismiss(set_open, set_restore_focus);
      None
    }
    _ => self::typeahead(event.payload().text.as_str(), options),
  };
  if let Some(next) = next {
    set_active.set(next);
    event.prevent_default();
    event.stop_propagation();
  } else if event.payload().physical_key == Some(PhysicalKey::Escape) {
    event.prevent_default();
    event.stop_propagation();
  }
}

/// Moves the active option from a controller direction.
pub(crate) fn list_navigation(
  event: ReactantEvent<NavigationMoveEvent>,
  active: usize,
  option_count: usize,
  set_active: StateSetter<usize>,
) {
  let next = match event.payload().direction {
    NavigationDirection::Down => Some((active + 1).min(option_count - 1)),
    NavigationDirection::Up => Some(active.saturating_sub(1)),
    _ => None,
  };
  if let Some(next) = next {
    set_active.set(next);
    event.prevent_default();
    event.stop_propagation();
  }
}

/// Closes the list and queues focus restoration to its trigger.
pub(crate) fn dismiss(set_open: StateSetter<bool>, set_restore_focus: StateSetter<bool>) {
  set_open.set(false);
  set_restore_focus.set(true);
}

fn typeahead(text: &str, options: &[String]) -> Option<usize> {
  let query = text.trim().to_lowercase();
  (!query.is_empty()).then(|| {
    options
      .iter()
      .position(|option| option.to_lowercase().starts_with(&query))
  })?
}

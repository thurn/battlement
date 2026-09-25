//! Directional selection and focus behavior for settings categories.

use battlement::{KeyEvent, NavigationDirection, NavigationMoveEvent, PhysicalKey};
use reactant::{
  callback::Callback, element_ref::ElementRef, event::ReactantEvent, prelude::EventCallback,
};

use crate::menu::settings_tabs::SettingsTab;

pub(crate) fn key_callback(
  current: SettingsTab,
  input_available: bool,
  references: [ElementRef; 4],
  on_select: EventCallback<SettingsTab>,
) -> Callback<ReactantEvent<KeyEvent>> {
  on_select.filter_map_input(move |event| self::key(event, current, input_available, &references))
}

pub(crate) fn controller_callback(
  current: SettingsTab,
  input_available: bool,
  references: [ElementRef; 4],
  on_select: EventCallback<SettingsTab>,
) -> Callback<ReactantEvent<NavigationMoveEvent>> {
  on_select
    .filter_map_input(move |event| self::controller(event, current, input_available, &references))
}

pub(crate) fn key(
  event: ReactantEvent<KeyEvent>,
  current: SettingsTab,
  input_available: bool,
  references: &[ElementRef; 4],
) -> Option<SettingsTab> {
  let next = match event.payload().physical_key {
    Some(PhysicalKey::ArrowRight | PhysicalKey::ArrowDown) => self::next(current, input_available),
    Some(PhysicalKey::ArrowLeft | PhysicalKey::ArrowUp) => self::previous(current, input_available),
    Some(PhysicalKey::Home) => SettingsTab::Gameplay,
    Some(PhysicalKey::End) => {
      if input_available {
        SettingsTab::Input
      } else {
        SettingsTab::Sound
      }
    }
    _ => return None,
  };
  self::accept(event, next, references)
}

pub(crate) fn controller(
  event: ReactantEvent<NavigationMoveEvent>,
  current: SettingsTab,
  input_available: bool,
  references: &[ElementRef; 4],
) -> Option<SettingsTab> {
  let next = match event.payload().direction {
    NavigationDirection::Right | NavigationDirection::Down => self::next(current, input_available),
    NavigationDirection::Left | NavigationDirection::Up => self::previous(current, input_available),
    NavigationDirection::None | NavigationDirection::Next | NavigationDirection::Previous => {
      return None;
    }
  };
  self::accept(event, next, references)
}

fn accept<T>(
  event: ReactantEvent<T>,
  next: SettingsTab,
  references: &[ElementRef; 4],
) -> Option<SettingsTab> {
  event.prevent_default();
  event.stop_propagation();
  references[next as usize].focus();
  Some(next)
}

fn next(current: SettingsTab, input_available: bool) -> SettingsTab {
  SettingsTab::ALL[(current as usize + 1) % self::count(input_available)]
}

fn previous(current: SettingsTab, input_available: bool) -> SettingsTab {
  let count = self::count(input_available);
  SettingsTab::ALL[(current as usize + count - 1) % count]
}

fn count(input_available: bool) -> usize {
  if input_available { 4 } else { 3 }
}

//! Directional selection and focus behavior for settings categories.

use battlement::{KeyEvent, NavigationDirection, NavigationMoveEvent, PhysicalKey};
use battlement_reactant::{
  callback::Callback, element_ref::ElementRef, event::ReactantEvent, prelude::EventCallback,
};

use crate::settings_tabs::SettingsTab;

pub(crate) fn key_callback(
  current: SettingsTab,
  references: [ElementRef; 4],
  on_select: EventCallback<SettingsTab>,
) -> Callback<ReactantEvent<KeyEvent>> {
  on_select.filter_map_input(move |event| self::key(event, current, &references))
}

pub(crate) fn controller_callback(
  current: SettingsTab,
  references: [ElementRef; 4],
  on_select: EventCallback<SettingsTab>,
) -> Callback<ReactantEvent<NavigationMoveEvent>> {
  on_select.filter_map_input(move |event| self::controller(event, current, &references))
}

pub(crate) fn key(
  event: ReactantEvent<KeyEvent>,
  current: SettingsTab,
  references: &[ElementRef; 4],
) -> Option<SettingsTab> {
  let next = match event.payload().physical_key {
    Some(PhysicalKey::ArrowRight | PhysicalKey::ArrowDown) => self::next(current),
    Some(PhysicalKey::ArrowLeft | PhysicalKey::ArrowUp) => self::previous(current),
    Some(PhysicalKey::Home) => SettingsTab::Gameplay,
    Some(PhysicalKey::End) => SettingsTab::Input,
    _ => return None,
  };
  self::accept(event, next, references)
}

pub(crate) fn controller(
  event: ReactantEvent<NavigationMoveEvent>,
  current: SettingsTab,
  references: &[ElementRef; 4],
) -> Option<SettingsTab> {
  let next = match event.payload().direction {
    NavigationDirection::Right | NavigationDirection::Down => self::next(current),
    NavigationDirection::Left | NavigationDirection::Up => self::previous(current),
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

fn next(current: SettingsTab) -> SettingsTab {
  SettingsTab::ALL[(current as usize + 1) % SettingsTab::ALL.len()]
}

fn previous(current: SettingsTab) -> SettingsTab {
  SettingsTab::ALL[(current as usize + SettingsTab::ALL.len() - 1) % SettingsTab::ALL.len()]
}

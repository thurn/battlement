//! Localizable binding names, independent of their stable input identities.

use crate::settings::bindings::ControllerBinding;
use battlement::PhysicalKey;
use trox::{LocalizedString, ls, tx};

pub fn action(index: usize) -> LocalizedString {
  match index {
    0 => tx("Left", "Gameplay binding action."),
    1 => tx("Right", "Gameplay binding action."),
    2 => tx("Up", "Gameplay binding action."),
    3 => tx("Down", "Gameplay binding action."),
    4 => tx("Move Piece", "Gameplay binding action."),
    5 => tx("Pause", "Gameplay binding action."),
    6 => tx("Restart", "Gameplay binding action."),
    _ => panic!("unknown binding action"),
  }
}

pub fn controller(binding: ControllerBinding) -> LocalizedString {
  match binding {
    ControllerBinding::DpadLeft => tx("D-pad left", "Controller binding name."),
    ControllerBinding::DpadRight => tx("D-pad right", "Controller binding name."),
    ControllerBinding::DpadUp => tx("D-pad up", "Controller binding name."),
    ControllerBinding::DpadDown => tx("D-pad down", "Controller binding name."),
    ControllerBinding::South => ls("A"),
    ControllerBinding::West => ls("X"),
    ControllerBinding::North => ls("Y"),
    ControllerBinding::LeftShoulder => ls("LB"),
    ControllerBinding::RightShoulder => ls("RB"),
    ControllerBinding::Start => tx("menu", "Controller binding name."),
    ControllerBinding::Select => tx("View", "Controller binding name."),
  }
}

pub fn key(key: PhysicalKey) -> LocalizedString {
  match key {
    PhysicalKey::Escape => tx("Esc", "Keyboard key name."),
    PhysicalKey::Space => tx("Space", "Keyboard key name."),
    PhysicalKey::ArrowLeft => tx("Left arrow", "Keyboard key name."),
    PhysicalKey::ArrowRight => tx("Right arrow", "Keyboard key name."),
    PhysicalKey::ArrowUp => tx("Up arrow", "Keyboard key name."),
    PhysicalKey::ArrowDown => tx("Down arrow", "Keyboard key name."),
    PhysicalKey::Enter => tx("Enter", "Keyboard key name."),
    PhysicalKey::Backspace => tx("Backspace", "Keyboard key name."),
    PhysicalKey::Tab => tx("Tab", "Keyboard key name."),
    PhysicalKey::Delete => tx("Delete", "Keyboard key name."),
    PhysicalKey::Insert => tx("Insert", "Keyboard key name."),
    PhysicalKey::Home => tx("Home", "Keyboard key name."),
    PhysicalKey::End => tx("End", "Keyboard key name."),
    PhysicalKey::PageUp => tx("Page up", "Keyboard key name."),
    PhysicalKey::PageDown => tx("Page down", "Keyboard key name."),
    PhysicalKey::Backquote => ls("`"),
    PhysicalKey::Minus => ls("−"),
    PhysicalKey::Equal => ls("="),
    PhysicalKey::BracketLeft => ls("["),
    PhysicalKey::BracketRight => ls("]"),
    PhysicalKey::Backslash => ls("\\"),
    PhysicalKey::Semicolon => ls(";"),
    PhysicalKey::Quote => ls("'"),
    PhysicalKey::Comma => ls(","),
    PhysicalKey::Period => ls("."),
    PhysicalKey::Slash => ls("/"),
    PhysicalKey::CapsLock => tx("Caps Lock", "Keyboard key name."),
    PhysicalKey::ContextMenu => tx("Context menu", "Keyboard key name."),
    PhysicalKey::PrintScreen => tx("Print Screen", "Keyboard key name."),
    PhysicalKey::ScrollLock => tx("Scroll Lock", "Keyboard key name."),
    PhysicalKey::Pause => tx("Pause key", "Keyboard key name."),
    PhysicalKey::NumLock => tx("Num Lock", "Keyboard key name."),
    PhysicalKey::NumpadDecimal => tx("Numpad decimal", "Keyboard key name."),
    PhysicalKey::NumpadAdd => tx("Numpad +", "Keyboard key name."),
    PhysicalKey::NumpadSubtract => tx("Numpad −", "Keyboard key name."),
    PhysicalKey::NumpadMultiply => tx("Numpad ×", "Keyboard key name."),
    PhysicalKey::NumpadDivide => tx("Numpad ÷", "Keyboard key name."),
    PhysicalKey::NumpadEnter => tx("Numpad Enter", "Keyboard key name."),
    PhysicalKey::Numpad0 => tx("Numpad 0", "Keyboard key name."),
    PhysicalKey::Numpad1 => tx("Numpad 1", "Keyboard key name."),
    PhysicalKey::Numpad2 => tx("Numpad 2", "Keyboard key name."),
    PhysicalKey::Numpad3 => tx("Numpad 3", "Keyboard key name."),
    PhysicalKey::Numpad4 => tx("Numpad 4", "Keyboard key name."),
    PhysicalKey::Numpad5 => tx("Numpad 5", "Keyboard key name."),
    PhysicalKey::Numpad6 => tx("Numpad 6", "Keyboard key name."),
    PhysicalKey::Numpad7 => tx("Numpad 7", "Keyboard key name."),
    PhysicalKey::Numpad8 => tx("Numpad 8", "Keyboard key name."),
    PhysicalKey::Numpad9 => tx("Numpad 9", "Keyboard key name."),
    _ => {
      let name = format!("{key:?}");
      ls(
        name
          .strip_prefix("Key")
          .or_else(|| name.strip_prefix("Digit"))
          .unwrap_or(&name),
      )
    }
  }
}

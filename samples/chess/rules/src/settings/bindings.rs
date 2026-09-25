//! Stable gameplay actions and validated physical binding maps.

use battlement::PhysicalKey;
use serde::{Deserialize, Serialize};

/// Gameplay actions whose bindings are independent of translated labels.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GameplayAction {
  Left,
  Right,
  Up,
  Down,
  MovePiece,
  Pause,
  Restart,
}

/// Controller controls accepted as single gameplay bindings.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControllerBinding {
  DpadLeft,
  DpadRight,
  DpadUp,
  DpadDown,
  South,
  West,
  North,
  LeftShoulder,
  RightShoulder,
  Start,
  Select,
}

/// Named action fields keep stored maps stable when their presentation changes.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bindings<T> {
  pub left: T,
  pub right: T,
  pub up: T,
  pub down: T,
  pub move_piece: T,
  pub pause: T,
  pub restart: T,
}

/// Rejects duplicates, bare modifiers, and Escape outside Pause.
pub fn valid_keyboard(bindings: Bindings<PhysicalKey>) -> bool {
  let values = bindings.values();
  self::unique(&values)
    && values
      .iter()
      .enumerate()
      .all(|(index, key)| !self::is_modifier(*key) && (*key != PhysicalKey::Escape || index == 5))
}

/// East, sticks, and triggers are absent from the accepted binding type.
pub fn valid_controller(bindings: Bindings<ControllerBinding>) -> bool {
  self::unique(&bindings.values())
}

pub fn is_modifier(key: PhysicalKey) -> bool {
  matches!(
    key,
    PhysicalKey::ShiftLeft
      | PhysicalKey::ShiftRight
      | PhysicalKey::ControlLeft
      | PhysicalKey::ControlRight
      | PhysicalKey::AltLeft
      | PhysicalKey::AltRight
      | PhysicalKey::MetaLeft
      | PhysicalKey::MetaRight
  )
}

impl GameplayAction {
  pub const ALL: [Self; 7] = [
    Self::Left,
    Self::Right,
    Self::Up,
    Self::Down,
    Self::MovePiece,
    Self::Pause,
    Self::Restart,
  ];
}

impl<T: Copy> Bindings<T> {
  pub const fn values(self) -> [T; 7] {
    [
      self.left,
      self.right,
      self.up,
      self.down,
      self.move_piece,
      self.pause,
      self.restart,
    ]
  }

  pub const fn from_values(values: [T; 7]) -> Self {
    Self {
      left: values[0],
      right: values[1],
      up: values[2],
      down: values[3],
      move_piece: values[4],
      pause: values[5],
      restart: values[6],
    }
  }
}

impl Default for Bindings<PhysicalKey> {
  fn default() -> Self {
    Self::from_values([
      PhysicalKey::ArrowLeft,
      PhysicalKey::ArrowRight,
      PhysicalKey::ArrowUp,
      PhysicalKey::ArrowDown,
      PhysicalKey::Space,
      PhysicalKey::Escape,
      PhysicalKey::KeyR,
    ])
  }
}

impl Default for Bindings<ControllerBinding> {
  fn default() -> Self {
    Self::from_values([
      ControllerBinding::DpadLeft,
      ControllerBinding::DpadRight,
      ControllerBinding::DpadUp,
      ControllerBinding::DpadDown,
      ControllerBinding::South,
      ControllerBinding::Start,
      ControllerBinding::North,
    ])
  }
}

fn unique<T: PartialEq>(values: &[T]) -> bool {
  values
    .iter()
    .enumerate()
    .all(|(index, value)| !values[..index].contains(value))
}

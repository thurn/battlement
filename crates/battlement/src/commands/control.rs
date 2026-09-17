use crate::{CommandId, ControllerButton, ObjectId, PhysicalKey, PointerEvent};

/// A Battlement-owned developer interface surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DebugUiSurface {
  /// The structured in-game log viewer.
  LogViewer,
  /// The compact frames-per-second viewer.
  FpsViewer,
}

/// Sets whether one Battlement developer interface surface is visible.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DebugUiPayload {
  /// The developer interface surface to control.
  pub surface: DebugUiSurface,
  /// Whether the selected surface is visible.
  pub visible: bool,
}

/// Waits for a fixed positive duration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaitPayload {
  /// Positive wait duration in milliseconds.
  pub duration_ms: u64,
}

/// Cancels an operation by the command identity that started it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CancelOperationPayload {
  /// Command and operation identity to cancel.
  pub command_id: CommandId,
}

/// Gates every pointer and key action.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SetInputEnabledPayload {
  /// Whether Battlement accepts input actions.
  pub enabled: bool,
}

/// Replaces the enabled pointer-event set for one game object.
#[derive(Clone, Debug, PartialEq)]
pub struct PointerEventsPayload {
  /// Target game object.
  pub object_id: ObjectId,
  /// Unique enabled pointer-event kinds.
  pub events: Vec<PointerEvent>,
}

/// Native arbitration and capture for logical world pointer events.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorldPointerSettings {
  /// Higher layers win before distance is compared.
  pub interaction_layer: i32,
  /// Committed traversal order; later siblings win exact depth ties.
  pub order: u32,
  /// Capture the primary pointer after an unprevented press.
  pub capture_on_press: bool,
}

/// Replaces a world's logical pointer route without replacing its host.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldPointerPayload {
  /// Existing native target identity.
  pub object_id: ObjectId,
  /// None restores legacy core pointer routing.
  pub settings: Option<WorldPointerSettings>,
}

/// Replaces the global physical-key set enabled for the session.
#[derive(Clone, Debug, PartialEq)]
pub struct GlobalKeysPayload {
  /// Unique enabled W3C physical key codes.
  pub keys: Vec<PhysicalKey>,
}

/// Controller input selected for a session.
#[derive(Clone, Debug, PartialEq)]
pub struct ControllerInputSettings {
  /// Unique enabled controller buttons.
  pub buttons: Vec<ControllerButton>,
  /// Whether the D-pad and left stick emit cardinal navigation actions.
  pub navigation_enabled: bool,
  /// Optional analog dead-zone override; the client default applies when omitted.
  pub stick_dead_zone: Option<f64>,
  /// Optional delay override before a held direction starts repeating.
  pub repeat_delay_ms: Option<u64>,
  /// Optional interval override between held-direction repeats.
  pub repeat_interval_ms: Option<u64>,
}

impl ControllerInputSettings {
  /// Creates controller settings with no buttons and client-native navigation behavior.
  #[must_use]
  pub fn new() -> Self {
    Self {
      buttons: Vec::new(),
      navigation_enabled: true,
      stick_dead_zone: None,
      repeat_delay_ms: None,
      repeat_interval_ms: None,
    }
  }

  /// Replaces enabled buttons and returns the updated settings.
  #[must_use]
  pub fn buttons(mut self, values: impl IntoIterator<Item = ControllerButton>) -> Self {
    self.buttons = values.into_iter().collect();
    self
  }

  /// Overrides the client's native left-stick dead zone.
  #[must_use]
  pub fn stick_dead_zone(mut self, value: f64) -> Self {
    self.stick_dead_zone = Some(value);
    self
  }

  /// Overrides the client's native held-navigation repeat timing.
  #[must_use]
  pub fn repeat_timing_ms(mut self, delay: u64, interval: u64) -> Self {
    self.repeat_delay_ms = Some(delay);
    self.repeat_interval_ms = Some(interval);
    self
  }
}

impl Default for ControllerInputSettings {
  fn default() -> Self {
    Self::new()
  }
}

/// Runs both controller vibration motors for a bounded duration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControllerVibrationPayload {
  /// Low-frequency motor intensity in the inclusive range `[0, 1]`.
  pub low_frequency: f64,
  /// High-frequency motor intensity in the inclusive range `[0, 1]`.
  pub high_frequency: f64,
  /// Vibration duration in milliseconds.
  pub duration_ms: u64,
}

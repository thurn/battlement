//! Scoped physical input capture commands and terminal observations.

use crate::{ControllerButtonPayload, ControllerNavigationPayload, ObjectId, PhysicalKey};

/// Device family accepted by an exclusive physical input capture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputCaptureDevice {
  /// One physical keyboard key.
  Keyboard,
  /// One controller button or D-pad direction.
  Controller,
}

/// A scoped exclusive capture, identified independently of its owning component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputCaptureRequest {
  /// Stable capture ownership token.
  pub id: ObjectId,
  /// Device family to observe.
  pub device: InputCaptureDevice,
}

/// Starts capture or releases only the matching owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputCaptureCommand {
  /// Replace the current exclusive capture.
  Begin(InputCaptureRequest),
  /// Release only the matching ownership token.
  End(ObjectId),
}

/// Why capture ended without a binding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputCaptureCancellation {
  /// The application lost focus or suspended.
  FocusLost,
  /// The selected device became unavailable.
  DeviceDisconnected,
  /// Another capture replaced this owner.
  Superseded,
}

/// Exactly one eligible physical input, or an explicit cancellation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputCaptureResult {
  /// A non-modifier physical key press.
  Key {
    /// Native keyboard device identity.
    device_id: i32,
    /// Platform-independent key code.
    key: PhysicalKey,
  },
  /// A controller button press.
  Button(ControllerButtonPayload),
  /// An initial D-pad direction, never stick input or a repeat.
  Direction(ControllerNavigationPayload),
  /// Capture ended without a binding.
  Cancelled(InputCaptureCancellation),
}

/// Terminal capture result addressed to its still-mounted owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputCaptureEvent {
  /// Stable capture ownership token.
  pub id: ObjectId,
  /// The captured input or cancellation reason.
  pub result: InputCaptureResult,
}

//! Desktop display transactions are owned and timed by the host.

use crate::{CommandId, host_settings::DisplayConfiguration};

/// Preview identity prevents stale dialogs from confirming a newer display change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplayCommand {
  /// Save the prior state before applying a coupled mode and resolution.
  Preview(DisplayConfiguration),
  /// Keep a preview only after host readback and durable confirmation.
  Confirm(CommandId),
  /// Restore the state preceding the identified preview.
  Cancel(CommandId),
}

impl DisplayCommand {
  /// Checks dimensions before a request reaches a native display API.
  pub fn is_valid(self) -> bool {
    let Self::Preview(value) = self else {
      return true;
    };
    if !(1..=32768).contains(&value.resolution.width) {
      return false;
    }
    if !(1..=32768).contains(&value.resolution.height) {
      return false;
    }
    value.resolution.refresh_denominator != 0
  }
}

/// Host state of a display transaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplayPreviewState {
  /// The requested display has not yet matched readback.
  Applying,
  /// Readback matches and the watchdog is awaiting confirmation.
  Confirmable,
  /// The host is restoring a safe prior configuration.
  Reverting,
}

/// A host-owned preview and its remaining real-time confirmation window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplayPreview {
  /// Identity of the active operation.
  pub request_id: CommandId,
  /// Whether confirmation is currently safe.
  pub state: DisplayPreviewState,
  /// Whole seconds remaining, rounded upward; zero while reverting.
  pub remaining_seconds: u32,
}

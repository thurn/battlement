//! Commands for adding custom metadata to Unity Diagnostics reports.
//!
//! Unity attaches current custom metadata and buffered Unity logs to later crashes,
//! exceptions, and supported Android Application Not Responding reports. Battlement
//! forwards Rust `tracing` events through Unity logging, so game code should use its
//! normal tracing events for chronological context and reserve metadata for bounded
//! context that should remain attached to future reports.
//!
//! Exception capture and the recent-log buffer are configured on the
//! `BattlementDiagnosticsModule` asset in Unity. Diagnostic data collection itself
//! remains a Unity project or build-profile setting.
//!
//! A successful command means the local Unity API call returned. It does not mean
//! that Unity created, uploaded, grouped, or symbolicated a report.
//!
//! ```
//! use battlement_cloud::diagnostics::{DiagnosticsCommand, DiagnosticsMetadata};
//!
//! let set = DiagnosticsCommand::SetMetadata(
//!   DiagnosticsMetadata::set("battlement.scene", "main-menu")?,
//! );
//! let clear = DiagnosticsCommand::SetMetadata(
//!   DiagnosticsMetadata::clear("battlement.scene")?,
//! );
//! # let _ = (set, clear);
//! # Ok::<(), battlement_cloud::diagnostics::DiagnosticsValidationError>(())
//! ```
//!
//! # Unity documentation
//!
//! - [View Diagnostics for your project](https://docs.unity.com/en-us/cloud/developer-data/diagnostics)
//! - [Configure diagnostic data collection in the Editor](https://docs.unity.com/en-us/cloud/developer-data/configure-diagnostics-editor)
//! - [Set up custom reports](https://docs.unity.com/en-us/cloud/developer-data/custom-reports)
//! - [Test reports](https://docs.unity.com/en-us/cloud/developer-data/test-reports)
//! - [`CrashReportHandler` scripting API](https://docs.unity3d.com/6000.0/Documentation/ScriptReference/CrashReportHandler.CrashReportHandler.html)

use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// Maximum custom metadata-key length accepted by `CrashReportHandler`.
pub const MAXIMUM_METADATA_KEY_LENGTH: usize = 255;
/// Maximum custom metadata-value length accepted by `CrashReportHandler`.
pub const MAXIMUM_METADATA_VALUE_LENGTH: usize = 1_024;

/// Commands supported by Battlement's Unity Diagnostics module.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DiagnosticsCommand {
  /// Sets one metadata value, or clears the key when `value` is absent.
  SetMetadata(DiagnosticsMetadata),
}

impl DiagnosticsCommand {
  /// Validates the command before it is sent to Unity.
  ///
  /// # Errors
  ///
  /// Returns [`DiagnosticsValidationError::Metadata`] for an invalid key or value.
  pub fn validate(&self) -> Result<(), DiagnosticsValidationError> {
    match self {
      Self::SetMetadata(metadata) => metadata.validate(),
    }
  }
}

/// One write to Unity Diagnostics custom metadata.
///
/// `Some(value)` sets the key. `None` clears it. Unity allows at most 64 custom
/// metadata keys in total, including keys written outside Battlement.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiagnosticsMetadata {
  /// Stable metadata key shown in Diagnostics occurrence details.
  pub key: String,
  /// Bounded debugging value, or `None` to clear the key.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub value: Option<String>,
}

impl DiagnosticsMetadata {
  /// Creates a validated metadata write.
  ///
  /// # Errors
  ///
  /// Returns [`DiagnosticsValidationError::Metadata`] when the key or value is
  /// outside Unity's supported bounds.
  pub fn set(
    key: impl Into<String>,
    value: impl Into<String>,
  ) -> Result<Self, DiagnosticsValidationError> {
    Self::new(key.into(), Some(value.into()))
  }

  /// Creates a validated request to clear one metadata key.
  ///
  /// # Errors
  ///
  /// Returns [`DiagnosticsValidationError::Metadata`] when the key is invalid.
  pub fn clear(key: impl Into<String>) -> Result<Self, DiagnosticsValidationError> {
    Self::new(key.into(), None)
  }

  /// Validates this metadata write.
  ///
  /// # Errors
  ///
  /// Returns [`DiagnosticsValidationError::Metadata`] when the key or value is
  /// outside Unity's supported bounds.
  pub fn validate(&self) -> Result<(), DiagnosticsValidationError> {
    validate_metadata_key(&self.key)?;
    if let Some(value) = &self.value {
      validate_metadata_value(value)?;
    }
    Ok(())
  }

  fn new(key: String, value: Option<String>) -> Result<Self, DiagnosticsValidationError> {
    let metadata = Self { key, value };
    metadata.validate()?;
    Ok(metadata)
  }
}

/// Validation failure for a Unity Diagnostics command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticsValidationError {
  /// A metadata key or value is outside Unity's supported bounds.
  Metadata,
}

impl fmt::Display for DiagnosticsValidationError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("invalid Unity Diagnostics metadata")
  }
}

impl Error for DiagnosticsValidationError {}

fn validate_metadata_key(key: &str) -> Result<(), DiagnosticsValidationError> {
  if key.is_empty() || key.chars().count() > MAXIMUM_METADATA_KEY_LENGTH {
    return Err(DiagnosticsValidationError::Metadata);
  }
  if key.trim() != key || key.chars().any(char::is_control) {
    return Err(DiagnosticsValidationError::Metadata);
  }
  Ok(())
}

fn validate_metadata_value(value: &str) -> Result<(), DiagnosticsValidationError> {
  if value.chars().count() > MAXIMUM_METADATA_VALUE_LENGTH || value.contains('\0') {
    return Err(DiagnosticsValidationError::Metadata);
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::{DiagnosticsCommand, DiagnosticsMetadata, DiagnosticsValidationError};

  #[test]
  fn metadata_set_and_clear_have_distinct_wire_shapes() {
    let set = DiagnosticsCommand::SetMetadata(
      DiagnosticsMetadata::set("battlement.scene", "castle").expect("valid metadata"),
    );
    let clear = DiagnosticsCommand::SetMetadata(
      DiagnosticsMetadata::clear("battlement.scene").expect("valid metadata key"),
    );

    assert_eq!(
      serde_json::to_string(&set).expect("serialize set"),
      r#"{"SetMetadata":{"key":"battlement.scene","value":"castle"}}"#
    );
    assert_eq!(
      serde_json::to_string(&clear).expect("serialize clear"),
      r#"{"SetMetadata":{"key":"battlement.scene"}}"#
    );
  }

  #[test]
  fn metadata_validation_matches_unity_bounds() {
    assert!(DiagnosticsMetadata::set("key", "").is_ok());
    assert_eq!(
      DiagnosticsMetadata::set(" key", "value"),
      Err(DiagnosticsValidationError::Metadata)
    );
    assert_eq!(
      DiagnosticsMetadata::set("key", "\0"),
      Err(DiagnosticsValidationError::Metadata)
    );
  }
}

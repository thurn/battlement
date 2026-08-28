//! Deterministic in-memory execution of Diagnostics metadata commands.
//!
//! The fake models the local command boundary only. It does not simulate Unity
//! Dashboard ingestion, report upload, issue grouping, or symbolication.

use std::collections::BTreeMap;

use battlement::{CommandId, CoreErrorCode};
use battlement_cloud::diagnostics::{
  DiagnosticsCommand, DiagnosticsMetadata, DiagnosticsValidationError,
};

/// Journal entry for one Diagnostics command executed by [`DiagnosticsFake`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FakeDiagnosticsCommandResult {
  /// Identity of the attempted command.
  pub command_id: CommandId,
  /// Local completion or the stable core error returned to the rules engine.
  pub outcome: FakeDiagnosticsCommandOutcome,
}

/// Stable local outcome of one fake Diagnostics command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FakeDiagnosticsCommandOutcome {
  /// Validation and local execution completed.
  Completed,
  /// Validation or module lookup failed.
  Failed(CoreErrorCode),
}

/// In-memory implementation of the Diagnostics metadata command contract.
pub struct DiagnosticsFake {
  available: bool,
  metadata: BTreeMap<String, String>,
  command_results: Vec<FakeDiagnosticsCommandResult>,
}

impl Default for DiagnosticsFake {
  fn default() -> Self {
    Self {
      available: true,
      metadata: BTreeMap::new(),
      command_results: Vec::new(),
    }
  }
}

impl DiagnosticsFake {
  /// Creates a fake connection with no selected Diagnostics module.
  #[must_use]
  pub fn absent() -> Self {
    Self {
      available: false,
      ..Self::default()
    }
  }

  /// Returns whether the Diagnostics module is selected.
  #[must_use]
  pub const fn is_available(&self) -> bool {
    self.available
  }

  /// Returns metadata currently held by the simulated Unity API.
  #[must_use]
  pub fn metadata(&self) -> &BTreeMap<String, String> {
    &self.metadata
  }

  /// Returns every attempted command outcome in execution order.
  #[must_use]
  pub fn command_results(&self) -> &[FakeDiagnosticsCommandResult] {
    &self.command_results
  }

  /// Executes one typed metadata command.
  ///
  /// # Errors
  ///
  /// Returns a stable [`CoreErrorCode`] when the module is absent or the payload is
  /// invalid.
  pub fn execute(
    &mut self,
    command_id: CommandId,
    command: &DiagnosticsCommand,
  ) -> Result<(), CoreErrorCode> {
    let outcome = self.execute_inner(command);
    self.command_results.push(FakeDiagnosticsCommandResult {
      command_id,
      outcome: match outcome {
        Ok(()) => FakeDiagnosticsCommandOutcome::Completed,
        Err(code) => FakeDiagnosticsCommandOutcome::Failed(code),
      },
    });
    outcome
  }

  fn execute_inner(&mut self, command: &DiagnosticsCommand) -> Result<(), CoreErrorCode> {
    if !self.available {
      return Err(CoreErrorCode::ModuleUnavailable);
    }
    command.validate().map_err(map_validation)?;
    match command {
      DiagnosticsCommand::SetMetadata(metadata) => self.set_metadata(metadata),
    }
    Ok(())
  }

  fn set_metadata(&mut self, metadata: &DiagnosticsMetadata) {
    match &metadata.value {
      Some(value) => {
        self.metadata.insert(metadata.key.clone(), value.clone());
      }
      None => {
        self.metadata.remove(&metadata.key);
      }
    }
  }
}

fn map_validation(error: DiagnosticsValidationError) -> CoreErrorCode {
  match error {
    DiagnosticsValidationError::Metadata => CoreErrorCode::DiagnosticsMetadataInvalid,
  }
}

#[cfg(test)]
mod tests {
  use battlement_cloud::diagnostics::{DiagnosticsCommand, DiagnosticsMetadata};

  use super::DiagnosticsFake;
  use battlement::CommandId;

  #[test]
  fn sets_and_clears_metadata() {
    let mut fake = DiagnosticsFake::default();
    fake
      .execute(
        CommandId::new_v4(),
        &DiagnosticsCommand::SetMetadata(
          DiagnosticsMetadata::set("battlement.scene", "castle").expect("valid metadata"),
        ),
      )
      .expect("set metadata");
    assert_eq!(
      fake.metadata().get("battlement.scene").map(String::as_str),
      Some("castle")
    );

    fake
      .execute(
        CommandId::new_v4(),
        &DiagnosticsCommand::SetMetadata(
          DiagnosticsMetadata::clear("battlement.scene").expect("valid metadata"),
        ),
      )
      .expect("clear metadata");
    assert!(fake.metadata().is_empty());
  }
}

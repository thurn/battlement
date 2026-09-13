use battlement::UiEventDisposition;
use battlement_flatbuffers::{ConnectView, FinishedMessage, ResponseView, UiEventActionView};

use crate::{EngineError, FlatBufferSubmitError};

/// One finished response and the session identity validated while constructing it.
pub struct NativeResponse {
  session_id: [u8; 16],
  message: FinishedMessage,
}

impl NativeResponse {
  /// Constructs and verifies an empty response directly in FlatBuffer storage.
  pub fn empty(session_id: [u8; 16]) -> Result<Self, EngineError> {
    let message = battlement_flatbuffers::write_empty_response(session_id)
      .map_err(|error| EngineError::new(error.to_string()))?;
    Self::from_core(session_id, message)
  }

  /// Verifies a finished core response and its claimed session identity.
  pub fn from_core(session_id: [u8; 16], message: FinishedMessage) -> Result<Self, EngineError> {
    let response = ResponseView::read(message.as_bytes())
      .map_err(|error| EngineError::new(error.to_string()))?;
    Self::from_verified(session_id, response.session_id(), message)
  }

  /// Verifies a build-composed response and its claimed session identity.
  ///
  /// Generated schema code supplies `verify`, which must run structural and
  /// semantic validation and return the response root's canonical session UUID.
  pub fn from_composed(
    session_id: [u8; 16],
    message: FinishedMessage,
    verify: impl FnOnce(&[u8]) -> Result<[u8; 16], EngineError>,
  ) -> Result<Self, EngineError> {
    let actual_session_id = verify(message.as_bytes())?;
    Self::from_verified(session_id, actual_session_id, message)
  }

  /// Returns the canonical session UUID bytes.
  #[must_use]
  pub const fn session_id(&self) -> [u8; 16] {
    self.session_id
  }

  /// Returns the immutable finished response bytes.
  #[must_use]
  pub fn as_bytes(&self) -> &[u8] {
    self.message.as_bytes()
  }

  pub(crate) fn into_message(self) -> FinishedMessage {
    self.message
  }

  fn from_verified(
    session_id: [u8; 16],
    actual_session_id: [u8; 16],
    message: FinishedMessage,
  ) -> Result<Self, EngineError> {
    if session_id == [0; 16] {
      return Err(EngineError::new("native response session UUID is zero"));
    }
    if actual_session_id != session_id {
      return Err(EngineError::new(
        "native response root used a different session UUID",
      ));
    }
    if message.as_bytes().len() > battlement_flatbuffers::MAXIMUM_MESSAGE_BYTES {
      return Err(EngineError::new("native response exceeds 16 MiB"));
    }
    if message.allocation_bytes() > 32 * 1024 * 1024 {
      return Err(EngineError::new(
        "native response builder allocation exceeds 32 MiB",
      ));
    }
    Ok(Self {
      session_id,
      message,
    })
  }
}

/// A finished synchronous UI-event result for the native FlatBuffers boundary.
pub struct NativeUiEventResponse {
  /// Immediate native-default disposition.
  pub disposition: UiEventDisposition,
  /// Complete verified response allocation.
  pub response: NativeResponse,
}

/// A rules engine whose exported path constructs finished FlatBuffers directly.
///
/// Unlike [`crate::Engine`], this contract cannot return an owned protocol
/// response. Implementations construct their response in schema-specific
/// writers and relinquish the finished allocation exactly once.
pub trait NativeEngine {
  /// NUL-terminated SHA-256 digest of the complete build-composed wire contract.
  const WIRE_CONTRACT_DIGEST_C: &'static [u8; 65];

  /// Starts a new session and returns its finished initial response.
  fn connect_native(&mut self, message: ConnectView<'_>) -> Result<NativeResponse, EngineError>;

  /// Verifies and applies one build-composed client message.
  fn submit_native(&mut self, bytes: &[u8]) -> Result<NativeResponse, FlatBufferSubmitError>;

  /// Applies one verified borrowed UI event synchronously.
  fn submit_ui_event_native(
    &mut self,
    action: UiEventActionView<'_>,
  ) -> Result<NativeUiEventResponse, EngineError>;

  /// Returns one already-finished queued response, if any.
  fn poll_native(&mut self) -> Result<Option<NativeResponse>, EngineError>;
}

/// Constructs one concrete direct native engine.
pub trait NativeEngineFactory: Sized {
  /// Direct engine implementation produced by this factory.
  type Engine: NativeEngine;

  /// Creates the engine or returns diagnostic text.
  fn create_native(self) -> Result<Self::Engine, EngineError>;
}

/// Converts an infallible or fallible factory result into a direct native engine.
pub trait IntoNativeEngine {
  /// Constructed engine type.
  type Engine: NativeEngine;

  /// Returns the engine or its initialization error.
  fn into_native_engine(self) -> Result<Self::Engine, EngineError>;
}

impl<E: NativeEngine> IntoNativeEngine for E {
  type Engine = E;

  fn into_native_engine(self) -> Result<Self::Engine, EngineError> {
    Ok(self)
  }
}

impl<E: NativeEngine> IntoNativeEngine for Result<E, EngineError> {
  type Engine = E;

  fn into_native_engine(self) -> Result<Self::Engine, EngineError> {
    self
  }
}

impl<E, F> NativeEngineFactory for F
where
  E: IntoNativeEngine,
  F: FnOnce() -> E,
{
  type Engine = E::Engine;

  fn create_native(self) -> Result<Self::Engine, EngineError> {
    self().into_native_engine()
  }
}

#[cfg(test)]
mod tests {
  use battlement_flatbuffers::write_empty_response;

  use super::*;

  #[test]
  fn core_response_claim_must_match_the_verified_root() {
    let message = write_empty_response([1; 16]).unwrap();
    let error = NativeResponse::from_core([2; 16], message)
      .err()
      .expect("mismatched session must fail");
    assert!(error.to_string().contains("different session"));
  }
}

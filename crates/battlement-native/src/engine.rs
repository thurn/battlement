use std::{error::Error, fmt};

use crate::ResponseLease;
use battlement::UiEventDisposition;
use battlement_flatbuffers::{ConnectView, FinishedMessage, ResponseView, UiEventActionView};
/// A diagnostic returned when a rules engine cannot complete an operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineError {
  message: String,
}

impl EngineError {
  /// Creates an engine error from human-readable diagnostic text.
  pub fn new(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
    }
  }
}

impl fmt::Display for EngineError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(&self.message)
  }
}

impl Error for EngineError {}

/// Failure returned while verifying or applying one FlatBuffers client message.
#[derive(Debug)]
pub enum FlatBufferSubmitError {
  /// The submitted bytes do not satisfy the configured schema or its semantic contract.
  InvalidArgument(EngineError),
  /// The message verified, but the rules engine could not apply it.
  Engine(EngineError),
}

impl fmt::Display for FlatBufferSubmitError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::InvalidArgument(error) => write!(formatter, "invalid argument: {error}"),
      Self::Engine(error) => error.fmt(formatter),
    }
  }
}

impl Error for FlatBufferSubmitError {}

impl FlatBufferSubmitError {
  /// Creates a malformed-input failure.
  pub fn invalid_argument(message: impl Into<String>) -> Self {
    Self::InvalidArgument(EngineError::new(message))
  }

  /// Creates an engine-application failure.
  pub fn engine(error: EngineError) -> Self {
    Self::Engine(error)
  }
}

/// One finished response and the session identity validated while constructing it.
pub struct EngineResponse {
  session_id: [u8; 16],
  message: FinishedMessage,
  pub(crate) lease: Option<ResponseLease>,
}

impl EngineResponse {
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
  pub fn from_composed(
    session_id: [u8; 16],
    message: FinishedMessage,
    verify: impl FnOnce(&[u8]) -> Result<[u8; 16], EngineError>,
  ) -> Result<Self, EngineError> {
    Self::from_verified(session_id, verify(message.as_bytes())?, message)
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

  /// Returns the builder allocation charged to downstream admission.
  pub fn allocation_bytes(&self) -> usize {
    self.message.allocation_bytes()
  }

  /// Retains admission for a host that copies verified messages into its own storage.
  pub fn retention(&self) -> Option<ResponseLease> {
    self.lease.clone()
  }

  pub(crate) fn into_parts(self) -> (FinishedMessage, Option<ResponseLease>) {
    (self.message, self.lease)
  }

  fn from_verified(
    session_id: [u8; 16],
    actual_session_id: [u8; 16],
    message: FinishedMessage,
  ) -> Result<Self, EngineError> {
    if session_id == [0; 16] {
      return Err(EngineError::new("engine response session UUID is zero"));
    }
    if actual_session_id != session_id {
      return Err(EngineError::new(
        "engine response root used a different session UUID",
      ));
    }
    if message.as_bytes().len() > battlement_flatbuffers::MAXIMUM_MESSAGE_BYTES {
      return Err(EngineError::new("engine response exceeds 16 MiB"));
    }
    if message.allocation_bytes() > 32 * 1024 * 1024 {
      return Err(EngineError::new(
        "engine response builder allocation exceeds 32 MiB",
      ));
    }
    Ok(Self {
      session_id,
      message,
      lease: None,
    })
  }
}

/// A finished synchronous UI-event result.
pub struct UiEventResult {
  /// Immediate native-default disposition.
  pub disposition: UiEventDisposition,
  /// Complete verified response allocation.
  pub response: EngineResponse,
}

/// A rules engine whose exported path constructs finished FlatBuffers directly.
pub trait Engine {
  /// NUL-terminated SHA-256 digest of the complete build-composed wire contract.
  const WIRE_DIGEST_C: &'static [u8; 65];

  /// Starts a new session and returns its finished initial response.
  fn connect(&mut self, message: ConnectView<'_>) -> Result<EngineResponse, EngineError>;

  /// Verifies and applies one build-composed client message.
  fn submit(&mut self, bytes: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError>;

  /// Applies one verified borrowed UI event synchronously.
  fn submit_ui_event(
    &mut self,
    action: UiEventActionView<'_>,
  ) -> Result<UiEventResult, EngineError>;

  /// Returns one already-finished queued response, if any.
  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError>;
}

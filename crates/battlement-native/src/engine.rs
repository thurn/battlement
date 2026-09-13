use std::{error::Error, fmt};

use battlement::{
  AnyCommand, ClientMessage, Command, Connect, Response, UiEventAction, UiEventResponse,
};
use battlement_flatbuffers::{
  ConnectInput, ConnectView, CoreClientMessageView, FinishedMessage, ReducedMotionPreference,
  UiEventActionView, write_connect, write_core_response,
};
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

/// Encodes one engine response using its build-composed FlatBuffers schema.
pub trait FlatBufferResponseCommand: Sized {
  /// NUL-terminated SHA-256 digest of the complete generated wire contract.
  const WIRE_CONTRACT_DIGEST_C: &'static [u8; 65];

  /// Finishes and verifies a response without an intermediate transport object graph.
  fn write_response(response: &Response<Self>) -> Result<FinishedMessage, EngineError>;
}

impl FlatBufferResponseCommand for Command {
  const WIRE_CONTRACT_DIGEST_C: &'static [u8; 65] = crate::WIRE_CONTRACT_DIGEST_C;

  fn write_response(response: &Response<Self>) -> Result<FinishedMessage, EngineError> {
    write_core_response(response).map_err(|error| EngineError::new(error.to_string()))
  }
}

/// Supplies a generated game-specific response schema for [`AnyCommand`].
///
/// Implement this on the game's local payload type, using the Rust generated
/// from the same composed schema as the Unity command handlers.
pub trait CustomFlatBufferResponseSchema: Sized {
  /// NUL-terminated SHA-256 digest of the build-composed schema manifest.
  const WIRE_CONTRACT_DIGEST_C: &'static [u8; 65];

  /// Finishes and verifies a response containing core and game-specific commands.
  fn write_response(response: &Response<AnyCommand<Self>>) -> Result<FinishedMessage, EngineError>;
}

impl<P: CustomFlatBufferResponseSchema> FlatBufferResponseCommand for AnyCommand<P> {
  const WIRE_CONTRACT_DIGEST_C: &'static [u8; 65] = P::WIRE_CONTRACT_DIGEST_C;

  fn write_response(response: &Response<Self>) -> Result<FinishedMessage, EngineError> {
    P::write_response(response)
  }
}

/// Presents owned connection metadata through the same verified schema used by native clients.
///
/// This helper exists for in-process fakes and tests that originate with the public owned model.
pub fn with_connect_view<R>(message: &Connect, operation: impl FnOnce(ConnectView<'_>) -> R) -> R {
  let custom_command_types = message
    .custom_command_types
    .iter()
    .map(String::as_str)
    .collect::<Vec<_>>();
  let modules = message
    .modules
    .iter()
    .map(String::as_str)
    .collect::<Vec<_>>();
  let input = ConnectInput {
    platform: &message.platform,
    unity_version: &message.unity_version,
    screen_width: message.screen.width,
    screen_height: message.screen.height,
    focused: message.application_state.focused,
    paused: message.application_state.paused,
    reduced_motion_preference: match message.reduced_motion_preference {
      battlement::application::ReducedMotionPreference::Unavailable => {
        ReducedMotionPreference::Unavailable
      }
      battlement::application::ReducedMotionPreference::Reduce => ReducedMotionPreference::Reduce,
      battlement::application::ReducedMotionPreference::NoPreference => {
        ReducedMotionPreference::NoPreference
      }
    },
    custom_command_types: &custom_command_types,
    modules: &modules,
    persistent_data_path: message.persistent_data_path.as_deref(),
    streaming_assets_path: message.streaming_assets_path.as_deref(),
  };
  let bytes = write_connect(&input).expect("owned Connect must satisfy the FlatBuffers contract");
  operation(ConnectView::read(bytes.as_bytes()).expect("writer output must verify"))
}

/// A typed rules engine hosted by the native adapter.
///
/// Calls are serial and non-reentrant. `connect` begins a new session on the
/// existing instance: implementations must cancel old-session work and clear
/// responses that were pending for the prior session while preserving any
/// authoritative game state needed to build the new snapshot. Worker threads
/// may enqueue responses in engine-owned synchronization primitives for
/// `poll` to drain.
pub trait Engine {
  /// Game-owned custom action payload accepted by this engine.
  type ActionPayload;

  /// Error-code union accepted in client failure reports.
  type ErrorCode;

  /// Command union serialized in responses from this engine.
  type Command: FlatBufferResponseCommand;

  /// Starts a new session and returns its initial response.
  fn connect(&mut self, message: ConnectView<'_>) -> Result<Response<Self::Command>, EngineError>;

  /// Starts an in-process test session from the public owned connection model.
  fn connect_owned(&mut self, message: &Connect) -> Result<Response<Self::Command>, EngineError>
  where
    Self: Sized,
  {
    with_connect_view(message, |view| self.connect(view))
  }

  /// Applies one client submission and returns its immediate response.
  fn submit(
    &mut self,
    message: ClientMessage<Self::ActionPayload, Self::ErrorCode>,
  ) -> Result<Response<Self::Command>, EngineError>;

  /// Applies one verified borrowed core client submission.
  ///
  /// Native engines must override this during the FlatBuffers migration. The
  /// default deliberately rejects the call instead of reconstructing the old
  /// owned protocol graph.
  fn submit_core_view(
    &mut self,
    _message: CoreClientMessageView<'_>,
  ) -> Result<Response<Self::Command>, EngineError> {
    Err(EngineError::new(
      "this engine has not implemented borrowed core submissions",
    ))
  }

  /// Verifies and applies one size-prefixed client message from the native ABI.
  ///
  /// The default accepts the closed core schema. Engines with custom actions or
  /// custom failure codes override this method with the reader generated from
  /// their build-composed schema, then route its core variants through
  /// [`Engine::submit_core_view`].
  fn submit_flatbuffer(
    &mut self,
    bytes: &[u8],
  ) -> Result<Response<Self::Command>, FlatBufferSubmitError> {
    let message = CoreClientMessageView::read(bytes).map_err(|error| {
      FlatBufferSubmitError::invalid_argument(format!("invalid client message: {error}"))
    })?;
    self
      .submit_core_view(message)
      .map_err(FlatBufferSubmitError::engine)
  }

  /// Processes one UI event before its originating native callback returns.
  fn submit_ui_event(
    &mut self,
    action: UiEventAction,
  ) -> Result<UiEventResponse<Self::Command>, EngineError>;

  /// Processes one verified borrowed UI event before its native callback returns.
  ///
  /// Native engines must override this method and consume the generated view directly.
  /// The default deliberately rejects the call instead of reconstructing the old
  /// owned callback payload.
  fn submit_ui_event_view(
    &mut self,
    _action: UiEventActionView<'_>,
  ) -> Result<UiEventResponse<Self::Command>, EngineError> {
    Err(EngineError::new(
      "this engine has not implemented borrowed UI-event submissions",
    ))
  }

  /// Returns one queued response immediately, or `None` when no work is ready.
  fn poll(&mut self) -> Result<Option<Response<Self::Command>>, EngineError>;
}

/// Constructs one concrete engine instance for a native client.
pub trait EngineFactory: Sized {
  /// Engine implementation produced by this factory.
  type Engine: Engine;

  /// Creates the engine or returns diagnostic text for the caller.
  fn create(self) -> Result<Self::Engine, EngineError>;
}

/// Converts an infallible or fallible factory result into an engine.
pub trait IntoEngine {
  /// The constructed engine.
  type Engine: Engine;

  /// Returns the engine or its initialization error.
  fn into_engine(self) -> Result<Self::Engine, EngineError>;
}

impl<E: Engine> IntoEngine for E {
  type Engine = E;

  fn into_engine(self) -> Result<E, EngineError> {
    Ok(self)
  }
}

impl<E: Engine> IntoEngine for Result<E, EngineError> {
  type Engine = E;

  fn into_engine(self) -> Result<E, EngineError> {
    self
  }
}

impl<E, F> EngineFactory for F
where
  E: IntoEngine,
  F: FnOnce() -> E,
{
  type Engine = E::Engine;

  fn create(self) -> Result<Self::Engine, EngineError> {
    self().into_engine()
  }
}

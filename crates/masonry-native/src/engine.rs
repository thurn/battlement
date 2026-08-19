use std::{error::Error, fmt};

use masonry::{ClientMessage, Connect, Response};
use serde::{Serialize, de::DeserializeOwned};

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
    type ActionPayload: DeserializeOwned;

    /// Error-code union accepted in client failure reports.
    type ErrorCode: DeserializeOwned;

    /// Command union serialized in responses from this engine.
    type Command: Serialize;

    /// Starts a new session and returns its initial response.
    fn connect(&mut self, message: Connect) -> Result<Response<Self::Command>, EngineError>;

    /// Applies one client submission and returns its immediate response.
    fn submit(
        &mut self,
        message: ClientMessage<Self::ActionPayload, Self::ErrorCode>,
    ) -> Result<Response<Self::Command>, EngineError>;

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

impl<E, F> EngineFactory for F
where
    E: Engine,
    F: FnOnce() -> Result<E, EngineError>,
{
    type Engine = E;

    fn create(self) -> Result<Self::Engine, EngineError> {
        self()
    }
}

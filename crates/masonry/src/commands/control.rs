use serde::{Deserialize, Serialize};

use crate::{CommandId, KeyCode, ObjectId, PointerEvent};

/// Waits for a fixed positive duration.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct WaitPayload {
    /// Positive wait duration in milliseconds.
    pub duration_ms: u64,
}

/// Cancels an operation by the command identity that started it.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct CancelOperationPayload {
    /// Command and operation identity to cancel.
    pub command_id: CommandId,
}

/// Gates every pointer and key action.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct SetInputEnabledPayload {
    /// Whether Masonry accepts input actions.
    pub enabled: bool,
}

/// Replaces the enabled pointer-event set for one game object.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PointerEventsPayload {
    /// Target game object.
    pub object_id: ObjectId,
    /// Unique enabled pointer-event kinds.
    pub events: Vec<PointerEvent>,
}

/// Replaces the global physical-key set enabled for the session.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GlobalKeysPayload {
    /// Unique enabled W3C physical key codes.
    pub keys: Vec<KeyCode>,
}

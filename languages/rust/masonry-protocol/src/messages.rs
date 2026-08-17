//! Connection, response, snapshot, batch, action, and result messages.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ActionId, BatchId, BatchStart, Command, CommandId, GameObject, KeyCode, ObjectId,
    PointerButton, PreparedAsset, Scene, SceneId, ScreenPosition, ScreenSize, SessionId, Vector3,
};

/// Unity's initial connection message to the rules engine.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct Connect {
    #[serde(rename = "type")]
    message_type: ConnectTypeTag,
    /// Unity platform name, such as `macOS`.
    #[schemars(length(max = 65_536))]
    pub platform: String,
    /// Exact Unity editor/player version used by the build.
    #[schemars(length(max = 65_536))]
    pub unity_version: String,
    /// Current screen dimensions in physical pixels.
    pub screen: ScreenSize,
    /// Sorted or otherwise deterministic list of custom command types compiled into the build.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(inner(length(max = 65_536)), extend("uniqueItems" = true))]
    pub custom_command_types: Vec<String>,
    /// Absolute UTF-8 persistent-data path supplied by the native transport.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(max = 65_536))]
    pub persistent_data_path: Option<String>,
    /// Absolute UTF-8 StreamingAssets path supplied by the native transport.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(max = 65_536))]
    pub streaming_assets_path: Option<String>,
}

impl Connect {
    /// Creates a transport-neutral connect message without custom commands or native paths.
    #[must_use]
    pub fn new(
        platform: impl Into<String>,
        unity_version: impl Into<String>,
        screen: ScreenSize,
    ) -> Self {
        Self {
            message_type: ConnectTypeTag::Connect,
            platform: platform.into(),
            unity_version: unity_version.into(),
            screen,
            custom_command_types: Vec::new(),
            persistent_data_path: None,
            streaming_assets_path: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
enum ConnectTypeTag {
    #[default]
    #[serde(rename = "masonry.connect")]
    Connect,
}

/// One ordered response returned by connect, submit, or nonempty poll.
///
/// The generic command type defaults to the core [`Command`] union. Games may
/// substitute their own enum containing core and custom commands while reusing
/// the rest of the response, batch, and snapshot model.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct Response<C = Command> {
    #[serde(rename = "type")]
    message_type: ResponseTypeTag,
    /// Session to which every contained response message belongs.
    pub session_id: SessionId,
    /// Ordered snapshot and batch messages. Submit may return an empty list.
    #[schemars(length(max = 256))]
    pub messages: Vec<ResponseMessage<C>>,
}

impl<C> Response<C> {
    /// Creates a response for a session.
    #[must_use]
    pub fn new(session_id: SessionId, messages: Vec<ResponseMessage<C>>) -> Self {
        Self {
            message_type: ResponseTypeTag::Response,
            session_id,
            messages,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
enum ResponseTypeTag {
    #[default]
    #[serde(rename = "masonry.response")]
    Response,
}

/// A response message carried in a [`Response`].
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(tag = "type")]
pub enum ResponseMessage<C = Command> {
    /// A complete replacement description of Masonry-controlled Unity content.
    #[serde(rename = "masonry.snapshot")]
    Snapshot(Snapshot),
    /// An ordered batch of parallel command groups.
    #[serde(rename = "masonry.batch")]
    Batch(Batch<C>),
}

/// A complete replacement description of Masonry-controlled Unity content.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// Session this snapshot establishes or replaces.
    pub session_id: SessionId,
    /// Complete set of prepared Addressables assets.
    #[schemars(length(max = 16_384))]
    pub prepared_assets: Vec<PreparedAsset>,
    /// Complete nonempty set of loaded content scenes.
    #[schemars(length(min = 1, max = 32))]
    pub scenes: Vec<Scene>,
    /// Primary scene identity; optional only when `scenes` contains exactly one entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_scene_id: Option<SceneId>,
    /// Complete set of game objects.
    #[schemars(length(max = 100_000))]
    pub objects: Vec<GameObject>,
    /// Camera used for input raycasting and billboards.
    ///
    /// The referenced GameObject is created from an entry in this snapshot's
    /// `objects` list. It must be active in the Unity hierarchy and its Camera
    /// component must be enabled. This is unrelated to Unity's active Scene.
    pub input_camera_id: ObjectId,
    /// Whether pointer and keyboard input remains disabled after application.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub input_disabled: bool,
    /// Unique physical key codes enabled globally for this session.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("uniqueItems" = true))]
    pub global_keys: Vec<KeyCode>,
}

impl Snapshot {
    /// Creates a snapshot with input enabled and no global keys.
    ///
    /// `primary_scene_id` starts unset, which is valid only when `scenes`
    /// contains exactly one entry. Set it before serialization for a
    /// multi-scene snapshot.
    #[must_use]
    pub fn new(
        session_id: SessionId,
        prepared_assets: Vec<PreparedAsset>,
        scenes: Vec<Scene>,
        objects: Vec<GameObject>,
        input_camera_id: ObjectId,
    ) -> Self {
        Self {
            session_id,
            prepared_assets,
            scenes,
            primary_scene_id: None,
            objects,
            input_camera_id,
            input_disabled: false,
            global_keys: Vec::new(),
        }
    }
}

/// One ordered batch of parallel command groups.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct Batch<C = Command> {
    /// Session-unique batch identity used for duplicate suppression.
    pub batch_id: BatchId,
    /// Session in which this batch may execute.
    pub session_id: SessionId,
    /// Optional action whose processing caused this batch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caused_by_action_id: Option<ActionId>,
    /// Whether to start independently or after earlier blocking batches.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub start: BatchStart,
    /// Nonempty ordered list of parallel command groups.
    #[schemars(length(min = 1, max = 256))]
    pub groups: Vec<ParallelCommandGroup<C>>,
}

impl<C> Batch<C> {
    /// Creates an independent batch.
    #[must_use]
    pub fn new(
        batch_id: BatchId,
        session_id: SessionId,
        groups: Vec<ParallelCommandGroup<C>>,
    ) -> Self {
        Self {
            batch_id,
            session_id,
            caused_by_action_id: None,
            start: BatchStart::Now,
            groups,
        }
    }
}

/// Commands launched together before the batch considers the next group.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct ParallelCommandGroup<C = Command> {
    /// Nonempty ordered command list. Commands launch without waiting for peers in this group.
    #[schemars(length(min = 1, max = 4_096))]
    pub commands: Vec<C>,
}

impl<C> ParallelCommandGroup<C> {
    /// Creates a parallel command group from its launch-order command list.
    #[must_use]
    pub fn new(commands: Vec<C>) -> Self {
        Self { commands }
    }
}

/// A typed built-in action emitted by pointer or keyboard input.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    /// Session-unique action identity used by rules engines for deduplication.
    pub action_id: ActionId,
    /// Session in which the input occurred.
    pub session_id: SessionId,
    /// Exact built-in input action and payload.
    #[serde(flatten)]
    pub body: ActionBody,
}

impl Action {
    /// Creates a built-in pointer or key action.
    #[must_use]
    pub fn new(action_id: ActionId, session_id: SessionId, body: ActionBody) -> Self {
        Self {
            action_id,
            session_id,
            body,
        }
    }
}

/// The exact union of built-in pointer and key actions.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(tag = "type", content = "payload")]
pub enum ActionBody {
    /// Pointer began hovering an enabled game object.
    #[serde(rename = "masonry.pointer.enter")]
    PointerEnter(PointerPayload),
    /// Pointer stopped hovering an enabled game object.
    #[serde(rename = "masonry.pointer.exit")]
    PointerExit(PointerPayload),
    /// Pointer button was pressed over an enabled game object.
    #[serde(rename = "masonry.pointer.down")]
    PointerDown(PointerButtonPayload),
    /// Pointer button was released over an enabled game object.
    #[serde(rename = "masonry.pointer.up")]
    PointerUp(PointerButtonPayload),
    /// A press and release resolved to the same game object.
    #[serde(rename = "masonry.pointer.click")]
    PointerClick(PointerButtonPayload),
    /// Enabled physical key transitioned to down.
    #[serde(rename = "masonry.key.down")]
    KeyDown(KeyPayload),
    /// Enabled physical key transitioned to up.
    #[serde(rename = "masonry.key.up")]
    KeyUp(KeyPayload),
}

/// Pointer location data shared by enter and exit actions.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PointerPayload {
    /// Game object resolved from the collider hit.
    pub object_id: ObjectId,
    /// Mouse pointer `0` or a stable positive touch pointer identity.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    #[schemars(range(min = 0))]
    pub pointer_id: i32,
    /// Screen position in pixels from the bottom-left.
    pub screen_position: ScreenPosition,
    /// World hit position; exit carries the last hit on the exited object.
    pub world_hit: Vector3,
}

/// Pointer location and button data shared by down, up, and click actions.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PointerButtonPayload {
    /// Game object resolved from the collider hit.
    pub object_id: ObjectId,
    /// Mouse pointer `0` or a stable positive touch pointer identity.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    #[schemars(range(min = 0))]
    pub pointer_id: i32,
    /// Screen position in pixels from the bottom-left.
    pub screen_position: ScreenPosition,
    /// World hit position.
    pub world_hit: Vector3,
    /// Mouse-style button; touch uses and defaults to `left`.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub button: PointerButton,
}

/// Payload for a discrete physical-key transition.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct KeyPayload {
    /// W3C physical key code.
    pub key: KeyCode,
}

/// A game-specific action using Masonry's shared action format.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CustomAction<P = Value> {
    /// Session-unique action identity used for deduplication.
    pub action_id: ActionId,
    /// Session in which game code emitted the action.
    pub session_id: SessionId,
    /// Game-owned namespaced action discriminator.
    #[serde(rename = "type")]
    #[schemars(length(max = 65_536))]
    pub action_type: String,
    /// Game-specific payload, raw JSON by default or a game-owned Rust type.
    pub payload: P,
}

impl<P> CustomAction<P> {
    /// Creates a typed game-specific action.
    #[must_use]
    pub fn new(
        action_id: ActionId,
        session_id: SessionId,
        action_type: impl Into<String>,
        payload: P,
    ) -> Self {
        Self {
            action_id,
            session_id,
            action_type: action_type.into(),
            payload,
        }
    }
}

/// A client submission accepted by the common transport endpoint.
///
/// The error-code type defaults to [`CoreErrorCode`], and the custom-action
/// payload defaults to raw JSON. Games can substitute their own serializable
/// schema types for namespaced handler errors and typed custom actions.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(untagged)]
pub enum ClientMessage<E = CoreErrorCode, A = Value> {
    /// Built-in pointer or keyboard action.
    Action(Action),
    /// Game-specific typed action.
    CustomAction(CustomAction<A>),
    /// Batch validation or execution failure.
    BatchFailed(BatchFailed<E>),
    /// Late failure of a nonblocking custom operation.
    OperationFailed(OperationFailed<E>),
}

/// A validation or execution failure that stopped a batch.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct BatchFailed<E = CoreErrorCode> {
    #[serde(rename = "type")]
    message_type: BatchFailedTypeTag,
    /// Session in which the failure occurred.
    pub session_id: SessionId,
    /// Batch that failed.
    pub batch_id: BatchId,
    /// Command that failed, when the failure can be attributed to one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_id: Option<CommandId>,
    /// Stable core or game-specific error code.
    pub error_code: E,
    /// Bounded human-readable diagnostic text.
    #[schemars(length(max = 65_536))]
    pub message: String,
}

impl<E> BatchFailed<E> {
    /// Creates a batch-failure report.
    #[must_use]
    pub fn new(
        session_id: SessionId,
        batch_id: BatchId,
        command_id: Option<CommandId>,
        error_code: E,
        message: impl Into<String>,
    ) -> Self {
        Self {
            message_type: BatchFailedTypeTag::Failed,
            session_id,
            batch_id,
            command_id,
            error_code,
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
enum BatchFailedTypeTag {
    #[default]
    #[serde(rename = "masonry.batch.failed")]
    Failed,
}

/// A late failure from a nonblocking custom operation.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct OperationFailed<E = CoreErrorCode> {
    #[serde(rename = "type")]
    message_type: OperationFailedTypeTag,
    /// Session in which the operation failed.
    pub session_id: SessionId,
    /// Batch that launched the operation.
    pub batch_id: BatchId,
    /// Command identity, which is also the operation identity.
    pub command_id: CommandId,
    /// Stable core or game-specific error code.
    pub error_code: E,
    /// Bounded human-readable diagnostic text.
    #[schemars(length(max = 65_536))]
    pub message: String,
}

impl<E> OperationFailed<E> {
    /// Creates a nonblocking-operation failure report.
    #[must_use]
    pub fn new(
        session_id: SessionId,
        batch_id: BatchId,
        command_id: CommandId,
        error_code: E,
        message: impl Into<String>,
    ) -> Self {
        Self {
            message_type: OperationFailedTypeTag::Failed,
            session_id,
            batch_id,
            command_id,
            error_code,
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
enum OperationFailedTypeTag {
    #[default]
    #[serde(rename = "masonry.operation.failed")]
    Failed,
}

/// Stable error codes produced by Masonry's core validation and execution paths.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub enum CoreErrorCode {
    /// JSON could not be decoded into a reliable protocol record.
    InvalidJson,
    /// A fixed size or count limit was exceeded.
    LimitExceeded,
    /// A message belongs to another session.
    WrongSession,
    /// A session-unique identity was reused incorrectly.
    DuplicateId,
    /// A command identity was never executed in this session.
    UnknownCommand,
    /// A referenced game object does not exist.
    UnknownObject,
    /// A referenced content scene does not exist.
    UnknownScene,
    /// A referenced asset address is unknown.
    UnknownAsset,
    /// An asset address was not in the prepared set.
    AssetNotPrepared,
    /// A prepared address resolved to the wrong Unity asset type.
    AssetTypeMismatch,
    /// A prepared asset could not be removed while still in use.
    AssetInUse,
    /// A required supported component was missing from the target game object.
    ComponentMissing,
    /// A prefab game object contained too many supported components of one type.
    InvalidComponentCount,
    /// Game-object placement or parenting was invalid.
    InvalidHierarchy,
    /// A property value or property/type combination was invalid.
    InvalidProperty,
    /// A rotation write targeted an object whose face-camera billboard behavior is enabled.
    PropertyControlledByBillboard,
    /// Conflict waiting would wait forever.
    InfiniteWait,
    /// A batch depended on earlier blocking work that failed.
    EarlierBatchFailed,
    /// No custom handler was registered for the command type.
    HandlerNotRegistered,
    /// A registered custom handler failed.
    HandlerFailed,
    /// A Unity API call threw an exception.
    UnityException,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn connect_uses_the_fixed_discriminator_and_omits_transport_optional_fields() {
        let connect = Connect::new(
            "macOS",
            "6000.5.3f1",
            ScreenSize {
                width: 2560,
                height: 1440,
            },
        );

        assert_eq!(
            serde_json::to_value(connect).unwrap(),
            json!({
                "type": "masonry.connect",
                "platform": "macOS",
                "unityVersion": "6000.5.3f1",
                "screen": { "width": 2560, "height": 1440 }
            })
        );
    }

    #[test]
    fn click_requires_and_round_trips_screen_position() {
        let json = json!({
            "actionId": "28dfd8ca-4908-4bb8-86d7-5775d271fced",
            "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
            "type": "masonry.pointer.click",
            "payload": {
                "objectId": "cc847d6e-1468-42c6-9bec-9af5b5aa5c03",
                "screenPosition": { "x": 1280.0, "y": 720.0 },
                "worldHit": { "x": 0.1, "y": 0.4, "z": 0.0 }
            }
        });
        let action: Action = serde_json::from_value(json.clone()).unwrap();

        assert_eq!(serde_json::to_value(action).unwrap(), json);
    }

    #[test]
    fn click_without_screen_position_is_rejected() {
        let json = json!({
            "actionId": "28dfd8ca-4908-4bb8-86d7-5775d271fced",
            "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
            "type": "masonry.pointer.click",
            "payload": {
                "objectId": "cc847d6e-1468-42c6-9bec-9af5b5aa5c03",
                "worldHit": { "x": 0.1, "y": 0.4, "z": 0.0 }
            }
        });

        assert!(serde_json::from_value::<Action>(json).is_err());
    }

    #[test]
    fn response_round_trips_discriminators_and_disabled_input() {
        let json = json!({
            "type": "masonry.response",
            "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
            "messages": [{
                "type": "masonry.snapshot",
                "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
                "preparedAssets": [],
                "scenes": [{
                    "sceneId": "ca64d87d-33d9-4a19-be6e-597035312d01",
                    "address": "mygame/boards/forest"
                }],
                "objects": [{
                    "objectId": "8ff6f71c-6a74-41cf-8826-0e364abf9f97",
                    "kind": "camera",
                    "camera": {}
                }],
                "inputCameraId": "8ff6f71c-6a74-41cf-8826-0e364abf9f97",
                "inputDisabled": true
            }]
        });
        let response: Response = serde_json::from_value(json.clone()).unwrap();

        assert_eq!(serde_json::to_value(response).unwrap(), json);
    }
}

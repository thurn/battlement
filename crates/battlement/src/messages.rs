//! Connection, response, snapshot, batch, action, and result messages.

use serde::{Deserialize, Serialize};

use crate::application::ApplicationState;

use crate::{
  ActionId, BatchId, BatchStart, Command, CommandBody, CommandId, ControllerButton,
  ControllerDirection, ControllerInputSettings, ControllerNavigationSource, GameObject,
  GeometryObservationBatch, MotionEventBatch, ObjectId, PanelInputConfiguration, PhysicalKey,
  PointerButton, PreparedAsset, Scene, SceneId, ScreenPosition, ScreenSize, SessionId, UiDocument,
  UiEvent, Vector3,
};

/// Unity's initial connection message to the rules engine.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Connect {
  /// Unity platform name, such as `macOS`.
  pub platform: String,
  /// Exact Unity editor/player version used by the build.
  pub unity_version: String,
  /// Current screen dimensions in physical pixels.
  pub screen: ScreenSize,
  /// Initial application focus and suspension observations.
  pub application_state: ApplicationState,
  /// Sorted list of custom command types compiled into the build.
  pub custom_command_types: Vec<String>,
  /// Selected module identifiers in serialized Inspector order.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub modules: Vec<String>,
  /// Absolute UTF-8 persistent-data path supplied by Application.persistentDataPath.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub persistent_data_path: Option<String>,
  /// Absolute UTF-8 StreamingAssets path supplied by
  /// Application.streamingAssetsPath.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub streaming_assets_path: Option<String>,
}

impl Connect {
  /// Creates a connect message.
  #[must_use]
  pub fn new(
    platform: impl Into<String>,
    unity_version: impl Into<String>,
    screen: ScreenSize,
  ) -> Self {
    Self {
      platform: platform.into(),
      unity_version: unity_version.into(),
      screen,
      application_state: ApplicationState::default(),
      custom_command_types: Vec::new(),
      modules: Vec::new(),
      persistent_data_path: None,
      streaming_assets_path: None,
    }
  }
}

/// One ordered response returned by connect, submit, or nonempty poll.
///
/// The generic command type defaults to the core [`Command`] union. Games may
/// substitute their own enum containing core and custom commands while reusing
/// the rest of the response, batch, and snapshot model.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Response<C = Command> {
  /// Session to which every contained response message belongs.
  pub session_id: SessionId,
  /// Ordered snapshot and batch messages. Submit may return an empty list.
  pub messages: Vec<ResponseMessage<C>>,
}

impl<C> Response<C> {
  /// Creates a response for a session.
  #[must_use]
  pub fn new(session_id: SessionId, messages: Vec<ResponseMessage<C>>) -> Self {
    Self {
      session_id,
      messages,
    }
  }

  /// Creates a response containing no new work.
  #[must_use]
  pub fn empty(session_id: SessionId) -> Self {
    Self::new(session_id, Vec::new())
  }

  /// Creates a response containing one batch.
  #[must_use]
  pub fn batch(batch: Batch<C>) -> Self {
    Self::new(batch.session_id, vec![ResponseMessage::Batch(batch)])
  }
}

impl Response<Command> {
  /// Creates a response containing one replacement snapshot.
  #[must_use]
  pub fn snapshot(snapshot: Snapshot) -> Self {
    Self::new(
      snapshot.session_id,
      vec![ResponseMessage::Snapshot(snapshot)],
    )
  }

  /// Creates a response containing one parallel group of core command bodies.
  #[must_use]
  pub fn commands(session_id: SessionId, bodies: impl IntoIterator<Item = CommandBody>) -> Self {
    Self::batch(Batch::parallel(session_id, bodies))
  }

  /// Creates a command response caused by one client action.
  #[must_use]
  pub fn commands_for_action(
    session_id: SessionId,
    action_id: ActionId,
    bodies: impl IntoIterator<Item = CommandBody>,
  ) -> Self {
    Self::batch(Batch::parallel(session_id, bodies).caused_by_action_id(action_id))
  }
}

/// A response message carried in a [`Response`].
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ResponseMessage<C = Command> {
  /// A complete replacement description of Battlement-controlled Unity content.
  Snapshot(Snapshot),
  /// An ordered batch of parallel command groups.
  Batch(Batch<C>),
}

/// A complete replacement description of Battlement-controlled Unity content.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Snapshot {
  /// Session this snapshot establishes or replaces.
  pub session_id: SessionId,
  /// List of Addressables assets to fetch.
  pub prepared_assets: Vec<PreparedAsset>,
  /// Complete nonempty set of loaded content scenes.
  pub scenes: Vec<Scene>,
  /// Primary scene identifier; optional only when `scenes` contains exactly one entry.
  pub primary_scene_id: Option<SceneId>,
  /// List of game objects to create.
  pub objects: Vec<GameObject>,
  /// Battlement-owned UI documents and their root hierarchies.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub ui: Vec<UiDocument>,
  /// Process-wide settings used when world-space UI documents are present.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub panel_input_configuration: PanelInputConfiguration,
  /// Battlement camera used for input raycasting and billboards.
  ///
  /// When unset, the client uses Unity's enabled, active camera tagged
  /// `MainCamera`. Otherwise, the referenced GameObject is created from an
  /// entry in this snapshot's `objects` list and must contain an enabled
  /// Camera component.
  pub input_camera_id: Option<ObjectId>,
  /// Whether pointer and keyboard input remains disabled after application.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub input_disabled: bool,
  /// Unique physical key codes enabled globally for this session.
  pub global_keys: Vec<PhysicalKey>,
  /// Optional controller buttons and navigation behavior enabled for this session.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub controller_input: Option<ControllerInputSettings>,
}

impl Snapshot {
  /// Creates a snapshot with input enabled and no global keys.
  ///
  /// `primary_scene_id` starts unset, which is valid only when `scenes`
  /// contains exactly one entry. Set it before using a
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
      ui: Vec::new(),
      panel_input_configuration: PanelInputConfiguration::default(),
      input_camera_id: Some(input_camera_id),
      input_disabled: false,
      global_keys: Vec::new(),
      controller_input: None,
    }
  }

  /// Creates a snapshot that uses Unity's enabled, active `MainCamera`.
  #[must_use]
  pub fn new_with_main_camera(
    session_id: SessionId,
    prepared_assets: Vec<PreparedAsset>,
    scenes: Vec<Scene>,
    objects: Vec<GameObject>,
  ) -> Self {
    Self {
      session_id,
      prepared_assets,
      scenes,
      primary_scene_id: None,
      objects,
      ui: Vec::new(),
      panel_input_configuration: PanelInputConfiguration::default(),
      input_camera_id: None,
      input_disabled: false,
      global_keys: Vec::new(),
      controller_input: None,
    }
  }
}

/// One ordered batch of parallel command groups.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Batch<C = Command> {
  /// Batch identity used for duplicate suppression.
  pub batch_id: BatchId,
  /// Session in which this batch may execute.
  pub session_id: SessionId,
  /// Optional action whose processing caused this batch.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub caused_by_action_id: Option<ActionId>,
  /// Whether to start independently or after earlier blocking batches.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub start: BatchStart,
  /// Nonempty ordered list of parallel command groups.
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

impl Batch<Command> {
  /// Creates an independent batch with one parallel group of core command bodies.
  #[must_use]
  pub fn parallel(session_id: SessionId, bodies: impl IntoIterator<Item = CommandBody>) -> Self {
    Self::new(
      BatchId::new_v4(),
      session_id,
      vec![ParallelCommandGroup::from_bodies(bodies)],
    )
  }
}

/// Commands launched together before the batch considers the next group.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ParallelCommandGroup<C = Command> {
  /// Nonempty ordered command list. Commands launch without waiting for peers in this group.
  pub commands: Vec<C>,
}

impl<C> ParallelCommandGroup<C> {
  /// Creates a parallel command group from its launch-order command list.
  #[must_use]
  pub fn new(commands: Vec<C>) -> Self {
    Self { commands }
  }
}

impl ParallelCommandGroup<Command> {
  /// Creates a parallel group from core command bodies with generated identities.
  #[must_use]
  pub fn from_bodies(bodies: impl IntoIterator<Item = CommandBody>) -> Self {
    Self::new(bodies.into_iter().map(Command::new_v4).collect())
  }
}

/// A typed built-in action emitted by pointer, keyboard, or controller input.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Action {
  /// Session-unique action identity used by rules engines for deduplication.
  pub action_id: ActionId,
  /// Session in which the input occurred.
  pub session_id: SessionId,
  /// Exact built-in input action and payload.
  pub body: ActionBody,
}

/// One synchronous UI event submission with ordinary action identity.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct UiEventAction {
  /// Session-unique identity used to correlate resulting command batches.
  pub action_id: ActionId,
  /// Session in which the native event occurred.
  pub session_id: SessionId,
  /// Native UI event and its active cancellation state.
  pub event: UiEvent,
}

impl UiEventAction {
  /// Creates one synchronous UI event submission.
  #[must_use]
  pub const fn new(action_id: ActionId, session_id: SessionId, event: UiEvent) -> Self {
    Self {
      action_id,
      session_id,
      event,
    }
  }

  /// Returns whether the native cancellation state is internally consistent.
  #[must_use]
  pub const fn is_valid(&self) -> bool {
    self.event.cancelable || !self.event.default_prevented
  }
}

/// The immediate native-default decision returned by a UI event submission.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum UiEventDisposition {
  /// Let Unity continue its remaining default actions.
  #[default]
  Continue = 0,
  /// Ask Unity to skip the current event's remaining preventable defaults.
  PreventDefault = 1,
}

/// One immediate UI disposition paired with its deferred ordinary response.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct UiEventResponse<C = Command> {
  /// Decision consumed before the originating Unity callback returns.
  pub disposition: UiEventDisposition,
  /// Commands queued for later ordinary response processing.
  pub response: Response<C>,
}

impl<C> UiEventResponse<C> {
  /// Creates a synchronous UI event result.
  #[must_use]
  pub const fn new(disposition: UiEventDisposition, response: Response<C>) -> Self {
    Self {
      disposition,
      response,
    }
  }

  /// Creates a result that preserves the event's incoming prevention state.
  #[must_use]
  pub fn from_event(event: &UiEvent, response: Response<C>) -> Self {
    Self::new(
      if event.default_prevented {
        UiEventDisposition::PreventDefault
      } else {
        UiEventDisposition::Continue
      },
      response,
    )
  }
}

impl Action {
  /// Creates a built-in pointer, key, or controller action.
  #[must_use]
  pub fn new(action_id: ActionId, session_id: SessionId, body: ActionBody) -> Self {
    Self {
      action_id,
      session_id,
      body,
    }
  }
}

/// The exact union of built-in pointer, key, and controller actions.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ActionBody {
  /// Application focus or suspension changed, independently of input availability.
  ApplicationStateChanged(ApplicationState),
  /// Pointer began hovering an enabled game object.
  PointerEnter(PointerPayload),
  /// Pointer stopped hovering an enabled game object.
  PointerExit(PointerPayload),
  /// Pointer button was pressed over an enabled game object.
  PointerDown(PointerButtonPayload),
  /// Pointer button was released over an enabled game object.
  PointerUp(PointerButtonPayload),
  /// A press and release resolved to the same game object.
  PointerClick(PointerButtonPayload),
  /// The primary pointer picked up a draggable game object.
  DragStart(DragPayload),
  /// The primary pointer dropped a captured draggable game object.
  DragEnd(DragPayload),
  /// Enabled physical key transitioned to down.
  KeyDown(KeyPayload),
  /// Enabled physical key transitioned to up.
  KeyUp(KeyPayload),
  /// Enabled controller button transitioned to down.
  ControllerButtonDown(ControllerButtonPayload),
  /// Enabled controller button transitioned to up.
  ControllerButtonUp(ControllerButtonPayload),
  /// The D-pad or left stick requested one cardinal navigation step.
  ControllerNavigate(ControllerNavigationPayload),
  /// One coherent generation of changed geometry observations.
  GeometryObservations(GeometryObservationBatch),
  /// Ordered Motion lifecycle boundaries and coalesced samples.
  MotionEvents(MotionEventBatch),
}

/// Pointer location data shared by enter and exit actions.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct PointerPayload {
  /// Game object resolved from the collider hit.
  pub object_id: ObjectId,
  /// Mouse pointer `0` or a stable positive touch pointer identity.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pointer_id: i32,
  /// Screen position in pixels from the bottom-left.
  pub screen_position: ScreenPosition,
  /// World hit position; exit carries the last hit on the exited object.
  pub world_hit: Vector3,
}

/// Pointer location and button data shared by down, up, and click actions.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct PointerButtonPayload {
  /// Game object resolved from the collider hit.
  pub object_id: ObjectId,
  /// Mouse pointer `0` or a stable positive touch pointer identity.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pointer_id: i32,
  /// Screen position in pixels from the bottom-left.
  pub screen_position: ScreenPosition,
  /// World hit position.
  pub world_hit: Vector3,
  /// Mouse-style button; touch uses [`PointerButton::Left`].
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub button: PointerButton,
}

/// Object location data emitted at the start and end of a drag.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct DragPayload {
  /// Draggable game object captured by the pointer.
  pub object_id: ObjectId,
  /// Mouse pointer `0` or a stable positive touch pointer identity.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pointer_id: i32,
  /// Pointer position in pixels from the bottom-left.
  pub screen_position: ScreenPosition,
  /// World-space position of the object's transform.
  pub world_position: Vector3,
}

/// Payload for a discrete physical-key transition.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KeyPayload {
  /// W3C physical key code.
  pub key: PhysicalKey,
}

/// Payload for a discrete controller-button transition.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ControllerButtonPayload {
  /// Unity Input System device identity for the controller.
  pub controller_id: i32,
  /// Platform-independent physical button.
  pub button: ControllerButton,
}

/// Payload for one controller-navigation step.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ControllerNavigationPayload {
  /// Unity Input System device identity for the controller.
  pub controller_id: i32,
  /// Cardinal direction selected after dominant-axis resolution.
  pub direction: ControllerDirection,
  /// Physical control that produced the step.
  pub source: ControllerNavigationSource,
  /// Whether this step came from held-input repeat instead of the initial tilt or press.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub repeat: bool,
}

impl PointerPayload {
  /// Creates pointer-location payload data.
  #[must_use]
  pub fn new(
    object_id: ObjectId,
    pointer_id: i32,
    screen_position: ScreenPosition,
    world_hit: Vector3,
  ) -> Self {
    Self {
      object_id,
      pointer_id,
      screen_position,
      world_hit,
    }
  }
}

impl PointerButtonPayload {
  /// Creates pointer-button payload data.
  #[must_use]
  pub fn new(
    object_id: ObjectId,
    pointer_id: i32,
    screen_position: ScreenPosition,
    world_hit: Vector3,
    button: PointerButton,
  ) -> Self {
    Self {
      object_id,
      pointer_id,
      screen_position,
      world_hit,
      button,
    }
  }
}

impl DragPayload {
  /// Creates a drag lifecycle payload.
  #[must_use]
  pub fn new(
    object_id: ObjectId,
    pointer_id: i32,
    screen_position: ScreenPosition,
    world_position: Vector3,
  ) -> Self {
    Self {
      object_id,
      pointer_id,
      screen_position,
      world_position,
    }
  }
}

impl KeyPayload {
  /// Creates a physical-key payload.
  #[must_use]
  pub fn new(key: PhysicalKey) -> Self {
    Self { key }
  }
}

/// A game-specific action using Battlement's shared action format.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CustomAction<P> {
  /// Session-unique action identity used for deduplication.
  pub action_id: ActionId,
  /// Session in which game code emitted the action.
  pub session_id: SessionId,
  /// Game-owned namespaced action type.
  pub action_type: String,
  /// Game-specific payload.
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
/// Games supply their own payload and error-code types when extending the core
/// protocol.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ClientMessage<A, E = CoreErrorCode> {
  /// Built-in pointer or keyboard action.
  Action(Action),
  /// Game-specific typed action.
  CustomAction(CustomAction<A>),
  /// Batch validation or execution failure.
  BatchFailed(BatchFailed<E>),
  /// Late failure of a nonblocking custom operation.
  OperationFailed(OperationFailed<E>),
}

impl<A, E> ClientMessage<A, E> {
  /// Returns the built-in action, or `None` for every other message kind.
  #[must_use]
  pub fn into_action(self) -> Option<Action> {
    match self {
      Self::Action(action) => Some(action),
      _ => None,
    }
  }
}

/// A validation or execution failure that stopped a batch.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct BatchFailed<E = CoreErrorCode> {
  /// Session in which the failure occurred.
  pub session_id: SessionId,
  /// Batch that failed.
  pub batch_id: BatchId,
  /// Command that failed, when the failure can be attributed to one.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub command_id: Option<CommandId>,
  /// Stable core or game-specific error code.
  pub error_code: E,
  /// Short human-readable diagnostic text.
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
      session_id,
      batch_id,
      command_id,
      error_code,
      message: message.into(),
    }
  }
}

/// A late failure from a nonblocking custom operation.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OperationFailed<E = CoreErrorCode> {
  /// Session in which the operation failed.
  pub session_id: SessionId,
  /// Batch that launched the operation.
  pub batch_id: BatchId,
  /// Command identity, which is also the operation identity.
  pub command_id: CommandId,
  /// Stable core or game-specific error code.
  pub error_code: E,
  /// Bounded human-readable diagnostic text.
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
      session_id,
      batch_id,
      command_id,
      error_code,
      message: message.into(),
    }
  }
}

/// Stable error codes produced by Battlement's core validation and execution paths.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CoreErrorCode {
  /// Encoded input could not be decoded into a reliable protocol record.
  InvalidEncoding,
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
  /// No selected module owns the requested command.
  ///
  /// Diagnostics commands return this code when `Connect.modules` did not contain
  /// `battlement.diagnostics`. Unity's engine-owned diagnostic data collection can
  /// still be enabled independently for the build.
  ModuleUnavailable,
  /// Diagnostics metadata is outside Unity's supported key or value bounds.
  DiagnosticsMetadataInvalid,
  /// A local `CrashReportHandler` metadata call failed.
  DiagnosticsOperationFailed,
}

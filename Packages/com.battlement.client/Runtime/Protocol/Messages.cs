#nullable enable

using System;
using System.Collections.Generic;
using Newtonsoft.Json;

namespace Battlement
{
    /// <summary>The client's initial connection message to the rules engine.</summary>
    /// <param name="Platform">Platform name, such as <c>macOS</c>.</param>
    /// <param name="UnityVersion">Exact editor or player version used by the build.</param>
    /// <param name="Screen">Current screen dimensions in physical pixels.</param>
    /// <param name="CustomCommandTypes">Custom command types compiled into the build.</param>
    /// <param name="PersistentDataPath">The application's persistent-data path.</param>
    /// <param name="StreamingAssetsPath">The application's StreamingAssets path.</param>
    /// <param name="Modules">Selected module identifiers in Inspector order.</param>
    public sealed record Connect(
        string Platform,
        string UnityVersion,
        ScreenSize Screen,
        IReadOnlyList<string> CustomCommandTypes,
        string? PersistentDataPath = null,
        string? StreamingAssetsPath = null,
        IReadOnlyList<string>? Modules = null
    )
    {
        public Connect(string platform, string unityVersion, ScreenSize screen)
            : this(platform, unityVersion, screen, Array.Empty<string>()) { }
    }

    /// <summary>One ordered response returned by connect, submit, or nonempty poll.</summary>
    /// <typeparam name="TCommand">Command type used by response batches.</typeparam>
    public record Response<TCommand>(
        SessionId SessionId,
        IReadOnlyList<ResponseMessage<TCommand>> Messages
    )
        where TCommand : ICommand;

    /// <summary>A response containing Battlement core commands.</summary>
    public sealed record Response(
        SessionId SessionId,
        IReadOnlyList<ResponseMessage<Command>> Messages
    ) : Response<Command>(SessionId, Messages);

    /// <summary>A response message carried in a <see cref="Response{TCommand}"/>.</summary>
    public abstract record ResponseMessage<TCommand>
        where TCommand : ICommand
    {
        private ResponseMessage() { }

        /// <summary>A complete replacement description of controlled content.</summary>
        public sealed record SnapshotMessage(Snapshot Snapshot) : ResponseMessage<TCommand>;

        /// <summary>An ordered batch of parallel command groups.</summary>
        public sealed record BatchMessage(Batch<TCommand> Batch) : ResponseMessage<TCommand>;
    }

    /// <summary>A complete replacement description of Battlement-controlled content.</summary>
    /// <param name="SessionId">Session this snapshot establishes or replaces.</param>
    /// <param name="PreparedAssets">List of Addressables assets to fetch.</param>
    /// <param name="Scenes">Complete nonempty set of loaded content scenes.</param>
    /// <param name="Objects">List of game objects to create.</param>
    /// <param name="InputCameraId">
    /// Battlement camera used for input and billboards, or null to use Unity's main camera.
    /// </param>
    /// <param name="PrimarySceneId">Primary scene, optional for a single-scene snapshot.</param>
    /// <param name="IsInputDisabled">
    /// Whether pointer, keyboard, and controller input remains disabled.
    /// </param>
    /// <param name="GlobalKeys">Unique physical key codes enabled globally.</param>
    /// <param name="ControllerInput">Optional enabled controller input settings.</param>
    public sealed record Snapshot(
        SessionId SessionId,
        IReadOnlyList<PreparedAsset> PreparedAssets,
        IReadOnlyList<BattlementScene> Scenes,
        IReadOnlyList<BattlementGameObject> Objects,
        ObjectId? InputCameraId,
        SceneId? PrimarySceneId,
        [property: JsonProperty("input_disabled")] bool IsInputDisabled,
        IReadOnlyList<PhysicalKey> GlobalKeys,
        ControllerInputSettings? ControllerInput = null,
        IReadOnlyList<UiDocument>? Ui = null,
        PanelInputConfigurationValue? PanelInputConfiguration = null
    )
    {
        public Snapshot(
            SessionId sessionId,
            IReadOnlyList<PreparedAsset> preparedAssets,
            IReadOnlyList<BattlementScene> scenes,
            IReadOnlyList<BattlementGameObject> objects,
            ObjectId inputCameraId
        )
            : this(
                sessionId,
                preparedAssets,
                scenes,
                objects,
                inputCameraId,
                null,
                false,
                Array.Empty<PhysicalKey>()
            ) { }

        /// <summary>Creates a snapshot that uses Unity's enabled, active main camera.</summary>
        public Snapshot(
            SessionId sessionId,
            IReadOnlyList<PreparedAsset> preparedAssets,
            IReadOnlyList<BattlementScene> scenes,
            IReadOnlyList<BattlementGameObject> objects
        )
            : this(
                sessionId,
                preparedAssets,
                scenes,
                objects,
                null,
                null,
                false,
                Array.Empty<PhysicalKey>()
            ) { }
    }

    /// <summary>One ordered batch of parallel command groups.</summary>
    /// <typeparam name="TCommand">The command type carried by this batch.</typeparam>
    /// <param name="Id">Batch identity used for duplicate suppression.</param>
    /// <param name="SessionId">Session in which this batch may execute.</param>
    /// <param name="Groups">Nonempty ordered list of parallel command groups.</param>
    /// <param name="CausedByActionId">Action whose processing caused this batch, if any.</param>
    /// <param name="Start">How this batch relates to earlier blocking batches.</param>
    public record Batch<TCommand>(
        [property: JsonProperty("batch_id")] BatchId Id,
        SessionId SessionId,
        IReadOnlyList<ParallelCommandGroup<TCommand>> Groups,
        ActionId? CausedByActionId = null,
        BatchStart Start = BatchStart.Now
    )
        where TCommand : ICommand;

    /// <summary>An ordered batch containing Battlement core commands.</summary>
    public sealed record Batch(
        BatchId Id,
        SessionId SessionId,
        IReadOnlyList<ParallelCommandGroup<Command>> Groups,
        ActionId? CausedByActionId = null,
        BatchStart Start = BatchStart.Now
    ) : Batch<Command>(Id, SessionId, Groups, CausedByActionId, Start);

    /// <summary>Commands launched together before the batch considers the next group.</summary>
    /// <typeparam name="TCommand">The command type carried by this group.</typeparam>
    /// <param name="Commands">Nonempty command list in launch order.</param>
    public sealed record ParallelCommandGroup<TCommand>(IReadOnlyList<TCommand> Commands)
        where TCommand : ICommand;

    /// <summary>
    /// A typed built-in action emitted by pointer, keyboard, or controller input.
    /// </summary>
    /// <param name="Id">Session-unique identity used for deduplication.</param>
    /// <param name="SessionId">Session in which the input occurred.</param>
    /// <param name="Body">Exact built-in input action and its data.</param>
    public sealed record Action(
        [property: JsonProperty("action_id")] ActionId Id,
        SessionId SessionId,
        ActionBody Body
    );

    /// <summary>The exact union of built-in pointer, key, and controller actions.</summary>
    public abstract record ActionBody
    {
        private ActionBody() { }

        /// <summary>Pointer began hovering an enabled game object.</summary>
        /// <param name="ObjectId">Game object resolved from the collider hit.</param>
        /// <param name="ScreenPosition">Screen position in pixels from the bottom-left.</param>
        /// <param name="WorldHit">World hit position.</param>
        /// <param name="PointerId">Mouse pointer zero or stable touch identity.</param>
        public sealed record PointerEnter(
            ObjectId ObjectId,
            ScreenPosition ScreenPosition,
            Vector3 WorldHit,
            int PointerId = 0
        ) : ActionBody;

        /// <summary>Pointer stopped hovering an enabled game object.</summary>
        /// <param name="ObjectId">Game object resolved from the collider hit.</param>
        /// <param name="ScreenPosition">Screen position in pixels from the bottom-left.</param>
        /// <param name="WorldHit">Last world hit position on the exited object.</param>
        /// <param name="PointerId">Mouse pointer zero or stable touch identity.</param>
        public sealed record PointerExit(
            ObjectId ObjectId,
            ScreenPosition ScreenPosition,
            Vector3 WorldHit,
            int PointerId = 0
        ) : ActionBody;

        /// <summary>Pointer button was pressed over an enabled game object.</summary>
        /// <param name="ObjectId">Game object resolved from the collider hit.</param>
        /// <param name="ScreenPosition">Screen position in pixels from the bottom-left.</param>
        /// <param name="WorldHit">World hit position.</param>
        /// <param name="PointerId">Mouse pointer zero or stable touch identity.</param>
        /// <param name="Button">Mouse-style button; touch uses left.</param>
        public sealed record PointerDown(
            ObjectId ObjectId,
            ScreenPosition ScreenPosition,
            Vector3 WorldHit,
            int PointerId = 0,
            PointerButton Button = PointerButton.Left
        ) : ActionBody;

        /// <summary>Pointer button was released over an enabled game object.</summary>
        /// <param name="ObjectId">Game object resolved from the collider hit.</param>
        /// <param name="ScreenPosition">Screen position in pixels from the bottom-left.</param>
        /// <param name="WorldHit">World hit position.</param>
        /// <param name="PointerId">Mouse pointer zero or stable touch identity.</param>
        /// <param name="Button">Mouse-style button; touch uses left.</param>
        public sealed record PointerUp(
            ObjectId ObjectId,
            ScreenPosition ScreenPosition,
            Vector3 WorldHit,
            int PointerId = 0,
            PointerButton Button = PointerButton.Left
        ) : ActionBody;

        /// <summary>A press and release resolved to the same game object.</summary>
        /// <param name="ObjectId">Game object resolved from the collider hit.</param>
        /// <param name="ScreenPosition">Screen position in pixels from the bottom-left.</param>
        /// <param name="WorldHit">World hit position.</param>
        /// <param name="PointerId">Mouse pointer zero or stable touch identity.</param>
        /// <param name="Button">Mouse-style button; touch uses left.</param>
        public sealed record PointerClick(
            ObjectId ObjectId,
            ScreenPosition ScreenPosition,
            Vector3 WorldHit,
            int PointerId = 0,
            PointerButton Button = PointerButton.Left
        ) : ActionBody;

        /// <summary>The primary pointer picked up a draggable game object.</summary>
        /// <param name="ObjectId">Draggable game object captured by the pointer.</param>
        /// <param name="ScreenPosition">Pointer position in pixels from the bottom-left.</param>
        /// <param name="WorldPosition">World-space position of the object's transform.</param>
        /// <param name="PointerId">Mouse pointer zero or stable touch identity.</param>
        public sealed record DragStart(
            ObjectId ObjectId,
            ScreenPosition ScreenPosition,
            Vector3 WorldPosition,
            int PointerId = 0
        ) : ActionBody;

        /// <summary>The primary pointer dropped a captured draggable game object.</summary>
        /// <param name="ObjectId">Draggable game object captured by the pointer.</param>
        /// <param name="ScreenPosition">Pointer position in pixels from the bottom-left.</param>
        /// <param name="WorldPosition">World-space position of the object's transform.</param>
        /// <param name="PointerId">Mouse pointer zero or stable touch identity.</param>
        public sealed record DragEnd(
            ObjectId ObjectId,
            ScreenPosition ScreenPosition,
            Vector3 WorldPosition,
            int PointerId = 0
        ) : ActionBody;

        /// <summary>Enabled physical key transitioned to down.</summary>
        /// <param name="Key">W3C physical key code.</param>
        public sealed record KeyDown(PhysicalKey Key) : ActionBody;

        /// <summary>Enabled physical key transitioned to up.</summary>
        /// <param name="Key">W3C physical key code.</param>
        public sealed record KeyUp(PhysicalKey Key) : ActionBody;

        /// <summary>Enabled controller button transitioned to down.</summary>
        public sealed record ControllerButtonDown(int ControllerId, ControllerButton Button)
            : ActionBody;

        /// <summary>Enabled controller button transitioned to up.</summary>
        public sealed record ControllerButtonUp(int ControllerId, ControllerButton Button)
            : ActionBody;

        /// <summary>The D-pad or left stick requested one cardinal navigation step.</summary>
        public sealed record ControllerNavigate(
            int ControllerId,
            ControllerDirection Direction,
            ControllerNavigationSource Source,
            bool Repeat = false
        ) : ActionBody;

        /// <summary>One coherent generation of changed geometry observations.</summary>
        public sealed record GeometryObservations(GeometryObservationBatch Value) : ActionBody;

        /// <summary>A subscribed event from a Rust-authored UI element.</summary>
        public sealed record VisualElement(ObjectId TargetId, UiEventBody Body) : ActionBody;
    }

    /// <summary>A game-specific action using Battlement's shared action format.</summary>
    /// <typeparam name="TPayload">Game-owned action payload type.</typeparam>
    /// <param name="Id">Session-unique action identity used for deduplication.</param>
    /// <param name="SessionId">Session in which game code emitted the action.</param>
    /// <param name="Type">Game-owned namespaced action discriminator.</param>
    /// <param name="Payload">Game-specific payload.</param>
    public sealed record CustomAction<TPayload>(
        [property: JsonProperty("action_id")] ActionId Id,
        SessionId SessionId,
        [property: JsonProperty("action_type")] string Type,
        TPayload Payload
    );

    /// <summary>A client submission accepted by the common transport endpoint.</summary>
    /// <typeparam name="TError">Core or game-specific error-code type.</typeparam>
    /// <typeparam name="TCustomActionPayload">Game-specific action payload type.</typeparam>
    public abstract record ClientMessage<TError, TCustomActionPayload>
    {
        private ClientMessage() { }

        /// <summary>A built-in pointer, keyboard, or controller action.</summary>
        public sealed record ActionMessage(Action Action)
            : ClientMessage<TError, TCustomActionPayload>;

        /// <summary>A game-specific typed action.</summary>
        public sealed record CustomActionMessage(CustomAction<TCustomActionPayload> Action)
            : ClientMessage<TError, TCustomActionPayload>;

        /// <summary>A batch validation or execution failure.</summary>
        public sealed record BatchFailedMessage(BatchFailed<TError> Failure)
            : ClientMessage<TError, TCustomActionPayload>;

        /// <summary>A late failure of a nonblocking custom operation.</summary>
        public sealed record OperationFailedMessage(OperationFailed<TError> Failure)
            : ClientMessage<TError, TCustomActionPayload>;
    }

    /// <summary>A validation or execution failure that stopped a batch.</summary>
    /// <typeparam name="TError">Core or game-specific error-code type.</typeparam>
    /// <param name="SessionId">Session in which the failure occurred.</param>
    /// <param name="BatchId">Batch that failed.</param>
    /// <param name="ErrorCode">Stable core or game-specific error code.</param>
    /// <param name="Message">Short human-readable diagnostic text.</param>
    /// <param name="CommandId">Command that failed, when attributable to one.</param>
    public sealed record BatchFailed<TError>(
        SessionId SessionId,
        BatchId BatchId,
        TError ErrorCode,
        string Message,
        CommandId? CommandId = null
    );

    /// <summary>A late failure from a nonblocking custom operation.</summary>
    /// <typeparam name="TError">Core or game-specific error-code type.</typeparam>
    /// <param name="SessionId">Session in which the operation failed.</param>
    /// <param name="BatchId">Batch that launched the operation.</param>
    /// <param name="CommandId">Command and operation identity.</param>
    /// <param name="ErrorCode">Stable core or game-specific error code.</param>
    /// <param name="Message">Short human-readable diagnostic text.</param>
    public sealed record OperationFailed<TError>(
        SessionId SessionId,
        BatchId BatchId,
        CommandId CommandId,
        TError ErrorCode,
        string Message
    );

    /// <summary>Stable error codes produced by core validation and execution paths.</summary>
    public enum CoreErrorCode
    {
        /// <summary>The message could not be decoded into a reliable protocol record.</summary>
        InvalidEncoding,

        /// <summary>A fixed size or count limit was exceeded.</summary>
        LimitExceeded,

        /// <summary>A message belongs to another session.</summary>
        WrongSession,

        /// <summary>A session-unique identity was reused incorrectly.</summary>
        DuplicateId,

        /// <summary>A command identity was never executed in this session.</summary>
        UnknownCommand,

        /// <summary>A referenced game object does not exist.</summary>
        UnknownObject,

        /// <summary>A referenced content scene does not exist.</summary>
        UnknownScene,

        /// <summary>A referenced asset address is unknown.</summary>
        UnknownAsset,

        /// <summary>An asset address was not in the prepared set.</summary>
        AssetNotPrepared,

        /// <summary>A prepared address resolved to the wrong asset type.</summary>
        AssetTypeMismatch,

        /// <summary>A prepared asset could not be removed while still in use.</summary>
        AssetInUse,

        /// <summary>A required supported component was missing from the target object.</summary>
        ComponentMissing,

        /// <summary>A prefab object contained too many supported components of one type.</summary>
        InvalidComponentCount,

        /// <summary>Game-object placement or parenting was invalid.</summary>
        InvalidHierarchy,

        /// <summary>A property value or property/type combination was invalid.</summary>
        InvalidProperty,

        /// <summary>A rotation write targeted an object controlled by a billboard.</summary>
        PropertyControlledByBillboard,

        /// <summary>Conflict waiting would wait forever.</summary>
        InfiniteWait,

        /// <summary>A batch depended on earlier blocking work that failed.</summary>
        EarlierBatchFailed,

        /// <summary>No custom handler was registered for the command type.</summary>
        HandlerNotRegistered,

        /// <summary>A registered custom handler failed.</summary>
        HandlerFailed,

        /// <summary>A Unity API call threw an exception.</summary>
        UnityException,

        /// <summary>No selected module owns the requested command.</summary>
        ModuleUnavailable,

        /// <summary>Diagnostics metadata is outside Unity's supported bounds.</summary>
        DiagnosticsMetadataInvalid,

        /// <summary>A local CrashReportHandler metadata call failed.</summary>
        DiagnosticsOperationFailed,
    }
}

#nullable enable

using System;
using UnityEngine.SceneManagement;

namespace Battlement
{
    /// <summary>
    /// Generated random-access reader for a build-composed response schema.
    /// Implementations must not retain the supplied byte buffer or generated table structs.
    /// </summary>
    public interface IBattlementFlatBufferResponseViewSchema
    {
        /// <summary>SHA-256 digest of the complete build-composed wire manifest.</summary>
        string WireContractDigest { get; }

        void ValidateResponse(Google.FlatBuffers.ByteBuffer bytes);
        SessionId ReadSessionId(Google.FlatBuffers.ByteBuffer bytes);
        int ReadMessageCount(Google.FlatBuffers.ByteBuffer bytes);
        bool IsSnapshot(Google.FlatBuffers.ByteBuffer bytes, int messageIndex);
        Battlement.FlatBuffers.Generated.Snapshot ReadSnapshotTable(
            Google.FlatBuffers.ByteBuffer bytes,
            int messageIndex
        );
        BatchId ReadBatchId(Google.FlatBuffers.ByteBuffer bytes, int messageIndex);
        SessionId ReadBatchSessionId(Google.FlatBuffers.ByteBuffer bytes, int messageIndex);
        ActionId? ReadCausedByActionId(Google.FlatBuffers.ByteBuffer bytes, int messageIndex);
        BatchStart ReadBatchStart(Google.FlatBuffers.ByteBuffer bytes, int messageIndex);
        ulong? ReadWorkScope(Google.FlatBuffers.ByteBuffer bytes, int messageIndex);
        ulong? ReadCancelScope(Google.FlatBuffers.ByteBuffer bytes, int messageIndex);
        PresentationControl? ReadPresentationControl(
            Google.FlatBuffers.ByteBuffer bytes,
            int messageIndex
        );
        int ReadGroupCount(Google.FlatBuffers.ByteBuffer bytes, int messageIndex);
        int ReadCommandCount(Google.FlatBuffers.ByteBuffer bytes, int messageIndex, int groupIndex);
        CommandId ReadCommandId(
            Google.FlatBuffers.ByteBuffer bytes,
            int messageIndex,
            int groupIndex,
            int commandIndex
        );
        bool ReadCommandIsBlocking(
            Google.FlatBuffers.ByteBuffer bytes,
            int messageIndex,
            int groupIndex,
            int commandIndex
        );
        bool IsCoreCommand(
            Google.FlatBuffers.ByteBuffer bytes,
            int messageIndex,
            int groupIndex,
            int commandIndex
        );
        Battlement.FlatBuffers.Generated.CoreCommand ReadCoreCommandTable(
            Google.FlatBuffers.ByteBuffer bytes,
            int messageIndex,
            int groupIndex,
            int commandIndex
        );
        IBattlementCommandOperation? LaunchCustomCommand(
            Google.FlatBuffers.ByteBuffer bytes,
            int messageIndex,
            int groupIndex,
            int commandIndex,
            CommandId commandId,
            bool isBlocking,
            IBattlementFlatBufferCustomCommandDispatcher dispatcher,
            TimeSpan now
        );
        bool IsAssetPreparation(
            Google.FlatBuffers.ByteBuffer bytes,
            int messageIndex,
            int groupIndex,
            int commandIndex
        );
    }

    /// <summary>Generated writer for one build-composed client-message schema.</summary>
    public interface IBattlementFlatBufferClientSchema
    {
        /// <summary>Encodes one game-owned action.</summary>
        ReadOnlyMemory<byte> SerializeCustomAction<TPayload>(CustomAction<TPayload> value);

        /// <summary>Encodes one game-owned batch failure.</summary>
        ReadOnlyMemory<byte> SerializeBatchFailure<TError>(BatchFailed<TError> value);

        /// <summary>Encodes one game-owned operation failure.</summary>
        ReadOnlyMemory<byte> SerializeOperationFailure<TError>(OperationFailed<TError> value);
    }

    /// <summary>A rules-engine transport owned by one <see cref="BattlementRunner"/>.</summary>
    public interface IBattlementTransport : IDisposable
    {
        /// <summary>Starts a new transport session.</summary>
        BattlementTransportResult Connect(ReadOnlyMemory<byte> message);

        /// <summary>Submits one already encoded client message synchronously.</summary>
        BattlementTransportResult Submit(ReadOnlyMemory<byte> message);

        /// <summary>Submits one UI event before its native callback returns.</summary>
        BattlementUiEventTransportResult SubmitUiEvent(ReadOnlyMemory<byte> message);

        /// <summary>Polls immediately for one response.</summary>
        BattlementTransportResult Poll();

        /// <summary>
        /// Stops the active session without disposing reusable transport resources.
        /// </summary>
        void Stop();
    }

    /// <summary>Observes typed core messages before transport submission.</summary>
    public interface IBattlementCoreMessageObserver
    {
        void RecordAction(Action value);

        void RecordBatchFailure(BatchFailed<CoreErrorCode> value);

        void RecordOperationFailure(OperationFailed<CoreErrorCode> value);
    }

    internal interface IBattlementClientMessageObserver
    {
        void RecordConnect(Connect value);

        void RecordUiEvent(UiEventAction value);
    }

    /// <summary>The transport-level outcome of one synchronous engine call.</summary>
    public enum BattlementTransportStatus
    {
        Success,
        NoMessage,
        InvalidArgument,
        EngineError,
        Panic,
        AbiError,
        TransportError,
    }

    /// <summary>A response payload or diagnostic returned by a transport call.</summary>
    public sealed record BattlementTransportResult : IDisposable
    {
        private readonly ReadOnlyMemory<byte> payload;
        private bool nativeBacked;

        public BattlementTransportResult(
            BattlementTransportStatus status,
            ReadOnlyMemory<byte> payload = default,
            string? diagnostic = null,
            int? nativeStatus = null
        )
        {
            Status = status;
            this.payload = payload;
            Diagnostic = diagnostic;
            NativeStatus = nativeStatus;
        }

        public BattlementTransportStatus Status { get; }

        /// <summary>
        /// Returns an independently owned copy when the response is backed by native memory.
        /// </summary>
        public ReadOnlyMemory<byte> Payload => nativeBacked ? payload.ToArray() : payload;

        internal ReadOnlyMemory<byte> BorrowedPayload => payload;

        internal int PayloadLength => payload.Length;

        public string? Diagnostic { get; }

        public int? NativeStatus { get; }

        internal IDisposable? PayloadOwner { get; private set; }

        internal BattlementTransportResult OwnPayload(IDisposable owner)
        {
            PayloadOwner = owner;
            nativeBacked = owner is BattlementNativeBufferMemory;
            return this;
        }

        internal IDisposable? DetachPayloadOwner()
        {
            IDisposable? owner = PayloadOwner;
            PayloadOwner = null;
            return owner;
        }

        public void Dispose()
        {
            PayloadOwner?.Dispose();
            PayloadOwner = null;
        }
    }

    /// <summary>An immediate UI disposition and opaque deferred response payload.</summary>
    public sealed record BattlementUiEventTransportResult : IDisposable
    {
        private readonly ReadOnlyMemory<byte> responsePayload;
        private bool nativeBacked;

        public BattlementUiEventTransportResult(
            BattlementTransportStatus status,
            UiEventDisposition disposition,
            ReadOnlyMemory<byte> responsePayload,
            string? diagnostic = null,
            int? nativeStatus = null
        ) =>
            (Status, Disposition, this.responsePayload, Diagnostic, NativeStatus) = (
                status,
                disposition,
                responsePayload,
                diagnostic,
                nativeStatus
            );

        public BattlementTransportStatus Status { get; }

        public UiEventDisposition Disposition { get; }

        /// <summary>
        /// Returns an independently owned copy when the response is backed by native memory.
        /// </summary>
        public ReadOnlyMemory<byte> ResponsePayload =>
            nativeBacked ? responsePayload.ToArray() : responsePayload;

        public string? Diagnostic { get; }

        public int? NativeStatus { get; }

        internal ReadOnlyMemory<byte> BorrowedResponsePayload => responsePayload;

        internal int ResponsePayloadLength => responsePayload.Length;

        internal IDisposable? PayloadOwner { get; private set; }

        internal BattlementUiEventTransportResult OwnPayload(IDisposable? owner)
        {
            PayloadOwner = owner;
            nativeBacked = owner is BattlementNativeBufferMemory;
            return this;
        }

        internal IDisposable? DetachPayloadOwner()
        {
            IDisposable? owner = PayloadOwner;
            PayloadOwner = null;
            return owner;
        }

        public void Dispose()
        {
            PayloadOwner?.Dispose();
            PayloadOwner = null;
        }
    }

    /// <summary>Prepares Addressables entries for use by Battlement-controlled content.</summary>
    public interface IBattlementAssetStorage : IDisposable
    {
        /// <summary>Begins preparing one declared asset.</summary>
        IBattlementAssetHandle Prepare(PreparedAsset asset);

        /// <summary>Begins loading one prepared scene additively.</summary>
        IBattlementSceneHandle LoadScene(IBattlementAssetLease sceneAsset);
    }

    /// <summary>An owned asset preparation operation and its retained load handle.</summary>
    public interface IBattlementAssetHandle : IDisposable
    {
        /// <summary>Gets the asset declaration associated with this handle.</summary>
        PreparedAsset Asset { get; }

        /// <summary>Gets a value indicating whether preparation has finished.</summary>
        bool IsDone { get; }

        /// <summary>Gets the prepared value after successful completion.</summary>
        object? Value { get; }

        /// <summary>Gets the preparation error after failed completion.</summary>
        Exception? Error { get; }
    }

    /// <summary>
    /// Keeps a prepared value available while Battlement-controlled content references it.
    /// </summary>
    public interface IBattlementAssetLease : Battlement.UI.IBattlementUiAssetLease { }

    /// <summary>An owned additive scene load and its eventual unload operation.</summary>
    public interface IBattlementSceneHandle : IDisposable
    {
        /// <summary>Gets the prepared scene declaration used by this load.</summary>
        PreparedAsset.Scene Asset { get; }

        /// <summary>Gets whether the additive load completed successfully.</summary>
        bool IsLoaded { get; }

        /// <summary>Gets the loaded Unity scene after successful completion.</summary>
        Scene Scene { get; }

        /// <summary>Gets a scene load or unload error.</summary>
        Exception? Error { get; }

        /// <summary>Starts unloading the owned scene. Repeated calls are no-ops.</summary>
        void BeginUnload();

        /// <summary>Gets whether the scene finished unloading.</summary>
        bool IsUnloaded { get; }
    }

    /// <summary>A stable asset preparation or lookup failure.</summary>
    public sealed class BattlementAssetException : Exception
    {
        /// <summary>Creates a failure with its protocol-visible error code.</summary>
        public BattlementAssetException(
            CoreErrorCode errorCode,
            string message,
            Exception? innerException = null
        )
            : base(message, innerException) => ErrorCode = errorCode;

        /// <summary>Gets the core error code reported for this failure.</summary>
        public CoreErrorCode ErrorCode { get; }
    }

    /// <summary>A monotonic time source used by Battlement scheduling.</summary>
    public interface IBattlementClock
    {
        /// <summary>Gets elapsed monotonic time since an arbitrary origin.</summary>
        TimeSpan Elapsed { get; }
    }
}

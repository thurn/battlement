#nullable enable

using System;
using Newtonsoft.Json;
using UnityEngine.SceneManagement;

namespace Battlement
{
    /// <summary>A rules-engine transport owned by one <see cref="BattlementRunner"/>.</summary>
    public interface IBattlementTransport : IDisposable
    {
        /// <summary>Starts a new transport session.</summary>
        BattlementTransportResult Connect(ReadOnlyMemory<byte> json);

        /// <summary>Submits one JSON client message synchronously.</summary>
        BattlementTransportResult Submit(ReadOnlyMemory<byte> json);

        /// <summary>Polls immediately for one response.</summary>
        BattlementTransportResult Poll();

        /// <summary>
        /// Stops the active session without disposing reusable transport resources.
        /// </summary>
        void Stop();
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

    /// <summary>An owned response payload or diagnostic returned by a transport call.</summary>
    public sealed record BattlementTransportResult
    {
        public BattlementTransportResult(
            BattlementTransportStatus status,
            ReadOnlyMemory<byte> payload = default,
            string? diagnostic = null,
            int? nativeStatus = null
        )
        {
            Status = status;
            Payload = payload;
            Diagnostic = diagnostic;
            NativeStatus = nativeStatus;
        }

        public BattlementTransportStatus Status { get; }

        public ReadOnlyMemory<byte> Payload { get; }

        public string? Diagnostic { get; }

        public int? NativeStatus { get; }
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

    /// <summary>Encodes and decodes the protocol values used by the host.</summary>
    public interface IBattlementProtocolCodec
    {
        /// <summary>Encodes one connection message.</summary>
        byte[] SerializeConnect(Connect value);

        /// <summary>Encodes one core batch-failure submission.</summary>
        byte[] SerializeBatchFailure(BatchFailed<CoreErrorCode> value);

        /// <summary>Encodes one core operation-failure submission.</summary>
        byte[] SerializeOperationFailure(OperationFailed<CoreErrorCode> value);

        /// <summary>Encodes one built-in pointer or keyboard action.</summary>
        byte[] SerializeAction(Action value);

        /// <summary>Decodes one response containing core commands.</summary>
        Response DeserializeResponse(ReadOnlyMemory<byte> bytes);
    }

    /// <summary>JSON extension operations needed by registered game code.</summary>
    public interface IBattlementExtensionProtocolCodec : IBattlementProtocolCodec
    {
        /// <summary>Decodes a response and delegates each custom payload to its registry.</summary>
        Response<ICommand> DeserializeResponse(
            ReadOnlyMemory<byte> bytes,
            Func<CommandId, string, bool, ReadOnlyMemory<byte>, ICommand> decodeCustomCommand
        );

        /// <summary>Encodes one typed game-owned action.</summary>
        byte[] SerializeCustomAction<TPayload>(
            CustomAction<TPayload> value,
            JsonConverter<TPayload>? payloadConverter
        );

        /// <summary>Encodes one game-owned batch failure.</summary>
        byte[] SerializeBatchFailure<TError>(
            BatchFailed<TError> value,
            JsonConverter<TError>? errorConverter
        );

        /// <summary>Encodes one game-owned late operation failure.</summary>
        byte[] SerializeOperationFailure<TError>(
            OperationFailed<TError> value,
            JsonConverter<TError>? errorConverter
        );
    }
}

#nullable enable

using System;
using UnityEngine.SceneManagement;

namespace Masonry
{
    /// <summary>A rules-engine transport owned by one <see cref="MasonryRunner"/>.</summary>
    public interface IMasonryTransport : IDisposable
    {
        /// <summary>
        /// Gets the transport kind used to shape environment-specific connect data.
        /// </summary>
        MasonryTransportKind Kind { get; }

        /// <summary>Starts a new transport session.</summary>
        MasonryTransportResult Connect(ReadOnlyMemory<byte> messagePack);

        /// <summary>Submits one MessagePack client message synchronously.</summary>
        MasonryTransportResult Submit(ReadOnlyMemory<byte> messagePack);

        /// <summary>Polls immediately for one response.</summary>
        MasonryTransportResult Poll();

        /// <summary>
        /// Stops the active session without disposing reusable transport resources.
        /// </summary>
        void Stop();
    }

    /// <summary>The transport-level outcome of one synchronous engine call.</summary>
    public enum MasonryTransportStatus
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
    public sealed record MasonryTransportResult
    {
        public MasonryTransportResult(
            MasonryTransportStatus status,
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

        public MasonryTransportStatus Status { get; }

        public ReadOnlyMemory<byte> Payload { get; }

        public string? Diagnostic { get; }

        public int? NativeStatus { get; }
    }

    /// <summary>Prepares Addressables entries for use by Masonry-controlled content.</summary>
    public interface IMasonryAssetStorage : IDisposable
    {
        /// <summary>Begins preparing one declared asset.</summary>
        IMasonryAssetHandle Prepare(PreparedAsset asset);

        /// <summary>Begins loading one prepared scene additively.</summary>
        IMasonrySceneHandle LoadScene(IMasonryAssetLease sceneAsset);
    }

    /// <summary>An owned asset preparation operation and its retained load handle.</summary>
    public interface IMasonryAssetHandle : IDisposable
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
    /// Keeps a prepared value available while Masonry-controlled content references it.
    /// </summary>
    public interface IMasonryAssetLease : IDisposable
    {
        /// <summary>Gets the declaration whose prepared value is retained.</summary>
        PreparedAsset Asset { get; }

        /// <summary>Gets the prepared Unity or Addressables value.</summary>
        object Value { get; }
    }

    /// <summary>An owned additive scene load and its eventual unload operation.</summary>
    public interface IMasonrySceneHandle : IDisposable
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
    public sealed class MasonryAssetException : Exception
    {
        /// <summary>Creates a failure with its protocol-visible error code.</summary>
        public MasonryAssetException(
            CoreErrorCode errorCode,
            string message,
            Exception? innerException = null
        )
            : base(message, innerException) => ErrorCode = errorCode;

        /// <summary>Gets the core error code reported for this failure.</summary>
        public CoreErrorCode ErrorCode { get; }
    }

    /// <summary>A monotonic time source used by Masonry scheduling.</summary>
    public interface IMasonryClock
    {
        /// <summary>Gets elapsed monotonic time since an arbitrary origin.</summary>
        TimeSpan Elapsed { get; }
    }

    /// <summary>Encodes and decodes the protocol values used by the host.</summary>
    public interface IMasonryProtocolCodec
    {
        /// <summary>Encodes one connection message.</summary>
        byte[] SerializeConnect(Connect value);

        /// <summary>Encodes one core batch-failure submission.</summary>
        byte[] SerializeBatchFailure(BatchFailed<CoreErrorCode> value);

        /// <summary>Encodes one core operation-failure submission.</summary>
        byte[] SerializeOperationFailure(OperationFailed<CoreErrorCode> value);

        /// <summary>Decodes one response containing core commands.</summary>
        Response DeserializeResponse(ReadOnlyMemory<byte> bytes);
    }
}

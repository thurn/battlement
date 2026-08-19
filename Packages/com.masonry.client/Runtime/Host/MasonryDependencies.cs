#nullable enable

using System;

namespace Masonry
{
    /// <summary>A rules-engine transport owned by one <see cref="MasonryRunner"/>.</summary>
    public interface IMasonryTransport : IDisposable
    {
        /// <summary>Starts a new transport session.</summary>
        MasonryTransportResult Connect();

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

    /// <summary>A monotonic time source used by Masonry scheduling.</summary>
    public interface IMasonryClock
    {
        /// <summary>Gets elapsed monotonic time since an arbitrary origin.</summary>
        TimeSpan Elapsed { get; }
    }
}

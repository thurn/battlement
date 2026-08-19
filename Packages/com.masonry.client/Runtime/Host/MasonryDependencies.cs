#nullable enable

using System;

namespace Masonry
{
    /// <summary>A rules-engine transport owned by one <see cref="MasonryRunner"/>.</summary>
    public interface IMasonryTransport : IDisposable
    {
        /// <summary>Starts a new transport session.</summary>
        void Connect();

        /// <summary>
        /// Stops the active session without disposing reusable transport resources.
        /// </summary>
        void Stop();
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

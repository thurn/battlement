#nullable enable

using System;
using UnityEngine;

namespace Masonry
{
    /// <summary>Immutable host dependencies and test behavior for a runner.</summary>
    public sealed record MasonryRunnerOptions
    {
        public MasonryRunnerOptions(
            IMasonryTransport transport,
            IMasonryAssetStorage assetStorage,
            IMasonryClock? clock = null,
            IMasonryLogger? logger = null,
            bool useInstantAnimations = false
        )
        {
            Transport = Errors.CheckNotNull(transport, nameof(transport));
            AssetStorage = Errors.CheckNotNull(assetStorage, nameof(assetStorage));
            Clock = clock ?? new UnityMasonryClock();
            Logger = logger ?? new MasonryUnityLogger();
            UseInstantAnimations = useInstantAnimations;
        }

        public IMasonryTransport Transport { get; }

        public IMasonryAssetStorage AssetStorage { get; }

        public IMasonryClock Clock { get; }

        public IMasonryLogger Logger { get; }

        public bool UseInstantAnimations { get; }
    }

    internal sealed class UnityMasonryClock : IMasonryClock
    {
        public TimeSpan Elapsed => TimeSpan.FromSeconds(Time.realtimeSinceStartupAsDouble);
    }
}

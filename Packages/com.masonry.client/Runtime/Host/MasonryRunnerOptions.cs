#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

namespace Masonry
{
    /// <summary>Immutable host dependencies and test behavior for a runner.</summary>
    public sealed record MasonryRunnerOptions
    {
        public MasonryRunnerOptions(
            IMasonryTransport transport,
            IMasonryAssetStorage assetStorage,
            IMasonryProtocolCodec protocolCodec,
            IMasonryClock? clock = null,
            IMasonryLogger? logger = null,
            bool useInstantAnimations = false,
            IEnumerable<string>? customCommandTypes = null
        )
        {
            Transport = Errors.CheckNotNull(transport, nameof(transport));
            AssetStorage = Errors.CheckNotNull(assetStorage, nameof(assetStorage));
            ProtocolCodec = Errors.CheckNotNull(protocolCodec, nameof(protocolCodec));
            Clock = clock ?? new UnityMasonryClock();
            Logger = logger ?? new MasonryUnityLogger();
            UseInstantAnimations = useInstantAnimations;
            CustomCommandTypes = (customCommandTypes ?? Array.Empty<string>())
                .OrderBy(type => type, StringComparer.Ordinal)
                .ToArray();
        }

        public IMasonryTransport Transport { get; }

        public IMasonryAssetStorage AssetStorage { get; }

        public IMasonryProtocolCodec ProtocolCodec { get; }

        public IMasonryClock Clock { get; }

        public IMasonryLogger Logger { get; }

        public bool UseInstantAnimations { get; }

        public IReadOnlyList<string> CustomCommandTypes { get; }
    }

    internal sealed class UnityMasonryClock : IMasonryClock
    {
        public TimeSpan Elapsed => TimeSpan.FromSeconds(Time.realtimeSinceStartupAsDouble);
    }
}

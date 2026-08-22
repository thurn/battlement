#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

namespace Battlement
{
    /// <summary>Immutable host dependencies and test behavior for a runner.</summary>
    public sealed record BattlementRunnerOptions
    {
        public BattlementRunnerOptions(
            IBattlementTransport transport,
            IBattlementAssetStorage assetStorage,
            IBattlementProtocolCodec protocolCodec,
            IBattlementClock? clock = null,
            IBattlementLogger? logger = null,
            bool useInstantAnimations = false,
            IEnumerable<string>? customCommandTypes = null
        )
        {
            Transport = Errors.CheckNotNull(transport, nameof(transport));
            AssetStorage = Errors.CheckNotNull(assetStorage, nameof(assetStorage));
            ProtocolCodec = Errors.CheckNotNull(protocolCodec, nameof(protocolCodec));
            Clock = clock ?? new UnityBattlementClock();
            Logger = logger ?? new BattlementUnityLogger();
            UseInstantAnimations = useInstantAnimations;
            CustomCommandTypes = (customCommandTypes ?? Array.Empty<string>())
                .OrderBy(type => type, StringComparer.Ordinal)
                .ToArray();
        }

        public IBattlementTransport Transport { get; }

        public IBattlementAssetStorage AssetStorage { get; }

        public IBattlementProtocolCodec ProtocolCodec { get; }

        public IBattlementClock Clock { get; }

        public IBattlementLogger Logger { get; }

        public bool UseInstantAnimations { get; }

        public IReadOnlyList<string> CustomCommandTypes { get; }
    }

    internal sealed class UnityBattlementClock : IBattlementClock
    {
        public TimeSpan Elapsed => TimeSpan.FromSeconds(Time.realtimeSinceStartupAsDouble);
    }
}

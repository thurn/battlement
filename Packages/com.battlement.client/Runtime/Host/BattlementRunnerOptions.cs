#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Battlement.Errors;
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
            IEnumerable<string>? customCommandTypes = null,
            IBattlementErrorSink? errorSink = null,
            IBattlementFailurePresenter? failurePresenter = null,
            bool suppressDevelopmentErrorDialogs = false
        )
        {
            Transport = Preconditions.CheckNotNull(transport, nameof(transport));
            AssetStorage = Preconditions.CheckNotNull(assetStorage, nameof(assetStorage));
            ProtocolCodec = Preconditions.CheckNotNull(protocolCodec, nameof(protocolCodec));
            Clock = clock ?? new UnityBattlementClock();
            Logger = logger ?? new BattlementUnityLogger();
            ErrorSink = errorSink ?? new BattlementFileErrorSink();
            FailurePresenter = failurePresenter;
            SuppressDevelopmentErrorDialogs = suppressDevelopmentErrorDialogs;
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

        public IBattlementErrorSink ErrorSink { get; }

        public IBattlementFailurePresenter? FailurePresenter { get; }

        /// <summary>Whether detailed runtime error dialogs are explicitly disabled.</summary>
        public bool SuppressDevelopmentErrorDialogs { get; }

        public bool UseInstantAnimations { get; }

        public IReadOnlyList<string> CustomCommandTypes { get; }
    }

    internal sealed class UnityBattlementClock : IBattlementClock
    {
        public TimeSpan Elapsed => TimeSpan.FromSeconds(Time.realtimeSinceStartupAsDouble);
    }
}

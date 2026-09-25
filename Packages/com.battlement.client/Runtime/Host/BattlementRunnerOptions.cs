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
            IBattlementClock? clock = null,
            IBattlementLogger? logger = null,
            bool useInstantAnimations = false,
            IEnumerable<string>? customCommandTypes = null,
            IBattlementErrorSink? errorSink = null,
            IBattlementFailurePresenter? failurePresenter = null,
            bool suppressDevelopmentErrorDialogs = false,
            IBattlementCaughtFailureReporter? caughtFailureReporter = null,
            Action<string>? openExternalUrl = null,
            IBattlementFlatBufferResponseViewSchema? flatBufferResponseSchema = null,
            IBattlementFlatBufferClientSchema? flatBufferClientSchema = null,
            IBattlementCoreMessageObserver? coreMessageObserver = null,
            Func<HostSettings>? readHostSettings = null
        )
        {
            ReadHostSettings = readHostSettings;
            OpenExternalUrl = openExternalUrl ?? Application.OpenURL;
            Transport = Preconditions.CheckNotNull(transport, nameof(transport));
            AssetStorage = Preconditions.CheckNotNull(assetStorage, nameof(assetStorage));
            Clock = clock ?? new UnityBattlementClock();
            Logger = logger ?? new BattlementUnityLogger();
            ErrorSink = errorSink ?? new BattlementFileErrorSink();
            FailurePresenter = failurePresenter;
            SuppressDevelopmentErrorDialogs = suppressDevelopmentErrorDialogs;
            UseInstantAnimations = useInstantAnimations;
            CustomCommandTypes = (customCommandTypes ?? Array.Empty<string>())
                .OrderBy(type => type, StringComparer.Ordinal)
                .ToArray();
            CaughtFailureReporter = caughtFailureReporter ?? new UnityCaughtFailureReporter();
            FlatBufferResponseSchema = flatBufferResponseSchema;
            FlatBufferClientSchema = flatBufferClientSchema;
            CoreMessageObserver = coreMessageObserver;
        }

        /// <summary>Dispatches an absolute URL to the platform external handler.</summary>
        public Action<string> OpenExternalUrl { get; }

        public Func<HostSettings>? ReadHostSettings { get; }

        public IBattlementTransport Transport { get; }

        public IBattlementAssetStorage AssetStorage { get; }

        public IBattlementClock Clock { get; }

        public IBattlementLogger Logger { get; }

        public IBattlementErrorSink ErrorSink { get; }

        public IBattlementFailurePresenter? FailurePresenter { get; }

        /// <summary>Whether detailed runtime error dialogs are explicitly disabled.</summary>
        public bool SuppressDevelopmentErrorDialogs { get; }

        public bool UseInstantAnimations { get; }

        public IReadOnlyList<string> CustomCommandTypes { get; }

        public IBattlementCaughtFailureReporter CaughtFailureReporter { get; }

        /// <summary>Generated response reader for a build containing custom commands.</summary>
        public IBattlementFlatBufferResponseViewSchema? FlatBufferResponseSchema { get; }

        /// <summary>Generated client writer for a build containing custom protocol types.</summary>
        public IBattlementFlatBufferClientSchema? FlatBufferClientSchema { get; }

        /// <summary>Optional diagnostics hook invoked before core message submission.</summary>
        public IBattlementCoreMessageObserver? CoreMessageObserver { get; }
    }

    internal sealed class UnityBattlementClock : IBattlementClock
    {
        public TimeSpan Elapsed => TimeSpan.FromSeconds(Time.realtimeSinceStartupAsDouble);
    }
}

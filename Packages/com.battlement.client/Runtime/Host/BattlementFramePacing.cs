#nullable enable

using System;
using System.Linq;
using UnityEngine;

namespace Battlement
{
    internal interface IFramePacingBackend
    {
        int SyncCount { get; set; }
        int TargetRate { get; set; }
    }

    /// <summary>Retains pacing intent independently of display rollback.</summary>
    internal sealed class BattlementFramePacing
    {
        private readonly IFramePacingBackend backend;
        private FramePacing? requested;
        private CommandId requestId;

        public BattlementFramePacing(IFramePacingBackend? backend = null) =>
            this.backend = backend ?? new UnityBackend();

        public void Reset() => requested = null;

        public void Apply(CommandId id, FramePacing value, HostSettings host)
        {
            if (value.MaximumFrameRate is < 1 or > 1000)
                throw new BattlementCommandException(
                    CoreErrorCode.InvalidProperty,
                    "Invalid frame-rate ceiling."
                );
            requestId = id;
            requested = value;
            HostSettings result = Reconcile(host);
            if (result.LastResult?.Error is string error)
                throw new BattlementCommandException(CoreErrorCode.InvalidProperty, error);
        }

        public HostSettings Reconcile(HostSettings host)
        {
            if (requested is null)
                return host;
            string? error = null;
            try
            {
                if (host.FramePacing != SettingAvailability.Available || host.FrameRates.Count == 0)
                    throw new InvalidOperationException("Frame pacing is unavailable.");
                uint rate = host
                    .FrameRates.Where(rate => rate <= requested.MaximumFrameRate)
                    .DefaultIfEmpty(host.FrameRates.Min())
                    .Max();
                bool sync = host.VsyncAvailable && requested.Vsync;
                int target = sync ? -1 : checked((int)rate);
                if (backend.SyncCount != (sync ? 1 : 0))
                    backend.SyncCount = sync ? 1 : 0;
                if (backend.TargetRate != target)
                    backend.TargetRate = target;
                if (backend.SyncCount != (sync ? 1 : 0) || backend.TargetRate != target)
                    throw new InvalidOperationException(
                        "The host did not apply the requested pacing policy."
                    );
            }
            catch (Exception exception)
            {
                error = exception.Message;
            }
            try
            {
                host = host with
                {
                    AppliedVsync = host.VsyncAvailable && backend.SyncCount > 0,
                    AppliedFrameRate = backend.TargetRate,
                };
            }
            catch (Exception exception)
            {
                error ??= exception.Message;
                host = host with { FramePacing = SettingAvailability.Failed };
            }
            return host with { LastResult = new HostSettingsResult(requestId, error) };
        }

        public static uint[] Rates(HostPlatform platform, RefreshRate refresh)
        {
            if (platform == HostPlatform.Web)
                return new uint[] { 30, 60, 120, 144, 240 };
            if (platform is HostPlatform.MacOs or HostPlatform.Windows)
                return new uint[] { 60, 120, 144, 240 };
            if (platform != HostPlatform.Ios || refresh.denominator == 0 || refresh.numerator == 0)
                return Array.Empty<uint>();
            // Unity's reported iOS refresh already includes the build's ProMotion setting.
            double maximum = (double)refresh.numerator / refresh.denominator;
            return new uint[] { 30, 60, 120 }
                .Where(rate =>
                    maximum >= rate && Math.Abs(maximum / rate - Math.Round(maximum / rate)) < 0.01
                )
                .ToArray();
        }

        private sealed class UnityBackend : IFramePacingBackend
        {
            public int SyncCount
            {
                get => QualitySettings.vSyncCount;
                set => QualitySettings.vSyncCount = value;
            }
            public int TargetRate
            {
                get => Application.targetFrameRate;
                set => Application.targetFrameRate = value;
            }
        }
    }
}

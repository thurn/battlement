#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Battlement
{
    public enum HostPlatform : byte
    {
        Unavailable,
        MacOs,
        Windows,
        Web,
        Ios,
    }

    public enum SettingAvailability : byte
    {
        Unavailable,
        Available,
        Failed,
    }

    public enum DisplayMode : byte
    {
        Windowed,
        Borderless,
        Fullscreen,
    }

    /// <summary>Pixel dimensions and refresh ratio; zero numerator means unknown.</summary>
    public sealed record DisplayResolution(
        uint Width,
        uint Height,
        uint RefreshNumerator,
        uint RefreshDenominator
    );

    public sealed record DisplayConfiguration(DisplayMode Mode, DisplayResolution Resolution);

    public sealed record HostSettingsResult(CommandId RequestId, string? Error = null);

    /// <summary>Host observations independent of saved player preferences.</summary>
    public sealed record HostSettings
    {
        public HostPlatform Platform { get; init; }
        public SettingAvailability Display { get; init; }
        public IReadOnlyList<DisplayMode> DisplayModes { get; init; } = Array.Empty<DisplayMode>();
        public IReadOnlyList<DisplayResolution> Resolutions { get; init; } =
            Array.Empty<DisplayResolution>();
        public DisplayConfiguration? AppliedDisplay { get; init; }
        public SettingAvailability FramePacing { get; init; }
        public IReadOnlyList<uint> FrameRates { get; init; } = Array.Empty<uint>();
        public int AppliedFrameRate { get; init; } = -1;
        public bool VsyncAvailable { get; init; }
        public bool AppliedVsync { get; init; }
        public bool KeyboardConnected { get; init; }
        public uint ControllerCount { get; init; }
        public SettingAvailability Diagnostics { get; init; }
        public bool DiagnosticsConfigured { get; init; }
        public string? ObservationError { get; init; }
        public HostSettingsResult? LastResult { get; init; }

        public bool Equivalent(HostSettings other)
        {
            if (
                this with
                {
                    DisplayModes = other.DisplayModes,
                    Resolutions = other.Resolutions,
                    FrameRates = other.FrameRates,
                } != other
            )
                return false;
            return DisplayModes.SequenceEqual(other.DisplayModes)
                && Resolutions.SequenceEqual(other.Resolutions)
                && FrameRates.SequenceEqual(other.FrameRates);
        }
    }
}

#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.InputSystem;

namespace Battlement
{
    /// <summary>Reads host state without applying player preferences.</summary>
    internal static class BattlementHostSettings
    {
        private static bool browserKeyboardObserved;

        public static void ObserveInput()
        {
            if (Application.platform == RuntimePlatform.WebGLPlayer)
                browserKeyboardObserved |= InputSystem
                    .devices.OfType<Keyboard>()
                    .Any(keyboard => keyboard.anyKey.wasPressedThisFrame);
        }

        public static void Report(IBattlementLogger logger, HostSettings value) =>
            logger.Log(
                new BattlementLogRecord(
                    BattlementLogSeverity.Information,
                    "battlement.host.settings",
                    $"platform={value.Platform} keyboard={value.KeyboardConnected} "
                        + $"controllers={value.ControllerCount} display={value.Display} "
                        + $"pacing={value.FramePacing} diagnostics={value.Diagnostics} "
                        + $"diagnostics_configured={value.DiagnosticsConfigured} "
                        + $"capture_exceptions={value.CaptureExceptions} "
                        + $"performance_reporting={value.PerformanceReporting} "
                        + $"diagnostics_error={value.DiagnosticsError}"
                )
            );

        public static HostSettings Read(
            IReadOnlyList<string> modules,
            Func<DiagnosticsObservation>? readReporting = null
        )
        {
            HostPlatform platform = Platform(Application.platform);
            bool desktop = platform is HostPlatform.MacOs or HostPlatform.Windows;
            bool supported = platform != HostPlatform.Unavailable;
            bool diagnostics =
                modules.Contains("battlement.diagnostics")
                && platform is HostPlatform.MacOs or HostPlatform.Windows or HostPlatform.Ios;
            var result = new HostSettings
            {
                Platform = platform,
                KeyboardConnected =
                    platform == HostPlatform.Web
                        ? browserKeyboardObserved
                        : InputSystem.devices.OfType<Keyboard>().Any(keyboard => keyboard.enabled),
                ControllerCount = checked((uint)Gamepad.all.Count(gamepad => gamepad.enabled)),
                Diagnostics = diagnostics
                    ? SettingAvailability.Available
                    : SettingAvailability.Unavailable,
                DiagnosticsConfigured =
                    diagnostics && !string.IsNullOrEmpty(Application.cloudProjectId),
            };
            if (diagnostics && readReporting is not null)
            {
                DiagnosticsObservation observation = readReporting();
                result = result with
                {
                    CaptureExceptions = observation.CaptureExceptions,
                    PerformanceReporting = observation.PerformanceReporting,
                    DiagnosticsError = observation.Error,
                };
            }
            if (!supported)
                return result;
            try
            {
                RefreshRate refresh = Screen.currentResolution.refreshRateRatio;
                var applied = new DisplayResolution(
                    checked((uint)Math.Max(1, Screen.width)),
                    checked((uint)Math.Max(1, Screen.height)),
                    refresh.numerator,
                    Math.Max(1U, refresh.denominator)
                );
                var resolutions = desktop
                    ? Screen
                        .resolutions.Select(resolution => new DisplayResolution(
                            checked((uint)resolution.width),
                            checked((uint)resolution.height),
                            resolution.refreshRateRatio.numerator,
                            Math.Max(1U, resolution.refreshRateRatio.denominator)
                        ))
                        .Append(applied)
                        .Distinct()
                        .OrderBy(resolution => resolution.Width)
                        .ThenBy(resolution => resolution.Height)
                        .ThenBy(resolution =>
                            (double)resolution.RefreshNumerator / resolution.RefreshDenominator
                        )
                        .ToArray()
                    : Array.Empty<DisplayResolution>();
                uint[] rates = BattlementFramePacing.Rates(platform, refresh);
                return result with
                {
                    Display = desktop
                        ? SettingAvailability.Available
                        : SettingAvailability.Unavailable,
                    DisplayModes = Modes(platform),
                    Resolutions = resolutions,
                    AppliedDisplay = new DisplayConfiguration(Mode(Screen.fullScreenMode), applied),
                    WindowBounds = desktop ? BattlementDisplayBackend.ReadWindowBounds() : null,
                    FramePacing =
                        rates.Length > 0
                            ? SettingAvailability.Available
                            : SettingAvailability.Unavailable,
                    FrameRates = rates,
                    AppliedFrameRate = Application.targetFrameRate,
                    VsyncAvailable = desktop,
                    AppliedVsync = desktop && QualitySettings.vSyncCount > 0,
                };
            }
            catch (Exception exception)
            {
                return result with
                {
                    Display = desktop
                        ? SettingAvailability.Failed
                        : SettingAvailability.Unavailable,
                    FramePacing = SettingAvailability.Failed,
                    ObservationError = exception.Message,
                };
            }
        }

        private static HostPlatform Platform(RuntimePlatform platform) =>
            platform switch
            {
                RuntimePlatform.OSXPlayer or RuntimePlatform.OSXEditor => HostPlatform.MacOs,
                RuntimePlatform.WindowsPlayer or RuntimePlatform.WindowsEditor =>
                    HostPlatform.Windows,
                RuntimePlatform.WebGLPlayer => HostPlatform.Web,
                RuntimePlatform.IPhonePlayer => HostPlatform.Ios,
                _ => HostPlatform.Unavailable,
            };

        private static DisplayMode Mode(FullScreenMode mode) =>
            mode switch
            {
                FullScreenMode.Windowed => DisplayMode.Windowed,
                FullScreenMode.ExclusiveFullScreen => DisplayMode.Fullscreen,
                _ => DisplayMode.Borderless,
            };

        private static DisplayMode[] Modes(HostPlatform platform) =>
            platform switch
            {
                HostPlatform.Windows => new[]
                {
                    DisplayMode.Borderless,
                    DisplayMode.Fullscreen,
                    DisplayMode.Windowed,
                },
                HostPlatform.MacOs => new[] { DisplayMode.Borderless, DisplayMode.Windowed },
                _ => Array.Empty<DisplayMode>(),
            };
    }
}

#nullable enable

using System;
using System.IO;
using System.Linq;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    internal static class BattlementHostSettingsWriter
    {
        public static Offset<Wire.HostSettings> Write(FlatBufferBuilder builder, HostSettings value)
        {
            Validate(value);
            VectorOffset modes = Wire.HostSettings.CreateDisplayModesVector(
                builder,
                value.DisplayModes.Select(mode => (Wire.DisplayMode)mode).ToArray()
            );
            Wire.HostSettings.StartResolutionsVector(builder, value.Resolutions.Count);
            for (int index = value.Resolutions.Count - 1; index >= 0; index--)
                Resolution(builder, value.Resolutions[index]);
            VectorOffset resolutions = builder.EndVector();
            VectorOffset rates = Wire.HostSettings.CreateFrameRatesVector(
                builder,
                value.FrameRates.ToArray()
            );
            Offset<Wire.DisplayConfiguration> applied = default;
            if (value.AppliedDisplay is DisplayConfiguration display)
            {
                Wire.DisplayConfiguration.StartDisplayConfiguration(builder);
                Wire.DisplayConfiguration.AddMode(builder, (Wire.DisplayMode)display.Mode);
                Wire.DisplayConfiguration.AddResolution(
                    builder,
                    Resolution(builder, display.Resolution)
                );
                applied = Wire.DisplayConfiguration.EndDisplayConfiguration(builder);
            }
            Offset<Wire.HostSettingsResult> result = default;
            if (value.LastResult is HostSettingsResult last)
            {
                StringOffset error = last.Error is null
                    ? default
                    : builder.CreateString(last.Error);
                Wire.HostSettingsResult.StartHostSettingsResult(builder);
                Wire.HostSettingsResult.AddError(builder, error);
                Wire.HostSettingsResult.AddRequestId(
                    builder,
                    BattlementFlatBufferWriter.WriteUuid(builder, last.RequestId.Value)
                );
                result = Wire.HostSettingsResult.EndHostSettingsResult(builder);
            }
            StringOffset observationError = value.ObservationError is null
                ? default
                : builder.CreateString(value.ObservationError);
            Offset<Wire.DisplayPreview> preview = default;
            if (value.DisplayPreview is DisplayPreview active)
            {
                Wire.DisplayPreview.StartDisplayPreview(builder);
                Wire.DisplayPreview.AddState(builder, (Wire.DisplayPreviewState)active.State);
                Wire.DisplayPreview.AddRemainingSeconds(builder, active.RemainingSeconds);
                Wire.DisplayPreview.AddRequestId(
                    builder,
                    BattlementFlatBufferWriter.WriteUuid(builder, active.RequestId.Value)
                );
                preview = Wire.DisplayPreview.EndDisplayPreview(builder);
            }
            StringOffset diagnosticsError = value.DiagnosticsError is null
                ? default
                : builder.CreateString(value.DiagnosticsError);
            Wire.HostSettings.StartHostSettings(builder);
            Wire.HostSettings.AddPlatform(builder, (Wire.HostPlatform)value.Platform);
            Wire.HostSettings.AddDisplay(builder, (Wire.SettingAvailability)value.Display);
            Wire.HostSettings.AddDisplayModes(builder, modes);
            Wire.HostSettings.AddResolutions(builder, resolutions);
            Wire.HostSettings.AddAppliedDisplay(builder, applied);
            Wire.HostSettings.AddFramePacing(builder, (Wire.SettingAvailability)value.FramePacing);
            Wire.HostSettings.AddFrameRates(builder, rates);
            Wire.HostSettings.AddAppliedFrameRate(builder, value.AppliedFrameRate);
            Wire.HostSettings.AddVsyncAvailable(builder, value.VsyncAvailable);
            Wire.HostSettings.AddAppliedVsync(builder, value.AppliedVsync);
            Wire.HostSettings.AddKeyboardConnected(builder, value.KeyboardConnected);
            Wire.HostSettings.AddControllerCount(builder, value.ControllerCount);
            Wire.HostSettings.AddDiagnostics(builder, (Wire.SettingAvailability)value.Diagnostics);
            Wire.HostSettings.AddDiagnosticsConfigured(builder, value.DiagnosticsConfigured);
            Wire.HostSettings.AddCaptureExceptions(builder, value.CaptureExceptions);
            Wire.HostSettings.AddPerformanceReporting(builder, value.PerformanceReporting);
            Wire.HostSettings.AddDiagnosticsError(builder, diagnosticsError);
            Wire.HostSettings.AddObservationError(builder, observationError);
            Wire.HostSettings.AddLastResult(builder, result);
            Wire.HostSettings.AddDisplayPreview(builder, preview);
            if (value.WindowBounds is ScreenSize bounds)
                Wire.HostSettings.AddWindowBounds(
                    builder,
                    Wire.ScreenSize.CreateScreenSize(builder, bounds.Width, bounds.Height)
                );
            return Wire.HostSettings.EndHostSettings(builder);
        }

        private static Offset<Wire.DisplayResolution> Resolution(
            FlatBufferBuilder builder,
            DisplayResolution value
        ) =>
            Wire.DisplayResolution.CreateDisplayResolution(
                builder,
                value.Width,
                value.Height,
                value.RefreshNumerator,
                value.RefreshDenominator
            );

        private static void Validate(HostSettings value)
        {
            if (
                value.WindowBounds is ScreenSize bounds
                && (bounds.Width == 0 || bounds.Height == 0)
            )
                throw new InvalidDataException("Invalid window bounds.");
            if (value.DisplayPreview is DisplayPreview preview)
            {
                if (preview.RequestId.Value == Guid.Empty || preview.RemainingSeconds > 15)
                    throw new InvalidDataException("Invalid display preview.");
                if ((byte)preview.State > (byte)DisplayPreviewState.Reverting)
                    throw new InvalidDataException("Unknown display preview state.");
            }
            if ((byte)value.Platform > (byte)HostPlatform.Ios)
                throw new InvalidDataException("Unknown host platform.");
            foreach (
                SettingAvailability state in new[]
                {
                    value.Display,
                    value.FramePacing,
                    value.Diagnostics,
                }
            )
                if ((byte)state > (byte)SettingAvailability.Failed)
                    throw new InvalidDataException("Unknown host setting availability.");
            if (value.DisplayModes.Count > 3 || value.Resolutions.Count > 4096)
                throw new InvalidDataException("Too many display choices.");
            if (value.DisplayModes.Distinct().Count() != value.DisplayModes.Count)
                throw new InvalidDataException("Duplicate display modes.");
            foreach (DisplayMode mode in value.DisplayModes)
                ValidateMode(mode);
            foreach (DisplayResolution resolution in value.Resolutions)
                ValidateResolution(resolution);
            if (value.AppliedDisplay is DisplayConfiguration applied)
            {
                ValidateMode(applied.Mode);
                ValidateResolution(applied.Resolution);
            }
            if (value.FrameRates.Count > 256)
                throw new InvalidDataException("Too many frame rate choices.");
            uint previous = 0;
            foreach (uint rate in value.FrameRates)
            {
                if (rate <= previous)
                    throw new InvalidDataException(
                        "Frame rate choices must be positive, sorted and unique."
                    );
                previous = rate;
            }
            if (value.AppliedFrameRate != -1 && value.AppliedFrameRate <= 0)
                throw new InvalidDataException("Invalid applied frame rate.");
            if (
                value.LastResult is HostSettingsResult result
                && result.RequestId.Value == Guid.Empty
            )
                throw new InvalidDataException("Host setting request ID must be nonzero.");
        }

        private static void ValidateMode(DisplayMode value)
        {
            if ((byte)value > (byte)DisplayMode.Fullscreen)
                throw new InvalidDataException("Unknown display mode.");
        }

        private static void ValidateResolution(DisplayResolution value)
        {
            if (value.Width == 0 || value.Height == 0)
                throw new InvalidDataException("Invalid display dimensions.");
            if (value.RefreshDenominator == 0)
                throw new InvalidDataException("Invalid display refresh ratio.");
        }
    }
}

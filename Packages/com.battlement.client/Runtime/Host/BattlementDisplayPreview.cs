#nullable enable

using System;
using System.Linq;

namespace Battlement
{
    /// <summary>Owns display preview, durable confirmation, and real-time rollback.</summary>
    internal sealed class BattlementDisplayPreview
    {
        private sealed record Pending(
            CommandId Id,
            DisplayConfiguration Target,
            DisplayConfiguration Prior,
            double Deadline,
            bool Observed = false
        );

        private sealed record Restoring(
            CommandId? Owner,
            DisplayConfiguration Target,
            double Deadline,
            string? Error,
            bool Fallback = false
        );

        private readonly IDisplayBackend backend;
        private readonly IDisplayRecoveryStore store;
        private readonly Func<double> clock;
        private readonly System.Action<HostSettingsResult>? report;
        private bool initialized;
        private Pending? pending;
        private Restoring? restoring;
        private HostSettings? lastHost;

        public BattlementDisplayPreview(
            IDisplayBackend backend,
            IDisplayRecoveryStore store,
            Func<double> clock,
            System.Action<HostSettingsResult>? report = null
        )
        {
            this.backend = backend;
            this.store = store;
            this.clock = clock;
            this.report = report;
        }

        public HostSettingsResult? Result { get; private set; }
        public string? RecoveryError { get; private set; }
        public bool NeedsObservation => pending is not null || restoring is not null;

        public DisplayPreview? Observation =>
            pending is not null
                ? new DisplayPreview(
                    pending.Id,
                    pending.Observed
                        ? DisplayPreviewState.Confirmable
                        : DisplayPreviewState.Applying,
                    checked((uint)Math.Max(0, Math.Ceiling(pending.Deadline - clock())))
                )
            : restoring?.Owner is CommandId owner
                ? new DisplayPreview(owner, DisplayPreviewState.Reverting, 0)
            : null;

        public void Begin(
            CommandId id,
            DisplayConfiguration target,
            HostSettings host,
            bool focused = true
        )
        {
            Observe(host, focused);
            if (!focused)
            {
                Report(id, "Display preview requires application focus.");
                return;
            }
            if (restoring is not null)
            {
                Report(id, "The previous display is still being restored.");
                return;
            }
            if (!Supported(target, host) || host.AppliedDisplay is null)
            {
                Report(id, "The requested display configuration is unavailable.");
                return;
            }
            DisplayConfiguration prior = pending?.Prior ?? host.AppliedDisplay;
            try
            {
                store.Save(new DisplayRecoveryRecord(prior, true));
            }
            catch (Exception exception)
            {
                Report(id, "Could not save display recovery: " + exception.Message);
                return;
            }
            RecoveryError = null;
            pending = new Pending(id, target, prior, clock() + 15);
            try
            {
                backend.Apply(target);
            }
            catch (Exception exception)
            {
                Revert(id, "Could not apply the display: " + exception.Message);
            }
        }

        public void Confirm(CommandId commandId, CommandId previewId, HostSettings host)
        {
            Observe(host, true);
            if (pending?.Id != previewId || pending?.Observed != true)
                return;
            try
            {
                store.Save(new DisplayRecoveryRecord(host.AppliedDisplay!, false));
                pending = null;
                RecoveryError = null;
                Report(commandId, null);
            }
            catch (Exception exception)
            {
                Revert(commandId, "Could not keep the display: " + exception.Message);
            }
        }

        public void Cancel(CommandId commandId, CommandId previewId)
        {
            if (pending?.Id == previewId)
                Revert(commandId, null);
        }

        public void LoseOwner()
        {
            if (pending is not null)
                Revert(pending.Id, "The display preview lost its owner.");
        }

        public void Observe(HostSettings host, bool focused)
        {
            if (host.Display == SettingAvailability.Available)
                lastHost = host;
            if (!initialized)
                Initialize(host);
            if (restoring is not null)
            {
                ObserveRestore(host);
                return;
            }
            if (pending is null)
                return;
            if (!focused || clock() >= pending.Deadline)
            {
                Revert(pending.Id, focused ? "Display confirmation timed out." : "Focus was lost.");
                return;
            }
            if (!Supported(pending.Target, host))
            {
                Revert(pending.Id, "The requested display is no longer available.");
                return;
            }
            bool matches = Matches(host.AppliedDisplay, pending.Target);
            if (matches && !pending.Observed)
            {
                pending = pending with { Observed = true };
                Report(pending.Id, null);
            }
            else if (pending.Observed && !matches)
            {
                Revert(pending.Id, "The display changed before confirmation.");
            }
        }

        private void Initialize(HostSettings host)
        {
            if (host.Platform is not (HostPlatform.MacOs or HostPlatform.Windows))
                return;
            if (host.Display != SettingAvailability.Available || host.AppliedDisplay is null)
                return;
            initialized = true;
            try
            {
                DisplayRecoveryRecord? record = store.Load();
                if (record is null)
                    return;
                DisplayConfiguration target = Recoverable(record.Confirmed, host)
                    ? record.Confirmed
                    : backend.SafeWindow;
                if (Matches(host.AppliedDisplay, target) && !record.PreviewActive)
                    return;
                StartRestore(null, target, null);
            }
            catch (Exception exception)
            {
                string error = "Display recovery failed: " + exception.Message;
                try
                {
                    StartRestore(null, backend.SafeWindow, error);
                }
                catch (Exception fallbackFailure)
                {
                    RecoveryError = Join(error, fallbackFailure.Message);
                }
            }
        }

        private void Revert(CommandId id, string? error)
        {
            if (pending is null)
                return;
            DisplayConfiguration prior = pending.Prior;
            pending = null;
            try
            {
                if (lastHost is null || !Recoverable(prior, lastHost))
                    prior = backend.SafeWindow;
            }
            catch (Exception exception)
            {
                error = Join(error, "Could not inspect the current monitor: " + exception.Message);
            }
            // Reassert the prior record if a confirmation failed after replacing its file.
            try
            {
                store.Save(new DisplayRecoveryRecord(prior, true));
            }
            catch (Exception exception)
            {
                error = Join(error, "Could not retain recovery: " + exception.Message);
            }
            StartRestore(id, prior, error);
        }

        private void StartRestore(CommandId? owner, DisplayConfiguration target, string? error)
        {
            restoring = new Restoring(owner, target, clock() + 3, error);
            if (owner is CommandId id)
                Report(id, error);
            try
            {
                backend.Apply(target);
            }
            catch (Exception exception)
            {
                ApplyFallback("Could not restore the display: " + exception.Message);
            }
        }

        private void ObserveRestore(HostSettings host)
        {
            Restoring operation = restoring!;
            if (Matches(host.AppliedDisplay, operation.Target))
            {
                string? error = operation.Error;
                try
                {
                    store.Save(new DisplayRecoveryRecord(host.AppliedDisplay!, false));
                }
                catch (Exception exception)
                {
                    error = Join(error, "Could not finish recovery: " + exception.Message);
                }
                restoring = null;
                RecoveryError = error;
                if (operation.Owner is CommandId id)
                    Report(id, error);
            }
            else if (clock() >= operation.Deadline)
            {
                ApplyFallback("The restored display did not match host readback.");
            }
        }

        private void ApplyFallback(string error)
        {
            Restoring operation = restoring!;
            error = Join(operation.Error, error);
            if (operation.Fallback)
            {
                RecoveryError = error;
                restoring = null;
                if (operation.Owner is CommandId id)
                    Report(id, error);
                return;
            }
            try
            {
                DisplayConfiguration safe = backend.SafeWindow;
                restoring = operation with
                {
                    Target = safe,
                    Deadline = clock() + 3,
                    Error = error,
                    Fallback = true,
                };
                backend.Apply(safe);
            }
            catch (Exception exception)
            {
                RecoveryError = Join(error, exception.Message);
                restoring = null;
                if (operation.Owner is CommandId id)
                    Report(id, RecoveryError);
            }
        }

        private bool Recoverable(DisplayConfiguration value, HostSettings host)
        {
            if (!Valid(value))
                return false;
            return value.Mode == DisplayMode.Windowed
                ? backend.WindowFits(value.Resolution)
                : Supported(value, host);
        }

        private bool Supported(DisplayConfiguration value, HostSettings host)
        {
            if (host.Platform is not (HostPlatform.MacOs or HostPlatform.Windows))
                return false;
            if (value.Mode == DisplayMode.Fullscreen && host.Platform != HostPlatform.Windows)
                return false;
            if (!Valid(value) || host.Display != SettingAvailability.Available)
                return false;
            if (!host.DisplayModes.Contains(value.Mode))
                return false;
            if (value.Mode == DisplayMode.Windowed && !backend.WindowFits(value.Resolution))
                return false;
            return host.Resolutions.Any(resolution =>
                Matches(new DisplayConfiguration(value.Mode, resolution), value)
            );
        }

        private static bool Valid(DisplayConfiguration value) =>
            Enum.IsDefined(typeof(DisplayMode), value.Mode)
            && value.Resolution
                is { Width: > 0 and <= 32768, Height: > 0 and <= 32768, RefreshDenominator: > 0 };

        private static bool Matches(DisplayConfiguration? actual, DisplayConfiguration requested)
        {
            if (actual is null || actual.Mode != requested.Mode)
                return false;
            DisplayResolution a = actual.Resolution;
            DisplayResolution b = requested.Resolution;
            if (a.Width != b.Width || a.Height != b.Height)
                return false;
            if (requested.Mode != DisplayMode.Fullscreen || b.RefreshNumerator == 0)
                return true;
            return (ulong)a.RefreshNumerator * b.RefreshDenominator
                == (ulong)b.RefreshNumerator * a.RefreshDenominator;
        }

        private void Report(CommandId id, string? error)
        {
            Result = new HostSettingsResult(id, error);
            report?.Invoke(Result);
        }

        private static string Join(string? first, string second) =>
            first is null ? second : first + " " + second;
    }
}

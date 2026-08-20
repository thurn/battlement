#nullable enable

using System;

namespace Masonry
{
    internal enum MasonrySessionPhase
    {
        Stopped,
        AwaitingSnapshot,
        ApplyingSnapshot,
        Running,
    }

    /// <summary>Owns the valid lifecycle states for one runner session.</summary>
    internal sealed class MasonrySessionState
    {
        private string? pendingConnectionEvent;
        private string? pendingConnectionMessage;

        public MasonrySessionPhase Phase { get; private set; }

        public SessionId? LastSession { get; private set; }

        public TimeSpan? PreviousStepTime { get; set; }

        public bool InputDisabled { get; private set; } = true;

        public bool IsInputAvailable => Phase == MasonrySessionPhase.Running && !InputDisabled;

        public void BeginConnection(TimeSpan now, bool reconnecting)
        {
            if (Phase != MasonrySessionPhase.Stopped)
            {
                throw new InvalidOperationException("The runner is already connected.");
            }

            Phase = MasonrySessionPhase.AwaitingSnapshot;
            InputDisabled = true;
            PreviousStepTime = now;
            pendingConnectionEvent = reconnecting
                ? "masonry.host.reconnected"
                : "masonry.host.connected";
            pendingConnectionMessage = reconnecting ? "Host reconnected." : "Host connected.";
        }

        public bool BeginSnapshot(SessionId session)
        {
            if (
                Phase != MasonrySessionPhase.AwaitingSnapshot
                && Phase != MasonrySessionPhase.Running
            )
            {
                throw new InvalidOperationException(
                    "Snapshots may only begin for an active session."
                );
            }

            bool isInitial = Phase == MasonrySessionPhase.AwaitingSnapshot;
            LastSession = session;
            Phase = MasonrySessionPhase.ApplyingSnapshot;
            InputDisabled = true;
            return isInitial;
        }

        public void CompleteSnapshot(bool inputDisabled)
        {
            if (Phase != MasonrySessionPhase.ApplyingSnapshot)
            {
                throw new InvalidOperationException("No snapshot replacement is active.");
            }

            InputDisabled = inputDisabled;
            Phase = MasonrySessionPhase.Running;
        }

        public void SetInputEnabled(bool isEnabled)
        {
            if (Phase != MasonrySessionPhase.Running)
            {
                throw new InvalidOperationException("Input may only change in a running session.");
            }

            InputDisabled = !isEnabled;
        }

        public (string EventName, string Message)? TakeConnectionLog()
        {
            if (pendingConnectionEvent is null || pendingConnectionMessage is null)
            {
                return null;
            }

            var result = (pendingConnectionEvent, pendingConnectionMessage);
            pendingConnectionEvent = null;
            pendingConnectionMessage = null;
            return result;
        }

        public void Stop()
        {
            Phase = MasonrySessionPhase.Stopped;
            InputDisabled = true;
            PreviousStepTime = null;
            pendingConnectionEvent = null;
            pendingConnectionMessage = null;
        }
    }
}

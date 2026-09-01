#nullable enable

using System;

namespace Battlement
{
    internal enum BattlementSessionPhase
    {
        Stopped,
        AwaitingSnapshot,
        ApplyingSnapshot,
        Running,
    }

    /// <summary>Owns the valid lifecycle states for one runner session.</summary>
    internal sealed class BattlementSessionState
    {
        private string? pendingConnectionEvent;
        private string? pendingConnectionMessage;

        public BattlementSessionPhase Phase { get; private set; }

        public SessionId? LastSession { get; private set; }

        public TimeSpan? PreviousStepTime { get; set; }

        public bool InputDisabled { get; private set; } = true;

        public bool IsReconnecting { get; private set; }

        public bool IsInputAvailable => Phase == BattlementSessionPhase.Running && !InputDisabled;

        public void BeginConnection(TimeSpan now, bool reconnecting)
        {
            if (Phase != BattlementSessionPhase.Stopped)
            {
                throw new InvalidOperationException("The runner is already connected.");
            }

            Phase = BattlementSessionPhase.AwaitingSnapshot;
            InputDisabled = true;
            IsReconnecting = reconnecting;
            PreviousStepTime = now;
            pendingConnectionEvent = reconnecting
                ? "battlement.host.reconnected"
                : "battlement.host.connected";
            pendingConnectionMessage = reconnecting ? "Host reconnected." : "Host connected.";
        }

        public bool BeginSnapshot(SessionId session)
        {
            if (
                Phase != BattlementSessionPhase.AwaitingSnapshot
                && Phase != BattlementSessionPhase.Running
            )
            {
                throw new InvalidOperationException(
                    "Snapshots may only begin for an active session."
                );
            }

            bool isInitial = Phase == BattlementSessionPhase.AwaitingSnapshot;
            LastSession = session;
            Phase = BattlementSessionPhase.ApplyingSnapshot;
            InputDisabled = true;
            return isInitial;
        }

        public void CompleteSnapshot(bool inputDisabled)
        {
            if (Phase != BattlementSessionPhase.ApplyingSnapshot)
            {
                throw new InvalidOperationException("No snapshot replacement is active.");
            }

            InputDisabled = inputDisabled;
            IsReconnecting = false;
            Phase = BattlementSessionPhase.Running;
        }

        public void SetInputEnabled(bool isEnabled)
        {
            if (Phase != BattlementSessionPhase.Running)
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
            Phase = BattlementSessionPhase.Stopped;
            InputDisabled = true;
            IsReconnecting = false;
            PreviousStepTime = null;
            pendingConnectionEvent = null;
            pendingConnectionMessage = null;
        }
    }
}

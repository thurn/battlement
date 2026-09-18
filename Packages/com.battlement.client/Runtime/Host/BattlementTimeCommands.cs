#nullable enable

using System;

namespace Battlement
{
    internal static class BattlementTimeCommands
    {
        public static IBattlementCommandOperation? Wait(
            BattlementDirectWait command,
            TimeSpan now,
            bool completeImmediately = false
        )
        {
            TimeSpan duration = BattlementProtocolLimits.RequireDuration(
                TimeSpan.FromMilliseconds(command.DurationMilliseconds),
                "A wait duration",
                allowZero: false
            );
            return completeImmediately ? null : new WaitOperation(now + duration);
        }

        private sealed class WaitOperation
            : IBattlementCommandOperation,
                IBattlementPausableCommandOperation
        {
            private TimeSpan completion;
            private bool isCancelled;
            private TimeSpan? pausedAt;

            public WaitOperation(TimeSpan completion) => this.completion = completion;

            public bool IsInfinite => false;

            public bool IsComplete(TimeSpan now) =>
                isCancelled || (!pausedAt.HasValue && now >= completion);

            public void Cancel() => isCancelled = true;

            public void Pause(TimeSpan now) => pausedAt ??= now;

            public void Resume(TimeSpan now)
            {
                if (pausedAt is not TimeSpan started)
                    return;
                completion += now - started;
                pausedAt = null;
            }
        }
    }
}

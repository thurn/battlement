#nullable enable

using System;

namespace Battlement
{
    internal static class BattlementTimeCommands
    {
        public static IBattlementCommandOperation Wait(CommandBody.Time.Wait command, TimeSpan now)
        {
            BattlementProtocolLimits.RequireDuration(
                command.Duration,
                "A wait duration",
                allowZero: false
            );

            return new WaitOperation(now + command.Duration);
        }

        private sealed class WaitOperation : IBattlementCommandOperation
        {
            private readonly TimeSpan completion;
            private bool isCancelled;

            public WaitOperation(TimeSpan completion) => this.completion = completion;

            public bool IsInfinite => false;

            public bool IsComplete(TimeSpan now) => isCancelled || now >= completion;

            public void Cancel() => isCancelled = true;
        }
    }
}

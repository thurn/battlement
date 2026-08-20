#nullable enable

using System;

namespace Masonry
{
    internal static class MasonryTimeCommands
    {
        public static IMasonryCommandOperation Wait(CommandBody.Time.Wait command, TimeSpan now)
        {
            MasonryProtocolLimits.RequireDuration(
                command.Duration,
                "A wait duration",
                allowZero: false
            );

            return new WaitOperation(now + command.Duration);
        }

        private sealed class WaitOperation : IMasonryCommandOperation
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

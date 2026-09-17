#nullable enable

using System;

namespace Battlement.UI
{
    internal interface IMotionPlaybackStatus
    {
        MotionPlaybackOutcome? Outcome { get; }
        Exception? Failure { get; }
    }

    /// <summary>Polls shared Motion playback through the host command scheduler.</summary>
    internal sealed class RunningMotion : IBattlementHeldCommandOperation
    {
        private readonly IMotionPlaybackStatus playback;
        private readonly System.Action refresh;
        private readonly System.Action cancel;
        private readonly Func<bool>? held;
        private readonly Func<bool> infinite;

        public RunningMotion(
            IMotionPlaybackStatus playback,
            Func<bool> infinite,
            System.Action refresh,
            System.Action cancel,
            Func<bool>? held = null
        ) =>
            (this.playback, this.infinite, this.refresh, this.cancel, this.held) = (
                playback,
                infinite,
                refresh,
                cancel,
                held
            );

        public bool IsInfinite => playback.Outcome is null && infinite();

        public bool IsHeld => playback.Outcome is null && held?.Invoke() == true;

        public bool IsComplete(TimeSpan now)
        {
            refresh();
            if (playback.Failure is Exception failure)
                throw new InvalidOperationException("Motion playback failed.", failure);
            return playback.Outcome is not null;
        }

        public void Cancel()
        {
            if (playback.Outcome is null)
                cancel();
        }
    }
}

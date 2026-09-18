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
    internal sealed class RunningMotion
        : IBattlementHeldCommandOperation,
            IBattlementPausableCommandOperation
    {
        private readonly IMotionPlaybackStatus playback;
        private readonly System.Action refresh;
        private readonly System.Action cancel;
        private readonly Func<bool>? held;
        private readonly Func<bool> infinite;
        private readonly System.Action? pause;
        private readonly System.Action? resume;

        public RunningMotion(
            IMotionPlaybackStatus playback,
            Func<bool> infinite,
            System.Action refresh,
            System.Action cancel,
            Func<bool>? held = null,
            System.Action? pause = null,
            System.Action? resume = null
        ) =>
            (
                this.playback,
                this.infinite,
                this.refresh,
                this.cancel,
                this.held,
                this.pause,
                this.resume
            ) = (playback, infinite, refresh, cancel, held, pause, resume);

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

        public void Pause(TimeSpan now) => pause?.Invoke();

        public void Resume(TimeSpan now) => resume?.Invoke();
    }

    /// <summary>Tracks current declarative Motion work for a host across retargeting.</summary>
    internal sealed class RunningDescriptorMotion
        : IBattlementCommandOperation,
            IBattlementPausableCommandOperation
    {
        private readonly Func<(bool Complete, bool Infinite)> status;
        private readonly System.Action cancel;
        private readonly System.Action? pause;
        private readonly System.Action? resume;
        private bool cancelled;

        public RunningDescriptorMotion(
            Func<(bool Complete, bool Infinite)> status,
            System.Action cancel,
            System.Action? pause = null,
            System.Action? resume = null
        ) => (this.status, this.cancel, this.pause, this.resume) = (status, cancel, pause, resume);

        public bool IsInfinite => !cancelled && status().Infinite;

        public bool IsComplete(TimeSpan now) => cancelled || status().Complete;

        public void Cancel()
        {
            if (cancelled)
                return;
            cancelled = true;
            cancel();
        }

        public void Pause(TimeSpan now) => pause?.Invoke();

        public void Resume(TimeSpan now) => resume?.Invoke();
    }
}

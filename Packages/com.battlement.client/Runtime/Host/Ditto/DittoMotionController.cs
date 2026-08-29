#nullable enable

using System;

namespace Battlement
{
    internal sealed record DittoCommittedFrame(
        ulong Index,
        TimeSpan Elapsed,
        bool HasPendingWork,
        bool HasInfiniteOperations,
        bool StateChanged,
        bool LayoutChanged,
        int QuietFrameCount
    )
    {
        public bool IsSettled => QuietFrameCount >= 2;
    }

    internal sealed record DittoWorkObservation(
        ulong StateVersion,
        ulong LayoutFingerprint,
        bool HasPendingWork,
        bool HasInfiniteOperations
    );

    internal sealed class DittoMotionController
    {
        private readonly BattlementRunner runner;
        private DittoWorkObservation? previous;
        private bool started;
        private ulong frameIndex;
        private int quietFrames;

        public DittoMotionController(BattlementRunner runner)
        {
            if (runner == null)
            {
                throw new ArgumentNullException(nameof(runner));
            }
            this.runner = runner;
        }

        public const string UncontrolledWorkDiagnostic =
            "Game-owned scripts and shaders remain on Unity's uncontrolled clock.";

        public DittoMotion Motion { get; private set; }

        public void Begin(DittoMotion motion)
        {
            if (!Enum.IsDefined(typeof(DittoMotion), motion))
            {
                throw new ArgumentOutOfRangeException(nameof(motion));
            }

            runner.BeginDittoMotion(motion);
            Motion = motion;
            previous = runner.ObserveDittoWork();
            frameIndex = 0;
            quietFrames = 0;
            started = true;
        }

        public TimeSpan PrepareFrame()
        {
            RequireStarted();
            return runner.PrepareDittoFrame();
        }

        public DittoCommittedFrame ObserveCommittedFrame()
        {
            RequireStarted();
            DittoWorkObservation current = runner.ObserveDittoWork();
            DittoWorkObservation prior = previous!;
            bool stateChanged = current.StateVersion != prior.StateVersion;
            bool layoutChanged = current.LayoutFingerprint != prior.LayoutFingerprint;
            if (current.HasPendingWork || stateChanged || layoutChanged)
            {
                quietFrames = 0;
            }
            else
            {
                quietFrames++;
            }

            previous = current;
            return new DittoCommittedFrame(
                ++frameIndex,
                runner.DittoElapsed,
                current.HasPendingWork,
                current.HasInfiniteOperations,
                stateChanged,
                layoutChanged,
                quietFrames
            );
        }

        public void PreserveExactWaitState()
        {
            RequireStarted();
            previous = runner.ObserveDittoWork();
            quietFrames = 0;
        }

        private void RequireStarted()
        {
            if (!started)
            {
                throw new InvalidOperationException("Ditto motion has not started.");
            }
        }
    }

    internal sealed class DittoMotionClock : IBattlementClock
    {
        private const long FramesPerSecond = 30;

        private readonly IBattlementClock source;
        private DittoMotion? motion;
        private TimeSpan motionEpoch;
        private TimeSpan sourceEpoch;
        private ulong controlledFrames;

        public DittoMotionClock(IBattlementClock source) =>
            this.source = source ?? throw new ArgumentNullException(nameof(source));

        public TimeSpan Elapsed =>
            motion == DittoMotion.Controlled
                ? motionEpoch
                    + TimeSpan.FromTicks(
                        checked((long)controlledFrames * TimeSpan.TicksPerSecond) / FramesPerSecond
                    )
            : motion is null ? source.Elapsed
            : motionEpoch + (source.Elapsed - sourceEpoch);

        public bool IsInstant => motion == DittoMotion.Instant;

        public bool IsControlled => motion == DittoMotion.Controlled;

        public void Begin(DittoMotion value)
        {
            TimeSpan elapsed = Elapsed;
            motion = value;
            motionEpoch = elapsed;
            sourceEpoch = source.Elapsed;
            controlledFrames = 0;
        }

        public TimeSpan PrepareFrame()
        {
            if (IsControlled)
            {
                controlledFrames++;
            }
            return Elapsed;
        }

        public void Reset()
        {
            motion = null;
            motionEpoch = default;
            sourceEpoch = default;
            controlledFrames = 0;
        }
    }
}

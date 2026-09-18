#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Battlement.UI
{
    internal sealed class BattlementImperativePlaybacks
    {
        private readonly Dictionary<Guid, ImperativePlayback> values = new();
        private readonly List<MotionPlaybackEvent> events = new();

        public void Clear()
        {
            foreach (ImperativePlayback playback in values.Values)
                playback.Outcome = MotionPlaybackOutcome.Cancelled;
            values.Clear();
            events.Clear();
        }

        public void Register(
            ObjectId playbackId,
            uint generation,
            List<MotionPlaybackAddress> addresses
        )
        {
            Finish(playbackId.Value, MotionPlaybackOutcome.Cancelled);
            values[playbackId.Value] = new ImperativePlayback(generation, addresses);
        }

        public bool TryGet(Guid playbackId, out ImperativePlayback playback) =>
            values.TryGetValue(playbackId, out playback!);

        public IReadOnlyList<Guid> Complete(
            IReadOnlyDictionary<Guid, DescriptorState> descriptors,
            IReadOnlyCollection<Guid>? deferred = null
        )
        {
            if (values.Count == 0)
                return Array.Empty<Guid>();
            var finished = new List<Guid>();
            foreach ((Guid id, ImperativePlayback playback) in values.ToArray())
            {
                if (deferred?.Contains(id) == true)
                    continue;
                if (playback.Addresses.Count == 0)
                    continue;
                bool complete = true;
                foreach (MotionPlaybackAddress address in playback.Addresses)
                {
                    if (
                        !descriptors.TryGetValue(address.DescriptorId.Value, out var descriptor)
                        || descriptor.FindSlot(address.Slot) is not SlotState slot
                        || slot.Definition.Generation != address.Generation
                    )
                    {
                        Finish(id, MotionPlaybackOutcome.Cancelled);
                        finished.Add(id);
                        complete = false;
                        break;
                    }
                    if (
                        slot.Outcome
                        is MotionPlaybackOutcome.Stopped
                            or MotionPlaybackOutcome.Cancelled
                    )
                    {
                        Finish(id, slot.Outcome.Value);
                        finished.Add(id);
                        complete = false;
                        break;
                    }
                    if (!slot.Terminal)
                        complete = false;
                }
                if (complete && values.ContainsKey(id))
                {
                    Finish(id, MotionPlaybackOutcome.Completed);
                    finished.Add(id);
                }
            }
            return finished;
        }

        public bool Finish(Guid playbackId, MotionPlaybackOutcome outcome)
        {
            if (!values.Remove(playbackId, out ImperativePlayback playback))
                return false;
            playback.Outcome = outcome;
            events.Add(
                new MotionPlaybackEvent(new ObjectId(playbackId), playback.Generation, outcome)
            );
            return true;
        }

        public bool OwnsDescriptor(Guid descriptorId) =>
            values.Values.Any(playback =>
                playback.Addresses.Any(address => address.DescriptorId.Value == descriptorId)
            );

        public IReadOnlyList<Guid> FailDescriptor(
            Guid descriptorId,
            Exception failure,
            Action<MotionPlaybackAddress> cancel
        )
        {
            var failed = new List<Guid>();
            foreach ((Guid id, ImperativePlayback playback) in values.ToArray())
                if (playback.Addresses.Any(address => address.DescriptorId.Value == descriptorId))
                {
                    foreach (MotionPlaybackAddress address in playback.Addresses)
                        cancel(address);
                    playback.Failure = failure;
                    Finish(id, MotionPlaybackOutcome.Failed);
                    failed.Add(id);
                }
            return failed;
        }

        public IBattlementCommandOperation? Operation(
            ObjectId playbackId,
            IReadOnlyDictionary<Guid, DescriptorState> descriptors,
            System.Action refresh,
            Action<ImperativePlayback> cancel,
            System.Action pause,
            System.Action resume
        )
        {
            if (!values.TryGetValue(playbackId.Value, out ImperativePlayback playback))
                return null;
            bool Infinite() =>
                playback.Addresses.Any(address =>
                    descriptors.TryGetValue(
                        address.DescriptorId.Value,
                        out DescriptorState descriptor
                    )
                    && descriptor
                        .FindSlot(address.Slot)
                        ?.Definition.Target.Tracks.Any(track =>
                            track.Transition.Repeat is MotionRepeat.Forever
                        ) == true
                );
            return new RunningMotion(
                playback,
                Infinite,
                refresh,
                () => cancel(playback),
                () =>
                    playback.Addresses.All(address =>
                        descriptors.TryGetValue(
                            address.DescriptorId.Value,
                            out DescriptorState descriptor
                        )
                        && descriptor.FindSlot(address.Slot) is SlotState slot
                        && (
                            slot.Terminal
                            || slot.Paused
                            || slot.Clock is MotionClockSource.Controlled
                        )
                    ),
                pause,
                resume
            );
        }

        public IReadOnlyList<MotionPlaybackEvent> DrainEvents()
        {
            MotionPlaybackEvent[] drained = events.ToArray();
            events.Clear();
            return drained;
        }
    }

    internal sealed record MotionPlaybackAddress(
        ObjectId DescriptorId,
        ulong Slot,
        uint Generation
    );

    internal sealed record ImperativePlayback(
        uint Generation,
        List<MotionPlaybackAddress> Addresses
    ) : IMotionPlaybackStatus
    {
        public MotionPlaybackOutcome? Outcome { get; set; }
        public Exception? Failure { get; set; }
    }
}

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
            values.Clear();
            events.Clear();
        }

        public void Register(
            ObjectId playbackId,
            uint generation,
            List<MotionPlaybackAddress> addresses
        ) => values[playbackId.Value] = new ImperativePlayback(generation, addresses);

        public bool TryGet(Guid playbackId, out ImperativePlayback playback) =>
            values.TryGetValue(playbackId, out playback!);

        public IReadOnlyList<Guid> Complete(IReadOnlyDictionary<Guid, DescriptorState> descriptors)
        {
            if (values.Count == 0)
                return Array.Empty<Guid>();
            var finished = new List<Guid>();
            foreach ((Guid id, ImperativePlayback playback) in values.ToArray())
            {
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
            events.Add(
                new MotionPlaybackEvent(new ObjectId(playbackId), playback.Generation, outcome)
            );
            return true;
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
    );
}

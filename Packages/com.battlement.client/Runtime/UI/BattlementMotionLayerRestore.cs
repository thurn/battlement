#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Battlement.UI
{
    /// <summary>Returns released transform gestures to the current lower layer.</summary>
    internal sealed class BattlementMotionLayerRestore
    {
        private readonly Dictionary<MotionProperty, MotionValue> baseline = new();
        private readonly Dictionary<ulong, Restore> releases = new();

        public BattlementMotionLayerRestore(
            MotionDescriptor descriptor,
            IBattlementMotionTarget target,
            BattlementMotionLayerRestore? previous
        )
        {
            var bindings = new HashSet<MotionProperty>(
                (descriptor.ValueBindings ?? Array.Empty<MotionValueBinding>()).Select(value =>
                    value.Property
                )
            );
            foreach (MotionSlotDescriptor slot in descriptor.Slots)
            {
                if (slot.Layer is MotionLayer.Animate or MotionLayer.Exit)
                    continue;
                if (
                    previous is not null
                    && previous.releases.TryGetValue(slot.Slot, out Restore release)
                )
                    if (release.Generation == slot.Generation)
                        releases[slot.Slot] = release;
                foreach (MotionPropertyTrack track in slot.Target.Tracks)
                    if (!bindings.Contains(track.Property))
                        baseline[track.Property] =
                            previous is not null
                            && previous.baseline.TryGetValue(track.Property, out MotionValue value)
                                ? value
                                : target.Read(track.Property);
            }
        }

        public void BeginSample(IBattlementMotionTarget target)
        {
            foreach ((MotionProperty property, MotionValue value) in baseline)
                target.Write(property, value);
        }

        public void SetActive(
            SlotState slot,
            bool active,
            IBattlementMotionTarget target,
            ulong now
        )
        {
            if (active)
            {
                releases.Remove(slot.Definition.Slot);
                return;
            }
            if (!slot.Active)
                return;
            var tracks = new List<ReleaseTrack>();
            foreach (MotionPropertyTrack track in slot.Definition.Target.Tracks)
                tracks.Add(
                    new ReleaseTrack(
                        track with
                        {
                            Transition = track.Transition with { Repeat = new MotionRepeat.None() },
                        },
                        ((MotionValue.Scalar)target.Read(track.Property)).Value,
                        slot.FindTrack(track.Property)?.Velocity ?? 0
                    )
                );
            releases[slot.Definition.Slot] = new Restore(now, slot.Definition.Generation, tracks);
        }

        public void Sample(SlotState slot, IBattlementMotionTarget target, ulong now, bool reduced)
        {
            if (!releases.TryGetValue(slot.Definition.Slot, out Restore restore))
                return;
            bool done = true;
            foreach (ReleaseTrack track in restore.Tracks)
            {
                double lower = ((MotionValue.Scalar)target.Read(track.Definition.Property)).Value;
                if (reduced)
                    continue;
                MotionScalarSample sample = BattlementMotionScalarSampler.Sample(
                    track.Origin,
                    lower,
                    track.Velocity,
                    track.Definition.Transition,
                    now - restore.Anchor
                );
                target.WriteScalar(track.Definition.Property, sample.Value);
                done &= sample.Done;
            }
            if (done)
                releases.Remove(slot.Definition.Slot);
        }

        private sealed record Restore(ulong Anchor, uint Generation, List<ReleaseTrack> Tracks);

        private sealed record ReleaseTrack(
            MotionPropertyTrack Definition,
            double Origin,
            double Velocity
        );
    }
}

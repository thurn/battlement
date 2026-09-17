#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Battlement.UI
{
    internal static class BattlementMotionOwnership
    {
        internal static List<MotionSlotDescriptor> RetainDisjoint(
            DescriptorState descriptor,
            IReadOnlyList<MotionSlotDescriptor> replacements,
            out List<(ulong Slot, SlotState Previous)> retained
        )
        {
            var claimed = replacements
                .SelectMany(slot =>
                    slot.Target.Tracks.Select(track => track.Property)
                        .Concat(slot.Target.TransitionEnd.Select(value => value.Property))
                )
                .ToHashSet();
            var replacementIds = replacements.Select(slot => slot.Slot).ToHashSet();
            var occupied = descriptor
                .Descriptor.Slots.Select(slot => slot.Slot)
                .Concat(replacementIds)
                .ToHashSet();
            retained = new();
            var result = new List<MotionSlotDescriptor>();
            foreach (MotionSlotDescriptor slot in descriptor.Descriptor.Slots)
            {
                if (slot.Slot < ulong.MaxValue - 2048)
                {
                    result.Add(slot);
                    continue;
                }
                MotionPropertyTrack[] tracks = slot
                    .Target.Tracks.Where(track => !claimed.Contains(track.Property))
                    .ToArray();
                MotionPropertyValue[] end = slot
                    .Target.TransitionEnd.Where(value => !claimed.Contains(value.Property))
                    .ToArray();
                if (tracks.Length == 0 && end.Length == 0)
                    continue;
                bool collision = replacementIds.Contains(slot.Slot);
                if (
                    tracks.Length == slot.Target.Tracks.Count
                    && end.Length == slot.Target.TransitionEnd.Count
                    && !collision
                )
                {
                    result.Add(slot);
                    continue;
                }
                ulong id = slot.Slot;
                if (collision)
                {
                    id = ulong.MaxValue - 2048;
                    while (occupied.Contains(id) && id < ulong.MaxValue - 1024)
                        id++;
                    if (id >= ulong.MaxValue - 1024)
                        throw new InvalidOperationException(
                            "Motion imperative property ownership capacity exceeded."
                        );
                    occupied.Add(id);
                }
                result.Add(
                    slot with
                    {
                        Slot = id,
                        Generation = checked(slot.Generation + 1),
                        Target = new MotionTargetDescriptor(tracks, end),
                    }
                );
                retained.Add((id, descriptor.FindSlot(slot.Slot)!));
            }
            return result;
        }
    }
}

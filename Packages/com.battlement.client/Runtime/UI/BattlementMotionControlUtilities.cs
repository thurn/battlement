#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Battlement.UI
{
    internal static class BattlementMotionControlUtilities
    {
        public static void ApplyImmediately(
            DescriptorState descriptor,
            MotionTargetDescriptor target
        )
        {
            foreach (MotionPropertyTrack track in target.Tracks)
                if (track.Values.Count != 0)
                    BattlementMotionPropertyWriter.Write(
                        descriptor.Target,
                        track.Property,
                        track.Values[^1]
                    );
            foreach (MotionPropertyValue value in target.TransitionEnd)
                BattlementMotionPropertyWriter.Write(
                    descriptor.Target,
                    value.Property,
                    value.Value
                );
        }

        public static MotionTargetDescriptor Resolve(
            DescriptorState descriptor,
            MotionControlTarget target
        ) =>
            target switch
            {
                MotionControlTarget.Target value => value.Value,
                MotionControlTarget.Variant value => (
                    descriptor.Descriptor.NamedTargets ?? Array.Empty<MotionNamedTarget>()
                )
                    .FirstOrDefault(candidate => candidate.Name == value.Value)
                    ?.Target
                    ?? throw Invalid($"Controlled variant '{value.Value}' is unavailable."),
                _ => throw Invalid("Unknown animation-controls target."),
            };

        public static IEnumerable<DescriptorState> Select(
            IEnumerable<DescriptorState> descriptors,
            DescriptorState root,
            MotionSelector selector
        )
        {
            DescriptorState[] snapshot = descriptors.ToArray();
            return selector switch
            {
                MotionSelector.Element value => snapshot.Where(candidate =>
                    candidate.Descriptor.HostId == value.Value
                ),
                MotionSelector.Name value => snapshot.Where(candidate =>
                    root.Target.Contains(candidate.Target)
                    && candidate.Descriptor.MotionName == value.Value
                ),
                MotionSelector.ScopeRoot => new[] { root },
                MotionSelector.Children => snapshot.Where(candidate =>
                    ReferenceEquals(candidate.Target.parent, root.Target)
                ),
                MotionSelector.Descendants => snapshot.Where(candidate =>
                    !ReferenceEquals(candidate, root) && root.Target.Contains(candidate.Target)
                ),
                _ => throw Invalid("Unknown animation-scope selector."),
            };
        }

        public static MotionTargetDescriptor Delay(
            MotionTargetDescriptor target,
            ulong delayMicros
        ) =>
            new(
                target
                    .Tracks.Select(track =>
                        track with
                        {
                            Transition = track.Transition with
                            {
                                DelayMicros = checked(
                                    track.Transition.DelayMicros + (long)delayMicros
                                ),
                            },
                        }
                    )
                    .ToArray(),
                target.TransitionEnd
            );

        private static BattlementUiException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}

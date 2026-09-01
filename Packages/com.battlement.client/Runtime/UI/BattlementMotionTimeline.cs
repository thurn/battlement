#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class DescriptorState : IDisposable
    {
        private readonly SlotState[] slots;
        private readonly HashSet<ulong> noBackwardsFill = new();
        private readonly HashSet<ulong> noForwardsFill = new();
        private readonly Dictionary<ulong, CssAnimationDescriptor> cssAnimations = new();
        private readonly BattlementPseudoStyleState? pseudoStyles;
        private readonly BattlementDecorationState? decorations;

        public DescriptorState(
            MotionDescriptor descriptor,
            VisualElement target,
            ulong clockMicros,
            DescriptorState? previous
        )
        {
            Descriptor = descriptor;
            Target = target;
            IReadOnlyList<CssAnimationDescriptor> cssAnimations =
                descriptor.Animations ?? Array.Empty<CssAnimationDescriptor>();
            slots = new SlotState[descriptor.Slots.Count + cssAnimations.Count];
            for (int index = 0; index < descriptor.Slots.Count; index++)
            {
                MotionSlotDescriptor slot = descriptor.Slots[index];
                SlotState? oldSlot = previous?.FindSlot(slot.Slot);
                if (oldSlot is not null && slot.Generation <= oldSlot.Definition.Generation)
                    throw Invalid("A motion slot update must advance its generation.");
                slots[index] = new SlotState(
                    slot,
                    descriptor.Clock,
                    target,
                    clockMicros,
                    previous is null ? InitialOrigins(descriptor) : null,
                    previous
                );
            }
            for (int cssIndex = 0; cssIndex < cssAnimations.Count; cssIndex++)
            {
                CssAnimationDescriptor animation = cssAnimations[cssIndex];
                this.cssAnimations.Add(animation.Slot, animation);
                var definition = new MotionSlotDescriptor(
                    animation.Slot,
                    animation.Generation,
                    MotionLayer.Animate,
                    new MotionTargetDescriptor(
                        BattlementCssTracks.Resolve(animation.Tracks, target),
                        Array.Empty<MotionPropertyValue>()
                    ),
                    new MotionCallbackSubscriptions(false, false, false, false, false, false)
                );
                var state = new SlotState(
                    definition,
                    descriptor.Clock,
                    target,
                    clockMicros,
                    null,
                    previous
                );
                state.Direction = animation.Direction switch
                {
                    AnimationDirection.Normal => MotionPlaybackDirection.Forward,
                    AnimationDirection.Reverse => MotionPlaybackDirection.Reverse,
                    AnimationDirection.Alternate => MotionPlaybackDirection.Alternate,
                    AnimationDirection.AlternateReverse => MotionPlaybackDirection.AlternateReverse,
                    _ => throw Invalid("Unknown CSS animation direction."),
                };
                CssAnimationDescriptor? oldAnimation = previous?.FindCssAnimation(animation.Slot);
                SlotState? oldState = previous?.FindSlot(animation.Slot);
                if (
                    oldAnimation is not null
                    && oldState is not null
                    && oldAnimation.RestartKey == animation.RestartKey
                )
                    state.AdoptPlayback(
                        oldState,
                        clockMicros,
                        animation.PlayState == AnimationPlayState.Paused
                    );
                else
                    state.Paused = animation.PlayState == AnimationPlayState.Paused;
                slots[descriptor.Slots.Count + cssIndex] = state;
                if (animation.Fill is AnimationFill.None or AnimationFill.Forwards)
                    noBackwardsFill.Add(animation.Slot);
                if (animation.Fill is AnimationFill.None or AnimationFill.Backwards)
                    noForwardsFill.Add(animation.Slot);
            }
            Array.Sort(
                slots,
                (left, right) =>
                {
                    int layer = left.Definition.Layer.CompareTo(right.Definition.Layer);
                    return layer != 0
                        ? layer
                        : left.Definition.Slot.CompareTo(right.Definition.Slot);
                }
            );
            IReadOnlyList<MotionPseudoStyle> pseudo =
                descriptor.PseudoStyles ?? Array.Empty<MotionPseudoStyle>();
            var cssProperties = new HashSet<MotionProperty>();
            foreach (CssAnimationDescriptor animation in cssAnimations)
            foreach (CssPropertyTrack track in animation.Tracks)
                cssProperties.Add(track.Property);
            StyleTransitionDescriptor? styleTransition = descriptor.StyleTransition;
            bool hasStyleTransition =
                styleTransition?.All is not null || styleTransition?.Properties.Count > 0;
            if (pseudo.Count != 0 || hasStyleTransition || cssProperties.Count != 0)
                pseudoStyles = new BattlementPseudoStyleState(
                    target,
                    pseudo,
                    cssProperties,
                    styleTransition,
                    clockMicros,
                    previous?.pseudoStyles
                );
            IReadOnlyList<MotionDecorationDescriptor> decorationDescriptors =
                descriptor.Decorations ?? Array.Empty<MotionDecorationDescriptor>();
            if (decorationDescriptors.Count != 0)
                decorations = new BattlementDecorationState(
                    target,
                    decorationDescriptors,
                    clockMicros,
                    previous?.decorations
                );
        }

        public MotionDescriptor Descriptor { get; }

        public VisualElement Target { get; }

        public void SetPseudoState(MotionPseudoState state, bool value) =>
            pseudoStyles?.SetState(state, value);

        public void SetGestureLayer(MotionLayer layer, bool value, ulong clockMicros)
        {
            foreach (SlotState slot in slots)
                if (slot.Definition.Layer == layer)
                    slot.SetActive(value, Target, clockMicros);
        }

        public SlotState? FindSlot(ulong slot)
        {
            foreach (SlotState state in slots)
                if (state.Definition.Slot == slot)
                    return state;
            return null;
        }

        public CssAnimationDescriptor? FindCssAnimation(ulong slot) =>
            cssAnimations.TryGetValue(slot, out CssAnimationDescriptor value) ? value : null;

        public double IncomingVelocity(MotionProperty property)
        {
            for (int index = slots.Length - 1; index >= 0; index--)
            {
                TrackState? track = slots[index].FindTrack(property);
                if (track is not null)
                    return track.Velocity;
            }
            return 0;
        }

        public void ApplyInitialPresentation()
        {
            foreach (SlotState slot in slots)
                if (slot.Active && !noBackwardsFill.Contains(slot.Definition.Slot))
                    slot.ApplyOrigin(Target);
        }

        public void SynchronizeStaticStyles() => pseudoStyles?.SynchronizeStaticStyles();

        public void EmitActivated(BattlementMotionWorld world)
        {
            foreach (SlotState slot in slots)
                world.Emit(this, slot, new MotionEventKind.Activated(), slot.HeldMicros);
        }

        public void Sample(ulong clockMicros, bool layout, BattlementMotionWorld world)
        {
            pseudoStyles?.Sample(clockMicros, layout);
            foreach (SlotState slot in slots)
                if (slot.Active && slot.Cancelled)
                    slot.ApplyOrigin(Target, layout);
            foreach (SlotState slot in slots)
            {
                if (!slot.Active)
                    continue;
                bool css = cssAnimations.TryGetValue(
                    slot.Definition.Slot,
                    out CssAnimationDescriptor animation
                );
                if (
                    css
                    && noBackwardsFill.Contains(slot.Definition.Slot)
                    && slot.InDelay(clockMicros)
                )
                    continue;
                IReadOnlyDictionary<MotionProperty, MotionValue>? lower =
                    css
                    && (
                        animation.Composition != AnimationComposition.Replace
                        || noForwardsFill.Contains(slot.Definition.Slot)
                    )
                        ? slot.CaptureValues(Target, layout)
                        : null;
                slot.Sample(Target, clockMicros, layout);
                if (lower is null)
                    continue;
                if (noForwardsFill.Contains(slot.Definition.Slot) && slot.AllTracksDone)
                    slot.RestoreValues(Target, lower);
                else if (animation.Composition != AnimationComposition.Replace)
                    slot.Compose(Target, lower, animation.Composition, layout);
            }
            decorations?.Sample(clockMicros, layout);
            if (layout)
                return;
            foreach (SlotState slot in slots)
            {
                if (!slot.Active)
                    continue;
                slot.EmitCrossedBoundaries(this, world);
                if (slot.Definition.Callbacks.Update)
                    world.MarkUpdate(this, slot);
            }
        }

        public void CompleteSlots(BattlementMotionWorld world)
        {
            foreach (SlotState slot in slots)
            {
                if (!slot.Active)
                    continue;
                if (slot.SeekPending)
                {
                    slot.ConsumeSeek();
                    continue;
                }
                if (slot.Terminal || slot.Paused || !slot.AllTracksDone)
                    continue;
                foreach (MotionPropertyValue value in slot.Definition.Target.TransitionEnd)
                    BattlementMotionPropertyWriter.Write(Target, value.Property, value.Value);
                slot.MarkCompleted();
                if (slot.Definition.Callbacks.Complete)
                    world.Emit(this, slot, new MotionEventKind.Completed(), slot.LastElapsedMicros);
            }
        }

        public void CancelActiveSlots(BattlementMotionWorld world, ulong clockMicros)
        {
            foreach (SlotState slot in slots)
                Cancel(slot, world, slot.Elapsed(clockMicros));
        }

        public void Dispose()
        {
            pseudoStyles?.Dispose();
            decorations?.Dispose();
        }

        public void Stop(SlotState slot, BattlementMotionWorld world, ulong elapsedMicros)
        {
            if (slot.Terminal)
                return;
            slot.MarkStopped(elapsedMicros);
            if (slot.Definition.Callbacks.Stop)
                world.Emit(this, slot, new MotionEventKind.Stopped(), elapsedMicros);
        }

        public void Cancel(SlotState slot, BattlementMotionWorld world, ulong elapsedMicros)
        {
            if (slot.Terminal)
                return;
            slot.MarkCancelled(elapsedMicros);
            if (slot.Definition.Callbacks.Cancel)
                world.Emit(this, slot, new MotionEventKind.Cancelled(), elapsedMicros);
        }

        public void Complete(SlotState slot, BattlementMotionWorld world, ulong elapsedMicros)
        {
            if (slot.Terminal)
                return;
            slot.ApplyTerminal(Target);
            foreach (MotionPropertyValue value in slot.Definition.Target.TransitionEnd)
                BattlementMotionPropertyWriter.Write(Target, value.Property, value.Value);
            slot.MarkCompleted();
            if (slot.Definition.Callbacks.Complete)
                world.Emit(this, slot, new MotionEventKind.Completed(), elapsedMicros);
        }

        private static Dictionary<MotionProperty, MotionValue>? InitialOrigins(
            MotionDescriptor descriptor
        )
        {
            if (descriptor.InitialDisabled || descriptor.Initial is null)
                return null;
            var values = new Dictionary<MotionProperty, MotionValue>();
            foreach (MotionPropertyTrack track in descriptor.Initial.Tracks)
                values[track.Property] = track.Values[^1];
            return values;
        }

        private static BattlementUiException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }

    internal sealed class SlotState
    {
        private readonly TrackState[] tracks;
        private uint emittedIteration;
        private bool emittedStart;

        public SlotState(
            MotionSlotDescriptor definition,
            MotionClockSource clock,
            VisualElement target,
            ulong anchorMicros,
            IReadOnlyDictionary<MotionProperty, MotionValue>? initial,
            DescriptorState? previous
        )
        {
            Definition = definition;
            Clock = clock;
            AnchorMicros = anchorMicros;
            Speed = 1;
            Direction = MotionPlaybackDirection.Forward;
            Active = definition.Layer is MotionLayer.Animate or MotionLayer.Exit;
            tracks = new TrackState[definition.Target.Tracks.Count];
            for (int index = 0; index < tracks.Length; index++)
            {
                MotionPropertyTrack track = definition.Target.Tracks[index];
                MotionValue presentation = BattlementMotionPropertyWriter.Read(
                    target,
                    track.Property
                );
                MotionValue origin =
                    initial is not null
                    && initial.TryGetValue(track.Property, out MotionValue value)
                        ? value
                    : track.Values.Count > 1 ? track.Values[0]
                    : presentation;
                tracks[index] = new TrackState(
                    track,
                    origin,
                    previous?.IncomingVelocity(track.Property) ?? 0
                );
            }
        }

        public MotionSlotDescriptor Definition { get; }

        public MotionClockSource Clock { get; }

        public ulong AnchorMicros { get; set; }

        public ulong HeldMicros { get; set; }

        public double Speed { get; set; }

        public bool Paused { get; set; }

        public bool SeekPending { get; set; }

        public bool Active { get; private set; }

        public MotionPlaybackDirection Direction { get; set; }

        public bool Terminal { get; private set; }

        public bool Cancelled { get; private set; }

        public ulong LastElapsedMicros { get; private set; }

        public bool AllTracksDone
        {
            get
            {
                foreach (TrackState track in tracks)
                    if (!track.Done)
                        return false;
                return tracks.Length != 0;
            }
        }

        public ulong Elapsed(ulong clockMicros)
        {
            if (Paused)
                return HeldMicros;
            double advanced = (clockMicros - AnchorMicros) * Speed;
            return checked(HeldMicros + (ulong)Math.Round(advanced));
        }

        public bool InDelay(ulong clockMicros)
        {
            ulong elapsed = Elapsed(clockMicros);
            foreach (TrackState track in tracks)
                if (track.Definition.Transition.DelayMicros <= 0)
                    return false;
                else if (elapsed >= (ulong)track.Definition.Transition.DelayMicros)
                    return false;
            return tracks.Length != 0;
        }

        public void AdoptPlayback(SlotState previous, ulong clockMicros, bool paused)
        {
            HeldMicros = previous.Elapsed(clockMicros);
            AnchorMicros = clockMicros;
            Speed = previous.Speed;
            Paused = paused;
            SeekPending = previous.SeekPending;
            Terminal = previous.Terminal;
            Cancelled = previous.Cancelled;
            emittedStart = previous.emittedStart;
            emittedIteration = previous.emittedIteration;
            foreach (TrackState track in tracks)
            {
                TrackState? oldTrack = previous.FindTrack(track.Definition.Property);
                if (oldTrack is not null)
                    track.Adopt(oldTrack);
            }
        }

        public void Reset(ulong clockMicros)
        {
            AnchorMicros = clockMicros;
            HeldMicros = 0;
            Paused = false;
            SeekPending = false;
            Terminal = false;
            Cancelled = false;
            emittedStart = false;
            emittedIteration = 0;
            foreach (TrackState track in tracks)
                track.Reset();
        }

        public void SetActive(bool value, VisualElement target, ulong clockMicros)
        {
            if (Active == value)
                return;
            Active = value;
            if (!value)
                return;
            AnchorMicros = clockMicros;
            HeldMicros = 0;
            Paused = false;
            SeekPending = false;
            Terminal = false;
            Cancelled = false;
            emittedStart = false;
            emittedIteration = 0;
            foreach (TrackState track in tracks)
                track.Retarget(target);
        }

        public TrackState? FindTrack(MotionProperty property)
        {
            foreach (TrackState state in tracks)
                if (state.Definition.Property == property)
                    return state;
            return null;
        }

        public void ApplyOrigin(VisualElement target)
        {
            foreach (TrackState track in tracks)
                track.ApplyOrigin(target);
        }

        public void ApplyOrigin(VisualElement target, bool layout)
        {
            foreach (TrackState track in tracks)
            {
                if (BattlementMotionPropertyWriter.IsLayout(track.Definition.Property) == layout)
                    track.ApplyOrigin(target);
            }
        }

        public void ApplyTerminal(VisualElement target)
        {
            foreach (TrackState track in tracks)
                track.ApplyTerminal(target);
        }

        public void Sample(VisualElement target, ulong clockMicros, bool layout)
        {
            if (Cancelled || (Terminal && !Paused))
                return;
            LastElapsedMicros = Elapsed(clockMicros);
            foreach (TrackState track in tracks)
            {
                if (BattlementMotionPropertyWriter.IsLayout(track.Definition.Property) != layout)
                    continue;
                track.Sample(target, LastElapsedMicros, Direction);
            }
        }

        public IReadOnlyDictionary<MotionProperty, MotionValue> CaptureValues(
            VisualElement target,
            bool layout
        )
        {
            var values = new Dictionary<MotionProperty, MotionValue>();
            foreach (TrackState track in tracks)
                if (BattlementMotionPropertyWriter.IsLayout(track.Definition.Property) == layout)
                    values[track.Definition.Property] = BattlementMotionPropertyWriter.Read(
                        target,
                        track.Definition.Property
                    );
            return values;
        }

        public void RestoreValues(
            VisualElement target,
            IReadOnlyDictionary<MotionProperty, MotionValue> values
        )
        {
            foreach ((MotionProperty property, MotionValue value) in values)
                BattlementMotionPropertyWriter.Write(target, property, value);
        }

        public void Compose(
            VisualElement target,
            IReadOnlyDictionary<MotionProperty, MotionValue> lower,
            AnimationComposition composition,
            bool layout
        )
        {
            foreach (TrackState track in tracks)
            {
                MotionProperty property = track.Definition.Property;
                if (BattlementMotionPropertyWriter.IsLayout(property) != layout)
                    continue;
                if (!lower.TryGetValue(property, out MotionValue under))
                    continue;
                MotionValue sample = BattlementMotionPropertyWriter.Read(target, property);
                BattlementMotionPropertyWriter.Write(
                    target,
                    property,
                    BattlementMotionComposition.Compose(
                        property,
                        under,
                        sample,
                        track.Origin,
                        track.End,
                        track.Iteration,
                        composition
                    )
                );
            }
        }

        public void EmitCrossedBoundaries(DescriptorState descriptor, BattlementMotionWorld world)
        {
            if (SeekPending)
                return;
            long earliestDelay = long.MaxValue;
            uint iteration = 0;
            foreach (TrackState track in tracks)
            {
                earliestDelay = Math.Min(earliestDelay, track.Definition.Transition.DelayMicros);
                iteration = Math.Max(iteration, track.Iteration);
            }
            ulong startBoundary = earliestDelay <= 0 ? 0 : checked((ulong)earliestDelay);
            if (!emittedStart && LastElapsedMicros >= startBoundary)
            {
                emittedStart = true;
                if (Definition.Callbacks.Start)
                    world.Emit(descriptor, this, new MotionEventKind.Started(), startBoundary);
            }
            if (iteration <= emittedIteration)
                return;
            if (Definition.Callbacks.Repeat)
            {
                world.Emit(
                    descriptor,
                    this,
                    new MotionEventKind.Repeated(emittedIteration + 1, iteration),
                    LastElapsedMicros
                );
            }
            emittedIteration = iteration;
        }

        public void ConsumeSeek()
        {
            SeekPending = false;
            emittedStart = true;
            foreach (TrackState track in tracks)
                emittedIteration = Math.Max(emittedIteration, track.Iteration);
        }

        public IReadOnlyList<MotionPropertyValue> CaptureValues(VisualElement target)
        {
            var values = new MotionPropertyValue[tracks.Length];
            for (int index = 0; index < tracks.Length; index++)
            {
                MotionProperty property = tracks[index].Definition.Property;
                values[index] = new MotionPropertyValue(
                    property,
                    BattlementMotionPropertyWriter.Read(target, property)
                );
            }
            return values;
        }

        public void MarkStopped(ulong elapsedMicros)
        {
            HeldMicros = elapsedMicros;
            Paused = true;
            Terminal = true;
            foreach (TrackState track in tracks)
                track.Freeze();
        }

        public void MarkCancelled(ulong elapsedMicros)
        {
            HeldMicros = elapsedMicros;
            Paused = true;
            Terminal = true;
            Cancelled = true;
            foreach (TrackState track in tracks)
                track.Freeze();
        }

        public void MarkCompleted()
        {
            Paused = false;
            Terminal = true;
            foreach (TrackState track in tracks)
                track.Freeze();
        }
    }

    internal sealed class TrackState
    {
        private MotionValue origin;
        private double incomingVelocity;

        public TrackState(
            MotionPropertyTrack definition,
            MotionValue origin,
            double incomingVelocity
        )
        {
            Definition = definition;
            this.origin = origin;
            this.incomingVelocity = incomingVelocity;
        }

        public MotionPropertyTrack Definition { get; }

        public double Velocity { get; private set; }

        public bool Done { get; private set; }

        public uint Iteration { get; private set; }

        public MotionValue Origin => origin;

        public MotionValue End => EndValue();

        public void Adopt(TrackState previous)
        {
            Velocity = previous.Velocity;
            Done = previous.Done;
            Iteration = previous.Iteration;
        }

        public void Reset()
        {
            Velocity = incomingVelocity;
            Done = false;
            Iteration = 0;
        }

        public void Retarget(VisualElement target)
        {
            origin = BattlementMotionPropertyWriter.Read(target, Definition.Property);
            incomingVelocity = Velocity;
            Reset();
        }

        public void Freeze()
        {
            Velocity = 0;
            Done = true;
        }

        public void ApplyOrigin(VisualElement target) =>
            BattlementMotionPropertyWriter.Write(target, Definition.Property, origin);

        public void ApplyTerminal(VisualElement target) =>
            BattlementMotionPropertyWriter.Write(target, Definition.Property, EndValue());

        public void Sample(
            VisualElement target,
            ulong elapsedMicros,
            MotionPlaybackDirection direction
        )
        {
            bool reverse =
                direction
                is MotionPlaybackDirection.Reverse
                    or MotionPlaybackDirection.AlternateReverse;
            MotionValue from = reverse ? EndValue() : origin;
            MotionValue to = reverse ? origin : EndValue();
            double velocity = reverse ? -incomingVelocity : incomingVelocity;
            TransitionDefinition transition = DirectedTransition(direction);
            if (
                from is MotionValue.Scalar left
                && to is MotionValue.Scalar right
                && Definition.Values.Count <= 1
            )
            {
                MotionScalarSample scalar = BattlementMotionScalarSampler.Sample(
                    left.Value,
                    right.Value,
                    velocity,
                    transition,
                    elapsedMicros
                );
                BattlementMotionPropertyWriter.WriteScalar(
                    target,
                    Definition.Property,
                    scalar.Value
                );
                Velocity = scalar.Velocity;
                Done = scalar.Done;
                Iteration = scalar.Iteration;
                return;
            }
            bool reverseSequence =
                direction
                is MotionPlaybackDirection.Reverse
                    or MotionPlaybackDirection.AlternateReverse;
            MotionPropertyTrack definition = new(
                Definition.Property,
                reverseSequence ? Reverse(Definition.Values) : Definition.Values,
                transition,
                reverseSequence ? ReverseTimes(Definition.Times) : Definition.Times
            );
            MotionTrackSample sample = BattlementMotionValueSampler.Sample(
                definition,
                from,
                velocity,
                elapsedMicros
            );
            BattlementMotionPropertyWriter.Write(target, Definition.Property, sample.Value);
            Velocity = sample.Velocity;
            Done = sample.Done;
            Iteration = sample.Iteration;
        }

        private MotionValue EndValue() =>
            Definition.Values.Count == 0 ? origin : Definition.Values[^1];

        private TransitionDefinition DirectedTransition(MotionPlaybackDirection direction)
        {
            if (direction is MotionPlaybackDirection.Forward or MotionPlaybackDirection.Reverse)
                return Definition.Transition;
            return new TransitionDefinition(
                Definition.Transition.Generator,
                Definition.Transition.DelayMicros,
                Definition.Transition.Repeat,
                Definition.Transition.RepeatDelayMicros,
                MotionRepeatType.Reverse
            );
        }

        private static IReadOnlyList<MotionValue> Reverse(IReadOnlyList<MotionValue> values)
        {
            var result = new MotionValue[values.Count];
            for (int index = 0; index < result.Length; index++)
                result[index] = values[values.Count - index - 1];
            return result;
        }

        private static IReadOnlyList<double>? ReverseTimes(IReadOnlyList<double>? times)
        {
            if (times is null)
                return null;
            var result = new double[times.Count];
            for (int index = 0; index < result.Length; index++)
                result[index] = 1 - times[times.Count - index - 1];
            return result;
        }
    }
}

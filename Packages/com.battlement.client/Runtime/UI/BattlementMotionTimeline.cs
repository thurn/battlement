#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class DescriptorState : IDisposable
    {
        private readonly SlotState[] slots;
        private readonly HashSet<ulong> retainedSlots = new();
        private readonly HashSet<ulong> noBackwardsFill = new();
        private readonly HashSet<ulong> noForwardsFill = new();
        private readonly Dictionary<ulong, CssAnimationDescriptor> cssAnimations = new();
        private readonly BattlementPseudoStyleState? pseudoStyles;
        private readonly BattlementMotionLayerRestore? layerRestore;
        private readonly BattlementDecorationState? decorations;
        private readonly BattlementLayoutProjection? layoutProjection;
        private readonly IReadOnlyDictionary<MotionProperty, MotionValue>? reconnectPresentation;

        public DescriptorState(
            MotionDescriptor descriptor,
            IBattlementMotionTarget properties,
            ulong clockMicros,
            DescriptorState? previous,
            BattlementLayoutOrigin? layoutOrigin = null,
            bool reconnecting = false,
            bool retainUnchangedSlots = false
        )
        {
            Descriptor = descriptor;
            Properties = properties;
            IReadOnlyList<CssAnimationDescriptor> cssAnimations =
                descriptor.Animations ?? Array.Empty<CssAnimationDescriptor>();
            slots = new SlotState[descriptor.Slots.Count + cssAnimations.Count];
            for (int index = 0; index < descriptor.Slots.Count; index++)
            {
                MotionSlotDescriptor slot = descriptor.Slots[index];
                SlotState? oldSlot = previous?.FindSlot(slot.Slot);
                if (retainUnchangedSlots && ReferenceEquals(slot, oldSlot?.Definition))
                {
                    slots[index] = oldSlot!;
                    retainedSlots.Add(slot.Slot);
                    continue;
                }
                if (
                    oldSlot is not null
                    && slot.Generation <= oldSlot.Definition.Generation
                    && !(reconnecting && slot.Generation == oldSlot.Definition.Generation)
                )
                    throw Invalid("A motion slot update must advance its generation.");
                var state = new SlotState(
                    slot,
                    descriptor.Clock,
                    Properties,
                    clockMicros,
                    previous is null ? InitialOrigins(descriptor) : null,
                    previous
                );
                if (
                    reconnecting
                    && oldSlot is not null
                    && slot.Generation == oldSlot.Definition.Generation
                )
                    state.AdoptPlayback(oldSlot, clockMicros, oldSlot.Paused);
                slots[index] = state;
            }
            if (properties is not BattlementUiMotionTarget ui)
            {
                layerRestore = new BattlementMotionLayerRestore(
                    descriptor,
                    properties,
                    previous?.layerRestore
                );
                Array.Sort(slots, CompareSlots);
                if (reconnecting && previous is not null)
                    reconnectPresentation = CapturePresentation(previous);
                return;
            }
            VisualElement target = ui.Element;
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
                    Properties,
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
            Array.Sort(slots, CompareSlots);
            IReadOnlyList<MotionPseudoStyle> pseudo =
                descriptor.PseudoStyles ?? Array.Empty<MotionPseudoStyle>();
            var baselineProperties = new HashSet<MotionProperty>();
            foreach (CssAnimationDescriptor animation in cssAnimations)
            foreach (CssPropertyTrack track in animation.Tracks)
                baselineProperties.Add(track.Property);
            var boundProperties = new HashSet<MotionProperty>();
            foreach (
                MotionValueBinding binding in descriptor.ValueBindings
                    ?? Array.Empty<MotionValueBinding>()
            )
                if (binding.Composition == MotionBindingComposition.Replace)
                    boundProperties.Add(binding.Property);
            foreach (MotionSlotDescriptor slot in descriptor.Slots)
            {
                if (slot.Layer is MotionLayer.Animate or MotionLayer.Exit)
                    continue;
                foreach (MotionPropertyTrack track in slot.Target.Tracks)
                    if (!boundProperties.Contains(track.Property))
                        baselineProperties.Add(track.Property);
            }
            StyleTransitionDescriptor? styleTransition = descriptor.StyleTransition;
            bool hasStyleTransition =
                styleTransition?.All is not null || styleTransition?.Properties.Count > 0;
            if (pseudo.Count != 0 || hasStyleTransition || baselineProperties.Count != 0)
                pseudoStyles = new BattlementPseudoStyleState(
                    target,
                    pseudo,
                    baselineProperties,
                    styleTransition,
                    descriptor.Clock,
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
            if (descriptor.Layout is not null)
                layoutProjection = new BattlementLayoutProjection(
                    target,
                    descriptor.Layout,
                    layoutOrigin
                        ?? new BattlementLayoutOrigin(
                            previous?.Target.worldBound ?? target.worldBound,
                            previous?.Target.panel ?? target.panel
                        ),
                    clockMicros
                );
            if (reconnecting && previous is not null)
                reconnectPresentation = CapturePresentation(previous);
        }

        public MotionDescriptor Descriptor { get; private set; }

        public VisualElement? Element => (Properties as BattlementUiMotionTarget)?.Element;

        public VisualElement Target =>
            Element ?? throw Invalid("This Motion host has no UI element.");

        public IBattlementMotionTarget Properties { get; }

        public BattlementLayoutProjection? LayoutProjection => layoutProjection;

        public int ActiveTimelineCount
        {
            get
            {
                int count = 0;
                foreach (SlotState slot in slots)
                    if (slot.Active && !slot.Terminal)
                        count++;
                return count;
            }
        }

        internal int ActiveFiniteTimelineCount =>
            slots.Count(slot => slot.IsFiniteActive)
            + (pseudoStyles?.ActiveFiniteTimelineCount ?? 0)
            + (decorations?.ActiveFiniteTimelineCount ?? 0);

        internal int ActiveInfiniteTimelineCount =>
            slots.Count(slot => slot.IsInfiniteActive)
            + (pseudoStyles?.ActiveInfiniteTimelineCount ?? 0)
            + (decorations?.ActiveInfiniteTimelineCount ?? 0);

        internal int ActiveHeldTimelineCount =>
            slots.Count(slot => slot.IsHeldActive)
            + (pseudoStyles?.ActiveHeldTimelineCount ?? 0)
            + (decorations?.ActiveHeldTimelineCount ?? 0);

        internal IEnumerable<string> ActiveTimelineDiagnostics() =>
            slots
                .Where(slot => slot.IsFiniteActive)
                .Select(slot => slot.ReadinessDiagnostic())
                .Concat(pseudoStyles?.ActiveTimelineDiagnostics() ?? Enumerable.Empty<string>())
                .Concat(decorations?.ActiveTimelineDiagnostics() ?? Enumerable.Empty<string>());

        public int ActiveLayoutTrackCount
        {
            get
            {
                int count = 0;
                foreach (SlotState slot in slots)
                    if (slot.Active && !slot.Terminal)
                        count += slot.LayoutTrackCount;
                return count;
            }
        }

        public int ActivePropertyCount
        {
            get
            {
                int count = 0;
                foreach (SlotState slot in slots)
                    if (slot.Active && !slot.Terminal)
                        count += slot.TrackCount;
                return count;
            }
        }

        public int NativeOptimizedTrackCount => Descriptor.StyleTransition?.Properties.Count ?? 0;

        public void SetPseudoState(MotionPseudoState state, bool value) =>
            pseudoStyles?.SetState(state, value);

        public void SetGestureLayer(MotionLayer layer, bool value, ulong clockMicros)
        {
            foreach (SlotState slot in slots)
                if (slot.Definition.Layer == layer)
                {
                    layerRestore?.SetActive(slot, value, Properties, clockMicros);
                    slot.SetActive(value, Properties, clockMicros);
                }
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

        internal void RetainPlayback(ulong slot, SlotState previous, ulong clockMicros)
        {
            FindSlot(slot)!.AdoptPlayback(previous, clockMicros, previous.Paused);
            retainedSlots.Add(slot);
        }

        public void ApplyInitialPresentation(bool reducedMotion)
        {
            foreach (SlotState slot in slots)
                if (!retainedSlots.Contains(slot.Definition.Slot))
                    if (slot.Active && !noBackwardsFill.Contains(slot.Definition.Slot))
                        slot.ApplyInitialOrigin(Properties, reducedMotion);
        }

        public void ApplyReconnectPresentation()
        {
            if (reconnectPresentation is null)
                return;
            foreach ((MotionProperty property, MotionValue value) in reconnectPresentation)
                Properties.Write(property, value);
        }

        public void SynchronizeStaticStyles()
        {
            pseudoStyles?.SynchronizeStaticStyles();
        }

        public void CommitPaint() => pseudoStyles?.CommitPaint();

        public System.Action PrepareStyle(UiStyle style)
        {
            System.Action? commit = pseudoStyles?.PrepareStyle(style);
            return () =>
            {
                BattlementMotionPropertyWriter.CommitAuthoredStyle(Target, style);
                commit?.Invoke();
            };
        }

        public void CaptureLayoutTarget() => layoutProjection?.CaptureDestination();

        public void SampleLayout(ulong clockMicros, bool reducedMotion) =>
            layoutProjection?.Sample(clockMicros, reducedMotion);

        public void EmitActivated(BattlementMotionWorld world)
        {
            foreach (SlotState slot in slots)
                if (!retainedSlots.Contains(slot.Definition.Slot))
                    world.Emit(this, slot, new MotionEventKind.Activated(), slot.HeldMicros);
        }

        public void Sample(
            ulong clockMicros,
            bool layout,
            bool reducedMotion,
            BattlementMotionWorld world
        )
        {
            pseudoStyles?.Sample(clockMicros, layout, reducedMotion);
            if (!layout)
                layerRestore?.BeginSample(Properties);
            foreach (SlotState slot in slots)
                if (slot.Active && slot.Cancelled)
                    slot.ApplyOrigin(Properties, reducedMotion);
            foreach (SlotState slot in slots)
            {
                if (!slot.Active)
                {
                    if (!layout)
                        layerRestore?.Sample(slot, Properties, clockMicros, reducedMotion);
                    continue;
                }
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
                        ? slot.CaptureValues(Properties, layout)
                        : null;
                slot.Sample(Properties, clockMicros, layout, reducedMotion);
                if (slot.AllTracksDone && !slot.Paused && !slot.Cancelled)
                    foreach (MotionPropertyValue value in slot.Definition.Target.TransitionEnd)
                        if (Properties.IsLayout(value.Property) == layout)
                            Properties.Write(value.Property, value.Value);
                if (lower is null)
                    continue;
                if (noForwardsFill.Contains(slot.Definition.Slot) && slot.AllTracksDone)
                    slot.RestoreValues(Properties, lower);
                else if (animation.Composition != AnimationComposition.Replace)
                    slot.Compose(Properties, lower, animation.Composition, layout);
            }
            decorations?.Sample(clockMicros, layout, reducedMotion);
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
            CompleteSlots(world);
        }

        public int CompleteSlots(BattlementMotionWorld world)
        {
            var completed = 0;
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
                slot.MarkCompleted();
                completed++;
                if (slot.Definition.Callbacks.Complete)
                    world.Emit(this, slot, new MotionEventKind.Completed(), slot.LastElapsedMicros);
            }
            return completed + (decorations?.CompleteReadySlots() ?? 0);
        }

        public void CancelActiveSlots(
            BattlementMotionWorld world,
            ulong clockMicros,
            DescriptorState? replacement = null
        )
        {
            foreach (SlotState slot in slots)
                if (!ReferenceEquals(slot, replacement?.FindSlot(slot.Definition.Slot)))
                    Cancel(slot, world, slot.Elapsed(clockMicros));
        }

        public void Dispose()
        {
            layoutProjection?.Release();
            pseudoStyles?.Dispose();
            decorations?.Dispose();
        }

        public void Abort()
        {
            layoutProjection?.Abort();
            pseudoStyles?.Dispose();
            decorations?.Abort();
        }

        private IReadOnlyDictionary<MotionProperty, MotionValue> CapturePresentation(
            DescriptorState previous
        )
        {
            var properties = new HashSet<MotionProperty>();
            foreach (SlotState slot in slots)
            foreach (MotionPropertyTrack track in slot.Definition.Target.Tracks)
                properties.Add(track.Property);
            var presentation = new Dictionary<MotionProperty, MotionValue>();
            foreach (MotionProperty property in properties)
                presentation[property] = previous.Properties.Read(property);
            return presentation;
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
            slot.ApplyTerminal(Properties);
            foreach (MotionPropertyValue value in slot.Definition.Target.TransitionEnd)
                Properties.Write(value.Property, value.Value);
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

        private static int CompareSlots(SlotState left, SlotState right)
        {
            int layer = left.Definition.Layer.CompareTo(right.Definition.Layer);
            return layer != 0 ? layer : left.Definition.Slot.CompareTo(right.Definition.Slot);
        }

        private static BattlementUiException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}

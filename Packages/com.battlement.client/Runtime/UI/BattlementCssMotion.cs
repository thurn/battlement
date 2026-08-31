#nullable enable

using System;
using System.Collections.Generic;
using Newtonsoft.Json.Linq;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementCssTracks
    {
        public static IReadOnlyList<MotionPropertyTrack> Resolve(
            IReadOnlyList<CssPropertyTrack> tracks,
            VisualElement target
        )
        {
            var result = new MotionPropertyTrack[tracks.Count];
            for (int index = 0; index < result.Length; index++)
            {
                CssPropertyTrack track = tracks[index];
                MotionValue underlying = BattlementMotionPropertyWriter.Read(
                    target,
                    track.Property
                );
                var values = new List<MotionValue>(track.Values.Count + 2);
                var times = new List<double>(track.Times.Count + 2);
                if (track.Times[0] != 0)
                {
                    values.Add(underlying);
                    times.Add(0);
                }
                values.AddRange(track.Values);
                times.AddRange(track.Times);
                if (track.Times[^1] != 1)
                {
                    values.Add(underlying);
                    times.Add(1);
                }
                result[index] = new MotionPropertyTrack(
                    track.Property,
                    values,
                    track.Transition,
                    times
                );
            }
            return result;
        }
    }

    internal sealed class BattlementPseudoStyleState : IDisposable
    {
        private readonly VisualElement target;
        private readonly IReadOnlyList<MotionPseudoStyle> styles;
        private readonly StyleTransitionDescriptor transition;
        private readonly Dictionary<MotionProperty, MotionValue> baseline = new();
        private readonly Dictionary<MotionProperty, MotionValue> resolved = new();
        private readonly Dictionary<MotionProperty, MotionValue> preparedPresentation = new();
        private readonly Dictionary<MotionProperty, MotionValue> transitionOrigins = new();
        private readonly List<TrackState> tracks = new();
        private readonly EventCallback<PointerEnterEvent> pointerEnter;
        private readonly EventCallback<PointerLeaveEvent> pointerLeave;
        private readonly EventCallback<PointerDownEvent> pointerDown;
        private readonly EventCallback<PointerUpEvent> pointerUp;
        private readonly EventCallback<PointerCancelEvent> pointerCancel;
        private readonly EventCallback<FocusInEvent> focusIn;
        private readonly EventCallback<FocusOutEvent> focusOut;
        private ulong anchorMicros;
        private ulong currentMicros;
        private bool pendingStaticSync = true;
        private readonly BattlementPseudoStyleState? previous;
        private bool hover;
        private bool focus;
        private bool active;
        private bool disabled;

        public BattlementPseudoStyleState(
            VisualElement target,
            IReadOnlyList<MotionPseudoStyle> styles,
            IReadOnlyCollection<MotionProperty> cssProperties,
            StyleTransitionDescriptor? transition,
            ulong clockMicros,
            BattlementPseudoStyleState? previous
        )
        {
            this.target = target;
            this.styles = styles;
            this.transition =
                transition
                ?? new StyleTransitionDescriptor(
                    Array.Empty<StylePropertyTransition>(),
                    null,
                    false
                );
            currentMicros = clockMicros;
            this.previous = previous;
            foreach (MotionPseudoStyle style in styles)
            foreach (MotionPropertyValue value in style.Values)
                CaptureBaseline(value.Property);
            foreach (StylePropertyTransition value in this.transition.Properties)
                CaptureBaseline(value.Property);
            foreach (MotionProperty property in cssProperties)
                CaptureBaseline(property);
            if (this.transition.All is not null)
                foreach (MotionProperty property in Enum.GetValues(typeof(MotionProperty)))
                    if (
                        BattlementMotionPropertyWriter.Supports(property)
                        && !BattlementMotionPropertyWriter.IsDiscrete(property)
                    )
                        CaptureBaseline(property);
            foreach ((MotionProperty property, MotionValue value) in baseline)
                resolved[property] =
                    previous is not null
                    && previous.resolved.TryGetValue(property, out MotionValue prior)
                        ? prior
                        : value;
            hover = previous?.hover ?? false;
            focus = previous?.focus ?? false;
            active = previous?.active ?? false;
            disabled = previous?.disabled ?? !target.enabledInHierarchy;
            pointerEnter = _ => SetState(MotionPseudoState.Hover, true);
            pointerLeave = _ => SetState(MotionPseudoState.Hover, false);
            pointerDown = _ => SetState(MotionPseudoState.Active, true);
            pointerUp = _ => SetState(MotionPseudoState.Active, false);
            pointerCancel = _ => SetState(MotionPseudoState.Active, false);
            focusIn = _ => SetState(MotionPseudoState.Focus, true);
            focusOut = _ => SetState(MotionPseudoState.Focus, false);
            target.RegisterCallback(pointerEnter);
            target.RegisterCallback(pointerLeave);
            target.RegisterCallback(pointerDown);
            target.RegisterCallback(pointerUp);
            target.RegisterCallback(pointerCancel);
            target.RegisterCallback(focusIn);
            target.RegisterCallback(focusOut);
        }

        public void Sample(ulong clockMicros, bool layout)
        {
            currentMicros = clockMicros;
            SyncStaticBaseline();
            bool nextDisabled = !target.enabledInHierarchy;
            if (nextDisabled != disabled)
            {
                disabled = nextDisabled;
                Resolve(animate: true);
            }
            foreach ((MotionProperty property, MotionValue value) in resolved)
                if (
                    BattlementMotionPropertyWriter.IsLayout(property) == layout
                    && !tracks.Exists(track => track.Definition.Property == property)
                )
                    BattlementMotionPropertyWriter.Write(target, property, value);
            ulong elapsed = clockMicros >= anchorMicros ? clockMicros - anchorMicros : 0;
            foreach (TrackState track in tracks)
            {
                if (BattlementMotionPropertyWriter.IsLayout(track.Definition.Property) == layout)
                    track.Sample(target, elapsed, MotionPlaybackDirection.Forward);
            }
        }

        public void Dispose()
        {
            target.UnregisterCallback(pointerEnter);
            target.UnregisterCallback(pointerLeave);
            target.UnregisterCallback(pointerDown);
            target.UnregisterCallback(pointerUp);
            target.UnregisterCallback(pointerCancel);
            target.UnregisterCallback(focusIn);
            target.UnregisterCallback(focusOut);
        }

        public void SynchronizeStaticStyles() => SyncStaticBaseline();

        public void SetState(MotionPseudoState state, bool value)
        {
            switch (state)
            {
                case MotionPseudoState.Hover:
                    Change(ref hover, value);
                    break;
                case MotionPseudoState.Focus:
                    Change(ref focus, value);
                    break;
                case MotionPseudoState.Active:
                    Change(ref active, value);
                    break;
                case MotionPseudoState.Disabled:
                    Change(ref disabled, value);
                    break;
                default:
                    throw new InvalidOperationException("Unknown pseudo state.");
            }
        }

        private void Change(ref bool field, bool value)
        {
            if (field == value)
                return;
            SyncStaticBaseline();
            field = value;
            Resolve(animate: true);
        }

        private void Resolve(bool animate)
        {
            var next = new Dictionary<MotionProperty, MotionValue>(baseline);
            Overlay(next, MotionPseudoState.Hover, hover);
            Overlay(next, MotionPseudoState.Focus, focus);
            Overlay(next, MotionPseudoState.Active, active);
            Overlay(next, MotionPseudoState.Disabled, disabled);
            tracks.Clear();
            foreach ((MotionProperty property, MotionValue value) in next)
            {
                if (resolved.TryGetValue(property, out MotionValue old) && Equal(old, value))
                    continue;
                TransitionDefinition? timing = animate ? Timing(property, old, value) : null;
                if (timing is null)
                {
                    BattlementMotionPropertyWriter.Write(target, property, value);
                    continue;
                }
                tracks.Add(
                    new TrackState(
                        new MotionPropertyTrack(property, new[] { value }, timing),
                        transitionOrigins.TryGetValue(property, out MotionValue origin)
                            ? origin
                            : BattlementMotionPropertyWriter.Read(target, property),
                        0
                    )
                );
            }
            resolved.Clear();
            foreach ((MotionProperty property, MotionValue value) in next)
                resolved[property] = value;
            anchorMicros = currentMicros;
            transitionOrigins.Clear();
        }

        private void SyncStaticBaseline()
        {
            if (!pendingStaticSync)
                return;
            pendingStaticSync = false;
            foreach (MotionProperty property in new List<MotionProperty>(baseline.Keys))
            {
                MotionValue current = BattlementMotionPropertyWriter.Read(target, property);
                bool changedDuringCommit =
                    preparedPresentation.TryGetValue(property, out MotionValue prepared)
                    && !Equal(current, prepared);
                if (
                    previous is null
                    || changedDuringCommit
                    || !previous.baseline.TryGetValue(property, out MotionValue prior)
                )
                {
                    baseline[property] = current;
                    if (changedDuringCommit)
                        transitionOrigins[property] = prepared;
                }
                else
                    baseline[property] = prior;
            }
            Resolve(animate: previous is not null);
        }

        private void CaptureBaseline(MotionProperty property)
        {
            if (baseline.ContainsKey(property))
                return;
            MotionValue presentation = BattlementMotionPropertyWriter.Read(target, property);
            preparedPresentation[property] = presentation;
            baseline[property] =
                previous is not null
                && previous.baseline.TryGetValue(property, out MotionValue prior)
                    ? prior
                    : presentation;
        }

        private void Overlay(
            Dictionary<MotionProperty, MotionValue> targetValues,
            MotionPseudoState state,
            bool enabled
        )
        {
            if (!enabled)
                return;
            foreach (MotionPseudoStyle style in styles)
                if (style.State == state)
                    foreach (MotionPropertyValue value in style.Values)
                        targetValues[value.Property] = value.Value;
        }

        private TransitionDefinition? Timing(
            MotionProperty property,
            MotionValue? old,
            MotionValue value
        )
        {
            bool discrete = BattlementMotionPropertyWriter.IsDiscrete(property);
            foreach (StylePropertyTransition item in transition.Properties)
                if (item.Property == property)
                {
                    if (discrete && !transition.AllowDiscrete)
                        return null;
                    return DiscreteTiming(property, old, value, item.Transition);
                }
            return discrete ? null : transition.All;
        }

        private static TransitionDefinition? DiscreteTiming(
            MotionProperty property,
            MotionValue? old,
            MotionValue value,
            TransitionDefinition timing
        )
        {
            if (property is not MotionProperty.Visibility and not MotionProperty.Display)
                return timing;
            string next = ((MotionValue.Discrete)value).Value.ToObject<string>()!;
            string? previous = (old as MotionValue.Discrete)?.Value.ToObject<string>();
            bool appearing =
                (next == "visible" && previous == "hidden")
                || (next == "flex" && previous == "none");
            if (appearing)
                return null;
            if (timing.Generator is not TransitionGenerator.Tween tween)
                return timing;
            return timing with
            {
                Generator = new TransitionGenerator.Tween(
                    tween.DurationMicros,
                    new MotionEasing[] { new MotionEasing.Steps(1, MotionStepPosition.End) },
                    tween.Times
                ),
            };
        }

        private static bool Equal(MotionValue left, MotionValue right) =>
            JToken.DeepEquals(JToken.FromObject(left), JToken.FromObject(right));
    }

    internal sealed class BattlementDecorationState : IDisposable
    {
        private readonly Dictionary<ulong, DecorationEntry> entries = new();

        public BattlementDecorationState(
            VisualElement target,
            IReadOnlyList<MotionDecorationDescriptor> descriptors,
            ulong clockMicros,
            BattlementDecorationState? previous
        )
        {
            for (int index = 0; index < descriptors.Count; index++)
            {
                MotionDecorationDescriptor descriptor = descriptors[index];
                DecorationEntry? old = previous?.Take(descriptor.Key);
                var entry = new DecorationEntry(descriptor, old, clockMicros);
                VisualElement element = entry.Element;
                if (descriptor.Placement == DecorationPlacement.Before)
                    target.Insert(0, element);
                else
                    target.Add(element);
                entries.Add(descriptor.Key, entry);
            }
        }

        public void Sample(ulong clockMicros, bool layout)
        {
            foreach (DecorationEntry entry in entries.Values)
                entry.Sample(clockMicros, layout);
        }

        public void Dispose()
        {
            foreach (DecorationEntry entry in entries.Values)
                entry.Element.RemoveFromHierarchy();
            entries.Clear();
        }

        private DecorationEntry? Take(ulong key)
        {
            if (!entries.Remove(key, out DecorationEntry entry))
                return null;
            return entry;
        }

        private sealed class DecorationEntry
        {
            private readonly Dictionary<ulong, CssAnimationDescriptor> definitions = new();
            private readonly SlotState[] slots;
            private readonly HashSet<ulong> noBackwardsFill = new();
            private readonly HashSet<ulong> noForwardsFill = new();

            public DecorationEntry(
                MotionDecorationDescriptor descriptor,
                DecorationEntry? previous,
                ulong clockMicros
            )
            {
                Element = previous?.Element ?? CreateElement(descriptor.Key);
                Element.style.overflow =
                    descriptor.Overflow == DecorationOverflow.Visible
                        ? Overflow.Visible
                        : Overflow.Hidden;
                BattlementUiElementProperties.ApplyStyle(
                    Element,
                    descriptor.Style,
                    null,
                    null,
                    null,
                    null
                );
                slots = new SlotState[descriptor.Animations.Count];
                for (int index = 0; index < slots.Length; index++)
                {
                    CssAnimationDescriptor animation = descriptor.Animations[index];
                    definitions.Add(animation.Slot, animation);
                    var definition = new MotionSlotDescriptor(
                        animation.Slot,
                        animation.Generation,
                        MotionLayer.Animate,
                        new MotionTargetDescriptor(
                            BattlementCssTracks.Resolve(animation.Tracks, Element),
                            Array.Empty<MotionPropertyValue>()
                        ),
                        new MotionCallbackSubscriptions(false, false, false, false, false, false)
                    );
                    var slot = new SlotState(
                        definition,
                        new MotionClockSource.Unscaled(),
                        Element,
                        clockMicros,
                        null,
                        null
                    );
                    CssAnimationDescriptor? oldDefinition = previous?.FindDefinition(
                        animation.Slot
                    );
                    SlotState? oldSlot = previous?.FindSlot(animation.Slot);
                    bool preserved =
                        oldDefinition is not null
                        && oldSlot is not null
                        && oldDefinition.RestartKey == animation.RestartKey;
                    slot.Direction = Direction(animation.Direction);
                    if (preserved)
                        slot.AdoptPlayback(
                            oldSlot!,
                            clockMicros,
                            animation.PlayState == AnimationPlayState.Paused
                        );
                    else
                        slot.Paused = animation.PlayState == AnimationPlayState.Paused;
                    slots[index] = slot;
                    if (animation.Fill is AnimationFill.None or AnimationFill.Forwards)
                        noBackwardsFill.Add(animation.Slot);
                    else if (!preserved)
                        slot.ApplyOrigin(Element);
                    if (animation.Fill is AnimationFill.None or AnimationFill.Backwards)
                        noForwardsFill.Add(animation.Slot);
                }
            }

            public VisualElement Element { get; }

            public void Sample(ulong clockMicros, bool layout)
            {
                foreach (SlotState slot in slots)
                {
                    CssAnimationDescriptor animation = definitions[slot.Definition.Slot];
                    if (noBackwardsFill.Contains(animation.Slot) && slot.InDelay(clockMicros))
                        continue;
                    bool capture =
                        animation.Composition != AnimationComposition.Replace
                        || noForwardsFill.Contains(animation.Slot);
                    IReadOnlyDictionary<MotionProperty, MotionValue>? lower = capture
                        ? slot.CaptureValues(Element, layout)
                        : null;
                    slot.Sample(Element, clockMicros, layout);
                    if (lower is null)
                        continue;
                    if (noForwardsFill.Contains(animation.Slot) && slot.AllTracksDone)
                        slot.RestoreValues(Element, lower);
                    else if (animation.Composition != AnimationComposition.Replace)
                        slot.Compose(Element, lower, animation.Composition, layout);
                }
                if (layout)
                    return;
                foreach (SlotState slot in slots)
                    if (!slot.Terminal && !slot.Paused && slot.AllTracksDone)
                        slot.MarkCompleted();
            }

            private CssAnimationDescriptor? FindDefinition(ulong slot) =>
                definitions.TryGetValue(slot, out CssAnimationDescriptor value) ? value : null;

            private SlotState? FindSlot(ulong slot)
            {
                foreach (SlotState value in slots)
                    if (value.Definition.Slot == slot)
                        return value;
                return null;
            }

            private static VisualElement CreateElement(ulong key) =>
                new()
                {
                    name = $"battlement-decoration-{key}",
                    pickingMode = PickingMode.Ignore,
                    focusable = false,
                    style =
                    {
                        position = Position.Absolute,
                        left = 0,
                        right = 0,
                        top = 0,
                        bottom = 0,
                    },
                };

            private static MotionPlaybackDirection Direction(AnimationDirection value) =>
                value switch
                {
                    AnimationDirection.Normal => MotionPlaybackDirection.Forward,
                    AnimationDirection.Reverse => MotionPlaybackDirection.Reverse,
                    AnimationDirection.Alternate => MotionPlaybackDirection.Alternate,
                    AnimationDirection.AlternateReverse => MotionPlaybackDirection.AlternateReverse,
                    _ => throw new InvalidOperationException("Unknown CSS animation direction."),
                };
        }
    }
}

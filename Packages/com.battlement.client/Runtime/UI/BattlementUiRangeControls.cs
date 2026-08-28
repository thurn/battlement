#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using NativeMinMaxSlider = UnityEngine.UIElements.MinMaxSlider;
using NativeProgressBar = UnityEngine.UIElements.ProgressBar;

namespace Battlement.UI
{
    internal sealed class BattlementUiRangeControls
    {
        private readonly Dictionary<Guid, RangeState> ranges = new();
        private readonly BattlementUiEventForwarder events;

        public BattlementUiRangeControls(BattlementUiEventForwarder eventForwarder) =>
            events = eventForwarder;

        public void ApplyCreate(VisualElement target, ObjectId objectId, UiElement value)
        {
            switch (value)
            {
                case UiElement.MinMaxSlider range:
                    var native = (NativeMinMaxSlider)target;
                    native.lowLimit = float.MinValue;
                    native.highLimit = float.MaxValue;
                    native.SetValueWithoutNotify(new Vector2(0, 10));
                    Apply(native, range);
                    RangeState state = CreateState(native, objectId);
                    ranges.Add(objectId.Value, state);
                    break;
                case UiElement.ProgressBar progress:
                    Apply((NativeProgressBar)target, progress);
                    break;
                default:
                    return;
            }
        }

        public void ApplyUpdate(VisualElement target, ObjectId objectId, UiElement value)
        {
            switch (value)
            {
                case UiElement.MinMaxSlider range:
                    RangeState state = ranges[objectId.Value];
                    state.Cancel();
                    state.CommandOrigin = true;
                    try
                    {
                        Apply(state.Target, range);
                    }
                    finally
                    {
                        state.CommandOrigin = false;
                    }
                    if (TouchesRange(range))
                        state.Committed = state.Target.value;
                    break;
                case UiElement.ProgressBar progress:
                    Apply((NativeProgressBar)target, progress);
                    break;
                default:
                    return;
            }
        }

        public static void ValidateNode(UiElement element)
        {
            switch (element)
            {
                case UiElement.MinMaxSlider range:
                    Validate(
                        Resolve(range.LowLimit, float.MinValue, float.MinValue),
                        Resolve(range.HighLimit, float.MaxValue, float.MaxValue),
                        Resolve(range.MinValue, 0, 0),
                        Resolve(range.MaxValue, 10, 10)
                    );
                    break;
                case UiElement.ProgressBar progress:
                    Validate(
                        Resolve(progress.LowValue, 0, 0),
                        Resolve(progress.HighValue, 100, 100),
                        Resolve(progress.Value, 0, 0)
                    );
                    break;
                default:
                    break;
            }
        }

        public static void ValidateUpdate(UiElement element, VisualElement current)
        {
            switch (element)
            {
                case UiElement.MinMaxSlider range:
                    var nativeRange = (NativeMinMaxSlider)current;
                    Validate(
                        Resolve(range.LowLimit, nativeRange.lowLimit, float.MinValue),
                        Resolve(range.HighLimit, nativeRange.highLimit, float.MaxValue),
                        Resolve(range.MinValue, nativeRange.value.x, 0),
                        Resolve(range.MaxValue, nativeRange.value.y, 10)
                    );
                    break;
                case UiElement.ProgressBar progress:
                    var nativeProgress = (NativeProgressBar)current;
                    Validate(
                        Resolve(progress.LowValue, nativeProgress.lowValue, 0),
                        Resolve(progress.HighValue, nativeProgress.highValue, 100),
                        Resolve(progress.Value, nativeProgress.value, 0)
                    );
                    break;
                default:
                    break;
            }
        }

        public void Advance()
        {
            foreach (RangeState state in ranges.Values)
                TryForwardCommit(state);
        }

        public void Remove(Guid objectId)
        {
            if (ranges.Remove(objectId, out RangeState state))
                state.Dispose();
        }

        public void Clear()
        {
            foreach (RangeState state in ranges.Values)
                state.Dispose();
            ranges.Clear();
        }

        public void CancelAll()
        {
            foreach (RangeState state in ranges.Values)
            {
                state.Cancel();
                state.PendingCommits.Clear();
                ReleaseCaptures(state.Captures);
            }
        }

        private RangeState CreateState(NativeMinMaxSlider target, ObjectId objectId)
        {
            var state = new RangeState(target, objectId, this);
            state.ValueChanged = change =>
            {
                if (state.CommandOrigin || !target.enabledInHierarchy)
                    return;
                bool interacting = state.Interacting;
                events.ForwardValueChanging(objectId, ToRange(change.newValue));
                if (!interacting)
                    Commit(state, change.newValue);
            };
            state.PointerDown = _ => state.Interacting = state.Target.enabledInHierarchy;
            state.Capture = eventValue =>
            {
                state.Interacting = state.Target.enabledInHierarchy;
                if (state.Interacting && eventValue.target is VisualElement owner)
                    state.Captures[eventValue.pointerId] = owner;
            };
            state.PointerUp = _ => Commit(state, state.Target.value);
            state.PointerCancel = _ => Restore(state);
            state.CaptureOut = eventValue =>
            {
                if (!state.Captures.Remove(eventValue.pointerId))
                    return;
                Commit(state, state.Target.value);
            };
            state.Detach = _ => state.Cancel();
            target.RegisterValueChangedCallback(state.ValueChanged);
            target.RegisterCallback(state.PointerDown, TrickleDown.TrickleDown);
            target.RegisterCallback(state.Capture, TrickleDown.TrickleDown);
            target.RegisterCallback(state.PointerUp, TrickleDown.TrickleDown);
            target.RegisterCallback(state.PointerCancel, TrickleDown.TrickleDown);
            target.RegisterCallback(state.CaptureOut, TrickleDown.TrickleDown);
            target.RegisterCallback(state.Detach);
            return state;
        }

        private static void Apply(NativeMinMaxSlider target, UiElement.MinMaxSlider value)
        {
            var defaults = new NativeMinMaxSlider();
            Apply(value.Label, item => target.label = item, defaults.label);
            Vector2 selected = target.value;
            float low = Resolve(value.LowLimit, target.lowLimit, float.MinValue);
            float high = Resolve(value.HighLimit, target.highLimit, float.MaxValue);
            float min = Resolve(value.MinValue, selected.x, 0);
            float max = Resolve(value.MaxValue, selected.y, 10);
            if (!value.LowLimit.IsUnset || !value.HighLimit.IsUnset)
            {
                target.lowLimit = float.MinValue;
                target.highLimit = float.MaxValue;
                target.lowLimit = low;
                target.highLimit = high;
            }
            if (TouchesRange(value))
                target.SetValueWithoutNotify(new Vector2(min, max));
        }

        private static void Apply(NativeProgressBar target, UiElement.ProgressBar value)
        {
            var defaults = new NativeProgressBar();
            float low = Resolve(value.LowValue, target.lowValue, defaults.lowValue);
            float high = Resolve(value.HighValue, target.highValue, defaults.highValue);
            float selected = Resolve(value.Value, target.value, defaults.value);
            if (!value.LowValue.IsUnset || !value.HighValue.IsUnset)
            {
                target.lowValue = float.MinValue;
                target.highValue = float.MaxValue;
                target.lowValue = low;
                target.highValue = high;
            }
            if (!value.Value.IsUnset || !value.LowValue.IsUnset || !value.HighValue.IsUnset)
                target.SetValueWithoutNotify(selected);
            Apply(value.Title, item => target.title = item, defaults.title);
        }

        private static void Commit(RangeState state, Vector2 proposed)
        {
            if (!state.Interacting && proposed == state.Committed)
                return;
            Vector2 previous = state.Committed;
            state.Interacting = false;
            RestoreValue(state);
            state.Owner.ForwardCommit(state, ToRange(previous), ToRange(proposed));
        }

        private void ForwardCommit(RangeState state, FloatRange previous, FloatRange proposed)
        {
            if (!events.CanForward(state.ObjectId, UiEventKind.ValueCommitted))
                return;
            state.PendingCommits.Enqueue((previous, proposed));
            TryForwardCommit(state);
        }

        private bool TryForwardCommit(RangeState state)
        {
            if (!events.CanForward(state.ObjectId, UiEventKind.ValueCommitted))
            {
                state.PendingCommits.Clear();
                return true;
            }
            while (state.PendingCommits.TryPeek(out var pending))
            {
                if (
                    !events.ForwardValueCommitted(
                        state.ObjectId,
                        pending.Previous,
                        pending.Proposed
                    )
                )
                    return false;
                state.PendingCommits.Dequeue();
            }
            return true;
        }

        private static void Restore(RangeState state)
        {
            state.Interacting = false;
            RestoreValue(state);
        }

        private static void RestoreValue(RangeState state)
        {
            state.CommandOrigin = true;
            try
            {
                state.Target.SetValueWithoutNotify(state.Committed);
            }
            finally
            {
                state.CommandOrigin = false;
            }
        }

        private static float Resolve(Prop<LowerLimit> value, float fallback, float reset) =>
            value.IsUnset ? fallback
            : value.IsReset ? reset
            : value.Value switch
            {
                LowerLimit.Unbounded => float.MinValue,
                LowerLimit.Inclusive bounded => bounded.Value,
                _ => throw new InvalidOperationException("Unknown lower limit."),
            };

        private static float Resolve(Prop<UpperLimit> value, float fallback, float reset) =>
            value.IsUnset ? fallback
            : value.IsReset ? reset
            : value.Value switch
            {
                UpperLimit.Unbounded => float.MaxValue,
                UpperLimit.Inclusive bounded => bounded.Value,
                _ => throw new InvalidOperationException("Unknown upper limit."),
            };

        private static T Resolve<T>(Prop<T> value, T fallback, T reset) =>
            value.IsSet ? value.Value
            : value.IsReset ? reset
            : fallback;

        private static void Apply<T>(Prop<T> value, System.Action<T> assign, T reset)
        {
            if (value.IsSet)
                assign(value.Value);
            else if (value.IsReset)
                assign(reset);
        }

        private static FloatRange ToRange(Vector2 value) => new(value.x, value.y);

        private static void ReleaseCaptures(Dictionary<int, VisualElement> captures)
        {
            var owned = new Dictionary<int, VisualElement>(captures);
            captures.Clear();
            foreach ((int pointerId, VisualElement owner) in owned)
            {
                if (owner.HasPointerCapture(pointerId))
                    owner.ReleasePointer(pointerId);
            }
        }

        private static bool TouchesRange(UiElement.MinMaxSlider value) =>
            !value.MinValue.IsUnset
            || !value.MaxValue.IsUnset
            || !value.LowLimit.IsUnset
            || !value.HighLimit.IsUnset;

        private static void Validate(float low, float high, float min, float max)
        {
            if (!float.IsFinite(low) || !float.IsFinite(high))
                throw Failure("Range limits must be finite.");
            if (!float.IsFinite(min) || !float.IsFinite(max))
                throw Failure("Selected range values must be finite.");
            if (low > high || min > max || min < low || max > high)
                throw Failure("MinMaxSlider range is invalid.");
        }

        private static void Validate(float low, float high, float selected)
        {
            if (!float.IsFinite(low) || !float.IsFinite(high) || !float.IsFinite(selected))
                throw Failure("ProgressBar values must be finite.");
            if (low > high || selected < low || selected > high)
                throw Failure("ProgressBar range is invalid.");
        }

        private static BattlementUiException Failure(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private sealed class RangeState : IDisposable
        {
            public RangeState(
                NativeMinMaxSlider target,
                ObjectId objectId,
                BattlementUiRangeControls owner
            )
            {
                Target = target;
                ObjectId = objectId;
                Committed = target.value;
                Owner = owner;
            }

            public BattlementUiRangeControls Owner { get; }
            public NativeMinMaxSlider Target { get; }
            public ObjectId ObjectId { get; }
            public Vector2 Committed { get; set; }
            public bool CommandOrigin { get; set; }
            public bool Interacting { get; set; }
            public Dictionary<int, VisualElement> Captures { get; } = new();
            public Queue<(FloatRange Previous, FloatRange Proposed)> PendingCommits { get; } =
                new();
            public EventCallback<ChangeEvent<Vector2>> ValueChanged { get; set; } = null!;
            public EventCallback<PointerDownEvent> PointerDown { get; set; } = null!;
            public EventCallback<PointerCaptureEvent> Capture { get; set; } = null!;
            public EventCallback<PointerUpEvent> PointerUp { get; set; } = null!;
            public EventCallback<PointerCancelEvent> PointerCancel { get; set; } = null!;
            public EventCallback<PointerCaptureOutEvent> CaptureOut { get; set; } = null!;
            public EventCallback<DetachFromPanelEvent> Detach { get; set; } = null!;

            public void Cancel() => Restore(this);

            public void Dispose()
            {
                Cancel();
                ReleaseCaptures(Captures);
                Target.UnregisterValueChangedCallback(ValueChanged);
                Target.UnregisterCallback(PointerDown, TrickleDown.TrickleDown);
                Target.UnregisterCallback(Capture, TrickleDown.TrickleDown);
                Target.UnregisterCallback(PointerUp, TrickleDown.TrickleDown);
                Target.UnregisterCallback(PointerCancel, TrickleDown.TrickleDown);
                Target.UnregisterCallback(CaptureOut, TrickleDown.TrickleDown);
                Target.UnregisterCallback(Detach);
            }
        }
    }
}

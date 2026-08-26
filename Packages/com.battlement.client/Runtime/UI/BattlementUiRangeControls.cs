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
                        ToNative(range.LowLimit, float.MinValue),
                        ToNative(range.HighLimit, float.MaxValue),
                        range.MinValue ?? 0,
                        range.MaxValue ?? 10
                    );
                    break;
                case UiElement.ProgressBar progress:
                    Validate(
                        progress.LowValue ?? 0,
                        progress.HighValue ?? 100,
                        progress.Value ?? 0
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
                        ToNative(range.LowLimit, nativeRange.lowLimit),
                        ToNative(range.HighLimit, nativeRange.highLimit),
                        range.MinValue ?? nativeRange.value.x,
                        range.MaxValue ?? nativeRange.value.y
                    );
                    break;
                case UiElement.ProgressBar progress:
                    var nativeProgress = (NativeProgressBar)current;
                    Validate(
                        progress.LowValue ?? nativeProgress.lowValue,
                        progress.HighValue ?? nativeProgress.highValue,
                        progress.Value ?? nativeProgress.value
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

        private RangeState CreateState(NativeMinMaxSlider target, ObjectId objectId)
        {
            var state = new RangeState(target, objectId, this);
            state.ValueChanged = change =>
            {
                if (state.CommandOrigin || !target.enabledInHierarchy)
                    return;
                events.ForwardValueChanging(objectId, ToRange(change.newValue));
                if (!state.Interacting)
                    Commit(state, change.newValue);
            };
            state.PointerDown = _ => state.Interacting = state.Target.enabledInHierarchy;
            state.Capture = _ => state.Interacting = state.Target.enabledInHierarchy;
            state.PointerUp = _ => Commit(state, state.Target.value);
            state.PointerCancel = _ => Restore(state);
            state.CaptureOut = _ => Commit(state, state.Target.value);
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
            if (value.Label is not null)
                target.label = value.Label;
            Vector2 selected = target.value;
            float low = ToNative(value.LowLimit, target.lowLimit);
            float high = ToNative(value.HighLimit, target.highLimit);
            float min = value.MinValue ?? selected.x;
            float max = value.MaxValue ?? selected.y;
            if (value.LowLimit is not null || value.HighLimit is not null)
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
            float low = value.LowValue ?? target.lowValue;
            float high = value.HighValue ?? target.highValue;
            float selected = value.Value ?? target.value;
            if (value.LowValue is not null || value.HighValue is not null)
            {
                target.lowValue = float.MinValue;
                target.highValue = float.MaxValue;
                target.lowValue = low;
                target.highValue = high;
            }
            if (
                value.Value is not null
                || value.LowValue is not null
                || value.HighValue is not null
            )
                target.SetValueWithoutNotify(selected);
            if (value.Title is not null)
                target.title = value.Title;
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

        private static float ToNative(LowerLimit? value, float fallback) =>
            value switch
            {
                null => fallback,
                LowerLimit.Unbounded => float.MinValue,
                LowerLimit.Inclusive bounded => bounded.Value,
                _ => throw new InvalidOperationException("Unknown lower limit."),
            };

        private static float ToNative(UpperLimit? value, float fallback) =>
            value switch
            {
                null => fallback,
                UpperLimit.Unbounded => float.MaxValue,
                UpperLimit.Inclusive bounded => bounded.Value,
                _ => throw new InvalidOperationException("Unknown upper limit."),
            };

        private static FloatRange ToRange(Vector2 value) => new(value.x, value.y);

        private static bool TouchesRange(UiElement.MinMaxSlider value) =>
            value.MinValue is not null
            || value.MaxValue is not null
            || value.LowLimit is not null
            || value.HighLimit is not null;

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

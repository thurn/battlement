#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using ProtocolDirection = Battlement.UiSliderDirection;
using ProtocolNestedInteraction = Battlement.UiNestedInteraction;
using ProtocolScrollerVisibility = Battlement.UiScrollerVisibility;
using ProtocolScrollMode = Battlement.UiScrollViewMode;
using ProtocolTouchBehavior = Battlement.UiTouchScrollBehavior;
using UnityDirection = UnityEngine.UIElements.SliderDirection;
using UnityNestedInteraction = UnityEngine.UIElements.ScrollView.NestedInteractionKind;
using UnityScrollerVisibility = UnityEngine.UIElements.ScrollerVisibility;
using UnityScrollMode = UnityEngine.UIElements.ScrollViewMode;
using UnityTouchBehavior = UnityEngine.UIElements.ScrollView.TouchScrollBehavior;

namespace Battlement.UI
{
    internal sealed class BattlementUiScrollControls
    {
        private static readonly TimeSpan SettlementDelay = TimeSpan.FromMilliseconds(100);

        private readonly Dictionary<Guid, ScrollState> scrolls = new();
        private readonly Dictionary<Guid, ScrollerState> scrollers = new();
        private readonly BattlementUiEventForwarder events;
        private readonly Func<TimeSpan> now;

        public BattlementUiScrollControls(
            BattlementUiEventForwarder eventForwarder,
            Func<TimeSpan> currentTime
        ) => (events, now) = (eventForwarder, currentTime);

        public void ApplyCreate(VisualElement target, ObjectId objectId, UiElement value)
        {
            if (value is UiElement.ScrollView scroll)
            {
                var native = (ScrollView)target;
                ApplyScroll(native, scroll);
                scrolls.Add(objectId.Value, CreateScrollState(native, objectId));
            }
            if (value is UiElement.Scroller scroller)
            {
                var native = (Scroller)target;
                ApplyScroller(native, scroller);
                scrollers.Add(objectId.Value, CreateScrollerState(native, objectId));
            }
        }

        public void ApplyUpdate(VisualElement target, ObjectId objectId, UiElement value)
        {
            if (value is UiElement.ScrollView scroll)
            {
                ScrollState state = scrolls[objectId.Value];
                if (!scroll.ScrollOffset.IsUnset)
                    state.Cancel();
                state.CommandOrigin = true;
                try
                {
                    ApplyScroll((ScrollView)target, scroll);
                }
                finally
                {
                    state.CommandOrigin = false;
                }
                if (!target.enabledInHierarchy)
                    state.Cancel();
            }
            if (value is UiElement.Scroller scroller)
            {
                ScrollerState state = scrollers[objectId.Value];
                state.Cancel();
                state.CommandOrigin = true;
                try
                {
                    ApplyScroller((Scroller)target, scroller);
                    if (
                        !scroller.Value.IsUnset
                        || !scroller.LowValue.IsUnset
                        || !scroller.HighValue.IsUnset
                    )
                        state.Committed = ((Scroller)target).value;
                }
                finally
                {
                    state.CommandOrigin = false;
                }
                if (!target.enabledInHierarchy)
                    state.Cancel();
            }
        }

        public void Advance()
        {
            TimeSpan current = now();
            foreach (ScrollState state in new List<ScrollState>(scrolls.Values))
            {
                if (!state.Target.enabledInHierarchy || state.Target.panel is null)
                {
                    state.Cancel();
                    continue;
                }
                if (state.ChangedPending)
                {
                    state.ChangedPending = false;
                    events.ForwardScroll(state.ObjectId, UiEventKind.ScrollChanged, state.Latest);
                }
                if (!state.Armed || state.Captures.Count != 0)
                    continue;
                if (current - state.LastChanged < SettlementDelay)
                    continue;
                state.Armed = false;
                events.ForwardScroll(state.ObjectId, UiEventKind.ScrollSettled, state.Latest);
            }
            foreach (ScrollerState state in new List<ScrollerState>(scrollers.Values))
            {
                if (!state.Target.enabledInHierarchy || state.Target.panel is null)
                    state.Cancel();
            }
        }

        public void ScrollTo(ObjectId objectId, ScrollView target, VisualElement descendant)
        {
            ScrollState state = scrolls[objectId.Value];
            state.Cancel();
            state.CommandOrigin = true;
            try
            {
                target.ScrollTo(descendant);
            }
            finally
            {
                state.CommandOrigin = false;
            }
        }

        public void Remove(Guid objectId)
        {
            if (scrolls.Remove(objectId, out ScrollState scroll))
                scroll.Dispose();
            if (scrollers.Remove(objectId, out ScrollerState scroller))
                scroller.Dispose();
        }

        public void Clear()
        {
            foreach (ScrollState state in scrolls.Values)
                state.Dispose();
            foreach (ScrollerState state in scrollers.Values)
                state.Dispose();
            scrolls.Clear();
            scrollers.Clear();
        }

        public void CancelAll()
        {
            foreach (ScrollState state in scrolls.Values)
            {
                state.Cancel();
                ReleaseCaptures(state.Captures);
            }
            foreach (ScrollerState state in scrollers.Values)
            {
                state.Cancel();
                ReleaseCaptures(state.Captures);
            }
        }

        public static void ValidateUpdate(VisualElement target, UiElement value)
        {
            if (value is not UiElement.Scroller scroller)
                return;
            var native = (Scroller)target;
            var defaults = new Scroller();
            float low = Resolve(scroller.LowValue, native.lowValue, defaults.lowValue);
            float high = Resolve(scroller.HighValue, native.highValue, defaults.highValue);
            float selected = Resolve(scroller.Value, native.value, defaults.value);
            if (!float.IsFinite(low) || !float.IsFinite(high) || !float.IsFinite(selected))
                throw Failure("Scroller values must be finite.");
            if (low > high)
                throw Failure("Scroller limits are reversed.");
        }

        private ScrollState CreateScrollState(ScrollView target, ObjectId objectId)
        {
            var state = new ScrollState(target, objectId);
            state.ValueChanged = _ =>
            {
                if (state.CommandOrigin || !target.enabledInHierarchy)
                    return;
                state.Latest = target.scrollOffset;
                state.LastChanged = now();
                state.ChangedPending = true;
                state.Armed = true;
            };
            state.Capture = eventValue =>
            {
                if (eventValue.target is VisualElement owner)
                    state.Captures[eventValue.pointerId] = owner;
            };
            state.CaptureOut = eventValue => state.Captures.Remove(eventValue.pointerId);
            state.Detach = _ => state.Cancel();
            target.horizontalScroller.valueChanged += state.ValueChanged;
            target.verticalScroller.valueChanged += state.ValueChanged;
            target.RegisterCallback(state.Capture, TrickleDown.TrickleDown);
            target.RegisterCallback(state.CaptureOut, TrickleDown.TrickleDown);
            target.RegisterCallback(state.Detach);
            return state;
        }

        private ScrollerState CreateScrollerState(Scroller target, ObjectId objectId)
        {
            var state = new ScrollerState(target, objectId, target.value);
            state.ValueChanged = proposed =>
            {
                if (state.CommandOrigin || !target.enabledInHierarchy)
                    return;
                float previous = state.Committed;
                events.ForwardValueChanging(objectId, proposed);
                if (state.Interacting)
                    return;
                RestoreValue(state);
                events.ForwardValueCommitted(objectId, previous, proposed);
            };
            state.PointerDown = _ =>
            {
                if (target.enabledInHierarchy)
                    state.Interacting = true;
            };
            state.Capture = eventValue =>
            {
                if (target.enabledInHierarchy)
                {
                    state.Interacting = true;
                    if (eventValue.target is VisualElement owner)
                        state.Captures[eventValue.pointerId] = owner;
                }
            };
            state.PointerUp = _ => Commit(state);
            state.PointerCancel = _ => Restore(state);
            state.CaptureOut = eventValue =>
            {
                state.Captures.Remove(eventValue.pointerId);
                Commit(state);
            };
            state.Detach = _ => state.Cancel();
            target.valueChanged += state.ValueChanged;
            target.RegisterCallback(state.PointerDown, TrickleDown.TrickleDown);
            target.RegisterCallback(state.PointerUp, TrickleDown.TrickleDown);
            target.RegisterCallback(state.PointerCancel, TrickleDown.TrickleDown);
            target.slider.RegisterCallback(state.PointerDown);
            target.slider.RegisterCallback(state.Capture);
            target.slider.RegisterCallback(state.PointerUp);
            target.slider.RegisterCallback(state.PointerCancel);
            target.slider.RegisterCallback(state.CaptureOut);
            target.RegisterCallback(state.Detach);
            return state;
        }

        private void Commit(ScrollerState state)
        {
            if (!state.Interacting)
                return;
            float proposed = state.Target.value;
            state.Interacting = false;
            RestoreValue(state);
            events.ForwardValueCommitted(state.ObjectId, state.Committed, proposed);
        }

        private static void Restore(ScrollerState state)
        {
            if (!state.Interacting)
                return;
            state.Interacting = false;
            RestoreValue(state);
        }

        private static void RestoreValue(ScrollerState state)
        {
            state.CommandOrigin = true;
            try
            {
                state.Target.slider.SetValueWithoutNotify(state.Committed);
            }
            finally
            {
                state.CommandOrigin = false;
            }
        }

        private static void ApplyScroll(ScrollView target, UiElement.ScrollView value)
        {
            var defaults = new ScrollView();
            if (!value.Mode.IsUnset)
                target.mode = value.Mode.IsReset
                    ? defaults.mode
                    : value.Mode.Value switch
                    {
                        ProtocolScrollMode.Vertical => UnityScrollMode.Vertical,
                        ProtocolScrollMode.Horizontal => UnityScrollMode.Horizontal,
                        _ => UnityScrollMode.VerticalAndHorizontal,
                    };
            if (!value.NestedInteraction.IsUnset)
                target.nestedInteractionKind = value.NestedInteraction.IsReset
                    ? defaults.nestedInteractionKind
                    : value.NestedInteraction.Value switch
                    {
                        ProtocolNestedInteraction.StopScrolling =>
                            UnityNestedInteraction.StopScrolling,
                        ProtocolNestedInteraction.ForwardScrolling =>
                            UnityNestedInteraction.ForwardScrolling,
                        _ => UnityNestedInteraction.Default,
                    };
            if (!value.HorizontalScrollerVisibility.IsUnset)
                target.horizontalScrollerVisibility = value.HorizontalScrollerVisibility.IsReset
                    ? defaults.horizontalScrollerVisibility
                    : ToUnity(value.HorizontalScrollerVisibility.Value);
            if (!value.VerticalScrollerVisibility.IsUnset)
                target.verticalScrollerVisibility = value.VerticalScrollerVisibility.IsReset
                    ? defaults.verticalScrollerVisibility
                    : ToUnity(value.VerticalScrollerVisibility.Value);
            if (!value.ScrollOffset.IsUnset)
                target.scrollOffset = value.ScrollOffset.IsReset
                    ? defaults.scrollOffset
                    : new Vector2(value.ScrollOffset.Value.X, value.ScrollOffset.Value.Y);
            Apply(
                value.HorizontalPageSize,
                next => target.horizontalPageSize = next,
                defaults.horizontalPageSize
            );
            Apply(
                value.VerticalPageSize,
                next => target.verticalPageSize = next,
                defaults.verticalPageSize
            );
            Apply(
                value.MouseWheelScrollSize,
                next => target.mouseWheelScrollSize = next,
                defaults.mouseWheelScrollSize
            );
            if (!value.TouchScrollBehavior.IsUnset)
                target.touchScrollBehavior = value.TouchScrollBehavior.IsReset
                    ? defaults.touchScrollBehavior
                    : value.TouchScrollBehavior.Value switch
                    {
                        ProtocolTouchBehavior.Unrestricted => UnityTouchBehavior.Unrestricted,
                        ProtocolTouchBehavior.Elastic => UnityTouchBehavior.Elastic,
                        _ => UnityTouchBehavior.Clamped,
                    };
            Apply(
                value.ScrollDecelerationRate,
                next => target.scrollDecelerationRate = next,
                defaults.scrollDecelerationRate
            );
            Apply(value.Elasticity, next => target.elasticity = next, defaults.elasticity);
            if (value.ElasticAnimationInterval.IsSet)
                target.elasticAnimationIntervalMs = value.ElasticAnimationInterval.Value;
            else if (value.ElasticAnimationInterval.IsReset)
                target.elasticAnimationIntervalMs = defaults.elasticAnimationIntervalMs;
        }

        private static void ApplyScroller(Scroller target, UiElement.Scroller value)
        {
            var defaults = new Scroller();
            Apply(value.LowValue, next => target.lowValue = next, defaults.lowValue);
            Apply(value.HighValue, next => target.highValue = next, defaults.highValue);
            if (!value.Direction.IsUnset)
                target.direction =
                    value.Direction.IsReset ? defaults.direction
                    : value.Direction.Value == ProtocolDirection.Horizontal
                        ? UnityDirection.Horizontal
                    : UnityDirection.Vertical;
            if (!value.Value.IsUnset)
                target.slider.SetValueWithoutNotify(
                    value.Value.IsReset ? defaults.value : value.Value.Value
                );
        }

        private static void Apply<T>(Prop<T> value, Action<T> assign, T reset)
        {
            if (value.IsSet)
                assign(value.Value);
            else if (value.IsReset)
                assign(reset);
        }

        private static float Resolve(Prop<float> value, float current, float reset) =>
            value.IsSet ? value.Value
            : value.IsReset ? reset
            : current;

        private static BattlementUiException Failure(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private static UnityScrollerVisibility ToUnity(ProtocolScrollerVisibility value) =>
            value switch
            {
                ProtocolScrollerVisibility.AlwaysVisible => UnityScrollerVisibility.AlwaysVisible,
                ProtocolScrollerVisibility.Hidden => UnityScrollerVisibility.Hidden,
                _ => UnityScrollerVisibility.Auto,
            };

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

        private sealed class ScrollState : IDisposable
        {
            public ScrollState(ScrollView target, ObjectId objectId) =>
                (Target, ObjectId, Latest) = (target, objectId, target.scrollOffset);

            public ScrollView Target { get; }
            public ObjectId ObjectId { get; }
            public Dictionary<int, VisualElement> Captures { get; } = new();
            public Action<float> ValueChanged { get; set; } = null!;
            public EventCallback<PointerCaptureEvent> Capture { get; set; } = null!;
            public EventCallback<PointerCaptureOutEvent> CaptureOut { get; set; } = null!;
            public EventCallback<DetachFromPanelEvent> Detach { get; set; } = null!;
            public Vector2 Latest { get; set; }
            public TimeSpan LastChanged { get; set; }
            public bool ChangedPending { get; set; }
            public bool Armed { get; set; }
            public bool CommandOrigin { get; set; }

            public void Cancel() => (ChangedPending, Armed) = (false, false);

            public void Dispose()
            {
                Cancel();
                ReleaseCaptures(Captures);
                Target.horizontalScroller.valueChanged -= ValueChanged;
                Target.verticalScroller.valueChanged -= ValueChanged;
                Target.UnregisterCallback(Capture, TrickleDown.TrickleDown);
                Target.UnregisterCallback(CaptureOut, TrickleDown.TrickleDown);
                Target.UnregisterCallback(Detach);
            }
        }

        private sealed class ScrollerState : IDisposable
        {
            public ScrollerState(Scroller target, ObjectId objectId, float committed) =>
                (Target, ObjectId, Committed) = (target, objectId, committed);

            public Scroller Target { get; }
            public ObjectId ObjectId { get; }
            public float Committed { get; set; }
            public bool CommandOrigin { get; set; }
            public bool Interacting { get; set; }
            public Dictionary<int, VisualElement> Captures { get; } = new();
            public Action<float> ValueChanged { get; set; } = null!;
            public EventCallback<PointerDownEvent> PointerDown { get; set; } = null!;
            public EventCallback<PointerCaptureEvent> Capture { get; set; } = null!;
            public EventCallback<PointerUpEvent> PointerUp { get; set; } = null!;
            public EventCallback<PointerCancelEvent> PointerCancel { get; set; } = null!;
            public EventCallback<PointerCaptureOutEvent> CaptureOut { get; set; } = null!;
            public EventCallback<DetachFromPanelEvent> Detach { get; set; } = null!;

            public void Cancel()
            {
                if (Interacting)
                    RestoreValue(this);
                Interacting = false;
            }

            public void Dispose()
            {
                Cancel();
                ReleaseCaptures(Captures);
                Target.valueChanged -= ValueChanged;
                Target.UnregisterCallback(PointerDown, TrickleDown.TrickleDown);
                Target.UnregisterCallback(PointerUp, TrickleDown.TrickleDown);
                Target.UnregisterCallback(PointerCancel, TrickleDown.TrickleDown);
                Target.slider.UnregisterCallback(PointerDown);
                Target.slider.UnregisterCallback(Capture);
                Target.slider.UnregisterCallback(PointerUp);
                Target.slider.UnregisterCallback(PointerCancel);
                Target.slider.UnregisterCallback(CaptureOut);
                Target.UnregisterCallback(Detach);
            }
        }
    }
}

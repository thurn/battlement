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
                ApplyScroller(native, scroller, commandOrigin: true);
                scrollers.Add(objectId.Value, CreateScrollerState(native, objectId));
            }
        }

        public void ApplyUpdate(VisualElement target, ObjectId objectId, UiElement value)
        {
            if (value is UiElement.ScrollView scroll)
            {
                ScrollState state = scrolls[objectId.Value];
                if (scroll.ScrollOffset is not null)
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
                    ApplyScroller((Scroller)target, scroller, commandOrigin: true);
                    if (
                        scroller.Value is not null
                        || scroller.LowValue is not null
                        || scroller.HighValue is not null
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
                if (!state.Armed || state.CapturedPointers.Count != 0)
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
            state.Capture = eventValue => state.CapturedPointers.Add(eventValue.pointerId);
            state.CaptureOut = eventValue => state.CapturedPointers.Remove(eventValue.pointerId);
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
            state.Capture = _ =>
            {
                if (target.enabledInHierarchy)
                    state.Interacting = true;
            };
            state.PointerUp = _ => Commit(state);
            state.PointerCancel = _ => Restore(state);
            state.CaptureOut = _ => Commit(state);
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
            if (value.Mode is ProtocolScrollMode mode)
                target.mode = mode switch
                {
                    ProtocolScrollMode.Vertical => UnityScrollMode.Vertical,
                    ProtocolScrollMode.Horizontal => UnityScrollMode.Horizontal,
                    _ => UnityScrollMode.VerticalAndHorizontal,
                };
            if (value.NestedInteraction is ProtocolNestedInteraction nested)
                target.nestedInteractionKind = nested switch
                {
                    ProtocolNestedInteraction.StopScrolling => UnityNestedInteraction.StopScrolling,
                    ProtocolNestedInteraction.ForwardScrolling =>
                        UnityNestedInteraction.ForwardScrolling,
                    _ => UnityNestedInteraction.Default,
                };
            if (value.HorizontalScrollerVisibility is ProtocolScrollerVisibility horizontal)
                target.horizontalScrollerVisibility = ToUnity(horizontal);
            if (value.VerticalScrollerVisibility is ProtocolScrollerVisibility vertical)
                target.verticalScrollerVisibility = ToUnity(vertical);
            if (value.ScrollOffset is Battlement.Vector offset)
                target.scrollOffset = new Vector2(offset.X, offset.Y);
            if (value.HorizontalPageSize is float horizontalPageSize)
                target.horizontalPageSize = horizontalPageSize;
            if (value.VerticalPageSize is float verticalPageSize)
                target.verticalPageSize = verticalPageSize;
            if (value.MouseWheelScrollSize is float wheelSize)
                target.mouseWheelScrollSize = wheelSize;
            if (value.TouchScrollBehavior is ProtocolTouchBehavior touch)
                target.touchScrollBehavior = touch switch
                {
                    ProtocolTouchBehavior.Unrestricted => UnityTouchBehavior.Unrestricted,
                    ProtocolTouchBehavior.Elastic => UnityTouchBehavior.Elastic,
                    _ => UnityTouchBehavior.Clamped,
                };
            if (value.ScrollDecelerationRate is float deceleration)
                target.scrollDecelerationRate = deceleration;
            if (value.Elasticity is float elasticity)
                target.elasticity = elasticity;
            if (value.ElasticAnimationInterval is uint interval)
                target.elasticAnimationIntervalMs = interval;
        }

        private static void ApplyScroller(
            Scroller target,
            UiElement.Scroller value,
            bool commandOrigin
        )
        {
            if (value.LowValue is float low)
                target.lowValue = low;
            if (value.HighValue is float high)
                target.highValue = high;
            if (value.Direction is ProtocolDirection direction)
                target.direction =
                    direction == ProtocolDirection.Horizontal
                        ? UnityDirection.Horizontal
                        : UnityDirection.Vertical;
            if (value.Value is float next)
                target.slider.SetValueWithoutNotify(next);
        }

        private static UnityScrollerVisibility ToUnity(ProtocolScrollerVisibility value) =>
            value switch
            {
                ProtocolScrollerVisibility.AlwaysVisible => UnityScrollerVisibility.AlwaysVisible,
                ProtocolScrollerVisibility.Hidden => UnityScrollerVisibility.Hidden,
                _ => UnityScrollerVisibility.Auto,
            };

        private sealed class ScrollState : IDisposable
        {
            public ScrollState(ScrollView target, ObjectId objectId) =>
                (Target, ObjectId, Latest) = (target, objectId, target.scrollOffset);

            public ScrollView Target { get; }
            public ObjectId ObjectId { get; }
            public HashSet<int> CapturedPointers { get; } = new();
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

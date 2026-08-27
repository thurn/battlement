#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine.UIElements;
using NativeRepeatButton = UnityEngine.UIElements.RepeatButton;

namespace Battlement.UI
{
    internal sealed class BattlementUiRepeatControls
    {
        private readonly Dictionary<Guid, System.Action> actions = new();
        private readonly Dictionary<Guid, (long Delay, long Interval)> timings = new();
        private readonly Dictionary<Guid, (long Delay, long Interval)> pendingTimings = new();
        private readonly Dictionary<Guid, Dictionary<int, VisualElement>> captures = new();
        private readonly HashSet<Guid> pressed = new();
        private readonly BattlementUiEventForwarder events;
        private readonly Func<Guid, IReadOnlyList<Guid>> route;

        public BattlementUiRepeatControls(
            BattlementUiEventForwarder eventForwarder,
            Func<Guid, IReadOnlyList<Guid>> routeFor
        )
        {
            events = eventForwarder;
            route = routeFor;
        }

        public NativeRepeatButton Create(ObjectId objectId, UiElement.RepeatButton value)
        {
            if (value.DelayMs is not uint delay || value.IntervalMs is not uint interval)
                throw Failure("RepeatButton creation requires delay and interval.");
            if (interval == 0)
                throw Failure("RepeatButton interval must be positive.");
            System.Action callback = () => events.ForwardRepeat(route(objectId.Value));
            actions.Add(objectId.Value, callback);
            timings.Add(objectId.Value, (delay, interval));
            captures.Add(objectId.Value, new Dictionary<int, VisualElement>());
            var result = new NativeRepeatButton(callback, delay, interval)
            {
                text = value.Text ?? string.Empty,
            };
            result.RegisterCallback<PointerDownEvent>(
                _ => pressed.Add(objectId.Value),
                TrickleDown.TrickleDown
            );
            result.RegisterCallback<PointerUpEvent>(_ => Release(result, objectId.Value));
            result.RegisterCallback<PointerCaptureEvent>(eventValue =>
            {
                if (
                    captures.TryGetValue(objectId.Value, out Dictionary<int, VisualElement> owned)
                    && eventValue.target is VisualElement owner
                )
                    owned[eventValue.pointerId] = owner;
            });
            result.RegisterCallback<PointerCaptureOutEvent>(eventValue =>
            {
                if (captures.TryGetValue(objectId.Value, out Dictionary<int, VisualElement> owned))
                    owned.Remove(eventValue.pointerId);
            });
            return result;
        }

        public void ApplyUpdate(
            NativeRepeatButton target,
            ObjectId objectId,
            UiElement.RepeatButton value
        )
        {
            if (value.DelayMs is null && value.IntervalMs is null)
                return;
            (long delay, long interval) = pendingTimings.TryGetValue(
                objectId.Value,
                out (long Delay, long Interval) pending
            )
                ? pending
                : timings[objectId.Value];
            delay = value.DelayMs is uint nextDelay ? nextDelay : delay;
            interval = value.IntervalMs is uint nextInterval ? nextInterval : interval;
            if (interval <= 0)
                throw Failure("RepeatButton interval must be positive.");
            if (pressed.Contains(objectId.Value))
            {
                pendingTimings[objectId.Value] = (delay, interval);
                return;
            }
            target.SetAction(actions[objectId.Value], delay, interval);
            timings[objectId.Value] = (delay, interval);
        }

        public void Remove(Guid objectId)
        {
            actions.Remove(objectId);
            timings.Remove(objectId);
            pendingTimings.Remove(objectId);
            pressed.Remove(objectId);
            if (captures.Remove(objectId, out Dictionary<int, VisualElement> owned))
                ReleaseCaptures(owned);
        }

        public void Clear()
        {
            actions.Clear();
            timings.Clear();
            pendingTimings.Clear();
            pressed.Clear();
            foreach (Dictionary<int, VisualElement> owned in captures.Values)
                ReleaseCaptures(owned);
            captures.Clear();
        }

        public void CancelAll()
        {
            pressed.Clear();
            pendingTimings.Clear();
            foreach (Dictionary<int, VisualElement> owned in captures.Values)
                ReleaseCaptures(owned);
        }

        private void Release(NativeRepeatButton target, Guid objectId)
        {
            pressed.Remove(objectId);
            if (!pendingTimings.Remove(objectId, out (long Delay, long Interval) timing))
                return;
            long previousInterval = timings[objectId].Interval;
            target
                .schedule.Execute(() =>
                {
                    target.SetAction(actions[objectId], timing.Delay, timing.Interval);
                    timings[objectId] = timing;
                })
                .StartingIn(previousInterval + 1);
        }

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

        private static BattlementUiException Failure(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}

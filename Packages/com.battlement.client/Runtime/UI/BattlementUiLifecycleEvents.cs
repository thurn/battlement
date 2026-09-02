#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine.UIElements;
using UnityAttachToPanelEvent = UnityEngine.UIElements.AttachToPanelEvent;
using UnityDetachFromPanelEvent = UnityEngine.UIElements.DetachFromPanelEvent;
using UnityGeometryChangedEvent = UnityEngine.UIElements.GeometryChangedEvent;
using UnityPointerDownLinkTagEvent = UnityEngine.UIElements.Experimental.PointerDownLinkTagEvent;
using UnityPointerOutLinkTagEvent = UnityEngine.UIElements.Experimental.PointerOutLinkTagEvent;
using UnityPointerOverLinkTagEvent = UnityEngine.UIElements.Experimental.PointerOverLinkTagEvent;
using UnityPointerUpLinkTagEvent = UnityEngine.UIElements.Experimental.PointerUpLinkTagEvent;

namespace Battlement.UI
{
    internal sealed class BattlementUiLifecycleEvents
    {
        private readonly Dictionary<(Guid ObjectId, int PointerId), LinkIdentity> links = new();
        private readonly Dictionary<Guid, SelectionState> selections = new();
        private readonly HashSet<Guid> activeTargets = new();
        private readonly BattlementUiEventForwarder events;
        private readonly Func<Guid, IReadOnlyList<Guid>> route;
        private bool inputEnabled = true;

        public BattlementUiLifecycleEvents(
            BattlementUiEventForwarder eventForwarder,
            Func<Guid, IReadOnlyList<Guid>> logicalRoute
        ) => (events, route) = (eventForwarder, logicalRoute);

        public int LinkIdentityCount => links.Count;

        public void Register(ObjectId objectId, VisualElement target)
        {
            activeTargets.Add(objectId.Value);
            target.RegisterCallback<UnityGeometryChangedEvent>(eventValue =>
            {
                if (eventValue.target == target)
                    ForwardGeometry(objectId, eventValue.oldRect, eventValue.newRect);
            });
            target.RegisterCallback<UnityAttachToPanelEvent>(eventValue =>
            {
                if (eventValue.target == target)
                    ForwardLifecycle(objectId, UiEventKind.AttachToPanel);
            });
            target.RegisterCallback<UnityDetachFromPanelEvent>(eventValue =>
            {
                if (eventValue.target != target)
                    return;
                ForwardLifecycle(objectId, UiEventKind.DetachFromPanel);
                RemoveLinks(objectId.Value);
            });
            target.RegisterCallback<UnityPointerOverLinkTagEvent>(eventValue =>
                ForwardEnter(objectId, target, eventValue)
            );
            target.RegisterCallback<UnityPointerOutLinkTagEvent>(eventValue =>
                ForwardLeave(objectId, target, eventValue)
            );
            target.RegisterCallback<UnityPointerDownLinkTagEvent>(eventValue =>
                ForwardButton(
                    objectId,
                    target,
                    UiEventKind.LinkDown,
                    eventValue,
                    eventValue,
                    eventValue.linkID,
                    eventValue.linkText
                )
            );
            target.RegisterCallback<UnityPointerUpLinkTagEvent>(eventValue =>
                ForwardButton(
                    objectId,
                    target,
                    UiEventKind.LinkUp,
                    eventValue,
                    eventValue,
                    eventValue.linkID,
                    eventValue.linkText
                )
            );
            if (target is UnityEngine.UIElements.TextElement text)
            {
                var state = new SelectionState(objectId, text);
                state.CursorChanged = () => state.Pending = true;
                state.SelectChanged = () => state.Pending = true;
                state.Selection.OnCursorIndexChange += state.CursorChanged;
                state.Selection.OnSelectIndexChange += state.SelectChanged;
                selections.Add(objectId.Value, state);
            }
        }

        public void Advance()
        {
            foreach (SelectionState state in selections.Values)
            {
                if (!state.Pending)
                    continue;
                state.Pending = false;
                events.ForwardEvent(
                    state.ObjectId,
                    route(state.ObjectId.Value),
                    UiEventKind.SelectionChanged,
                    new UiEventBody.SelectionChanged(
                        new SelectionEvent(
                            checked((uint)state.Selection.cursorIndex),
                            checked((uint)state.Selection.selectIndex)
                        )
                    ),
                    targetOnly: true
                );
            }
        }

        public void Remove(Guid objectId)
        {
            activeTargets.Remove(objectId);
            RemoveLinks(objectId);
            if (selections.Remove(objectId, out SelectionState state))
                state.Dispose();
        }

        public void Reset()
        {
            links.Clear();
            foreach (SelectionState state in selections.Values)
                state.Pending = false;
        }

        public void SetInputEnabled(bool enabled)
        {
            inputEnabled = enabled;
            if (!enabled)
                Reset();
        }

        public void Clear()
        {
            Reset();
            foreach (SelectionState state in selections.Values)
                state.Dispose();
            selections.Clear();
            activeTargets.Clear();
        }

        private void ForwardGeometry(
            ObjectId objectId,
            UnityEngine.Rect previous,
            UnityEngine.Rect current
        )
        {
            if (!Finite(previous) || !Finite(current))
                return;
            events.ForwardEvent(
                objectId,
                route(objectId.Value),
                UiEventKind.GeometryChanged,
                new UiEventBody.GeometryChanged(
                    new GeometryEvent(ToRect(previous), ToRect(current))
                ),
                targetOnly: true
            );
        }

        private void ForwardLifecycle(ObjectId objectId, UiEventKind kind)
        {
            UiEventBody body = kind switch
            {
                UiEventKind.AttachToPanel => new UiEventBody.AttachToPanel(new LifecycleEvent()),
                UiEventKind.DetachFromPanel => new UiEventBody.DetachFromPanel(
                    new LifecycleEvent()
                ),
                _ => throw new InvalidOperationException("Unknown panel lifecycle event kind."),
            };
            events.ForwardEvent(objectId, route(objectId.Value), kind, body, targetOnly: true);
        }

        private void ForwardEnter(
            ObjectId objectId,
            VisualElement target,
            UnityPointerOverLinkTagEvent eventValue
        )
        {
            if (
                eventValue.target != target
                || !inputEnabled
                || !activeTargets.Contains(objectId.Value)
            )
                return;
            var identity = new LinkIdentity(eventValue.linkID, eventValue.linkText);
            if (
                ForwardLink(objectId, UiEventKind.LinkEnter, eventValue, identity, null, eventValue)
            )
                links[(objectId.Value, eventValue.pointerId)] = identity;
        }

        private void ForwardLeave(
            ObjectId objectId,
            VisualElement target,
            UnityPointerOutLinkTagEvent eventValue
        )
        {
            if (eventValue.target != target || !inputEnabled)
                return;
            var key = (objectId.Value, eventValue.pointerId);
            if (!links.Remove(key, out LinkIdentity identity))
                return;
            ForwardLink(objectId, UiEventKind.LinkLeave, eventValue, identity, null, eventValue);
        }

        private void ForwardButton(
            ObjectId objectId,
            VisualElement target,
            UiEventKind kind,
            EventBase eventBase,
            IPointerEvent eventValue,
            string linkId,
            string linkText
        )
        {
            if (eventBase.target != target || !inputEnabled)
                return;
            ForwardLink(
                objectId,
                kind,
                eventValue,
                new LinkIdentity(linkId, linkText),
                ToPointerButton(eventValue.button),
                eventBase
            );
        }

        private bool ForwardLink(
            ObjectId objectId,
            UiEventKind kind,
            IPointerEvent eventValue,
            LinkIdentity identity,
            UiPointerButton? button,
            EventBase nativeEvent
        )
        {
            if (!float.IsFinite(eventValue.position.x) || !float.IsFinite(eventValue.position.y))
                return false;
            var value = new LinkEvent(
                identity.Id,
                identity.Text,
                new PanelPoint(eventValue.position.x, eventValue.position.y),
                eventValue.pointerId,
                button
            );
            UiEventBody body = kind switch
            {
                UiEventKind.LinkEnter => new UiEventBody.LinkEnter(value),
                UiEventKind.LinkLeave => new UiEventBody.LinkLeave(value),
                UiEventKind.LinkDown => new UiEventBody.LinkDown(value),
                UiEventKind.LinkUp => new UiEventBody.LinkUp(value),
                _ => throw new InvalidOperationException("Unknown link event kind."),
            };
            return events.ForwardEvent(
                objectId,
                route(objectId.Value),
                kind,
                body,
                nativeEvent: nativeEvent
            );
        }

        private static Battlement.Rect ToRect(UnityEngine.Rect value) =>
            new(value.x, value.y, value.width, value.height);

        private static bool Finite(UnityEngine.Rect value) =>
            float.IsFinite(value.x)
            && float.IsFinite(value.y)
            && float.IsFinite(value.width)
            && float.IsFinite(value.height);

        private static UiPointerButton? ToPointerButton(int value) =>
            value switch
            {
                0 => new UiPointerButton.Left(),
                1 => new UiPointerButton.Right(),
                2 => new UiPointerButton.Middle(),
                >= 3 => new UiPointerButton.Other(value),
                _ => null,
            };

        private void RemoveLinks(Guid objectId)
        {
            foreach ((Guid ObjectId, int PointerId) key in new List<(Guid, int)>(links.Keys))
            {
                if (key.ObjectId == objectId)
                    links.Remove(key);
            }
        }

        private sealed class SelectionState : IDisposable
        {
            public SelectionState(ObjectId objectId, UnityEngine.UIElements.TextElement target) =>
                (ObjectId, Selection) = (objectId, target);

            public ObjectId ObjectId { get; }
            public ITextSelection Selection { get; }
            public bool Pending { get; set; }
            public System.Action CursorChanged { get; set; } = null!;
            public System.Action SelectChanged { get; set; } = null!;

            public void Dispose()
            {
                Selection.OnCursorIndexChange -= CursorChanged;
                Selection.OnSelectIndexChange -= SelectChanged;
            }
        }

        private sealed record LinkIdentity(string Id, string Text);
    }
}

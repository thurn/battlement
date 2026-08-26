#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using UnityClickEvent = UnityEngine.UIElements.ClickEvent;

namespace Battlement.UI
{
    internal sealed class BattlementUiEventForwarder
    {
        private readonly Dictionary<Guid, HashSet<UiEventKind>> subscriptions = new();
        private readonly Func<UiEvent, bool>? emit;

        public BattlementUiEventForwarder(Func<UiEvent, bool>? emitUiEvent) => emit = emitUiEvent;

        public void SetSubscriptions(Guid objectId, IReadOnlyList<UiEventKind>? values) =>
            subscriptions[objectId] = new HashSet<UiEventKind>(
                values ?? Array.Empty<UiEventKind>()
            );

        public void ForwardClick(ObjectId objectId, UnityClickEvent eventValue)
        {
            if (emit is null || !IsSubscribed(objectId.Value, UiEventKind.Click))
                return;
            emit(
                new UiEvent(
                    objectId,
                    new UiEventBody.Click(
                        new Battlement.ClickEvent.Pointer(
                            new PanelPoint(eventValue.position.x, eventValue.position.y),
                            checked((uint)Math.Max(1, eventValue.clickCount)),
                            eventValue.pointerId,
                            ToPointerButton(eventValue.button),
                            ToModifiers(eventValue.modifiers)
                        )
                    )
                )
            );
        }

        public void ForwardNavigationSubmit(IReadOnlyList<Guid> route, bool buttonTarget)
        {
            if (emit is null)
                return;

            // A Button turns Unity's NavigationSubmitEvent into the same logical activation as a
            // pointer click. Forward it to the nearest Click subscription on the Button's route
            // so application code can handle every activation method once.
            if (buttonTarget && TrySubscribed(route, UiEventKind.Click, out Guid clickTarget))
            {
                emit(
                    new UiEvent(
                        new ObjectId(clickTarget),
                        new UiEventBody.Click(new Battlement.ClickEvent.NavigationSubmit())
                    )
                );
            }
        }

        public void ForwardRepeat(IReadOnlyList<Guid> route)
        {
            if (emit is null || !TrySubscribed(route, UiEventKind.Click, out Guid target))
                return;
            emit(
                new UiEvent(
                    new ObjectId(target),
                    new UiEventBody.Click(new Battlement.ClickEvent.Repeat())
                )
            );
        }

        public void ForwardTransition(
            ObjectId objectId,
            UiEventKind kind,
            IEnumerable<StylePropertyName> propertyNames,
            double elapsedSeconds
        )
        {
            if (emit is null || !IsSubscribed(objectId.Value, kind))
                return;
            var properties = new List<UiTransitionProperty>();
            foreach (StylePropertyName propertyName in propertyNames)
            {
                if (
                    BattlementUiStyleTransformProperties.TryFromUnity(
                        propertyName,
                        out UiTransitionProperty property
                    )
                )
                {
                    properties.Add(property);
                }
            }
            float elapsedMs = (float)(elapsedSeconds * 1_000.0);
            if (properties.Count == 0 || !float.IsFinite(elapsedMs))
                return;
            var transition = new TransitionEvent(properties, elapsedMs);
            UiEventBody body = kind switch
            {
                UiEventKind.TransitionStart => new UiEventBody.TransitionStart(transition),
                UiEventKind.TransitionEnd => new UiEventBody.TransitionEnd(transition),
                UiEventKind.TransitionCancel => new UiEventBody.TransitionCancel(transition),
                _ => throw new InvalidOperationException("Unknown transition event kind."),
            };
            emit(new UiEvent(objectId, body));
        }

        public bool ForwardValueChanging(ObjectId objectId, float proposed)
        {
            if (emit is null || !IsSubscribed(objectId.Value, UiEventKind.ValueChanging))
                return false;
            return emit(
                new UiEvent(
                    objectId,
                    new UiEventBody.ValueChanging(new ValueChangingEvent(new UiValue.F32(proposed)))
                )
            );
        }

        public bool ForwardValueCommitted(ObjectId objectId, float previous, float proposed)
        {
            if (emit is null || !IsSubscribed(objectId.Value, UiEventKind.ValueCommitted))
                return false;
            return emit(
                new UiEvent(
                    objectId,
                    new UiEventBody.ValueCommitted(
                        new ValueCommitEvent(new UiValue.F32(previous), new UiValue.F32(proposed))
                    )
                )
            );
        }

        public bool ForwardValueCommitted(ObjectId objectId, bool previous, bool proposed) =>
            Emit(
                objectId,
                UiEventKind.ValueCommitted,
                new UiEventBody.ValueCommitted(
                    new ValueCommitEvent(new UiValue.Bool(previous), new UiValue.Bool(proposed))
                )
            );

        public bool ForwardInput(ObjectId objectId, string value) =>
            Emit(objectId, UiEventKind.Input, new UiEventBody.Input(new TextInputEvent(value)));

        public bool ForwardValueCommitted(ObjectId objectId, string previous, string proposed) =>
            Emit(
                objectId,
                UiEventKind.ValueCommitted,
                new UiEventBody.ValueCommitted(
                    new ValueCommitEvent(new UiValue.String(previous), new UiValue.String(proposed))
                )
            );

        public bool ForwardSelectionChanged(ObjectId objectId, int cursorIndex, int selectIndex) =>
            Emit(
                objectId,
                UiEventKind.SelectionChanged,
                new UiEventBody.SelectionChanged(
                    new TextSelectionEvent(checked((uint)cursorIndex), checked((uint)selectIndex))
                )
            );

        public bool ForwardScroll(ObjectId objectId, UiEventKind kind, Vector2 offset)
        {
            if (emit is null || !IsSubscribed(objectId.Value, kind))
                return false;
            var value = new ScrollEvent(new Battlement.Vector(offset.x, offset.y));
            UiEventBody body = kind switch
            {
                UiEventKind.ScrollChanged => new UiEventBody.ScrollChanged(value),
                UiEventKind.ScrollSettled => new UiEventBody.ScrollSettled(value),
                _ => throw new InvalidOperationException("Unknown scroll event kind."),
            };
            return emit(new UiEvent(objectId, body));
        }

        public bool ForwardTabSelection(
            ObjectId objectId,
            int previousIndex,
            int proposedIndex,
            ObjectId proposedTabId
        ) =>
            Emit(
                objectId,
                UiEventKind.TabSelectionRequested,
                new UiEventBody.TabSelectionRequested(
                    new TabSelectionEvent(
                        checked((uint)previousIndex),
                        checked((uint)proposedIndex),
                        proposedTabId
                    )
                )
            );

        public bool ForwardTabClose(ObjectId objectId, ObjectId tabId, int index) =>
            Emit(
                objectId,
                UiEventKind.TabCloseRequested,
                new UiEventBody.TabCloseRequested(new TabCloseEvent(tabId, checked((uint)index)))
            );

        public bool ForwardTabReorder(
            ObjectId objectId,
            ObjectId tabId,
            int previousIndex,
            int proposedIndex
        ) =>
            Emit(
                objectId,
                UiEventKind.TabReorderRequested,
                new UiEventBody.TabReorderRequested(
                    new TabReorderEvent(
                        tabId,
                        checked((uint)previousIndex),
                        checked((uint)proposedIndex)
                    )
                )
            );

        public bool IsSubscribed(ObjectId objectId, UiEventKind kind) =>
            IsSubscribed(objectId.Value, kind);

        public void Remove(Guid objectId) => subscriptions.Remove(objectId);

        public void Clear() => subscriptions.Clear();

        private bool IsSubscribed(Guid objectId, UiEventKind kind) =>
            subscriptions.TryGetValue(objectId, out HashSet<UiEventKind> values)
            && values.Contains(kind);

        private bool Emit(ObjectId objectId, UiEventKind kind, UiEventBody body)
        {
            if (emit is null || !IsSubscribed(objectId.Value, kind))
                return false;
            return emit(new UiEvent(objectId, body));
        }

        private bool TrySubscribed(IReadOnlyList<Guid> route, UiEventKind kind, out Guid target)
        {
            foreach (Guid objectId in route)
            {
                if (IsSubscribed(objectId, kind))
                {
                    target = objectId;
                    return true;
                }
            }
            target = Guid.Empty;
            return false;
        }

        private static PointerButton ToPointerButton(int value) =>
            value switch
            {
                1 => PointerButton.Right,
                2 => PointerButton.Middle,
                _ => PointerButton.Left,
            };

        private static IReadOnlyList<KeyModifier> ToModifiers(EventModifiers values)
        {
            var result = new List<KeyModifier>();
            if ((values & EventModifiers.Alt) != 0)
                result.Add(KeyModifier.Alt);
            if ((values & EventModifiers.Control) != 0)
                result.Add(KeyModifier.Control);
            if ((values & EventModifiers.Command) != 0)
                result.Add(KeyModifier.Command);
            if ((values & EventModifiers.Shift) != 0)
                result.Add(KeyModifier.Shift);
            return result;
        }
    }
}

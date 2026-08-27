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
        private readonly Dictionary<Guid, SubscriptionState> subscriptions = new();
        private readonly Func<UiEvent, bool>? emit;

        public BattlementUiEventForwarder(Func<UiEvent, bool>? emitUiEvent) => emit = emitUiEvent;

        public void SetSubscriptions(
            Guid objectId,
            IReadOnlyList<UiEventKind>? targetValues,
            IReadOnlyList<UiEventSubscription>? routedValues,
            bool sparse
        )
        {
            if (!subscriptions.TryGetValue(objectId, out SubscriptionState state))
            {
                state = new SubscriptionState();
                subscriptions.Add(objectId, state);
            }
            if (!sparse || targetValues is not null)
            {
                state.Target = new HashSet<UiEventKind>(targetValues ?? Array.Empty<UiEventKind>());
            }
            if (!sparse || routedValues is not null)
            {
                state.Routed = new HashSet<UiEventSubscription>(
                    routedValues ?? Array.Empty<UiEventSubscription>()
                );
            }
        }

        public void ForwardClick(
            ObjectId objectId,
            IReadOnlyList<Guid> route,
            UnityClickEvent eventValue
        )
        {
            if (!CanForward(route, UiEventKind.Click))
                return;
            emit?.Invoke(
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

        public void ForwardPointerButton(
            ObjectId objectId,
            IReadOnlyList<Guid> route,
            UiEventKind kind,
            IPointerEvent eventValue
        )
        {
            var value = new UiPointerButtonEvent(
                Position(eventValue.position),
                Delta(eventValue.deltaPosition),
                eventValue.pointerId,
                ToPointerButton(eventValue.button),
                checked((uint)Math.Max(0, eventValue.pressedButtons)),
                eventValue.pressure,
                checked((uint)Math.Max(1, eventValue.clickCount)),
                ToModifiers(eventValue.modifiers),
                ToPointerType(eventValue.pointerType)
            );
            UiEventBody body = kind switch
            {
                UiEventKind.PointerDown => new UiEventBody.PointerDown(value),
                UiEventKind.PointerUp => new UiEventBody.PointerUp(value),
                _ => throw new InvalidOperationException("Unknown pointer-button event kind."),
            };
            EmitRouted(objectId, route, kind, body);
        }

        public void ForwardPointerMove(
            ObjectId objectId,
            IReadOnlyList<Guid> route,
            IPointerEvent eventValue
        ) =>
            EmitRouted(
                objectId,
                route,
                UiEventKind.PointerMove,
                new UiEventBody.PointerMove(
                    new UiPointerMoveEvent(
                        Position(eventValue.position),
                        Delta(eventValue.deltaPosition),
                        eventValue.pointerId,
                        eventValue.button < 0 ? null : ToPointerButton(eventValue.button),
                        checked((uint)Math.Max(0, eventValue.pressedButtons)),
                        eventValue.pressure,
                        checked((uint)Math.Max(0, eventValue.clickCount)),
                        ToModifiers(eventValue.modifiers),
                        ToPointerType(eventValue.pointerType)
                    )
                )
            );

        public void ForwardPointerCancel(
            ObjectId objectId,
            IReadOnlyList<Guid> route,
            IPointerEvent eventValue
        ) =>
            EmitRouted(
                objectId,
                route,
                UiEventKind.PointerCancel,
                new UiEventBody.PointerCancel(
                    new UiPointerCancelEvent(
                        Position(eventValue.position),
                        Delta(eventValue.deltaPosition),
                        eventValue.pointerId,
                        checked((uint)Math.Max(0, eventValue.pressedButtons)),
                        eventValue.pressure,
                        ToModifiers(eventValue.modifiers),
                        ToPointerType(eventValue.pointerType)
                    )
                )
            );

        public void ForwardPointerBoundary(
            ObjectId objectId,
            IReadOnlyList<Guid> route,
            UiEventKind kind,
            IPointerEvent eventValue
        )
        {
            var value = new UiPointerBoundaryEvent(
                Position(eventValue.position),
                eventValue.pointerId,
                ToPointerType(eventValue.pointerType)
            );
            UiEventBody body = kind switch
            {
                UiEventKind.PointerEnter => new UiEventBody.PointerEnter(value),
                UiEventKind.PointerLeave => new UiEventBody.PointerLeave(value),
                _ => throw new InvalidOperationException("Unknown pointer-boundary event kind."),
            };
            EmitRouted(objectId, route, kind, body, targetOnly: true);
        }

        public void ForwardPointerCrossing(
            ObjectId objectId,
            IReadOnlyList<Guid> route,
            UiEventKind kind,
            IPointerEvent eventValue
        )
        {
            var value = new UiPointerCrossingEvent(
                Position(eventValue.position),
                eventValue.pointerId,
                ToPointerType(eventValue.pointerType)
            );
            UiEventBody body = kind switch
            {
                UiEventKind.PointerOver => new UiEventBody.PointerOver(value),
                UiEventKind.PointerOut => new UiEventBody.PointerOut(value),
                _ => throw new InvalidOperationException("Unknown pointer-crossing event kind."),
            };
            EmitRouted(objectId, route, kind, body);
        }

        public void ForwardWheel(
            ObjectId objectId,
            IReadOnlyList<Guid> route,
            UnityEngine.UIElements.WheelEvent eventValue
        ) =>
            EmitRouted(
                objectId,
                route,
                UiEventKind.Wheel,
                new UiEventBody.Wheel(
                    new UiWheelEvent(
                        new PanelPoint(eventValue.mousePosition.x, eventValue.mousePosition.y),
                        new Battlement.UiVector3(
                            eventValue.delta.x,
                            eventValue.delta.y,
                            eventValue.delta.z
                        ),
                        ToModifiers(eventValue.modifiers)
                    )
                )
            );

        public void ForwardPointerCapture(
            ObjectId objectId,
            IReadOnlyList<Guid> route,
            UiEventKind kind,
            int pointerId
        )
        {
            var value = new UiPointerCaptureEvent(pointerId);
            UiEventBody body = kind switch
            {
                UiEventKind.PointerCapture => new UiEventBody.PointerCapture(value),
                UiEventKind.PointerCaptureOut => new UiEventBody.PointerCaptureOut(value),
                _ => throw new InvalidOperationException("Unknown pointer-capture event kind."),
            };
            EmitRouted(objectId, route, kind, body);
        }

        public void ForwardFocus(
            ObjectId objectId,
            IReadOnlyList<Guid> route,
            UiEventKind kind,
            ObjectId? relatedTargetId,
            UiFocusDirection? direction,
            bool targetOnly = false
        )
        {
            var value = new UiFocusEvent(relatedTargetId, direction);
            UiEventBody body = kind switch
            {
                UiEventKind.FocusIn => new UiEventBody.FocusIn(value),
                UiEventKind.Focus => new UiEventBody.Focus(value),
                UiEventKind.FocusOut => new UiEventBody.FocusOut(value),
                UiEventKind.Blur => new UiEventBody.Blur(value),
                _ => throw new InvalidOperationException("Unknown focus event kind."),
            };
            EmitRouted(objectId, route, kind, body, targetOnly);
        }

        public void ForwardKey(
            ObjectId objectId,
            IReadOnlyList<Guid> route,
            UiEventKind kind,
            KeyCode keyCode,
            char character,
            EventModifiers modifiers
        )
        {
            var value = new UiKeyEvent(
                BattlementUiKeyboardMapper.Physical(keyCode),
                character == '\0' ? string.Empty : character.ToString(),
                ToModifiers(modifiers)
            );
            UiEventBody body = kind switch
            {
                UiEventKind.KeyDown => new UiEventBody.KeyDown(value),
                UiEventKind.KeyUp => new UiEventBody.KeyUp(value),
                _ => throw new InvalidOperationException("Unknown key event kind."),
            };
            EmitRouted(objectId, route, kind, body);
        }

        public void ForwardNavigationMove(
            ObjectId objectId,
            IReadOnlyList<Guid> route,
            UnityEngine.UIElements.NavigationMoveEvent eventValue
        ) =>
            EmitRouted(
                objectId,
                route,
                UiEventKind.NavigationMove,
                new UiEventBody.NavigationMove(
                    new UiNavigationMoveEvent(
                        BattlementUiKeyboardMapper.Navigation(eventValue.direction),
                        new Battlement.Vector(eventValue.move.x, eventValue.move.y)
                    )
                )
            );

        public void ForwardNavigationCancel(ObjectId objectId, IReadOnlyList<Guid> route) =>
            EmitRouted(
                objectId,
                route,
                UiEventKind.NavigationCancel,
                new UiEventBody.NavigationCancel(new UiNavigationEvent())
            );

        public void ForwardNavigationSubmit(
            ObjectId objectId,
            IReadOnlyList<Guid> route,
            bool buttonTarget
        )
        {
            if (emit is null)
                return;

            // A Button turns Unity's NavigationSubmitEvent into the same logical activation as a
            // pointer click. Forward it to the nearest Click subscription on the Button's route
            // so application code can handle every activation method once.
            if (buttonTarget && CanForward(route, UiEventKind.Click))
            {
                emit(
                    new UiEvent(
                        objectId,
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

        public bool ForwardValueChanging(ObjectId objectId, int proposed) =>
            Emit(
                objectId,
                UiEventKind.ValueChanging,
                new UiEventBody.ValueChanging(new ValueChangingEvent(new UiValue.I32(proposed)))
            );

        public bool ForwardValueCommitted(ObjectId objectId, int previous, int proposed) =>
            Emit(
                objectId,
                UiEventKind.ValueCommitted,
                new UiEventBody.ValueCommitted(
                    new ValueCommitEvent(new UiValue.I32(previous), new UiValue.I32(proposed))
                )
            );

        public bool ForwardValueChanging(ObjectId objectId, FloatRange proposed) =>
            Emit(
                objectId,
                UiEventKind.ValueChanging,
                new UiEventBody.ValueChanging(
                    new ValueChangingEvent(new UiValue.F32Range(proposed))
                )
            );

        public bool ForwardValueCommitted(
            ObjectId objectId,
            FloatRange previous,
            FloatRange proposed
        ) =>
            Emit(
                objectId,
                UiEventKind.ValueCommitted,
                new UiEventBody.ValueCommitted(
                    new ValueCommitEvent(
                        new UiValue.F32Range(previous),
                        new UiValue.F32Range(proposed)
                    )
                )
            );

        public bool ForwardValueCommitted(ObjectId objectId, bool previous, bool proposed) =>
            Emit(
                objectId,
                UiEventKind.ValueCommitted,
                new UiEventBody.ValueCommitted(
                    new ValueCommitEvent(new UiValue.Bool(previous), new UiValue.Bool(proposed))
                )
            );

        public bool ForwardValueCommitted(ObjectId objectId, uint? previous, uint? proposed) =>
            Emit(
                objectId,
                UiEventKind.ValueCommitted,
                new UiEventBody.ValueCommitted(
                    new ValueCommitEvent(new UiValue.Index(previous), new UiValue.Index(proposed))
                )
            );

        public bool ForwardValueCommitted(
            ObjectId objectId,
            IReadOnlyList<uint> previous,
            IReadOnlyList<uint> proposed
        ) =>
            Emit(
                objectId,
                UiEventKind.ValueCommitted,
                new UiEventBody.ValueCommitted(
                    new ValueCommitEvent(
                        new UiValue.Indices(previous),
                        new UiValue.Indices(proposed)
                    )
                )
            );

        public bool ForwardValueCommitted(
            ObjectId objectId,
            DropdownChoice previous,
            DropdownChoice proposed
        ) =>
            Emit(
                objectId,
                UiEventKind.ValueCommitted,
                new UiEventBody.ValueCommitted(
                    new ValueCommitEvent(new UiValue.Choice(previous), new UiValue.Choice(proposed))
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
                    new SelectionEvent(checked((uint)cursorIndex), checked((uint)selectIndex))
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

        public bool CanForward(ObjectId objectId, UiEventKind kind) =>
            emit is not null && IsSubscribed(objectId.Value, kind);

        public void Remove(Guid objectId) => subscriptions.Remove(objectId);

        public void Clear() => subscriptions.Clear();

        internal bool ForwardEvent(
            ObjectId objectId,
            IReadOnlyList<Guid> route,
            UiEventKind kind,
            UiEventBody body,
            bool targetOnly = false
        ) => EmitRouted(objectId, route, kind, body, targetOnly);

        private bool IsSubscribed(Guid objectId, UiEventKind kind) =>
            IsSubscribed(objectId, kind, UiEventPhase.Target);

        private bool IsSubscribed(Guid objectId, UiEventKind kind, UiEventPhase phase) =>
            subscriptions.TryGetValue(objectId, out SubscriptionState state)
            && (
                state.Routed.Contains(new UiEventSubscription(kind, phase))
                || (phase == UiEventPhase.Target && state.Target.Contains(kind))
            );

        private bool CanForward(IReadOnlyList<Guid> route, UiEventKind kind)
        {
            if (emit is null || route.Count == 0)
                return false;
            if (IsSubscribed(route[0], kind, UiEventPhase.Target))
                return true;
            for (int index = 1; index < route.Count; index++)
            {
                if (IsSubscribed(route[index], kind, UiEventPhase.Trickle))
                    return true;
                if (IsSubscribed(route[index], kind, UiEventPhase.Bubble))
                    return true;
            }
            return false;
        }

        private bool EmitRouted(
            ObjectId objectId,
            IReadOnlyList<Guid> route,
            UiEventKind kind,
            UiEventBody body,
            bool targetOnly = false
        )
        {
            bool subscribed = targetOnly
                ? route.Count > 0 && IsSubscribed(route[0], kind, UiEventPhase.Target)
                : CanForward(route, kind);
            return emit is not null && subscribed && emit(new UiEvent(objectId, body));
        }

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

        private static UiPointerButton? ToPointerButton(int value) =>
            value switch
            {
                0 => null,
                1 => new UiPointerButton.Right(),
                2 => new UiPointerButton.Middle(),
                >= 3 => new UiPointerButton.Other(value),
                _ => null,
            };

        private static PanelPoint Position(UnityEngine.Vector3 value) => new(value.x, value.y);

        private static Battlement.Vector Delta(UnityEngine.Vector3 value) => new(value.x, value.y);

        private static UiPointerType ToPointerType(string value)
        {
            if (value == UnityEngine.UIElements.PointerType.touch)
                return UiPointerType.Touch;
            if (value == UnityEngine.UIElements.PointerType.pen)
                return UiPointerType.Pen;
            return value == UnityEngine.UIElements.PointerType.mouse
                ? UiPointerType.Mouse
                : UiPointerType.Unknown;
        }

        private static IReadOnlyList<KeyModifier>? ToModifiers(EventModifiers values)
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
            if ((values & EventModifiers.CapsLock) != 0)
                result.Add(KeyModifier.CapsLock);
            if ((values & EventModifiers.Numeric) != 0)
                result.Add(KeyModifier.Numeric);
            if ((values & EventModifiers.FunctionKey) != 0)
                result.Add(KeyModifier.FunctionKey);
            return result.Count == 0 ? null : result;
        }

        private sealed class SubscriptionState
        {
            public HashSet<UiEventKind> Target { get; set; } = new();

            public HashSet<UiEventSubscription> Routed { get; set; } = new();
        }
    }
}

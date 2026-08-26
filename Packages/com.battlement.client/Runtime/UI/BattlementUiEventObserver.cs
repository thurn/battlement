#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine.UIElements;
using UnityBlurEvent = UnityEngine.UIElements.BlurEvent;
using UnityClickEvent = UnityEngine.UIElements.ClickEvent;
using UnityFocusEvent = UnityEngine.UIElements.FocusEvent;
using UnityFocusInEvent = UnityEngine.UIElements.FocusInEvent;
using UnityFocusOutEvent = UnityEngine.UIElements.FocusOutEvent;
using UnityKeyDownEvent = UnityEngine.UIElements.KeyDownEvent;
using UnityKeyUpEvent = UnityEngine.UIElements.KeyUpEvent;
using UnityNavigationCancelEvent = UnityEngine.UIElements.NavigationCancelEvent;
using UnityNavigationMoveEvent = UnityEngine.UIElements.NavigationMoveEvent;
using UnityNavigationSubmitEvent = UnityEngine.UIElements.NavigationSubmitEvent;
using UnityPointerCancelEvent = UnityEngine.UIElements.PointerCancelEvent;
using UnityPointerCaptureEvent = UnityEngine.UIElements.PointerCaptureEvent;
using UnityPointerCaptureOutEvent = UnityEngine.UIElements.PointerCaptureOutEvent;
using UnityPointerDownEvent = UnityEngine.UIElements.PointerDownEvent;
using UnityPointerEnterEvent = UnityEngine.UIElements.PointerEnterEvent;
using UnityPointerLeaveEvent = UnityEngine.UIElements.PointerLeaveEvent;
using UnityPointerMoveEvent = UnityEngine.UIElements.PointerMoveEvent;
using UnityPointerOutEvent = UnityEngine.UIElements.PointerOutEvent;
using UnityPointerOverEvent = UnityEngine.UIElements.PointerOverEvent;
using UnityPointerUpEvent = UnityEngine.UIElements.PointerUpEvent;
using UnityWheelEvent = UnityEngine.UIElements.WheelEvent;

namespace Battlement.UI
{
    internal sealed class BattlementUiEventObserver
    {
        private readonly BattlementUiEventForwarder events;
        private readonly Func<VisualElement?, Guid?> nearestId;
        private readonly Func<Guid, IReadOnlyList<Guid>> route;
        private readonly Func<Guid, bool> isButton;

        public BattlementUiEventObserver(
            BattlementUiEventForwarder eventForwarder,
            Func<VisualElement?, Guid?> nearestOwnedId,
            Func<Guid, IReadOnlyList<Guid>> logicalRoute,
            Func<Guid, bool> isOrdinaryButton
        )
        {
            events = eventForwarder;
            nearestId = nearestOwnedId;
            route = logicalRoute;
            isButton = isOrdinaryButton;
        }

        public void RegisterRoot(VisualElement root)
        {
            root.RegisterCallback<UnityNavigationSubmitEvent>(
                eventValue =>
                {
                    Guid? targetId = nearestId(eventValue.target as VisualElement);
                    if (targetId is Guid id)
                        events.ForwardNavigationSubmit(new ObjectId(id), route(id), isButton(id));
                },
                TrickleDown.TrickleDown
            );
            root.RegisterCallback<UnityKeyDownEvent>(
                eventValue =>
                    ForwardRoot(
                        eventValue,
                        (target, path) =>
                            events.ForwardKey(
                                target,
                                path,
                                UiEventKind.KeyDown,
                                eventValue.keyCode,
                                eventValue.character,
                                eventValue.modifiers
                            )
                    ),
                TrickleDown.TrickleDown
            );
            root.RegisterCallback<UnityKeyUpEvent>(
                eventValue =>
                    ForwardRoot(
                        eventValue,
                        (target, path) =>
                            events.ForwardKey(
                                target,
                                path,
                                UiEventKind.KeyUp,
                                eventValue.keyCode,
                                eventValue.character,
                                eventValue.modifiers
                            )
                    ),
                TrickleDown.TrickleDown
            );
            root.RegisterCallback<UnityNavigationMoveEvent>(
                eventValue =>
                    ForwardRoot(
                        eventValue,
                        (target, path) => events.ForwardNavigationMove(target, path, eventValue)
                    ),
                TrickleDown.TrickleDown
            );
            root.RegisterCallback<UnityNavigationCancelEvent>(
                eventValue =>
                    ForwardRoot(
                        eventValue,
                        (target, path) => events.ForwardNavigationCancel(target, path)
                    ),
                TrickleDown.TrickleDown
            );
            root.RegisterCallback<UnityPointerDownEvent>(
                eventValue =>
                    ForwardRoot(
                        eventValue,
                        (target, path) =>
                            events.ForwardPointerButton(
                                target,
                                path,
                                UiEventKind.PointerDown,
                                eventValue
                            )
                    ),
                TrickleDown.TrickleDown
            );
            root.RegisterCallback<UnityPointerMoveEvent>(
                eventValue =>
                    ForwardRoot(
                        eventValue,
                        (target, path) => events.ForwardPointerMove(target, path, eventValue)
                    ),
                TrickleDown.TrickleDown
            );
            root.RegisterCallback<UnityPointerUpEvent>(
                eventValue =>
                    ForwardRoot(
                        eventValue,
                        (target, path) =>
                            events.ForwardPointerButton(
                                target,
                                path,
                                UiEventKind.PointerUp,
                                eventValue
                            )
                    ),
                TrickleDown.TrickleDown
            );
            root.RegisterCallback<UnityPointerCancelEvent>(
                eventValue =>
                    ForwardRoot(
                        eventValue,
                        (target, path) => events.ForwardPointerCancel(target, path, eventValue)
                    ),
                TrickleDown.TrickleDown
            );
            root.RegisterCallback<UnityClickEvent>(
                eventValue =>
                    ForwardRoot(
                        eventValue,
                        (target, path) => events.ForwardClick(target, path, eventValue)
                    ),
                TrickleDown.TrickleDown
            );
            root.RegisterCallback<UnityPointerOverEvent>(
                eventValue =>
                    ForwardRoot(
                        eventValue,
                        (target, path) =>
                            events.ForwardPointerCrossing(
                                target,
                                path,
                                UiEventKind.PointerOver,
                                eventValue
                            )
                    ),
                TrickleDown.TrickleDown
            );
            root.RegisterCallback<UnityPointerOutEvent>(
                eventValue =>
                    ForwardRoot(
                        eventValue,
                        (target, path) =>
                            events.ForwardPointerCrossing(
                                target,
                                path,
                                UiEventKind.PointerOut,
                                eventValue
                            )
                    ),
                TrickleDown.TrickleDown
            );
            root.RegisterCallback<UnityWheelEvent>(
                eventValue =>
                    ForwardRoot(
                        eventValue,
                        (target, path) => events.ForwardWheel(target, path, eventValue)
                    ),
                TrickleDown.TrickleDown
            );
            root.RegisterCallback<UnityFocusInEvent>(
                eventValue => ForwardFocusRoot(eventValue, eventValue, UiEventKind.FocusIn),
                TrickleDown.TrickleDown
            );
            root.RegisterCallback<UnityFocusOutEvent>(
                eventValue => ForwardFocusRoot(eventValue, eventValue, UiEventKind.FocusOut),
                TrickleDown.TrickleDown
            );
        }

        public void RegisterElement(ObjectId objectId, VisualElement value)
        {
            value.RegisterCallback<UnityPointerEnterEvent>(eventValue =>
                ForwardOwnedBoundary(objectId, eventValue, eventValue, UiEventKind.PointerEnter)
            );
            value.RegisterCallback<UnityPointerLeaveEvent>(eventValue =>
                ForwardOwnedBoundary(objectId, eventValue, eventValue, UiEventKind.PointerLeave)
            );
            value.RegisterCallback<UnityPointerCaptureEvent>(eventValue =>
                ForwardOwnedCapture(
                    objectId,
                    eventValue,
                    UiEventKind.PointerCapture,
                    eventValue.pointerId
                )
            );
            value.RegisterCallback<UnityPointerCaptureOutEvent>(eventValue =>
                ForwardOwnedCapture(
                    objectId,
                    eventValue,
                    UiEventKind.PointerCaptureOut,
                    eventValue.pointerId
                )
            );
            value.RegisterCallback<UnityFocusEvent>(eventValue =>
                ForwardOwnedFocus(objectId, eventValue, eventValue, UiEventKind.Focus)
            );
            value.RegisterCallback<UnityBlurEvent>(eventValue =>
                ForwardOwnedFocus(objectId, eventValue, eventValue, UiEventKind.Blur)
            );
        }

        private void ForwardRoot(
            EventBase eventValue,
            Action<ObjectId, IReadOnlyList<Guid>> forward
        )
        {
            Guid? targetId = nearestId(eventValue.target as VisualElement);
            if (targetId is Guid id)
                forward(new ObjectId(id), route(id));
        }

        private void ForwardFocusRoot(EventBase eventBase, IFocusEvent eventValue, UiEventKind kind)
        {
            Guid? targetId = nearestId(eventBase.target as VisualElement);
            if (targetId is Guid id)
                events.ForwardFocus(
                    new ObjectId(id),
                    route(id),
                    kind,
                    RelatedTarget(eventValue),
                    BattlementUiKeyboardMapper.Focus(eventValue.direction)
                );
        }

        private void ForwardOwnedBoundary(
            ObjectId objectId,
            EventBase eventBase,
            IPointerEvent eventValue,
            UiEventKind kind
        )
        {
            if (nearestId(eventBase.target as VisualElement) != objectId.Value)
                return;
            events.ForwardPointerBoundary(objectId, route(objectId.Value), kind, eventValue);
        }

        private void ForwardOwnedCapture(
            ObjectId objectId,
            EventBase eventValue,
            UiEventKind kind,
            int pointerId
        )
        {
            if (nearestId(eventValue.target as VisualElement) != objectId.Value)
                return;
            events.ForwardPointerCapture(objectId, route(objectId.Value), kind, pointerId);
        }

        private void ForwardOwnedFocus(
            ObjectId objectId,
            EventBase eventBase,
            IFocusEvent eventValue,
            UiEventKind kind
        )
        {
            if (nearestId(eventBase.target as VisualElement) != objectId.Value)
                return;
            events.ForwardFocus(
                objectId,
                route(objectId.Value),
                kind,
                RelatedTarget(eventValue),
                BattlementUiKeyboardMapper.Focus(eventValue.direction),
                targetOnly: true
            );
        }

        private ObjectId? RelatedTarget(IFocusEvent eventValue) =>
            nearestId(eventValue.relatedTarget as VisualElement) is Guid id
                ? new ObjectId(id)
                : null;
    }
}

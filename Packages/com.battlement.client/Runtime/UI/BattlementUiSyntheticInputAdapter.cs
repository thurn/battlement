#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementUiSyntheticInputAdapter
    {
        private readonly BattlementUiHierarchy hierarchy;
        private readonly BattlementUiEventForwarder events;
        private readonly BattlementUiBooleanControls booleanControls;
        private ObjectId? syntheticPointerTarget;
        private PanelPoint? syntheticPointerPosition;
        private ObjectId? pendingPointerClickTarget;
        private PanelPoint? pendingPointerClickPosition;

        internal BattlementUiSyntheticInputAdapter(
            BattlementUiHierarchy hierarchy,
            BattlementUiEventForwarder events,
            BattlementUiBooleanControls booleanControls
        ) =>
            (this.hierarchy, this.events, this.booleanControls) = (
                hierarchy,
                events,
                booleanControls
            );

        internal bool DispatchSemanticActivation(ObjectId target, out string? diagnostic)
        {
            if (!hierarchy.TryGetGeometryTarget(target, out VisualElement element, out _, out _))
            {
                diagnostic = $"UI target {target.Value} is not attached.";
                return false;
            }
            if (!element.enabledInHierarchy || element.panel is null)
            {
                diagnostic = $"UI target {target.Value} is not enabled and attached.";
                return false;
            }
            IReadOnlyList<Guid> route = hierarchy.Route(target.Value);
            bool supported = element switch
            {
                Button => events.CanForwardRoute(route, UiEventKind.Click),
                Toggle => events.CanForward(target, UiEventKind.ValueCommitted),
                _ => false,
            };
            if (!supported)
            {
                diagnostic = $"UI target {target.Value} has no deterministic activation route.";
                return false;
            }
            bool dispatched = element switch
            {
                Button => events.ForwardEvent(
                    target,
                    route,
                    UiEventKind.Click,
                    new UiEventBody.Click(new Battlement.ClickEvent.NavigationSubmit())
                ),
                Toggle => booleanControls.Activate(target),
                _ => false,
            };
            if (!dispatched)
            {
                diagnostic = $"UI target {target.Value} rejected semantic activation.";
                return false;
            }
            diagnostic = null;
            return true;
        }

        internal bool BeginSyntheticPointer(
            ObjectId target,
            Vector2 screenPosition,
            out string? diagnostic
        )
        {
            if (
                !DispatchSyntheticPointer(
                    target,
                    screenPosition,
                    press: true,
                    out PanelPoint position,
                    out diagnostic
                )
            )
                return false;
            pendingPointerClickTarget = target;
            pendingPointerClickPosition = position;
            return true;
        }

        internal bool DispatchSyntheticHover(
            ObjectId target,
            Vector2 screenPosition,
            out string? diagnostic
        ) => DispatchSyntheticPointer(target, screenPosition, false, out _, out diagnostic);

        internal bool FinishSyntheticClick(ObjectId target, out string? diagnostic)
        {
            if (
                pendingPointerClickTarget != target
                || pendingPointerClickPosition is not PanelPoint pendingPosition
            )
            {
                diagnostic = $"UI target {target.Value} has no pending synthetic click.";
                return false;
            }
            ClearPendingSyntheticClick();
            if (
                !ResolveSyntheticPointerTarget(
                    target,
                    requireClick: true,
                    screenPosition: null,
                    preferredPosition: pendingPosition,
                    out PanelPoint position,
                    out IReadOnlyList<Guid> route,
                    out diagnostic
                )
            )
                return false;
            events.ForwardEvent(
                target,
                route,
                UiEventKind.PointerUp,
                new UiEventBody.PointerUp(
                    new UiPointerButtonEvent(
                        position,
                        new Vector(0, 0),
                        Button: new UiPointerButton.Left()
                    )
                )
            );
            bool dispatched = events.ForwardEvent(
                target,
                route,
                UiEventKind.Click,
                new UiEventBody.Click(
                    new Battlement.ClickEvent.Pointer(
                        position,
                        1,
                        Button: new UiPointerButton.Left()
                    )
                )
            );
            diagnostic = dispatched ? null : $"UI target {target.Value} rejected pointer click.";
            return dispatched;
        }

        internal void SetInputEnabled(bool enabled)
        {
            if (!enabled)
                Clear();
        }

        internal void Clear()
        {
            syntheticPointerTarget = null;
            syntheticPointerPosition = null;
            ClearPendingSyntheticClick();
        }

        internal void RemoveIdentity(Guid objectId)
        {
            if (
                syntheticPointerTarget?.Value != objectId
                && pendingPointerClickTarget?.Value != objectId
            )
                return;
            Clear();
        }

        private void ClearPendingSyntheticClick()
        {
            pendingPointerClickTarget = null;
            pendingPointerClickPosition = null;
        }

        private bool DispatchSyntheticPointer(
            ObjectId target,
            Vector2 screenPosition,
            bool press,
            out PanelPoint position,
            out string? diagnostic
        )
        {
            if (
                !ResolveSyntheticPointerTarget(
                    target,
                    press,
                    screenPosition,
                    preferredPosition: null,
                    out position,
                    out IReadOnlyList<Guid> route,
                    out diagnostic
                )
            )
                return false;
            MoveSyntheticPointer(target, position, route);
            events.ForwardEvent(
                target,
                route,
                UiEventKind.PointerMove,
                new UiEventBody.PointerMove(new UiPointerMoveEvent(position, new Vector(0, 0)))
            );
            if (press)
            {
                events.ForwardEvent(
                    target,
                    route,
                    UiEventKind.PointerDown,
                    new UiEventBody.PointerDown(
                        new UiPointerButtonEvent(
                            position,
                            new Vector(0, 0),
                            Button: new UiPointerButton.Left(),
                            Buttons: 1,
                            Pressure: 0.5f
                        )
                    )
                );
            }
            diagnostic = null;
            return true;
        }

        private bool ResolveSyntheticPointerTarget(
            ObjectId target,
            bool requireClick,
            Vector2? screenPosition,
            PanelPoint? preferredPosition,
            out PanelPoint position,
            out IReadOnlyList<Guid> route,
            out string? diagnostic
        )
        {
            route = hierarchy.Route(target.Value);
            if (!hierarchy.TryGetGeometryTarget(target, out VisualElement element, out _, out _))
            {
                position = default!;
                diagnostic = $"UI target {target.Value} is not attached.";
                return false;
            }
            if (!element.enabledInHierarchy || element.panel is null)
            {
                position = default!;
                diagnostic = $"UI target {target.Value} is not enabled and attached.";
                return false;
            }
            UnityEngine.Rect bounds = element.worldBound;
            if (screenPosition is Vector2 requested)
            {
                float scale = element.panel.scaledPixelsPerPoint;
                position = new PanelPoint(requested.x / scale, requested.y / scale);
            }
            else if (preferredPosition is PanelPoint preferred)
            {
                position = preferred;
            }
            else if (syntheticPointerTarget == target && syntheticPointerPosition is not null)
            {
                position = syntheticPointerPosition;
            }
            else
            {
                position = new PanelPoint(bounds.center.x, bounds.center.y);
            }
            bool hasHoverRoute =
                events.CanForward(target, UiEventKind.PointerEnter)
                || events.CanForwardRoute(route, UiEventKind.PointerOver)
                || events.CanForwardRoute(route, UiEventKind.PointerMove);
            if (requireClick ? !events.CanForwardRoute(route, UiEventKind.Click) : !hasHoverRoute)
            {
                diagnostic = requireClick
                    ? $"UI target {target.Value} has no click route."
                    : $"UI target {target.Value} has no hover route.";
                return false;
            }
            diagnostic = null;
            return true;
        }

        private void MoveSyntheticPointer(
            ObjectId target,
            PanelPoint position,
            IReadOnlyList<Guid> route
        )
        {
            ObjectId? previous = syntheticPointerTarget;
            if (previous == target)
                return;
            if (previous is ObjectId previousTarget && hierarchy.Contains(previousTarget.Value))
            {
                IReadOnlyList<Guid> previousRoute = hierarchy.Route(previousTarget.Value);
                events.ForwardEvent(
                    previousTarget,
                    previousRoute,
                    UiEventKind.PointerOut,
                    new UiEventBody.PointerOut(
                        new UiPointerCrossingEvent(position, RelatedTargetId: target)
                    )
                );
                events.ForwardEvent(
                    previousTarget,
                    previousRoute,
                    UiEventKind.PointerLeave,
                    new UiEventBody.PointerLeave(new UiPointerBoundaryEvent(position)),
                    targetOnly: true
                );
            }
            events.ForwardEvent(
                target,
                route,
                UiEventKind.PointerOver,
                new UiEventBody.PointerOver(
                    new UiPointerCrossingEvent(position, RelatedTargetId: previous)
                )
            );
            events.ForwardEvent(
                target,
                route,
                UiEventKind.PointerEnter,
                new UiEventBody.PointerEnter(new UiPointerBoundaryEvent(position)),
                targetOnly: true
            );
            syntheticPointerTarget = target;
            syntheticPointerPosition = position;
        }
    }
}

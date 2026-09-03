#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Battlement.UI;
using UnityEngine;
using UnityEngine.EventSystems;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UnityPanelRenderMode = UnityEngine.UIElements.PanelRenderMode;
using UnityRect = UnityEngine.Rect;
using UnityVector2 = UnityEngine.Vector2;

namespace Battlement
{
    internal sealed record DittoConditionResult(
        bool Matches,
        bool IsSupported,
        string? Diagnostic = null
    );

    internal sealed record DittoInputCandidate(UnityVector2 Position, ObjectId? BlockingObject);

    internal sealed record DittoInputResolution(
        bool IsReachable,
        UnityVector2 Position,
        UnityRect? Bounds,
        IReadOnlyList<DittoInputCandidate> Candidates,
        ObjectId? ObjectId
    );

    internal sealed class DittoInputTargets
    {
        private readonly BattlementRunner runner;
        private readonly BattlementUiDocuments documents;
        private readonly IReadOnlyDictionary<string, ObjectId> aliases;
        private readonly uint width;
        private readonly uint height;
        private readonly DisplaySource displays;
        private readonly List<RaycastResult> raycasts = new();

        public DittoInputTargets(
            BattlementRunner runner,
            IReadOnlyDictionary<string, ObjectId> aliases,
            uint width,
            uint height
        )
        {
            if (runner == null)
            {
                throw new ArgumentNullException(nameof(runner));
            }
            if (width == 0 || height == 0)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(width),
                    "The render surface is empty."
                );
            }

            this.runner = runner;
            documents = runner.DittoUiDocuments;
            this.aliases = aliases ?? throw new ArgumentNullException(nameof(aliases));
            this.width = width;
            this.height = height;
            displays = new DisplaySource(width, height);
        }

        public DittoConditionResult Evaluate(DittoObjectCondition condition)
        {
            if (!TryId(condition.Object, out ObjectId id))
            {
                return Present(condition.State, false, false, null);
            }

            bool hasUi = documents.TryGetGeometryTarget(
                id,
                out VisualElement element,
                out _,
                out UIDocument document
            );
            bool hasWorld = runner.TryGetObject(id, out GameObject? gameObject);
            bool exists = hasUi || hasWorld;
            if (condition.State is DittoObjectState.Enabled or DittoObjectState.Disabled)
            {
                if (hasWorld)
                {
                    return new DittoConditionResult(
                        false,
                        false,
                        $"World object {id.Value} has no enabled-state condition."
                    );
                }

                bool enabled = hasUi && element.enabledInHierarchy;
                return new DittoConditionResult(
                    condition.State == DittoObjectState.Enabled ? enabled : hasUi && !enabled,
                    true
                );
            }

            bool visible = hasUi
                ? TryUiBounds(element, document, out _)
                : hasWorld && TryWorldBounds(gameObject!, out _);
            return Present(condition.State, exists, visible, id);
        }

        public DittoConditionResult Evaluate(DittoAccessibilityAssertion assertion)
        {
            AccessibilityNodeSnapshot[] matches = AccessibilityMatches(assertion.Target);
            if (matches.Length != 1)
            {
                return new DittoConditionResult(
                    false,
                    true,
                    $"Accessibility target matched {matches.Length} nodes."
                );
            }
            AccessibilityNodeSnapshot node = matches[0];
            bool matched =
                node.Role == assertion.Role
                && string.Equals(node.Label, assertion.Name, StringComparison.Ordinal);
            if (assertion.Selected.HasValue)
                matched &= node.State.Selected == assertion.Selected;
            if (assertion.Checked.HasValue)
                matched &=
                    node.State.Checked
                    == (assertion.Checked.Value ? CheckedState.True : CheckedState.False);
            if (assertion.Disabled.HasValue)
                matched &= node.State.Disabled == assertion.Disabled;
            if (assertion.CurrentPage.HasValue)
                matched &= (node.State.Current == CurrentPage.Page) == assertion.CurrentPage;
            if (assertion.Parent is not null)
            {
                AccessibilityNodeSnapshot[] parents = AccessibilityMatches(assertion.Parent);
                matched &= parents.Length == 1 && node.ParentId == parents[0].ObjectId;
            }
            return new DittoConditionResult(
                matched,
                true,
                matched ? null : "Accessibility assertion failed."
            );
        }

        public bool AccessibilityAction(
            DittoAccessibilityTarget target,
            AccessibilityAction action,
            out string? diagnostic
        )
        {
            AccessibilityNodeSnapshot[] matches = AccessibilityMatches(target);
            string request = $"Accessibility {action} on {target.Role} '{target.Name}'";
            if (runner.DittoInputDiagnostic is string unavailable)
            {
                diagnostic = $"{request}: {unavailable}";
                return false;
            }
            if (matches.Length != 1)
            {
                diagnostic = $"{request}: matched {matches.Length} active nodes.";
                return false;
            }
            bool dispatched = documents.DispatchAccessibility(
                matches[0].ObjectId,
                action,
                out diagnostic
            );
            if (!dispatched)
                diagnostic = $"{request} (object {matches[0].ObjectId.Value}): {diagnostic}";
            return dispatched;
        }

        private AccessibilityNodeSnapshot[] AccessibilityMatches(DittoAccessibilityTarget target) =>
            documents
                .ActiveAccessibility.Where(node =>
                    node.Role == target.Role
                    && string.Equals(node.Label, target.Name, StringComparison.Ordinal)
                )
                .ToArray();

        public DittoInputResolution Resolve(DittoInputTarget target)
        {
            if (target is DittoInputTarget.Coordinates coordinates)
            {
                var position = new UnityVector2(
                    (float)(coordinates.X * (width - 1)),
                    (float)(coordinates.Y * (height - 1))
                );
                return new DittoInputResolution(
                    true,
                    position,
                    null,
                    new[] { new DittoInputCandidate(position, null) },
                    null
                );
            }

            string value = ((DittoInputTarget.Object)target).Id;
            if (!TryId(value, out ObjectId id) || !TryBounds(id, out UnityRect bounds))
            {
                return new DittoInputResolution(
                    false,
                    default,
                    null,
                    Array.Empty<DittoInputCandidate>(),
                    TryId(value, out id) ? id : null
                );
            }

            var candidates = new List<DittoInputCandidate>(25);
            foreach (UnityVector2 position in Lattice(bounds))
            {
                ObjectId? blocker = Hit(position, id, out bool reachesTarget);
                candidates.Add(new DittoInputCandidate(position, blocker));
                if (reachesTarget)
                {
                    return new DittoInputResolution(true, position, bounds, candidates, id);
                }
            }
            return new DittoInputResolution(false, default, bounds, candidates, id);
        }

        private bool TryBounds(ObjectId id, out UnityRect bounds)
        {
            bounds = default;
            if (
                documents.TryGetGeometryTarget(
                    id,
                    out VisualElement element,
                    out _,
                    out UIDocument document
                )
            )
            {
                return TryUiBounds(element, document, out bounds);
            }
            return runner.TryGetObject(id, out GameObject? value)
                && TryWorldBounds(value!, out bounds);
        }

        private bool TryUiBounds(VisualElement element, UIDocument document, out UnityRect bounds)
        {
            bounds = default;
            if (element.panel == null || document.panelSettings == null || IsHidden(element))
            {
                return false;
            }

            if (document.panelSettings.renderMode == UnityPanelRenderMode.WorldSpace)
            {
                GeometryObservationResult result = BattlementWorldPanelGeometry.Sample(
                    element,
                    default,
                    document,
                    runner.DittoInputCamera,
                    displays
                );
                if (
                    result
                    is not GeometryObservationResult.Current
                    {
                        Value: GeometryValue.Element elementGeometry,
                    }
                )
                {
                    return false;
                }
                return Clip(elementGeometry.Value.ViewportBound, out bounds);
            }

            float scale = element.panel.scaledPixelsPerPoint;
            bounds = Scale(element.worldBound, scale);
            for (
                VisualElement? ancestor = element.parent;
                ancestor != null;
                ancestor = ancestor.parent
            )
            {
                if (ancestor.style.overflow.value == Overflow.Hidden)
                {
                    bounds = Intersection(bounds, Scale(ancestor.worldBound, scale));
                }
            }
            bounds = Intersection(bounds, new UnityRect(0, 0, width, height));
            return bounds.width > 0 && bounds.height > 0;
        }

        private bool TryWorldBounds(GameObject target, out UnityRect bounds)
        {
            bounds = default;
            if (!target.activeInHierarchy || runner.DittoInputCamera == null)
            {
                return false;
            }

            GeometryObservationResult result = BattlementWorldBoundsGeometry.Sample(
                target,
                runner.DittoInputCamera,
                displays
            );
            if (
                result
                    is not GeometryObservationResult.Current
                    {
                        Value: GeometryValue.WorldBounds world,
                    }
                || !world.Value.IsInsideViewport
            )
            {
                return false;
            }
            return Clip(world.Value.Bound, out bounds);
        }

        private ObjectId? Hit(UnityVector2 position, ObjectId requested, out bool reachesTarget)
        {
            foreach (
                UIDocument document in documents.InputDocuments.OrderByDescending(value =>
                    value.sortingOrder
                )
            )
            {
                if (
                    document == null
                    || !document.isActiveAndEnabled
                    || document.panelSettings == null
                    || document.panelSettings.renderMode == UnityPanelRenderMode.WorldSpace
                    || document.rootVisualElement.panel == null
                )
                {
                    continue;
                }

                float scale = document.rootVisualElement.panel.scaledPixelsPerPoint;
                VisualElement? picked = document.rootVisualElement.panel.Pick(position / scale);
                if (picked == null)
                {
                    continue;
                }
                reachesTarget = Contains(picked, requested);
                return reachesTarget ? null : NearestId(picked);
            }

            EventSystem? eventSystem = Object.FindAnyObjectByType<EventSystem>(
                FindObjectsInactive.Exclude
            );
            raycasts.Clear();
            if (eventSystem != null)
            {
                var eventData = new PointerEventData(eventSystem)
                {
                    position = new UnityVector2(position.x, height - position.y),
                };
                eventSystem.RaycastAll(eventData, raycasts);
                Camera? camera = runner.DittoInputCamera;
                if (raycasts.Count == 0 && camera != null)
                {
                    if (camera.TryGetComponent(out PhysicsRaycaster raycaster))
                    {
                        raycaster.Raycast(eventData, raycasts);
                    }
                }
            }
            BattlementIdentity? identity =
                raycasts.Count == 0 ? null : BattlementIdentity.FindNearest(raycasts[0].gameObject);
            reachesTarget = identity != null && identity.Id == requested.Value;
            return identity == null || reachesTarget ? null : new ObjectId(identity.Id);
        }

        private bool Contains(VisualElement picked, ObjectId requested)
        {
            for (VisualElement? current = picked; current != null; current = current.parent)
            {
                if (documents.TryFindNearestId(current, out ObjectId id) && id == requested)
                {
                    return true;
                }
            }
            return false;
        }

        private ObjectId? NearestId(VisualElement picked) =>
            documents.TryFindNearestId(picked, out ObjectId id) ? id : null;

        private bool TryId(string value, out ObjectId id)
        {
            if (aliases.TryGetValue(value, out id))
            {
                return true;
            }
            bool parsed = Guid.TryParse(value, out Guid guid) && guid != Guid.Empty;
            id = new ObjectId(guid);
            return parsed;
        }

        private bool Clip(ViewportRect value, out UnityRect bounds)
        {
            bounds = Intersection(
                new UnityRect(
                    (float)value.X,
                    (float)value.Y,
                    (float)value.Width,
                    (float)value.Height
                ),
                new UnityRect(0, 0, width, height)
            );
            return bounds.width > 0 && bounds.height > 0;
        }

        private static DittoConditionResult Present(
            DittoObjectState state,
            bool exists,
            bool visible,
            ObjectId? id
        ) =>
            new(
                state switch
                {
                    DittoObjectState.Exists => exists,
                    DittoObjectState.Absent => !exists,
                    DittoObjectState.Visible => exists && visible,
                    DittoObjectState.Hidden => exists && !visible,
                    _ => false,
                },
                true,
                id is null
                    ? null
                    : $"Object {id.Value.Value} is {(visible ? "visible" : "hidden")}."
            );

        private static IEnumerable<UnityVector2> Lattice(UnityRect bounds) =>
            Enumerable
                .Range(0, 5)
                .SelectMany(y => Enumerable.Range(0, 5).Select(x => (X: x, Y: y)))
                .OrderBy(value => Square(value.X - 2) + Square(value.Y - 2))
                .ThenBy(value => value.Y)
                .ThenBy(value => value.X)
                .Select(value => new UnityVector2(
                    bounds.xMin + bounds.width * ((value.X + 0.5f) / 5),
                    bounds.yMin + bounds.height * ((value.Y + 0.5f) / 5)
                ));

        private static bool IsHidden(VisualElement element)
        {
            for (VisualElement? current = element; current != null; current = current.parent)
            {
                if (current.resolvedStyle.display == DisplayStyle.None)
                {
                    return true;
                }
                if (current.resolvedStyle.visibility == Visibility.Hidden)
                {
                    return true;
                }
                if (current.resolvedStyle.opacity <= 0)
                {
                    return true;
                }
            }
            return false;
        }

        private static UnityRect Scale(UnityRect value, float scale) =>
            new(value.x * scale, value.y * scale, value.width * scale, value.height * scale);

        private static UnityRect Intersection(UnityRect left, UnityRect right) =>
            UnityRect.MinMaxRect(
                Math.Max(left.xMin, right.xMin),
                Math.Max(left.yMin, right.yMin),
                Math.Min(left.xMax, right.xMax),
                Math.Min(left.yMax, right.yMax)
            );

        private static int Square(int value) => value * value;

        private sealed class DisplaySource : IBattlementGeometryDisplaySource
        {
            private readonly BattlementDisplayGeometry display;

            public DisplaySource(uint width, uint height) =>
                display = new BattlementDisplayGeometry(
                    width,
                    height,
                    new UnityRect(0, 0, width, height),
                    1,
                    null,
                    DisplayOrientation.Landscape
                );

            public bool TryGet(DisplayId id, out BattlementDisplayGeometry geometry)
            {
                geometry = display;
                return id.Value == 0;
            }
        }
    }
}

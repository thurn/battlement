#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    /// <summary>Owns the canonical accessibility mirror and active presentation.</summary>
    internal sealed class BattlementAccessibilityManager : IDisposable
    {
        private readonly Func<UiEvent, UiEventDisposition?> emit;
        private readonly Func<Guid, VisualElement?> resolveElement;
        private readonly Func<VisualElement, Guid?> resolveId;
        private readonly Func<Guid, bool> effectivelyInert;
        private readonly Func<IPanel?, VisualElement?> activeModal;
        private readonly Dictionary<Guid, AccessibilityNodeSnapshot> mirror = new();
        private readonly Dictionary<Guid, AccessibilityNodeSnapshot> active = new();
        private readonly List<Guid> roots = new();
        private readonly List<string> announcements = new();
        private readonly List<string> pendingAnnouncements = new();
        private readonly UnityAccessibilityBackend backend;
        private ulong generation = 1;
        private ulong commitSequence;
        private bool backendEnabled = true;
        private bool commitSuspended;

        public BattlementAccessibilityManager(
            Func<UiEvent, UiEventDisposition?>? emit,
            Func<Guid, VisualElement?> resolveElement,
            Func<VisualElement, Guid?> resolveId,
            Func<Guid, bool> effectivelyInert,
            Func<IPanel?, VisualElement?> activeModal
        )
        {
            this.emit = emit ?? (_ => null);
            this.resolveElement = resolveElement;
            this.resolveId = resolveId;
            this.effectivelyInert = effectivelyInert;
            this.activeModal = activeModal;
            backend = new UnityAccessibilityBackend(Dispatch, resolveElement, SetBackendAvailable);
        }

        public ulong Generation => generation;

        public IReadOnlyCollection<AccessibilityNodeSnapshot> Mirror => mirror.Values;

        public IReadOnlyCollection<AccessibilityNodeSnapshot> Active => active.Values;

        public IReadOnlyList<Guid> ActiveRoots => roots;

        public IReadOnlyList<string> SubmittedAnnouncements => announcements;

        public void Clear(bool reconnect = false)
        {
            mirror.Clear();
            active.Clear();
            roots.Clear();
            announcements.Clear();
            pendingAnnouncements.Clear();
            commitSequence = 0;
            if (reconnect)
                generation = checked(generation + 1);
            backend.Clear();
        }

        public void Suspend() => commitSuspended = true;

        public void Resume()
        {
            commitSuspended = false;
            PublishPresentation();
        }

        public void Apply(AccessibilityUpdatePayload update)
        {
            if (update.Snapshot is AccessibilitySnapshot snapshot)
            {
                Validate(snapshot);
                mirror.Clear();
                foreach (AccessibilityNodeSnapshot node in snapshot.Nodes)
                    mirror.Add(node.ObjectId.Value, node);
                commitSequence = snapshot.CommitSequence;
            }
            foreach (string announcement in update.Announcements)
            {
                if (!string.IsNullOrWhiteSpace(announcement))
                {
                    announcements.Add(announcement);
                    pendingAnnouncements.Add(announcement);
                }
            }
            if (!commitSuspended)
                PublishPresentation();
        }

        public void Dispose() => backend.Dispose();

        public void Refresh()
        {
            active.Clear();
            roots.Clear();
            HashSet<Guid> modalScopes = ActiveModalScopes();
            foreach (AccessibilityNodeSnapshot node in mirror.Values)
            {
                if (!IsPresented(node, modalScopes))
                    continue;
                active.Add(node.ObjectId.Value, node);
            }
            foreach (AccessibilityNodeSnapshot node in active.Values)
            {
                if (node.ParentId is not ObjectId parent || !active.ContainsKey(parent.Value))
                    roots.Add(node.ObjectId.Value);
            }
        }

        public bool Dispatch(AccessibilityEvent accessibilityEvent)
        {
            if (!backendEnabled || commitSuspended)
                return false;
            if (accessibilityEvent.BackendGeneration != generation)
                return false;
            if (
                !active.TryGetValue(
                    accessibilityEvent.Target.Value,
                    out AccessibilityNodeSnapshot node
                )
            )
                return false;
            if (node.State.Disabled || !Declares(node.Actions, accessibilityEvent.Action))
                return false;
            var body = new UiEventBody.AccessibilityAction(
                new AccessibilityActionEvent(generation, ToUiAction(accessibilityEvent.Action))
            );
            return emit(new UiEvent(accessibilityEvent.Target, true, false, body))
                == UiEventDisposition.PreventDefault;
        }

        public void SetBackendAvailable(bool available)
        {
            generation = checked(generation + 1);
            backendEnabled = available;
            if (available)
            {
                if (!commitSuspended)
                    PublishPresentation(screenChanged: true);
            }
            else
            {
                backend.Clear();
                active.Clear();
                roots.Clear();
                pendingAnnouncements.Clear();
            }
        }

        private void PublishPresentation(bool? screenChanged = null)
        {
            Guid[] previousRoots = roots.ToArray();
            Refresh();
            backend.Publish(
                generation,
                active.Values,
                roots,
                screenChanged ?? !previousRoots.SequenceEqual(roots)
            );
            foreach (string announcement in pendingAnnouncements)
                backend.Announce(announcement);
            pendingAnnouncements.Clear();
        }

        private void Validate(AccessibilitySnapshot snapshot)
        {
            if (snapshot.CommitSequence <= commitSequence)
                throw Failure("Accessibility commit sequence must increase.");
            var nodes = snapshot.Nodes.ToDictionary(node => node.ObjectId.Value);
            foreach (AccessibilityNodeSnapshot node in snapshot.Nodes)
            {
                if (resolveElement(node.ObjectId.Value) is null)
                    throw Failure($"Accessibility host {node.ObjectId.Value} is not live.");
                if (node.ParentId is ObjectId parent && !nodes.ContainsKey(parent.Value))
                    throw Failure($"Accessibility parent {parent.Value} is missing.");
                foreach (ObjectId child in node.Children)
                {
                    if (!nodes.TryGetValue(child.Value, out AccessibilityNodeSnapshot childNode))
                        throw Failure($"Accessibility child {child.Value} is missing.");
                    if (childNode.ParentId?.Value != node.ObjectId.Value)
                        throw Failure("Accessibility parent and child references disagree.");
                }
                ValidateRole(node);
            }
            foreach (ObjectId root in snapshot.Roots)
            {
                if (!nodes.TryGetValue(root.Value, out AccessibilityNodeSnapshot node))
                    throw Failure($"Accessibility root {root.Value} is missing.");
                if (node.ParentId is not null)
                    throw Failure("Accessibility root cannot declare a parent.");
            }
        }

        private bool IsPresented(AccessibilityNodeSnapshot node, HashSet<Guid> modalScopes)
        {
            VisualElement? element = resolveElement(node.ObjectId.Value);
            if (element?.panel is null || effectivelyInert(node.ObjectId.Value))
                return false;
            if (element.resolvedStyle.display == DisplayStyle.None)
                return false;
            if (element.resolvedStyle.visibility == Visibility.Hidden)
                return false;
            return modalScopes.Count == 0 || modalScopes.Contains(node.ObjectId.Value);
        }

        private HashSet<Guid> ActiveModalScopes()
        {
            var scopes = new HashSet<Guid>();
            foreach (AccessibilityNodeSnapshot node in mirror.Values)
            {
                VisualElement? element = resolveElement(node.ObjectId.Value);
                VisualElement? modal = activeModal(element?.panel);
                Guid? modalId = modal is null ? null : resolveId(modal);
                if (modalId is not Guid root || !mirror.ContainsKey(root))
                    continue;
                Guid? cursor = node.ObjectId.Value;
                while (cursor is Guid candidate)
                {
                    if (candidate == root)
                    {
                        scopes.Add(node.ObjectId.Value);
                        break;
                    }
                    cursor = mirror[candidate].ParentId?.Value;
                }
            }
            return scopes;
        }

        private static bool Declares(AccessibilityActionSet set, AccessibilityAction action) =>
            action switch
            {
                AccessibilityAction.Activate => set.Activate,
                AccessibilityAction.Increment => set.Increment,
                AccessibilityAction.Decrement => set.Decrement,
                AccessibilityAction.Dismiss => set.Dismiss,
                AccessibilityAction.Scroll scroll => set.Scroll?.Contains(scroll.Value) == true,
                _ => false,
            };

        private static UiAccessibilityAction ToUiAction(AccessibilityAction action) =>
            action switch
            {
                AccessibilityAction.Activate => new UiAccessibilityAction.Activate(),
                AccessibilityAction.Increment => new UiAccessibilityAction.Increment(),
                AccessibilityAction.Decrement => new UiAccessibilityAction.Decrement(),
                AccessibilityAction.Dismiss => new UiAccessibilityAction.Dismiss(),
                AccessibilityAction.Scroll { Value: AccessibilityScrollDirection.Forward } =>
                    new UiAccessibilityAction.ScrollForward(),
                AccessibilityAction.Scroll { Value: AccessibilityScrollDirection.Backward } =>
                    new UiAccessibilityAction.ScrollBackward(),
                _ => throw new InvalidOperationException("Unknown accessibility action."),
            };

        private static void ValidateRole(AccessibilityNodeSnapshot node)
        {
            if (RequiresName(node.Role) && string.IsNullOrWhiteSpace(node.Label))
                throw Failure($"Accessibility role {node.Role} requires a name.");
            if (node.Value is AccessibilityRangeValue value)
            {
                if (
                    !double.IsFinite(value.Minimum)
                    || !double.IsFinite(value.Maximum)
                    || !double.IsFinite(value.Current)
                )
                    throw Failure("Accessibility range values must be finite.");
                if (value.Minimum > value.Current || value.Current > value.Maximum)
                    throw Failure("Accessibility range value is outside its bounds.");
            }
            if (node.Role == SemanticRole.Switch && node.State.Checked == CheckedState.Mixed)
                throw Failure("Switch accessibility state cannot be mixed.");
            if (node.Role == SemanticRole.Heading && node.HeadingLevel is not (>= 1 and <= 6))
                throw Failure("Accessibility heading level must be one through six.");
        }

        private static bool RequiresName(SemanticRole role) =>
            role
                is not SemanticRole.Group
                    and not SemanticRole.TabPanel
                    and not SemanticRole.ScrollArea;

        private static BattlementUiException Failure(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}

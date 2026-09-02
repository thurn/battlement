#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.Accessibility;
using UnityEngine.UIElements;
using NativeScrollDirection = UnityEngine.Accessibility.AccessibilityScrollDirection;
using NativeState = UnityEngine.Accessibility.AccessibilityState;

namespace Battlement.UI
{
    /// <summary>Publishes the active semantic mirror through Unity accessibility.</summary>
    internal sealed class UnityAccessibilityBackend : IDisposable
    {
        private readonly Func<AccessibilityEvent, bool> dispatch;
        private readonly Func<Guid, VisualElement?> resolveElement;
        private readonly Action<bool> statusChanged;
        private AccessibilityHierarchy? hierarchy;
        private Dictionary<Guid, AccessibilityNode> nodes = new();

        public UnityAccessibilityBackend(
            Func<AccessibilityEvent, bool> dispatch,
            Func<Guid, VisualElement?> resolveElement,
            Action<bool> statusChanged
        )
        {
            this.dispatch = dispatch;
            this.resolveElement = resolveElement;
            this.statusChanged = statusChanged;
            AssistiveSupport.screenReaderStatusChanged += OnStatusChanged;
        }

        public bool Available => IsSupportedPlayer && AssistiveSupport.isScreenReaderEnabled;

        public void Dispose()
        {
            AssistiveSupport.screenReaderStatusChanged -= OnStatusChanged;
            Clear();
        }

        public void Publish(
            ulong generation,
            IReadOnlyCollection<AccessibilityNodeSnapshot> active,
            IReadOnlyList<Guid> roots,
            bool screenChanged
        )
        {
            if (!Available)
            {
                Clear();
                return;
            }
            var nextHierarchy = new AccessibilityHierarchy();
            var nextNodes = new Dictionary<Guid, AccessibilityNode>();
            var snapshots = new Dictionary<Guid, AccessibilityNodeSnapshot>();
            foreach (AccessibilityNodeSnapshot snapshot in active)
                snapshots.Add(snapshot.ObjectId.Value, snapshot);
            foreach (Guid root in roots)
                AddSubtree(nextHierarchy, null, root, snapshots, nextNodes, generation);
            hierarchy = nextHierarchy;
            nodes = nextNodes;
            AssistiveSupport.activeHierarchy = hierarchy;
            if (screenChanged)
            {
                AssistiveSupport.notificationDispatcher.SendScreenChanged(
                    roots.Count == 0 ? null : nodes[roots[0]]
                );
            }
            else
            {
                foreach (Guid root in roots)
                    AssistiveSupport.notificationDispatcher.SendLayoutChanged(nodes[root]);
            }
        }

        public void Announce(string value)
        {
            if (Available)
                AssistiveSupport.notificationDispatcher.SendAnnouncement(value);
        }

        public void Clear()
        {
            if (IsSupportedPlayer && ReferenceEquals(AssistiveSupport.activeHierarchy, hierarchy))
                AssistiveSupport.activeHierarchy = null;
            hierarchy?.Clear();
            hierarchy = null;
            nodes.Clear();
        }

        private AccessibilityNode AddSubtree(
            AccessibilityHierarchy targetHierarchy,
            AccessibilityNode? parent,
            Guid id,
            IReadOnlyDictionary<Guid, AccessibilityNodeSnapshot> snapshots,
            IDictionary<Guid, AccessibilityNode> targetNodes,
            ulong generation
        )
        {
            AccessibilityNodeSnapshot snapshot = snapshots[id];
            AccessibilityNode node = targetHierarchy.AddNode(id.ToString("D"), parent);
            targetNodes.Add(id, node);
            node.label = UnityAccessibilityMapping.Label(snapshot, snapshots);
            node.hint = snapshot.Hint ?? string.Empty;
            node.value = UnityAccessibilityMapping.Value(snapshot);
            node.role = UnityAccessibilityMapping.Role(snapshot.Role);
            node.state = MapState(snapshot.State);
            node.frameGetter = () => resolveElement(id)?.worldBound ?? default;
            BindActions(node, snapshot, generation);
            foreach (ObjectId child in snapshot.Children)
            {
                if (snapshots.ContainsKey(child.Value))
                    AddSubtree(
                        targetHierarchy,
                        node,
                        child.Value,
                        snapshots,
                        targetNodes,
                        generation
                    );
            }
            return node;
        }

        private void BindActions(
            AccessibilityNode node,
            AccessibilityNodeSnapshot snapshot,
            ulong generation
        )
        {
            ObjectId target = snapshot.ObjectId;
            if (snapshot.Actions.Activate)
                node.invoked += () =>
                    dispatch(
                        new AccessibilityEvent(
                            generation,
                            target,
                            new AccessibilityAction.Activate()
                        )
                    );
            if (snapshot.Actions.Increment)
                node.incremented += () =>
                    dispatch(
                        new AccessibilityEvent(
                            generation,
                            target,
                            new AccessibilityAction.Increment()
                        )
                    );
            if (snapshot.Actions.Decrement)
                node.decremented += () =>
                    dispatch(
                        new AccessibilityEvent(
                            generation,
                            target,
                            new AccessibilityAction.Decrement()
                        )
                    );
            if (snapshot.Actions.Dismiss)
                node.dismissed += () =>
                    dispatch(
                        new AccessibilityEvent(
                            generation,
                            target,
                            new AccessibilityAction.Dismiss()
                        )
                    );
            if (snapshot.Actions.Scroll is { Count: > 0 })
                node.scrolled += direction =>
                    DispatchScroll(generation, target, snapshot, direction);
        }

        private bool DispatchScroll(
            ulong generation,
            ObjectId target,
            AccessibilityNodeSnapshot snapshot,
            NativeScrollDirection direction
        )
        {
            AccessibilityScrollDirection? normalized = direction switch
            {
                NativeScrollDirection.Forward => AccessibilityScrollDirection.Forward,
                NativeScrollDirection.Down
                    when snapshot.ScrollAxis == AccessibilityScrollAxis.Vertical =>
                    AccessibilityScrollDirection.Forward,
                NativeScrollDirection.Right
                    when snapshot.ScrollAxis == AccessibilityScrollAxis.Horizontal =>
                    AccessibilityScrollDirection.Forward,
                NativeScrollDirection.Backward => AccessibilityScrollDirection.Backward,
                NativeScrollDirection.Up
                    when snapshot.ScrollAxis == AccessibilityScrollAxis.Vertical =>
                    AccessibilityScrollDirection.Backward,
                NativeScrollDirection.Left
                    when snapshot.ScrollAxis == AccessibilityScrollAxis.Horizontal =>
                    AccessibilityScrollDirection.Backward,
                _ => null,
            };
            return normalized is AccessibilityScrollDirection value
                && dispatch(
                    new AccessibilityEvent(
                        generation,
                        target,
                        new AccessibilityAction.Scroll(value)
                    )
                );
        }

        private static NativeState MapState(SemanticState state)
        {
            NativeState result = NativeState.None;
            if (state.Disabled)
                result |= NativeState.Disabled;
            if (state.Expanded == true)
                result |= NativeState.Expanded;
            if (state.Selected == true || state.Checked == CheckedState.True)
                result |= NativeState.Selected;
            return result;
        }

        private void OnStatusChanged(bool available) => statusChanged(available);

        private static bool IsSupportedPlayer =>
            Application.platform
                is RuntimePlatform.IPhonePlayer
                    or RuntimePlatform.Android
                    or RuntimePlatform.OSXPlayer
                    or RuntimePlatform.WindowsPlayer;
    }
}

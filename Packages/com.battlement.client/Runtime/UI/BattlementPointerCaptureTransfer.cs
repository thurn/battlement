#nullable enable
using System;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    // A hierarchy move preserves logical ownership while UI Toolkit detaches the native element.
    internal sealed class BattlementPointerCaptureTransfer : IDisposable
    {
        private static readonly HashSet<VisualElement> moving = new();
        private static readonly ConditionalWeakTable<VisualElement, HashSet<int>> observed = new();
        private readonly VisualElement root;
        private readonly List<(VisualElement Owner, int Pointer)> captures = new();

        public BattlementPointerCaptureTransfer(VisualElement root)
        {
            this.root = root;
            if (root.panel is IPanel panel)
                for (int id = 0; id < PointerId.maxPointers; id++)
                    if (panel.GetCapturingElement(id) is VisualElement owner)
                        if (owner == root || root.Contains(owner))
                        {
                            captures.Add((owner, id));
                        }
            moving.Add(root);
        }

        public static bool IsMoving(VisualElement? element)
        {
            for (VisualElement? current = element; current != null; current = current.parent)
                if (moving.Contains(current))
                    return true;
            return false;
        }

        public void Dispose()
        {
            try
            {
                foreach (var capture in captures)
                    if (capture.Owner.panel != null)
                    {
                        capture.Owner.CapturePointer(capture.Pointer);
                    }
            }
            finally
            {
                moving.Remove(root);
            }
        }

        public static bool SuppressCapture(VisualElement? owner, UiEventKind kind, int pointer)
        {
            if (IsMoving(owner))
                return true;
            if (owner is null)
                return false;
            HashSet<int> held = observed.GetOrCreateValue(owner);
            if (kind == UiEventKind.PointerCapture)
                return !held.Add(pointer);
            held.Remove(pointer);
            return false;
        }

        public static void ReleaseIneligible(IPanel panel, Func<VisualElement, bool> inert)
        {
            for (int id = 0; id < PointerId.maxPointers; id++)
            {
                if (panel.GetCapturingElement(id) is not VisualElement owner)
                    continue;
                for (VisualElement? current = owner; current != null; current = current.parent)
                {
                    bool hidden =
                        current.style.display == DisplayStyle.None
                        || current.resolvedStyle.display == DisplayStyle.None;
                    bool invisible =
                        current.style.visibility == Visibility.Hidden
                        || current.resolvedStyle.visibility == Visibility.Hidden;
                    bool unavailable = !current.enabledInHierarchy || inert(current);
                    if (hidden || invisible || unavailable)
                    {
                        owner.ReleasePointer(id);
                        break;
                    }
                }
            }
        }
    }
}

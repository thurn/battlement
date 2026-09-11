#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    /// <summary>Owns the live logical hierarchy behind Battlement UI documents.</summary>
    internal sealed class BattlementUiHierarchy
    {
        internal const int MaximumDepth = 256;

        private readonly Dictionary<Guid, Entry> entries = new();
        private readonly Dictionary<VisualElement, Guid> idsByElement = new();
        private readonly Dictionary<Guid, UIDocument> rootDocuments = new();

        internal sealed class Entry
        {
            private readonly List<Guid> children = new();

            internal Entry(Guid id, VisualElement element, Guid documentRoot, Guid? parentId)
            {
                Id = id;
                Element = element;
                DocumentRoot = documentRoot;
                ParentId = parentId;
            }

            internal Guid Id { get; }

            internal VisualElement Element { get; }

            internal Guid DocumentRoot { get; }

            internal Guid? ParentId { get; set; }

            internal IReadOnlyList<Guid> Children => children;

            internal void AddChild(Guid childId, int index) => children.Insert(index, childId);

            internal void RemoveChildAt(int index) => children.RemoveAt(index);
        }

        internal readonly struct MovePlan
        {
            internal MovePlan(
                Guid objectId,
                Guid oldParentId,
                Guid newParentId,
                int oldIndex,
                int newIndex
            )
            {
                ObjectId = objectId;
                OldParentId = oldParentId;
                NewParentId = newParentId;
                OldIndex = oldIndex;
                NewIndex = newIndex;
            }

            internal Guid ObjectId { get; }

            internal Guid OldParentId { get; }

            internal Guid NewParentId { get; }

            internal int OldIndex { get; }

            internal int NewIndex { get; }
        }

        internal readonly struct ReorderPlan
        {
            internal ReorderPlan(Guid objectId, Guid parentId, int oldIndex, int newIndex)
            {
                ObjectId = objectId;
                ParentId = parentId;
                OldIndex = oldIndex;
                NewIndex = newIndex;
            }

            internal Guid ObjectId { get; }

            internal Guid ParentId { get; }

            internal int OldIndex { get; }

            internal int NewIndex { get; }
        }

        internal int Count => entries.Count;

        internal IEnumerable<Entry> Entries => entries.Values;

        internal IEnumerable<VisualElement> Elements
        {
            get
            {
                foreach (Entry entry in entries.Values)
                    yield return entry.Element;
            }
        }

        internal IEnumerable<Guid> IdentityIds => entries.Keys;

        internal IEnumerable<UIDocument> InputDocuments => rootDocuments.Values;

        internal IEnumerable<Entry> Roots
        {
            get
            {
                foreach (Guid rootId in rootDocuments.Keys)
                    yield return entries[rootId];
            }
        }

        internal void AddRoot(
            ObjectId objectId,
            VisualElement element,
            UIDocument document,
            Action<ObjectId, VisualElement> register
        )
        {
            Add(objectId, element, objectId.Value, parentId: null, register);
            rootDocuments.Add(objectId.Value, document);
        }

        internal void Add(
            ObjectId objectId,
            VisualElement element,
            Guid documentRoot,
            Guid? parentId,
            Action<ObjectId, VisualElement> register
        )
        {
            if (entries.ContainsKey(objectId.Value))
                throw new InvalidOperationException($"UI identity {objectId} is duplicated.");
            if (idsByElement.ContainsKey(element))
                throw new InvalidOperationException("A native element is already registered.");
            var entry = new Entry(objectId.Value, element, documentRoot, parentId);
            entries.Add(objectId.Value, entry);
            idsByElement.Add(element, objectId.Value);
            try
            {
                register(objectId, element);
            }
            catch
            {
                idsByElement.Remove(element);
                entries.Remove(objectId.Value);
                throw;
            }
        }

        internal void AddChild(ObjectId parentId, ObjectId childId, int index)
        {
            Entry parent = Require(parentId.Value);
            Entry child = Require(childId.Value);
            if (child.ParentId != parent.Id)
                throw new InvalidOperationException("A UI child has the wrong logical parent.");
            if (index < 0 || index > parent.Children.Count)
                throw new InvalidOperationException("A UI child index is out of range.");
            parent.AddChild(child.Id, index);
        }

        internal MovePlan PrepareMove(ObjectId objectId, ObjectId parentId, uint? childIndex)
        {
            Entry child = Require(objectId.Value);
            if (IsRoot(objectId.Value))
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "A document root cannot be reparented."
                );
            Entry parent = Require(parentId.Value);
            Guid oldParentId =
                child.ParentId
                ?? throw new InvalidOperationException("A non-root UI element lost its parent.");
            if (child.DocumentRoot != parent.DocumentRoot)
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "UI elements cannot move between documents."
                );
            if (child.Id == parent.Id || IsDescendant(parent.Id, child.Id))
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "A UI placement cannot create a cycle."
                );
            if (DepthOf(parent.Id) + SubtreeDepth(child.Id) + 1 > MaximumDepth)
                throw Failure(CoreErrorCode.LimitExceeded, "The UI hierarchy is too deep.");

            int oldIndex = IndexOf(Require(oldParentId).Children, child.Id);
            int destinationLength = parent.Children.Count - (oldParentId == parent.Id ? 1 : 0);
            int newIndex = childIndex is uint requested
                ? checked((int)requested)
                : destinationLength;
            if (newIndex > destinationLength)
                throw Failure(CoreErrorCode.InvalidHierarchy, "UI child index is out of range.");
            return new MovePlan(child.Id, oldParentId, parent.Id, oldIndex, newIndex);
        }

        internal void ApplyMove(MovePlan plan)
        {
            Entry child = Require(plan.ObjectId);
            Entry oldParent = Require(plan.OldParentId);
            Entry newParent = Require(plan.NewParentId);
            oldParent.RemoveChildAt(plan.OldIndex);
            newParent.AddChild(plan.ObjectId, plan.NewIndex);
            child.ParentId = plan.NewParentId;
        }

        internal ReorderPlan PrepareReorder(ObjectId objectId, uint childIndex)
        {
            Entry child = Require(objectId.Value);
            if (IsRoot(objectId.Value))
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "A document root cannot be reordered."
                );
            Guid parentId =
                child.ParentId
                ?? throw new InvalidOperationException("A non-root UI element lost its parent.");
            int index = checked((int)childIndex);
            Entry parent = Require(parentId);
            if (index >= parent.Children.Count)
                throw Failure(CoreErrorCode.InvalidHierarchy, "UI child index is out of range.");
            return new ReorderPlan(
                objectId.Value,
                parentId,
                IndexOf(parent.Children, child.Id),
                index
            );
        }

        internal void ApplyReorder(ReorderPlan plan)
        {
            Entry parent = Require(plan.ParentId);
            parent.RemoveChildAt(plan.OldIndex);
            parent.AddChild(plan.ObjectId, plan.NewIndex);
        }

        internal IReadOnlyList<Entry> RemoveSubtree(ObjectId objectId)
        {
            Entry target = Require(objectId.Value);
            if (IsRoot(objectId.Value))
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "A document root cannot be destroyed by a UI command."
                );
            Guid parentId =
                target.ParentId
                ?? throw new InvalidOperationException("A non-root UI element lost its parent.");
            Entry parent = Require(parentId);
            int index = IndexOf(parent.Children, objectId.Value);
            parent.RemoveChildAt(index);

            List<Entry> removed = new();
            AddPostorder(target, removed);
            foreach (Entry entry in removed)
            {
                entries.Remove(entry.Id);
                idsByElement.Remove(entry.Element);
            }
            return removed;
        }

        internal IReadOnlyList<Entry> Clear()
        {
            Entry[] removed = new Entry[entries.Count];
            entries.Values.CopyTo(removed, 0);
            entries.Clear();
            idsByElement.Clear();
            rootDocuments.Clear();
            return removed;
        }

        internal void Remove(Guid objectId)
        {
            if (!entries.Remove(objectId, out Entry? entry))
                return;
            if (entry.ParentId is Guid parentId && entries.TryGetValue(parentId, out Entry? parent))
            {
                int index = IndexOf(parent.Children, objectId);
                if (index >= 0)
                    parent.RemoveChildAt(index);
            }
            idsByElement.Remove(entry.Element);
        }

        internal bool TryGet(
            ObjectId objectId,
            [System.Diagnostics.CodeAnalysis.NotNullWhen(true)] out VisualElement? value
        )
        {
            if (entries.TryGetValue(objectId.Value, out Entry entry))
            {
                value = entry.Element;
                return true;
            }
            value = null;
            return false;
        }

        internal bool TryGetEntry(
            Guid objectId,
            [System.Diagnostics.CodeAnalysis.NotNullWhen(true)] out Entry? entry
        ) => entries.TryGetValue(objectId, out entry);

        internal bool TryGetId(VisualElement element, out Guid objectId) =>
            idsByElement.TryGetValue(element, out objectId);

        internal bool Contains(Guid objectId) => entries.ContainsKey(objectId);

        internal bool IsRoot(Guid objectId) =>
            entries.TryGetValue(objectId, out Entry entry) && entry.DocumentRoot == objectId;

        internal Guid DocumentRoot(Guid objectId) => Require(objectId).DocumentRoot;

        internal Guid? ParentId(Guid objectId) => Require(objectId).ParentId;

        internal IReadOnlyList<Guid> Children(Guid objectId) => Require(objectId).Children;

        internal int IndexOfChild(Guid parentId, Guid childId) =>
            IndexOf(Children(parentId), childId);

        internal bool TryGetGeometryTarget(
            ObjectId objectId,
            out VisualElement element,
            out ObjectId panelId,
            out UIDocument document
        )
        {
            if (
                !entries.TryGetValue(objectId.Value, out Entry entry)
                || !rootDocuments.TryGetValue(entry.DocumentRoot, out document)
            )
            {
                element = null!;
                panelId = default;
                document = null!;
                return false;
            }
            element = entry.Element;
            panelId = new ObjectId(entry.DocumentRoot);
            return true;
        }

        internal Guid? NearestId(VisualElement? target)
        {
            for (VisualElement? value = target; value is not null; value = value.parent)
            {
                if (idsByElement.TryGetValue(value, out Guid objectId))
                    return objectId;
            }
            return null;
        }

        internal IReadOnlyList<Guid> Route(Guid objectId)
        {
            if (!entries.ContainsKey(objectId))
                return Array.Empty<Guid>();
            var result = new List<Guid>();
            Guid? current = objectId;
            while (current is Guid value)
            {
                result.Add(value);
                current = entries[value].ParentId;
            }
            return result;
        }

        internal IEnumerable<Guid> LogicalPreorder(Guid objectId)
        {
            yield return objectId;
            foreach (Guid child in Children(objectId))
            {
                foreach (Guid descendant in LogicalPreorder(child))
                    yield return descendant;
            }
        }

        internal int SourceOrdinal(VisualElement target)
        {
            if (!idsByElement.TryGetValue(target, out Guid targetId))
                return int.MaxValue;
            Guid root = DocumentRoot(targetId);
            int ordinal = 0;
            return FindOrdinal(root, targetId, ref ordinal) ? ordinal : int.MaxValue;
        }

        internal int DepthOf(Guid objectId)
        {
            int depth = 0;
            Guid? cursor = objectId;
            while (cursor is Guid value && entries[value].ParentId is Guid parent)
            {
                depth++;
                cursor = parent;
            }
            return depth;
        }

        internal int SubtreeDepth(Guid objectId)
        {
            int depth = 0;
            foreach (Guid child in Children(objectId))
                depth = Math.Max(depth, SubtreeDepth(child) + 1);
            return depth;
        }

        internal bool IsDescendant(Guid candidate, Guid ancestor)
        {
            Guid? cursor = candidate;
            while (cursor is Guid value)
            {
                if (value == ancestor)
                    return true;
                cursor = entries[value].ParentId;
            }
            return false;
        }

        internal List<Guid> SubtreeIds(Guid objectId)
        {
            var result = new List<Guid>();
            AddPostorder(Require(objectId), result);
            return result;
        }

        private void AddPostorder(Entry entry, List<Entry> result)
        {
            foreach (Guid child in entry.Children)
                AddPostorder(Require(child), result);
            result.Add(entry);
        }

        private void AddPostorder(Entry entry, List<Guid> result)
        {
            foreach (Guid child in entry.Children)
                AddPostorder(Require(child), result);
            result.Add(entry.Id);
        }

        private bool FindOrdinal(Guid current, Guid target, ref int ordinal)
        {
            foreach (Guid child in Children(current))
            {
                if (child == target)
                    return true;
                ordinal++;
                if (FindOrdinal(child, target, ref ordinal))
                    return true;
            }
            return false;
        }

        private static int IndexOf(IReadOnlyList<Guid> values, Guid target)
        {
            for (int index = 0; index < values.Count; index++)
            {
                if (values[index] == target)
                    return index;
            }
            return -1;
        }

        private Entry Require(Guid objectId) =>
            entries.TryGetValue(objectId, out Entry? entry)
                ? entry
                : throw new InvalidOperationException($"UI identity {objectId} is not registered.");

        private static BattlementUiException Failure(CoreErrorCode code, string message) =>
            new(code, message);
    }
}

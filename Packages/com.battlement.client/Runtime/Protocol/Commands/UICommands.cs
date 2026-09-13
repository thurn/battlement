#nullable enable

using System.Collections.Generic;

namespace Battlement
{
    public abstract partial record CommandBody
    {
        public static class VisualElement
        {
            /// <summary>Creates and attaches one logical UI node subtree.</summary>
            public sealed record Create(ObjectId ParentId, UiNode Node, uint? ChildIndex = null)
                : CommandBody;

            /// <summary>Applies one sparse property or hierarchy update.</summary>
            public sealed record Update(VisualElementUpdate Value) : CommandBody;

            /// <summary>Destroys one UI element and all logical descendants.</summary>
            public sealed record Destroy(ObjectId ObjectId) : CommandBody;

            /// <summary>Performs one transient UI operation.</summary>
            public sealed record PerformAction(ObjectId ObjectId, VisualElementAction Action)
                : CommandBody;
        }
    }

    /// <summary>One sparse visual-property or hierarchy update.</summary>
    public abstract record VisualElementUpdate
    {
        private VisualElementUpdate() { }

        /// <summary>Applies supplied properties without changing hierarchy.</summary>
        public sealed record Properties(ObjectId ObjectId, UiElement Element) : VisualElementUpdate;

        /// <summary>Moves an element beneath a parent at one optional child index.</summary>
        public sealed record Parent(ObjectId ObjectId, ObjectId ParentId, uint? ChildIndex = null)
            : VisualElementUpdate;

        /// <summary>Changes an element's index within its current parent.</summary>
        public sealed record Index(ObjectId ObjectId, uint ChildIndex) : VisualElementUpdate;
    }

    /// <summary>Declared one-shot UI actions.</summary>
    public abstract record VisualElementAction
    {
        private VisualElementAction() { }

        /// <summary>Restarts native particle streaks in local UI coordinates.</summary>
        public sealed record ParticleStreaks(IReadOnlyList<UiParticleStreak> Streaks)
            : VisualElementAction;

        public sealed record Focus : VisualElementAction;

        public sealed record Blur : VisualElementAction;

        public sealed record CapturePointer(int PointerId) : VisualElementAction;

        public sealed record ReleasePointer(int PointerId) : VisualElementAction;

        public sealed record ScrollTo(ObjectId DescendantId) : VisualElementAction;

        public sealed record SelectText(uint CursorIndex, uint SelectionIndex)
            : VisualElementAction;
    }

    /// <summary>A finite, unlit rectangular streak anchored within a UI element.</summary>
    public sealed record UiParticleStreak(
        IReadOnlyList<float> Origin,
        IReadOnlyList<float> Travel,
        IReadOnlyList<float> Size,
        float Rotation,
        Color Color,
        uint LifetimeMs,
        uint DelayMs
    );

    internal enum BattlementDirectVisualElementActionKind : byte
    {
        ParticleStreaks,
        Focus,
        Blur,
        CapturePointer,
        ReleasePointer,
        ScrollTo,
        SelectText,
    }

    internal readonly struct BattlementDirectVisualElementAction
    {
        internal BattlementDirectVisualElementAction(
            ObjectId objectId,
            BattlementDirectVisualElementActionKind kind,
            IReadOnlyList<UiParticleStreak>? streaks = null,
            int pointerId = 0,
            ObjectId? descendantId = null,
            uint cursorIndex = 0,
            uint selectionIndex = 0
        ) =>
            (ObjectId, Kind, Streaks, PointerId, DescendantId, CursorIndex, SelectionIndex) = (
                objectId,
                kind,
                streaks,
                pointerId,
                descendantId,
                cursorIndex,
                selectionIndex
            );

        internal ObjectId ObjectId { get; }
        internal BattlementDirectVisualElementActionKind Kind { get; }
        internal IReadOnlyList<UiParticleStreak>? Streaks { get; }
        internal int PointerId { get; }
        internal ObjectId? DescendantId { get; }
        internal uint CursorIndex { get; }
        internal uint SelectionIndex { get; }
    }

    internal readonly struct BattlementDirectVisualElementPlacement
    {
        internal BattlementDirectVisualElementPlacement(
            ObjectId objectId,
            ObjectId? parentId,
            uint? childIndex,
            bool changesParent
        ) =>
            (ObjectId, ParentId, ChildIndex, ChangesParent) = (
                objectId,
                parentId,
                childIndex,
                changesParent
            );

        internal ObjectId ObjectId { get; }
        internal ObjectId? ParentId { get; }
        internal uint? ChildIndex { get; }
        internal bool ChangesParent { get; }
    }

    internal enum BattlementUiScalarUpdateKind
    {
        TextFieldValue,
        BooleanValue,
        RadioSelection,
        ToggleSelection,
        DropdownSelection,
        ScrollerValue,
        SliderValue,
        SliderIntValue,
        RangeValue,
        TabSelection,
        ButtonText,
        ButtonEnabled,
        ButtonTextAndEnabled,
        RepeatTiming,
    }

    internal interface IBattlementUiScalarUpdateView
    {
        BattlementUiScalarUpdateKind Kind { get; }
        ObjectId ObjectId { get; }
        bool Boolean { get; }
        int Integer { get; }
        uint Unsigned { get; }
        uint UnsignedSecond { get; }
        float First { get; }
        float Second { get; }
        string? ReadText();
        int IndexCount { get; }
        uint ReadIndex(int index);
    }

    /// <summary>Verified flattened UI forest exposed without reconstructing a node tree.</summary>
    public interface IBattlementUiForestView
    {
        int NodeCount { get; }
        ObjectId ReadNodeId(int index);
        UiElement ReadNodeElement(int index);
        int ReadChildCount(int index);
        ObjectId ReadChildId(int nodeIndex, int childIndex);
    }

    /// <summary>A verified UI document exposed as a root and flattened descendants.</summary>
    public interface IBattlementUiDocumentView : IBattlementUiForestView
    {
        ObjectId DocumentId { get; }
        ObjectId RootId { get; }
        UiDocument ReadRoot();
        int RootChildCount { get; }
        ObjectId ReadRootChildId(int index);
    }

    /// <summary>A guarded collection of flattened UI documents.</summary>
    public interface IBattlementUiDocumentCollectionView
    {
        int DocumentCount { get; }
        IBattlementUiDocumentView ReadDocument(int index);
    }
}

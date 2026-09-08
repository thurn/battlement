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
}

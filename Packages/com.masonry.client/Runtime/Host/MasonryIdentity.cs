#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

namespace Masonry
{
    /// <summary>Identifies a Masonry-controlled game object for its session lifetime.</summary>
    [DisallowMultipleComponent]
    public sealed class MasonryIdentity : MonoBehaviour
    {
        private Guid id;
        private MasonryWorld? world;
        private HashSet<PointerEvent> pointerEvents = new();

        /// <summary>Gets the protocol UUID assigned to this game object.</summary>
        public Guid Id => id;

        internal bool UsesAutomaticPointerCollider { get; private set; }

        /// <summary>Returns whether this object currently accepts a pointer-event kind.</summary>
        public bool IsPointerEventEnabled(PointerEvent pointerEvent) =>
            pointerEvents.Contains(pointerEvent);

        /// <summary>Finds the closest identified ancestor of a Unity hit object.</summary>
        public static MasonryIdentity? FindNearest(GameObject? hitObject)
        {
            if (hitObject == null)
            {
                return null;
            }

            Transform? current = hitObject.transform;
            while (current != null)
            {
                MasonryIdentity? identity = current.GetComponent<MasonryIdentity>();
                if (
                    identity != null
                    && identity.world is not null
                    && identity.world.Contains(identity)
                )
                {
                    return identity;
                }

                current = current.parent;
            }

            return null;
        }

        internal void Initialize(
            MasonryWorld owningWorld,
            Guid value,
            IReadOnlyList<PointerEvent> enabledPointerEvents,
            bool usesAutomaticPointerCollider
        )
        {
            world = owningWorld;
            id = value;
            UsesAutomaticPointerCollider = usesAutomaticPointerCollider;
            SetPointerEvents(enabledPointerEvents);
        }

        internal void SetPointerEvents(IReadOnlyList<PointerEvent> enabledPointerEvents) =>
            pointerEvents = enabledPointerEvents.ToHashSet();

        private void OnDestroy()
        {
            MasonryWorld? owningWorld = world;
            world = null;
            owningWorld?.Unregister(this);
        }
    }
}

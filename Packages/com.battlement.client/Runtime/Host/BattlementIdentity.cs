#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

namespace Battlement
{
    /// <summary>Identifies a Battlement-controlled game object for its session lifetime.</summary>
    [DisallowMultipleComponent]
    public sealed class BattlementIdentity : MonoBehaviour
    {
        private Guid id;
        private BattlementWorld? world;
        private HashSet<PointerEvent> pointerEvents = new();

        /// <summary>Gets the protocol UUID assigned to this game object.</summary>
        public Guid Id => id;

        internal bool UsesAutomaticPointerCollider { get; private set; }

        internal DragMode? DragMode { get; private set; }

        internal bool IsAvailableForPointerInput
        {
            get
            {
                if (world is null || !world.Contains(this))
                {
                    return false;
                }

                return isActiveAndEnabled && gameObject.activeInHierarchy;
            }
        }

        /// <summary>Returns whether this object currently accepts a pointer-event kind.</summary>
        public bool IsPointerEventEnabled(PointerEvent pointerEvent) =>
            pointerEvents.Contains(pointerEvent);

        /// <summary>Finds the closest identified ancestor of a Unity hit object.</summary>
        public static BattlementIdentity? FindNearest(GameObject? hitObject)
        {
            if (hitObject == null)
            {
                return null;
            }

            Transform? current = hitObject.transform;
            while (current != null)
            {
                BattlementIdentity? identity = current.GetComponent<BattlementIdentity>();
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
            BattlementWorld owningWorld,
            Guid value,
            IReadOnlyList<PointerEvent> enabledPointerEvents,
            DragMode? dragMode,
            bool usesAutomaticPointerCollider
        )
        {
            world = owningWorld;
            id = value;
            DragMode = dragMode;
            UsesAutomaticPointerCollider = usesAutomaticPointerCollider;
            SetPointerEvents(enabledPointerEvents);
        }

        internal void SetPointerEvents(IReadOnlyList<PointerEvent> enabledPointerEvents) =>
            pointerEvents = enabledPointerEvents.ToHashSet();

        private void OnDestroy()
        {
            BattlementWorld? owningWorld = world;
            world = null;
            owningWorld?.Unregister(this);
        }
    }
}

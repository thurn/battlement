#nullable enable

using System;
using UnityEngine;

namespace Masonry
{
    /// <summary>Identifies a Masonry-controlled game object for its session lifetime.</summary>
    [DisallowMultipleComponent]
    public sealed class MasonryIdentity : MonoBehaviour
    {
        private Guid id;
        private MasonryWorld? world;

        /// <summary>Gets the protocol UUID assigned to this game object.</summary>
        public Guid Id => id;

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

        internal void Initialize(MasonryWorld owningWorld, Guid value)
        {
            world = owningWorld;
            id = value;
        }

        private void OnDestroy()
        {
            MasonryWorld? owningWorld = world;
            world = null;
            owningWorld?.Unregister(this);
        }
    }
}

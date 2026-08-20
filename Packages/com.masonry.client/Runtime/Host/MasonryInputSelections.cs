#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;

namespace Masonry
{
    internal sealed class MasonryInputSelections
    {
        private readonly HashSet<KeyCode> globalKeys = new();

        public Camera? Camera { get; private set; }

        public void Reset()
        {
            Camera = null;
            globalKeys.Clear();
        }

        public void SetCamera(GameObject gameObject, ObjectId id)
        {
            Camera[] cameras = gameObject.GetComponents<Camera>();
            if (cameras.Length != 1)
            {
                throw new MasonryWorldException(
                    cameras.Length == 0
                        ? CoreErrorCode.ComponentMissing
                        : CoreErrorCode.InvalidComponentCount,
                    "Input camera selection requires exactly one root Camera; "
                        + $"found {cameras.Length}."
                );
            }

            Camera camera = cameras[0];
            if (!camera.enabled || !camera.gameObject.activeInHierarchy)
            {
                throw new MasonryWorldException(
                    CoreErrorCode.InvalidProperty,
                    $"Input camera {id.Value} must be enabled and active."
                );
            }

            Camera = camera;
        }

        public void DisableCamera(Camera camera)
        {
            if (ReferenceEquals(Camera, camera))
            {
                Camera = null;
            }
        }

        public void SetPointerEvents(GameObject gameObject, IReadOnlyList<PointerEvent> events)
        {
            ValidateUnique(events, "Pointer event");
            MasonryIdentity identity = gameObject.GetComponent<MasonryIdentity>();
            if (identity.UsesAutomaticPointerCollider)
            {
                MasonryObjectFactory.SetPointerEventsEnabled(gameObject, events.Count > 0);
            }

            identity.SetPointerEvents(events);
        }

        public void SetGlobalKeys(IReadOnlyList<KeyCode> keys)
        {
            ValidateUnique(keys, "Global key");
            globalKeys.Clear();
            globalKeys.UnionWith(keys);
        }

        public bool IsGlobalKeyEnabled(KeyCode key) => globalKeys.Contains(key);

        private static void ValidateUnique<T>(IReadOnlyList<T> values, string name)
            where T : struct, Enum
        {
            var unique = new HashSet<T>();
            foreach (T value in values)
            {
                if (!Enum.IsDefined(typeof(T), value))
                {
                    throw new MasonryWorldException(
                        CoreErrorCode.InvalidProperty,
                        $"{name} {value} is unknown."
                    );
                }

                if (!unique.Add(value))
                {
                    throw new MasonryWorldException(
                        CoreErrorCode.InvalidProperty,
                        $"{name} {value} appeared more than once."
                    );
                }
            }
        }
    }
}

#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;

namespace Battlement
{
    internal sealed class BattlementInputSelections
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
                throw new BattlementWorldException(
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
                throw new BattlementWorldException(
                    CoreErrorCode.InvalidProperty,
                    $"Input camera {id.Value} must be enabled and active."
                );
            }

            Camera = camera;
        }

        public void SetMainCamera()
        {
            Camera? camera = UnityEngine.Camera.main;
            if (camera == null || !camera.enabled || !camera.gameObject.activeInHierarchy)
            {
                throw new BattlementWorldException(
                    CoreErrorCode.InvalidProperty,
                    "Main-camera input requires an enabled, active Camera tagged MainCamera."
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
            BattlementIdentity identity = gameObject.GetComponent<BattlementIdentity>();
            if (identity.UsesAutomaticPointerCollider)
            {
                BattlementObjectFactory.SetPointerEventsEnabled(
                    gameObject,
                    events.Count > 0 || identity.DragMode is not null
                );
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
                    throw new BattlementWorldException(
                        CoreErrorCode.InvalidProperty,
                        $"{name} {value} is unknown."
                    );
                }

                if (!unique.Add(value))
                {
                    throw new BattlementWorldException(
                        CoreErrorCode.InvalidProperty,
                        $"{name} {value} appeared more than once."
                    );
                }
            }
        }
    }
}

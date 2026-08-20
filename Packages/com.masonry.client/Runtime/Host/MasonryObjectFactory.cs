#nullable enable

using System.Collections.Generic;
using System.Linq;
using TMPro;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Masonry
{
    /// <summary>Constructs and initializes Unity objects described by the protocol.</summary>
    internal sealed class MasonryObjectFactory
    {
        private readonly MasonryPreparedAssets preparedAssets;

        public MasonryObjectFactory(MasonryPreparedAssets preparedAssets) =>
            this.preparedAssets = preparedAssets;

        public (GameObject GameObject, IMasonryAssetLease? Lease) Construct(
            MasonryGameObject description
        ) =>
            description.Kind switch
            {
                GameObjectKind.Empty => (new GameObject("Masonry Empty"), null),
                GameObjectKind.Cube cube => Primitive(
                    PrimitiveType.Cube,
                    description.PointerEvents,
                    cube.Materials
                ),
                GameObjectKind.Sphere sphere => Primitive(
                    PrimitiveType.Sphere,
                    description.PointerEvents,
                    sphere.Materials
                ),
                GameObjectKind.Capsule capsule => Primitive(
                    PrimitiveType.Capsule,
                    description.PointerEvents,
                    capsule.Materials
                ),
                GameObjectKind.Cylinder cylinder => Primitive(
                    PrimitiveType.Cylinder,
                    description.PointerEvents,
                    cylinder.Materials
                ),
                GameObjectKind.Plane plane => Primitive(
                    PrimitiveType.Plane,
                    description.PointerEvents,
                    plane.Materials
                ),
                GameObjectKind.Quad quad => Primitive(
                    PrimitiveType.Quad,
                    description.PointerEvents,
                    quad.Materials
                ),
                GameObjectKind.Image image => (
                    CreateImage(image.State, description.PointerEvents.Count > 0),
                    null
                ),
                GameObjectKind.Text text => (CreateText(text.State), null),
                GameObjectKind.Camera camera => (
                    MasonryStandardComponents.CreateCamera(camera.State),
                    null
                ),
                GameObjectKind.Light light => (
                    MasonryStandardComponents.CreateLight(light.State),
                    null
                ),
                GameObjectKind.Prefab prefab => InstantiatePrefab(prefab),
                _ => throw new MasonryWorldException(
                    CoreErrorCode.InvalidProperty,
                    $"Object kind {description.Kind.GetType().Name} is not implemented yet."
                ),
            };

        public static void ApplyStableState(GameObject gameObject, MasonryGameObject description)
        {
            ApplyLocalTransform(gameObject.transform, description.LocalTransform);
            gameObject.SetActive(description.IsActive);
            if (description.Kind is GameObjectKind.Prefab { Animator: { } animator })
            {
                ApplyAnimator(gameObject, animator);
            }
        }

        public static bool UsesAutomaticPointerCollider(GameObjectKind kind) =>
            kind
                is GameObjectKind.Cube
                    or GameObjectKind.Sphere
                    or GameObjectKind.Capsule
                    or GameObjectKind.Cylinder
                    or GameObjectKind.Plane
                    or GameObjectKind.Quad
                    or GameObjectKind.Image;

        public static void SetPointerEventsEnabled(GameObject gameObject, bool enabled)
        {
            if (gameObject.GetComponent<MasonryImage>() is MasonryImage image)
            {
                image.SetPointerEventsEnabled(enabled);
                return;
            }

            Collider[] colliders = gameObject.GetComponents<Collider>();
            if (colliders.Length > 0)
            {
                foreach (Collider collider in colliders)
                {
                    collider.enabled = enabled;
                }

                return;
            }

            if (!enabled || !gameObject.TryGetComponent(out Renderer _))
            {
                return;
            }

            if (gameObject.TryGetComponent(out MeshFilter meshFilter))
            {
                gameObject.AddComponent<MeshCollider>().sharedMesh = meshFilter.sharedMesh;
            }
        }

        private GameObject CreateImage(ImageState state, bool pointerEventsEnabled)
        {
            var asset = new PreparedAsset.Texture(state.Texture);
            IMasonryAssetLease lease = preparedAssets.Acquire(asset);
            var gameObject = new GameObject("Masonry Image");
            try
            {
                gameObject
                    .AddComponent<MasonryImage>()
                    .Initialize(lease, state, pointerEventsEnabled);
                return gameObject;
            }
            catch
            {
                lease.Dispose();
                DestroyUnityObject(gameObject);
                throw;
            }
        }

        private GameObject CreateText(TextState state)
        {
            var asset = new PreparedAsset.Font(state.Font);
            IMasonryAssetLease lease = preparedAssets.Acquire(asset);
            var gameObject = new GameObject(
                "Masonry Text",
                typeof(RectTransform),
                typeof(TextMeshPro),
                typeof(MasonryText)
            );
            try
            {
                gameObject.GetComponent<MasonryText>().Initialize(lease, state);
                return gameObject;
            }
            catch
            {
                lease.Dispose();
                DestroyUnityObject(gameObject);
                throw;
            }
        }

        private (GameObject GameObject, IMasonryAssetLease? Lease) InstantiatePrefab(
            GameObjectKind.Prefab description
        )
        {
            var asset = new PreparedAsset.Prefab(description.Address);
            IMasonryAssetLease lease = preparedAssets.Acquire(asset);
            GameObject? instance = null;
            try
            {
                if (lease.Value is not GameObject prefab)
                {
                    throw new MasonryWorldException(
                        CoreErrorCode.AssetTypeMismatch,
                        $"Prepared prefab '{description.Address.Value}' is not a GameObject."
                    );
                }

                if (description.Animator is not null && prefab.GetComponent<Animator>() == null)
                {
                    throw new MasonryWorldException(
                        CoreErrorCode.ComponentMissing,
                        $"Prefab '{description.Address.Value}' has no root Animator."
                    );
                }

                instance = Object.Instantiate(prefab);
                ApplyMaterials(instance, description.Materials);
                return (instance, lease);
            }
            catch
            {
                lease.Dispose();
                if (instance != null)
                {
                    DestroyUnityObject(instance);
                }

                throw;
            }
        }

        private (GameObject GameObject, IMasonryAssetLease? Lease) Primitive(
            PrimitiveType type,
            IReadOnlyList<PointerEvent> pointerEvents,
            IReadOnlyList<MaterialAssignment> materials
        )
        {
            GameObject gameObject = GameObject.CreatePrimitive(type);
            gameObject.name = $"Masonry {type}";
            try
            {
                if (pointerEvents.Count == 0)
                {
                    foreach (Collider collider in gameObject.GetComponents<Collider>())
                    {
                        DestroyUnityObject(collider);
                    }
                }

                ApplyMaterials(gameObject, materials);
                return (gameObject, null);
            }
            catch
            {
                DestroyUnityObject(gameObject);
                throw;
            }
        }

        private void ApplyMaterials(
            GameObject gameObject,
            IReadOnlyList<MaterialAssignment> assignments
        )
        {
            if (assignments.Count == 0)
            {
                return;
            }

            Renderer[] renderers = gameObject.GetComponents<Renderer>();
            if (renderers.Length != 1)
            {
                throw new MasonryWorldException(
                    renderers.Length == 0
                        ? CoreErrorCode.ComponentMissing
                        : CoreErrorCode.InvalidComponentCount,
                    $"Material state requires exactly one root Renderer; found {renderers.Length}."
                );
            }

            gameObject
                .AddComponent<MasonryMaterialAssignments>()
                .Initialize(renderers[0], preparedAssets, assignments);
        }

        private static void ApplyAnimator(GameObject gameObject, AnimatorState state)
        {
            Animator[] animators = gameObject.GetComponents<Animator>();
            if (animators.Length != 1)
            {
                throw new MasonryWorldException(
                    animators.Length == 0
                        ? CoreErrorCode.ComponentMissing
                        : CoreErrorCode.InvalidComponentCount,
                    $"Animator state requires exactly one root Animator; found {animators.Length}."
                );
            }

            Animator animator = animators[0];
            if (state.Layer >= animator.layerCount)
            {
                throw InvalidAnimator($"Animator layer {state.Layer} does not exist.");
            }

            int layer = checked((int)state.Layer);
            int stateHash = Animator.StringToHash(state.State);
            if (string.IsNullOrEmpty(state.State) || !animator.HasState(layer, stateHash))
            {
                throw InvalidAnimator(
                    $"Animator state '{state.State}' does not exist on layer {state.Layer}."
                );
            }

            float normalizedStartTime = RequireAnimatorUnit(
                state.NormalizedStartTime,
                "Animator normalized start time"
            );
            float speed = RequireAnimatorNonnegative(state.Speed, "Animator speed");
            foreach ((string name, bool value) in state.BoolParameters)
            {
                animator.SetBool(
                    RequireAnimatorParameter(animator, name, AnimatorControllerParameterType.Bool),
                    value
                );
            }

            foreach ((string name, int value) in state.IntParameters)
            {
                animator.SetInteger(
                    RequireAnimatorParameter(animator, name, AnimatorControllerParameterType.Int),
                    value
                );
            }

            foreach ((string name, double value) in state.FloatParameters)
            {
                animator.SetFloat(
                    RequireAnimatorParameter(animator, name, AnimatorControllerParameterType.Float),
                    RequireAnimatorFinite(value, $"Animator '{name}'")
                );
            }

            animator.speed = speed;
            animator.Play(stateHash, layer, normalizedStartTime);
            animator.Update(0);
        }

        private static int RequireAnimatorParameter(
            Animator animator,
            string name,
            AnimatorControllerParameterType expectedType
        )
        {
            AnimatorControllerParameter? parameter = animator.parameters.FirstOrDefault(candidate =>
                candidate.name == name
            );
            if (parameter == null || parameter.type != expectedType)
            {
                throw InvalidAnimator(
                    $"Animator parameter '{name}' is missing or has the wrong type."
                );
            }

            return parameter.nameHash;
        }

        private static float RequireAnimatorUnit(double value, string name) =>
            double.IsFinite(value) && value is >= 0 and <= 1
                ? (float)value
                : throw InvalidAnimator($"{name} must be in the inclusive range [0, 1].");

        private static float RequireAnimatorNonnegative(double value, string name)
        {
            float converted = RequireAnimatorFinite(value, name);
            return converted >= 0
                ? converted
                : throw InvalidAnimator($"{name} must be nonnegative.");
        }

        private static float RequireAnimatorFinite(double value, string name)
        {
            float converted = (float)value;
            return double.IsFinite(value) && float.IsFinite(converted)
                ? converted
                : throw InvalidAnimator($"{name} must be finite.");
        }

        private static MasonryWorldException InvalidAnimator(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private static void ApplyLocalTransform(Transform transform, LocalTransform value)
        {
            transform.SetLocalPositionAndRotation(
                new UnityEngine.Vector3(
                    (float)value.Position.X,
                    (float)value.Position.Y,
                    (float)value.Position.Z
                ),
                Normalize(value.Rotation)
            );
            transform.localScale = new UnityEngine.Vector3(
                (float)value.Scale.X,
                (float)value.Scale.Y,
                (float)value.Scale.Z
            );
        }

        private static UnityEngine.Quaternion Normalize(Quaternion value)
        {
            var rotation = new UnityEngine.Quaternion(
                (float)value.X,
                (float)value.Y,
                (float)value.Z,
                (float)value.W
            );
            float magnitude = Mathf.Sqrt(
                (rotation.x * rotation.x)
                    + (rotation.y * rotation.y)
                    + (rotation.z * rotation.z)
                    + (rotation.w * rotation.w)
            );
            if (magnitude <= Mathf.Epsilon)
            {
                throw new MasonryWorldException(
                    CoreErrorCode.InvalidProperty,
                    "An object rotation must have nonzero length."
                );
            }

            return new UnityEngine.Quaternion(
                rotation.x / magnitude,
                rotation.y / magnitude,
                rotation.z / magnitude,
                rotation.w / magnitude
            );
        }

        private static void DestroyUnityObject(Object value)
        {
            if (Application.isPlaying)
            {
                Object.Destroy(value);
            }
            else
            {
                Object.DestroyImmediate(value);
            }
        }
    }
}

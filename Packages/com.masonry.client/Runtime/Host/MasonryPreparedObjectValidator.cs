#nullable enable

using System.Collections.Generic;
using System.Linq;
using TMPro;
using UnityEngine;

namespace Masonry
{
    /// <summary>Validates snapshot objects against their resolved Unity assets.</summary>
    internal static class MasonryPreparedObjectValidator
    {
        public static void Validate(
            IReadOnlyList<MasonryGameObject> objects,
            IMasonryPreparedAssetLookup preparedAssets,
            ObjectId? inputCameraId
        )
        {
            foreach (MasonryGameObject description in objects)
            {
                switch (description.Kind)
                {
                    case GameObjectKind.Image image:
                        RequirePreparedValue<Texture>(
                            preparedAssets,
                            new PreparedAsset.Texture(image.State.Texture),
                            image.State.Texture.Value
                        );
                        break;
                    case GameObjectKind.Text text:
                        RequirePreparedValue<TMP_FontAsset>(
                            preparedAssets,
                            new PreparedAsset.Font(text.State.Font),
                            text.State.Font.Value
                        );
                        break;
                    case GameObjectKind.Prefab prefab:
                        ValidatePrefab(description, prefab, preparedAssets, inputCameraId);
                        break;
                    case GameObjectKind.Cube cube:
                        ValidatePrimitiveMaterials(cube.Materials, preparedAssets);
                        break;
                    case GameObjectKind.Sphere sphere:
                        ValidatePrimitiveMaterials(sphere.Materials, preparedAssets);
                        break;
                    case GameObjectKind.Capsule capsule:
                        ValidatePrimitiveMaterials(capsule.Materials, preparedAssets);
                        break;
                    case GameObjectKind.Cylinder cylinder:
                        ValidatePrimitiveMaterials(cylinder.Materials, preparedAssets);
                        break;
                    case GameObjectKind.Plane plane:
                        ValidatePrimitiveMaterials(plane.Materials, preparedAssets);
                        break;
                    case GameObjectKind.Quad quad:
                        ValidatePrimitiveMaterials(quad.Materials, preparedAssets);
                        break;
                    case GameObjectKind.Empty:
                    case GameObjectKind.Camera:
                    case GameObjectKind.Light:
                        break;
                    default:
                        throw Invalid(CoreErrorCode.InvalidProperty, "Unknown game-object kind.");
                }
            }
        }

        private static void ValidatePrefab(
            MasonryGameObject description,
            GameObjectKind.Prefab prefab,
            IMasonryPreparedAssetLookup preparedAssets,
            ObjectId? inputCameraId
        )
        {
            GameObject value = RequirePreparedValue<GameObject>(
                preparedAssets,
                new PreparedAsset.Prefab(prefab.Address),
                prefab.Address.Value
            );
            foreach (MaterialAssignment assignment in prefab.Materials)
            {
                RequirePreparedValue<Material>(
                    preparedAssets,
                    new PreparedAsset.Material(assignment.Address),
                    assignment.Address.Value
                );
            }

            ValidateRootState(value, prefab.Materials, prefab.Animator);
            if (
                inputCameraId is not ObjectId selectedCameraId
                || description.Id != selectedCameraId
            )
            {
                return;
            }

            Camera[] cameras = value.GetComponents<Camera>();
            if (cameras.Length != 1 || !cameras[0].enabled)
            {
                throw Invalid(
                    CoreErrorCode.InvalidProperty,
                    $"Input camera {selectedCameraId} must be enabled and active."
                );
            }
        }

        private static void ValidatePrimitiveMaterials(
            IReadOnlyList<MaterialAssignment> assignments,
            IMasonryPreparedAssetLookup preparedAssets
        )
        {
            foreach (MaterialAssignment assignment in assignments)
            {
                if (assignment.Slot != 0)
                {
                    throw Invalid(
                        CoreErrorCode.InvalidProperty,
                        $"Renderer material slot {assignment.Slot} is outside "
                            + "the available range [0, 0]."
                    );
                }

                RequirePreparedValue<Material>(
                    preparedAssets,
                    new PreparedAsset.Material(assignment.Address),
                    assignment.Address.Value
                );
            }
        }

        private static void ValidateRootState(
            GameObject prefab,
            IReadOnlyList<MaterialAssignment> materials,
            AnimatorState? animatorState
        )
        {
            if (materials.Count > 0)
            {
                Renderer[] renderers = prefab.GetComponents<Renderer>();
                if (renderers.Length != 1)
                {
                    throw Invalid(
                        renderers.Length == 0
                            ? CoreErrorCode.ComponentMissing
                            : CoreErrorCode.InvalidComponentCount,
                        "Material state requires exactly one root Renderer; "
                            + $"found {renderers.Length}."
                    );
                }

                int slotCount = renderers[0].sharedMaterials.Length;
                foreach (MaterialAssignment assignment in materials)
                {
                    if (assignment.Slot >= slotCount)
                    {
                        throw Invalid(
                            CoreErrorCode.InvalidProperty,
                            $"Renderer material slot {assignment.Slot} is outside "
                                + "the available range."
                        );
                    }
                }
            }

            if (animatorState is null)
            {
                return;
            }

            Animator[] animators = prefab.GetComponents<Animator>();
            if (animators.Length != 1)
            {
                throw Invalid(
                    animators.Length == 0
                        ? CoreErrorCode.ComponentMissing
                        : CoreErrorCode.InvalidComponentCount,
                    animators.Length == 0
                        ? "The prefab has no root Animator."
                        : $"Prefab root has {animators.Length} Animators; exactly one is required."
                );
            }

            GameObject validationInstance = Object.Instantiate(prefab);
            try
            {
                validationInstance.SetActive(true);
                Animator animator = validationInstance.GetComponent<Animator>();
                animator.Update(0);
                ValidateAnimatorController(animator, animatorState);
            }
            finally
            {
                if (Application.isPlaying)
                {
                    Object.Destroy(validationInstance);
                }
                else
                {
                    Object.DestroyImmediate(validationInstance);
                }
            }
        }

        private static void ValidateAnimatorController(Animator animator, AnimatorState state)
        {
            if (state.Layer >= animator.layerCount)
            {
                throw Invalid(
                    CoreErrorCode.InvalidProperty,
                    $"Animator layer {state.Layer} does not exist."
                );
            }

            int layer = checked((int)state.Layer);
            int stateHash = Animator.StringToHash(state.State);
            if (!animator.HasState(layer, stateHash))
            {
                throw Invalid(
                    CoreErrorCode.InvalidProperty,
                    $"Animator state '{state.State}' does not exist on layer {state.Layer}."
                );
            }

            ValidateAnimatorParameters(
                animator,
                state.BoolParameters.Keys,
                AnimatorControllerParameterType.Bool
            );
            ValidateAnimatorParameters(
                animator,
                state.IntParameters.Keys,
                AnimatorControllerParameterType.Int
            );
            ValidateAnimatorParameters(
                animator,
                state.FloatParameters.Keys,
                AnimatorControllerParameterType.Float
            );
        }

        private static void ValidateAnimatorParameters(
            Animator animator,
            IEnumerable<string> names,
            AnimatorControllerParameterType type
        )
        {
            foreach (string name in names)
            {
                if (
                    !animator.parameters.Any(parameter =>
                        parameter.name == name && parameter.type == type
                    )
                )
                {
                    throw Invalid(
                        CoreErrorCode.InvalidProperty,
                        $"Animator parameter '{name}' is missing or has the wrong type."
                    );
                }
            }
        }

        private static T RequirePreparedValue<T>(
            IMasonryPreparedAssetLookup preparedAssets,
            PreparedAsset asset,
            string address
        )
            where T : class
        {
            if (!preparedAssets.TryGet(asset, out object? value) || value is not T typed)
            {
                string expected =
                    typeof(T) == typeof(Texture) ? "a Unity Texture"
                    : typeof(T) == typeof(TMP_FontAsset) ? "a TextMesh Pro font"
                    : typeof(T) == typeof(Material) ? "a Unity Material"
                    : "a GameObject";
                throw Invalid(
                    CoreErrorCode.AssetTypeMismatch,
                    $"Prepared asset '{address}' is not {expected}."
                );
            }

            return typed;
        }

        private static MasonryWorldException Invalid(CoreErrorCode code, string message) =>
            new(code, message);
    }
}

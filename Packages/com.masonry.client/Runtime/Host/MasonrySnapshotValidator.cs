#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;

namespace Masonry
{
    internal static class MasonrySnapshotValidator
    {
        private const int MaximumAssets = 16_384;
        private const int MaximumScenes = 32;
        private const int MaximumObjects = 100_000;
        private const int MaximumHierarchyDepth = 256;
        private const int MaximumStringBytes = 65_536;

        public static IReadOnlyList<MasonryGameObject> Validate(Snapshot snapshot)
        {
            Errors.CheckNotNull(snapshot, nameof(snapshot));
            RequireId(snapshot.SessionId.Value, "session");
            Dictionary<string, PreparedAsset> prepared = ValidatePrepared(snapshot.PreparedAssets);
            (Guid primary, HashSet<Guid> sceneIds) = ValidateScenes(
                snapshot.Scenes,
                snapshot.PrimarySceneId,
                prepared
            );
            Dictionary<Guid, MasonryGameObject> objects = IndexObjects(snapshot.Objects, sceneIds);

            foreach (MasonryGameObject description in snapshot.Objects)
            {
                ValidateObject(description, prepared, sceneIds);
            }

            IReadOnlyList<MasonryGameObject> order = TopologicalOrder(
                snapshot.Objects,
                objects,
                primary
            );
            ValidateInputCamera(snapshot.InputCameraId, objects);
            ValidateUniqueEnums(snapshot.GlobalKeys, "global key");
            return order;
        }

        private static Dictionary<string, PreparedAsset> ValidatePrepared(
            IReadOnlyList<PreparedAsset> assets
        )
        {
            Errors.CheckNotNull(assets, nameof(assets));
            if (assets.Count > MaximumAssets)
            {
                throw Invalid(
                    CoreErrorCode.LimitExceeded,
                    $"A snapshot cannot prepare more than {MaximumAssets} assets."
                );
            }

            var result = new Dictionary<string, PreparedAsset>(
                assets.Count,
                StringComparer.Ordinal
            );
            foreach (PreparedAsset asset in assets)
            {
                string address = AssetAddress(asset);
                RequireString(address, "Prepared asset address", allowEmpty: false);
                if (!result.TryAdd(address, asset))
                {
                    throw Invalid(
                        CoreErrorCode.DuplicateId,
                        $"Prepared asset address '{address}' appeared more than once."
                    );
                }
            }

            return result;
        }

        private static (Guid Primary, HashSet<Guid> Ids) ValidateScenes(
            IReadOnlyList<MasonryScene> scenes,
            SceneId? primarySceneId,
            IReadOnlyDictionary<string, PreparedAsset> prepared
        )
        {
            Errors.CheckNotNull(scenes, nameof(scenes));
            if (scenes.Count is 0 or > MaximumScenes)
            {
                throw Invalid(
                    scenes.Count == 0 ? CoreErrorCode.UnknownScene : CoreErrorCode.LimitExceeded,
                    $"A snapshot must contain between 1 and {MaximumScenes} content scenes."
                );
            }

            var ids = new HashSet<Guid>();
            var addresses = new HashSet<string>(StringComparer.Ordinal);
            foreach (MasonryScene scene in scenes)
            {
                Guid id = RequireId(scene.Id.Value, "scene");
                string address = scene.Address.Value;
                RequireString(address, "Scene address", allowEmpty: false);
                if (!ids.Add(id) || !addresses.Add(address))
                {
                    throw Invalid(
                        CoreErrorCode.DuplicateId,
                        "Scene UUIDs and addresses must be unique within a snapshot."
                    );
                }

                RequirePrepared<PreparedAsset.Scene>(prepared, address, "scene");
            }

            Guid primary =
                primarySceneId?.Value ?? (scenes.Count == 1 ? scenes[0].Id.Value : default);
            if (primary == Guid.Empty || !ids.Contains(primary))
            {
                throw Invalid(
                    CoreErrorCode.UnknownScene,
                    "The primary scene must name a scene in the snapshot."
                );
            }

            return (primary, ids);
        }

        private static Dictionary<Guid, MasonryGameObject> IndexObjects(
            IReadOnlyList<MasonryGameObject> objects,
            ISet<Guid> sceneIds
        )
        {
            Errors.CheckNotNull(objects, nameof(objects));
            if (objects.Count > MaximumObjects)
            {
                throw Invalid(
                    CoreErrorCode.LimitExceeded,
                    $"A snapshot cannot contain more than {MaximumObjects} game objects."
                );
            }

            var result = new Dictionary<Guid, MasonryGameObject>(objects.Count);
            foreach (MasonryGameObject description in objects)
            {
                Guid id = RequireId(description.Id.Value, "game object");
                if (sceneIds.Contains(id) || !result.TryAdd(id, description))
                {
                    throw Invalid(
                        CoreErrorCode.DuplicateId,
                        $"UUID {id} appeared more than once in the snapshot."
                    );
                }
            }

            return result;
        }

        private static IReadOnlyList<MasonryGameObject> TopologicalOrder(
            IReadOnlyList<MasonryGameObject> descriptions,
            IReadOnlyDictionary<Guid, MasonryGameObject> objects,
            Guid primaryScene
        )
        {
            var depths = new Dictionary<Guid, int>(objects.Count);
            var visiting = new HashSet<Guid>();
            foreach (MasonryGameObject description in descriptions)
            {
                Depth(description, objects, primaryScene, depths, visiting);
            }

            return descriptions
                .Select((value, index) => (value, index, depth: depths[value.Id.Value]))
                .OrderBy(item => item.depth)
                .ThenBy(item => item.index)
                .Select(item => item.value)
                .ToArray();
        }

        private static int Depth(
            MasonryGameObject description,
            IReadOnlyDictionary<Guid, MasonryGameObject> objects,
            Guid primaryScene,
            IDictionary<Guid, int> depths,
            ISet<Guid> visiting
        )
        {
            Guid id = description.Id.Value;
            if (depths.TryGetValue(id, out int known))
            {
                return known;
            }

            if (!visiting.Add(id))
            {
                throw Invalid(CoreErrorCode.InvalidHierarchy, "The object hierarchy is cyclic.");
            }

            int depth = 0;
            if (description.ParentId is ObjectId parentId)
            {
                if (!objects.TryGetValue(parentId.Value, out MasonryGameObject parent))
                {
                    throw Invalid(
                        CoreErrorCode.UnknownObject,
                        $"Parent object {parentId} is not in the snapshot."
                    );
                }

                if (
                    Placement(description.ParentScene, primaryScene)
                    != Placement(parent.ParentScene, primaryScene)
                )
                {
                    throw Invalid(
                        CoreErrorCode.InvalidHierarchy,
                        $"Object {id} and its parent must belong to the same scene."
                    );
                }

                depth = checked(Depth(parent, objects, primaryScene, depths, visiting) + 1);
                if (depth > MaximumHierarchyDepth)
                {
                    throw Invalid(
                        CoreErrorCode.LimitExceeded,
                        $"The object hierarchy cannot exceed {MaximumHierarchyDepth} levels."
                    );
                }
            }

            visiting.Remove(id);
            depths.Add(id, depth);
            return depth;
        }

        private static Guid? Placement(ParentScene placement, Guid primaryScene) =>
            placement switch
            {
                ParentScene.Primary => primaryScene,
                ParentScene.Specific specific => specific.SceneId.Value,
                ParentScene.Persistent => null,
                _ => throw Invalid(CoreErrorCode.UnknownScene, "Unknown parent-scene selection."),
            };

        private static void ValidateObject(
            MasonryGameObject description,
            IReadOnlyDictionary<string, PreparedAsset> prepared,
            ISet<Guid> sceneIds
        )
        {
            Errors.CheckNotNull(description.Kind, nameof(description.Kind));
            Errors.CheckNotNull(description.ParentScene, nameof(description.ParentScene));
            ValidateTransform(description.LocalTransform);
            ValidateUniqueEnums(description.PointerEvents, "pointer event");
            if (description.DragMode is DragMode dragMode)
            {
                RequireEnum(dragMode, "drag mode");
            }

            switch (description.ParentScene)
            {
                case ParentScene.Specific specific:
                    Guid sceneId = RequireId(specific.SceneId.Value, "parent scene");
                    if (!sceneIds.Contains(sceneId))
                    {
                        throw Invalid(
                            CoreErrorCode.UnknownScene,
                            $"Parent scene {sceneId} is not in the snapshot."
                        );
                    }
                    break;
                case ParentScene.Primary:
                case ParentScene.Persistent:
                    break;
                default:
                    throw Invalid(CoreErrorCode.UnknownScene, "Unknown parent-scene selection.");
            }

            switch (description.Kind)
            {
                case GameObjectKind.Empty:
                    break;
                case GameObjectKind.Cube cube:
                    ValidateMaterials(cube.Materials, prepared);
                    break;
                case GameObjectKind.Sphere sphere:
                    ValidateMaterials(sphere.Materials, prepared);
                    break;
                case GameObjectKind.Capsule capsule:
                    ValidateMaterials(capsule.Materials, prepared);
                    break;
                case GameObjectKind.Cylinder cylinder:
                    ValidateMaterials(cylinder.Materials, prepared);
                    break;
                case GameObjectKind.Plane plane:
                    ValidateMaterials(plane.Materials, prepared);
                    break;
                case GameObjectKind.Quad quad:
                    ValidateMaterials(quad.Materials, prepared);
                    break;
                case GameObjectKind.Image image:
                    ValidateImage(image.State);
                    RequirePrepared<PreparedAsset.Texture>(
                        prepared,
                        image.State.Texture.Value,
                        "texture"
                    );
                    break;
                case GameObjectKind.Text text:
                    ValidateText(text.State);
                    RequirePrepared<PreparedAsset.Font>(prepared, text.State.Font.Value, "font");
                    break;
                case GameObjectKind.Camera camera:
                    ValidateCamera(camera.State);
                    break;
                case GameObjectKind.Light light:
                    ValidateLight(light.State);
                    break;
                case GameObjectKind.Prefab prefab:
                    RequirePrepared<PreparedAsset.Prefab>(prepared, prefab.Address.Value, "prefab");
                    ValidateMaterials(prefab.Materials, prepared);
                    if (prefab.Animator is not null)
                    {
                        ValidateAnimatorValues(prefab.Animator);
                    }
                    break;
                default:
                    throw Invalid(CoreErrorCode.InvalidProperty, "Unknown game-object kind.");
            }
        }

        private static void ValidateInputCamera(
            ObjectId inputCameraId,
            IReadOnlyDictionary<Guid, MasonryGameObject> objects
        )
        {
            Guid id = RequireId(inputCameraId.Value, "input camera");
            if (
                !objects.TryGetValue(id, out MasonryGameObject camera)
                || (
                    camera.Kind is not GameObjectKind.Camera { State: { IsEnabled: true } }
                    && camera.Kind is not GameObjectKind.Prefab
                )
            )
            {
                throw Invalid(
                    CoreErrorCode.InvalidProperty,
                    $"Input camera {id} must be enabled and active in the snapshot."
                );
            }

            MasonryGameObject current = camera;
            while (true)
            {
                if (!current.IsActive)
                {
                    throw Invalid(
                        CoreErrorCode.InvalidProperty,
                        $"Input camera {id} must be active in the hierarchy."
                    );
                }

                if (current.ParentId is not ObjectId parentId)
                {
                    return;
                }

                current = objects[parentId.Value];
            }
        }

        private static void ValidateMaterials(
            IReadOnlyList<MaterialAssignment> assignments,
            IReadOnlyDictionary<string, PreparedAsset> prepared
        )
        {
            Errors.CheckNotNull(assignments, nameof(assignments));
            var slots = new HashSet<uint>();
            foreach (MaterialAssignment assignment in assignments)
            {
                if (!slots.Add(assignment.Slot))
                {
                    throw Invalid(
                        CoreErrorCode.InvalidProperty,
                        $"Renderer material slot {assignment.Slot} appeared twice."
                    );
                }

                RequirePrepared<PreparedAsset.Material>(
                    prepared,
                    assignment.Address.Value,
                    "material"
                );
            }
        }

        private static void ValidateTransform(LocalTransform value)
        {
            RequireFinite(value.Position.X, "Local position");
            RequireFinite(value.Position.Y, "Local position");
            RequireFinite(value.Position.Z, "Local position");
            RequireFinite(value.Scale.X, "Local scale");
            RequireFinite(value.Scale.Y, "Local scale");
            RequireFinite(value.Scale.Z, "Local scale");
            double x = RequireFinite(value.Rotation.X, "Local rotation");
            double y = RequireFinite(value.Rotation.Y, "Local rotation");
            double z = RequireFinite(value.Rotation.Z, "Local rotation");
            double w = RequireFinite(value.Rotation.W, "Local rotation");
            if ((x * x) + (y * y) + (z * z) + (w * w) <= double.Epsilon)
            {
                throw Invalid(
                    CoreErrorCode.InvalidProperty,
                    "An object rotation must have nonzero length."
                );
            }
        }

        private static void ValidateImage(ImageState state)
        {
            Errors.CheckNotNull(state, nameof(state));
            RequirePositive(state.Width, "Image width");
            RequirePositive(state.Height, "Image height");
            RequireUnit(state.Tint.Red, "Image tint red");
            RequireUnit(state.Tint.Green, "Image tint green");
            RequireUnit(state.Tint.Blue, "Image tint blue");
            RequireUnit(state.Opacity, "Image opacity");
            RequireEnum(state.Fit, "image fit");
        }

        private static void ValidateText(TextState state)
        {
            Errors.CheckNotNull(state, nameof(state));
            RequireString(state.Text, "Text", allowEmpty: true);
            RequirePositive(state.Size, "Text size");
            if (state.WrapWidth is double width)
            {
                RequirePositive(width, "Text wrap width");
            }
            ValidateColor(state.Color, "Text color");
            RequireEnum(state.HorizontalAlignment, "horizontal text alignment");
            RequireEnum(state.VerticalAlignment, "vertical text alignment");
        }

        private static void ValidateCamera(CameraState state)
        {
            Errors.CheckNotNull(state, nameof(state));
            RequireEnum(state.Projection, "camera projection");
            RequireStrictRange(state.FieldOfView, 1, 179, "Camera field of view");
            RequirePositive(state.OrthographicSize, "Camera orthographic size");
            RequirePositive(state.NearClip, "Camera near clip");
            double far = RequireFinite(state.FarClip, "Camera far clip");
            if (far <= state.NearClip)
            {
                throw Invalid(
                    CoreErrorCode.InvalidProperty,
                    "Camera far clip must be greater than its near clip."
                );
            }
            RequireEnum(state.ClearMode, "camera clear mode");
            ValidateColor(state.ClearColor, "Camera clear color");
        }

        private static void ValidateLight(LightState state)
        {
            Errors.CheckNotNull(state, nameof(state));
            RequireEnum(state.Type, "light type");
            ValidateColor(state.Color, "Light color");
            RequireNonnegative(state.Intensity, "Light intensity");
            RequirePositive(state.Range, "Light range");
            double outer = RequireStrictRange(
                state.OuterSpotAngle,
                0,
                179,
                "Light outer spot angle"
            );
            double inner = RequireNonnegative(state.InnerSpotAngle, "Light inner spot angle");
            if (inner > outer)
            {
                throw Invalid(
                    CoreErrorCode.InvalidProperty,
                    "Light inner spot angle cannot exceed its outer angle."
                );
            }
            RequireEnum(state.Shadows, "light shadow mode");
        }

        private static void ValidateAnimatorValues(AnimatorState state)
        {
            RequireString(state.State, "Animator state", allowEmpty: false);
            RequireUnit(state.NormalizedStartTime, "Animator normalized start time");
            RequireNonnegative(state.Speed, "Animator speed");
            ValidateAnimatorMap(state.BoolParameters, _ => { });
            ValidateAnimatorMap(state.IntParameters, _ => { });
            ValidateAnimatorMap(
                state.FloatParameters,
                value => RequireFinite(value, "Animator parameter")
            );
        }

        private static void ValidateAnimatorMap<T>(
            IReadOnlyDictionary<string, T> values,
            Action<T> validate
        )
        {
            Errors.CheckNotNull(values, nameof(values));
            foreach ((string name, T value) in values)
            {
                RequireString(name, "Animator parameter", allowEmpty: false);
                validate(value);
            }
        }

        private static void ValidateColor(Color value, string name)
        {
            RequireUnit(value.Red, $"{name} red");
            RequireUnit(value.Green, $"{name} green");
            RequireUnit(value.Blue, $"{name} blue");
            RequireUnit(value.Alpha, $"{name} alpha");
        }

        private static void ValidateUniqueEnums<T>(IReadOnlyList<T> values, string name)
            where T : struct, Enum
        {
            Errors.CheckNotNull(values, nameof(values));
            var unique = new HashSet<T>();
            foreach (T value in values)
            {
                RequireEnum(value, name);
                if (!unique.Add(value))
                {
                    throw Invalid(
                        CoreErrorCode.InvalidProperty,
                        $"A {name} appeared more than once."
                    );
                }
            }
        }

        private static void RequirePrepared<T>(
            IReadOnlyDictionary<string, PreparedAsset> prepared,
            string address,
            string kind
        )
            where T : PreparedAsset
        {
            RequireString(address, $"{kind} address", allowEmpty: false);
            if (!prepared.TryGetValue(address, out PreparedAsset asset) || asset is not T)
            {
                throw Invalid(
                    CoreErrorCode.AssetNotPrepared,
                    $"The {kind} address '{address}' was not in the prepared set "
                        + "with the required type."
                );
            }
        }

        private static string AssetAddress(PreparedAsset asset) =>
            Errors.CheckNotNull(asset, nameof(asset)) switch
            {
                PreparedAsset.Scene value => value.Address.Value,
                PreparedAsset.Prefab value => value.Address.Value,
                PreparedAsset.ParticleEffect value => value.Address.Value,
                PreparedAsset.Material value => value.Address.Value,
                PreparedAsset.Texture value => value.Address.Value,
                PreparedAsset.AudioClip value => value.Address.Value,
                PreparedAsset.Font value => value.Address.Value,
                _ => throw Invalid(CoreErrorCode.UnknownAsset, "Unknown prepared asset kind."),
            };

        private static Guid RequireId(Guid value, string name) =>
            value != Guid.Empty
                ? value
                : throw Invalid(CoreErrorCode.InvalidProperty, $"The {name} UUID must be nonzero.");

        private static void RequireString(string? value, string name, bool allowEmpty)
        {
            if (value is null || (!allowEmpty && value.Length == 0))
            {
                throw Invalid(CoreErrorCode.InvalidProperty, $"{name} cannot be empty.");
            }
            if (Encoding.UTF8.GetByteCount(value) > MaximumStringBytes)
            {
                throw Invalid(
                    CoreErrorCode.LimitExceeded,
                    $"{name} exceeds {MaximumStringBytes} UTF-8 bytes."
                );
            }
        }

        private static double RequireFinite(double value, string name)
        {
            float converted = (float)value;
            return double.IsFinite(value) && float.IsFinite(converted)
                ? value
                : throw Invalid(CoreErrorCode.InvalidProperty, $"{name} must be finite.");
        }

        private static double RequirePositive(double value, string name)
        {
            float converted = (float)value;
            return double.IsFinite(value) && value > 0 && float.IsFinite(converted)
                ? value
                : throw Invalid(
                    CoreErrorCode.InvalidProperty,
                    $"{name} must be finite and positive."
                );
        }

        private static double RequireNonnegative(double value, string name)
        {
            RequireFinite(value, name);
            return value >= 0
                ? value
                : throw Invalid(CoreErrorCode.InvalidProperty, $"{name} must be nonnegative.");
        }

        private static double RequireUnit(double value, string name)
        {
            RequireFinite(value, name);
            return value is >= 0 and <= 1
                ? value
                : throw Invalid(CoreErrorCode.InvalidProperty, $"{name} must be in [0, 1].");
        }

        private static double RequireStrictRange(
            double value,
            double minimum,
            double maximum,
            string name
        )
        {
            RequireFinite(value, name);
            return value > minimum && value < maximum
                ? value
                : throw Invalid(
                    CoreErrorCode.InvalidProperty,
                    $"{name} must be between {minimum} and {maximum}."
                );
        }

        private static void RequireEnum<T>(T value, string name)
            where T : struct, Enum
        {
            if (!Enum.IsDefined(typeof(T), value))
            {
                throw Invalid(CoreErrorCode.InvalidProperty, $"Unknown {name} value.");
            }
        }

        private static MasonryWorldException Invalid(CoreErrorCode code, string message) =>
            new(code, message);
    }
}

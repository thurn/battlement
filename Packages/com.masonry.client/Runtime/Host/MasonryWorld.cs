#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.SceneManagement;
using Object = UnityEngine.Object;

namespace Masonry
{
    internal sealed class MasonryWorld : IDisposable
    {
        private readonly Dictionary<Guid, MasonryIdentity> objects = new();
        private readonly Dictionary<Guid, GameObject> sceneContainers = new();

        // Retain every object and scene ID for the session so engine bugs or replayed
        // creates cannot silently assign an old identity to a different Unity entity.
        private readonly HashSet<Guid> usedIds = new();
        private readonly MasonryPreparedAssets preparedAssets;
        private readonly GameObject persistentContainer;
        private Camera? inputCamera;
        private Guid? primarySceneId;
        private bool isDisposed;

        public MasonryWorld(MasonryRunner runner, MasonryPreparedAssets preparedAssets)
        {
            this.preparedAssets = preparedAssets;
            persistentContainer = new GameObject("Masonry Persistent");
            SceneManager.MoveGameObjectToScene(persistentContainer, runner.gameObject.scene);
        }

        public void BeginSession()
        {
            DestroyOwnedObjects();
            primarySceneId = null;
            inputCamera = null;
            usedIds.Clear();
        }

        public void CreateInitialObjects(IReadOnlyList<MasonryGameObject> descriptions)
        {
            var pendingIds = new HashSet<Guid>();
            foreach (MasonryGameObject description in descriptions)
            {
                Guid value = ValidateNewObjectId(description.Id);
                if (!pendingIds.Add(value))
                {
                    throw DuplicateId("object", value);
                }
            }

            foreach (MasonryGameObject description in descriptions)
            {
                Transform container = ResolveContainer(description.ParentScene);
                (GameObject gameObject, IMasonryAssetLease? lease) = Construct(description);
                try
                {
                    RegisterObject(description.Id, gameObject, container, lease);
                }
                catch
                {
                    lease?.Dispose();
                    DestroyUnityObject(gameObject);
                    throw;
                }
            }

            foreach (MasonryGameObject description in descriptions)
            {
                if (description.ParentId is null)
                {
                    continue;
                }

                GameObject gameObject = RequireObject(description.Id);
                GameObject parent = RequireObject(description.ParentId.Value);
                if (gameObject.scene != parent.scene)
                {
                    throw new MasonryWorldException(
                        CoreErrorCode.InvalidHierarchy,
                        $"Object {description.Id} and parent {description.ParentId} "
                            + "are in different scenes."
                    );
                }

                gameObject.transform.SetParent(parent.transform, false);
            }

            foreach (MasonryGameObject description in descriptions)
            {
                GameObject gameObject = RequireObject(description.Id);
                ApplyLocalTransform(gameObject.transform, description.LocalTransform);
                gameObject.SetActive(description.IsActive);
            }
        }

        public Transform RegisterScene(SceneId id, Scene scene)
        {
            Guid value = RequireNonzero(id.Value, nameof(id));
            if (!scene.IsValid() || !scene.isLoaded)
            {
                throw new MasonryWorldException(
                    CoreErrorCode.UnknownScene,
                    $"Scene {value} is not loaded."
                );
            }

            if (!usedIds.Add(value))
            {
                throw DuplicateId("scene", value);
            }

            var container = new GameObject("Masonry Scene");
            SceneManager.MoveGameObjectToScene(container, scene);
            sceneContainers.Add(value, container);
            return container.transform;
        }

        public void SetPrimaryScene(SceneId id)
        {
            Guid value = RequireNonzero(id.Value, nameof(id));
            Transform container = RequireSceneContainer(id);
            Scene scene = container.gameObject.scene;
            if (SceneManager.GetActiveScene() != scene && !SceneManager.SetActiveScene(scene))
            {
                throw new MasonryWorldException(
                    CoreErrorCode.UnknownScene,
                    $"Scene {value} could not become the active scene."
                );
            }

            primarySceneId = value;
        }

        public void RemoveScene(SceneId id)
        {
            Guid value = RequireNonzero(id.Value, nameof(id));
            if (!sceneContainers.TryGetValue(value, out GameObject container))
            {
                return;
            }

            Scene scene = container != null ? container.scene : default;
            foreach (MasonryIdentity identity in objects.Values.ToArray())
            {
                if (
                    identity != null
                    && identity.gameObject != null
                    && identity.gameObject.scene == scene
                )
                {
                    if (identity.TryGetComponent(out MasonryPrefabLease prefabLease))
                    {
                        prefabLease.Release();
                    }

                    if (identity.TryGetComponent(out MasonryImage image))
                    {
                        image.Release();
                    }

                    DestroyUnityObject(identity.gameObject);
                }
            }

            if (container != null)
            {
                DestroyUnityObject(container);
            }

            sceneContainers.Remove(value);
            if (primarySceneId == value)
            {
                primarySceneId = null;
            }
        }

        public GameObject RequireObject(ObjectId id)
        {
            Guid value = RequireNonzero(id.Value, nameof(id));
            if (objects.TryGetValue(value, out MasonryIdentity identity))
            {
                if (identity != null && identity.gameObject != null)
                {
                    return identity.gameObject;
                }

                objects.Remove(value);
            }

            throw new MasonryWorldException(
                CoreErrorCode.UnknownObject,
                $"Object {value} does not exist."
            );
        }

        public Transform RequireSceneContainer(SceneId id)
        {
            Guid value = RequireNonzero(id.Value, nameof(id));
            if (sceneContainers.TryGetValue(value, out GameObject container))
            {
                if (container != null)
                {
                    return container.transform;
                }

                sceneContainers.Remove(value);
            }

            throw new MasonryWorldException(
                CoreErrorCode.UnknownScene,
                $"Scene {value} does not exist."
            );
        }

        public bool Contains(MasonryIdentity identity)
        {
            if (identity == null || identity.Id == Guid.Empty)
            {
                return false;
            }

            return objects.TryGetValue(identity.Id, out MasonryIdentity registered)
                && ReferenceEquals(registered, identity);
        }

        public void ConfigureInputCamera(ObjectId id)
        {
            Guid value = RequireNonzero(id.Value, nameof(id));
            inputCamera = null;
            if (
                objects.TryGetValue(value, out MasonryIdentity identity)
                && identity != null
                && identity.TryGetComponent(out Camera camera)
            )
            {
                inputCamera = camera;
            }
        }

        public void UpdateBillboards()
        {
            if (inputCamera == null)
            {
                return;
            }

            foreach (MasonryIdentity identity in objects.Values)
            {
                if (identity != null && identity.TryGetComponent(out MasonryImage image))
                {
                    image.UpdateBillboard(inputCamera);
                }
            }
        }

        public void Unregister(MasonryIdentity identity)
        {
            if (
                identity.Id != Guid.Empty
                && objects.TryGetValue(identity.Id, out MasonryIdentity registered)
                && ReferenceEquals(registered, identity)
            )
            {
                objects.Remove(identity.Id);
            }
        }

        public void Dispose()
        {
            if (isDisposed)
            {
                return;
            }

            DestroyOwnedObjects();
            foreach (GameObject container in sceneContainers.Values.ToArray())
            {
                DestroyUnityObject(container);
            }

            sceneContainers.Clear();
            if (persistentContainer != null)
            {
                DestroyUnityObject(persistentContainer);
            }

            isDisposed = true;
        }

        private (GameObject GameObject, IMasonryAssetLease? Lease) Construct(
            MasonryGameObject description
        )
        {
            return description.Kind switch
            {
                GameObjectKind.Empty => (new GameObject("Masonry Empty"), null),
                GameObjectKind.Cube => Primitive(PrimitiveType.Cube, description.PointerEvents),
                GameObjectKind.Sphere => Primitive(PrimitiveType.Sphere, description.PointerEvents),
                GameObjectKind.Capsule => Primitive(
                    PrimitiveType.Capsule,
                    description.PointerEvents
                ),
                GameObjectKind.Cylinder => Primitive(
                    PrimitiveType.Cylinder,
                    description.PointerEvents
                ),
                GameObjectKind.Plane => Primitive(PrimitiveType.Plane, description.PointerEvents),
                GameObjectKind.Quad => Primitive(PrimitiveType.Quad, description.PointerEvents),
                GameObjectKind.Image image => (
                    CreateImage(image.State, description.PointerEvents.Count > 0),
                    null
                ),
                GameObjectKind.Prefab prefab => InstantiatePrefab(prefab),
                _ => throw new MasonryWorldException(
                    CoreErrorCode.InvalidProperty,
                    $"Object kind {description.Kind.GetType().Name} is not implemented yet."
                ),
            };
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

        private (GameObject GameObject, IMasonryAssetLease? Lease) InstantiatePrefab(
            GameObjectKind.Prefab description
        )
        {
            var asset = new PreparedAsset.Prefab(description.Address);
            IMasonryAssetLease lease = preparedAssets.Acquire(asset);
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

                return (Object.Instantiate(prefab), lease);
            }
            catch
            {
                lease.Dispose();
                throw;
            }
        }

        private void RegisterObject(
            ObjectId id,
            GameObject gameObject,
            Transform parent,
            IMasonryAssetLease? lease
        )
        {
            Guid value = RequireNonzero(id.Value, nameof(id));
            if (!usedIds.Add(value))
            {
                throw DuplicateId("object", value);
            }

            gameObject.transform.SetParent(parent, false);
            if (lease is not null)
            {
                gameObject.AddComponent<MasonryPrefabLease>().Initialize(lease);
            }

            MasonryIdentity identity = gameObject.AddComponent<MasonryIdentity>();
            identity.Initialize(this, value);
            objects.Add(value, identity);
        }

        private static (GameObject GameObject, IMasonryAssetLease? Lease) Primitive(
            PrimitiveType type,
            IReadOnlyList<PointerEvent> pointerEvents
        )
        {
            GameObject gameObject = GameObject.CreatePrimitive(type);
            gameObject.name = $"Masonry {type}";
            if (pointerEvents.Count == 0)
            {
                foreach (Collider collider in gameObject.GetComponents<Collider>())
                {
                    DestroyUnityObject(collider);
                }
            }

            return (gameObject, null);
        }

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

        private Guid ValidateNewObjectId(ObjectId id)
        {
            Guid value = RequireNonzero(id.Value, nameof(id));
            if (usedIds.Contains(value))
            {
                throw DuplicateId("object", value);
            }

            return value;
        }

        private Transform ResolveContainer(ParentScene parentScene) =>
            parentScene switch
            {
                ParentScene.Persistent => persistentContainer.transform,
                ParentScene.Specific specific => RequireSceneContainer(specific.SceneId),
                ParentScene.Primary => primarySceneId is Guid id
                    ? RequireSceneContainer(new SceneId(id))
                    : throw new MasonryWorldException(
                        CoreErrorCode.UnknownScene,
                        "The primary content scene is not loaded."
                    ),
                _ => throw new MasonryWorldException(
                    CoreErrorCode.UnknownScene,
                    "The parent scene selection is unknown."
                ),
            };

        private void DestroyOwnedObjects()
        {
            foreach (MasonryIdentity identity in objects.Values.ToArray())
            {
                if (identity != null && identity.gameObject != null)
                {
                    if (identity.TryGetComponent(out MasonryPrefabLease prefabLease))
                    {
                        prefabLease.Release();
                    }

                    if (identity.TryGetComponent(out MasonryImage image))
                    {
                        image.Release();
                    }

                    DestroyUnityObject(identity.gameObject);
                }
            }

            objects.Clear();
        }

        private static Guid RequireNonzero(Guid value, string parameterName)
        {
            if (value == Guid.Empty)
            {
                throw new ArgumentException("Masonry UUIDs must be nonzero.", parameterName);
            }

            return value;
        }

        private static MasonryWorldException DuplicateId(string kind, Guid id) =>
            new(CoreErrorCode.DuplicateId, $"The {kind} UUID {id} was already used.");

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

    internal sealed class MasonryWorldException : InvalidOperationException
    {
        public MasonryWorldException(CoreErrorCode errorCode, string message)
            : base(message) => ErrorCode = errorCode;

        public CoreErrorCode ErrorCode { get; }
    }
}

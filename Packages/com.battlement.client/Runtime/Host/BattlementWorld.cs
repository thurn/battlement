#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.SceneManagement;
using Object = UnityEngine.Object;

namespace Battlement
{
    internal sealed class BattlementWorld : IDisposable
    {
        private readonly Dictionary<Guid, BattlementIdentity> objects = new();
        private readonly Dictionary<Guid, GameObject> sceneContainers = new();

        // Retain every object and scene ID for the session so engine bugs or replayed
        // creates cannot silently assign an old identity to a different Unity entity.
        private readonly HashSet<Guid> usedIds = new();
        private readonly BattlementObjectFactory objectFactory;
        private readonly GameObject persistentContainer;
        private HashSet<Guid>? replacementIds;
        private HashSet<Guid>? replacementSceneIds;
        private readonly BattlementInputSelections input = new();
        private Guid? primarySceneId;
        private bool isDisposed;

        public event Action<Camera?>? InputCameraChanged;

        public BattlementWorld(Scene hostScene, BattlementPreparedAssets preparedAssets)
        {
            objectFactory = new BattlementObjectFactory(preparedAssets);
            persistentContainer = new GameObject("Battlement Persistent");
            SceneManager.MoveGameObjectToScene(persistentContainer, hostScene);
        }

        public void BeginSession()
        {
            DestroyOwnedObjects();
            primarySceneId = null;
            input.Reset();
            InputCameraChanged?.Invoke(null);
            replacementIds = null;
            replacementSceneIds = null;
            usedIds.Clear();
        }

        public void PrepareReplacement(
            IReadOnlyList<BattlementGameObject> descriptions,
            IReadOnlyList<BattlementScene> scenes
        )
        {
            var pendingIds = new HashSet<Guid>();
            foreach (BattlementGameObject description in descriptions)
            {
                Guid value = RequireNonzero(description.Id.Value, nameof(description.Id));
                if (!pendingIds.Add(value))
                {
                    throw DuplicateId("object", value);
                }

                if (
                    usedIds.Contains(value)
                    && (
                        !objects.TryGetValue(value, out BattlementIdentity identity)
                        || identity == null
                        || identity.gameObject == null
                    )
                )
                {
                    throw DuplicateId("object", value);
                }
            }

            replacementIds = pendingIds;
            replacementSceneIds = new HashSet<Guid>();
            foreach (BattlementScene description in scenes)
            {
                Guid value = RequireNonzero(description.Id.Value, nameof(description.Id));
                if (!replacementSceneIds.Add(value))
                {
                    throw DuplicateId("scene", value);
                }

                if (!usedIds.Contains(value))
                {
                    continue;
                }

                if (
                    sceneContainers.TryGetValue(value, out GameObject container)
                    && container != null
                )
                {
                    continue;
                }

                throw DuplicateId("scene", value);
            }
        }

        public void ReplaceObjects(IReadOnlyList<BattlementGameObject> descriptions)
        {
            HashSet<Guid> allowedIds =
                replacementIds
                ?? throw new InvalidOperationException("Object replacement was not prepared.");
            replacementIds = null;
            DestroyOwnedObjects();

            foreach (BattlementGameObject description in descriptions)
            {
                Transform container = ResolveContainer(description.ParentScene);
                (GameObject gameObject, IBattlementAssetLease? lease) = objectFactory.Construct(
                    description
                );
                try
                {
                    RegisterObject(
                        description.Id,
                        gameObject,
                        container,
                        lease,
                        description.PointerEvents,
                        description.DragMode,
                        BattlementObjectFactory.UsesAutomaticPointerCollider(description.Kind),
                        allowedIds.Contains(description.Id.Value)
                    );
                }
                catch
                {
                    lease?.Dispose();
                    DestroyUnityObject(gameObject);
                    throw;
                }
            }

            foreach (BattlementGameObject description in descriptions)
            {
                if (description.ParentId is null)
                {
                    continue;
                }

                GameObject gameObject = RequireObject(description.Id);
                GameObject parent = RequireObject(description.ParentId.Value);
                if (gameObject.scene != parent.scene)
                {
                    throw new BattlementWorldException(
                        CoreErrorCode.InvalidHierarchy,
                        $"Object {description.Id} and parent {description.ParentId} "
                            + "are in different scenes."
                    );
                }

                gameObject.transform.SetParent(parent.transform, false);
            }

            foreach (BattlementGameObject description in descriptions)
            {
                GameObject gameObject = RequireObject(description.Id);
                BattlementObjectFactory.ApplyStableState(gameObject, description);
            }
        }

        public void CreateObject(BattlementGameObject description)
        {
            Transform container = ResolveContainer(description.ParentScene);
            GameObject? parent = description.ParentId is ObjectId parentId
                ? RequireObject(parentId)
                : null;
            (GameObject gameObject, IBattlementAssetLease? lease) = objectFactory.Construct(
                description
            );
            bool registered = false;
            try
            {
                if (parent != null && parent.scene != container.gameObject.scene)
                {
                    throw new BattlementWorldException(
                        CoreErrorCode.InvalidHierarchy,
                        $"Object {description.Id} and parent {description.ParentId} "
                            + "are in different scenes."
                    );
                }

                RegisterObject(
                    description.Id,
                    gameObject,
                    container,
                    lease,
                    description.PointerEvents,
                    description.DragMode,
                    BattlementObjectFactory.UsesAutomaticPointerCollider(description.Kind)
                );
                registered = true;
                if (parent != null)
                {
                    gameObject.transform.SetParent(parent.transform, false);
                }

                BattlementObjectFactory.ApplyStableState(gameObject, description);
            }
            catch
            {
                if (registered)
                {
                    BattlementIdentity identity = gameObject.GetComponent<BattlementIdentity>();
                    Unregister(identity);
                    BattlementOwnedResources.Release(gameObject);
                }
                else
                {
                    lease?.Dispose();
                }

                DestroyUnityObject(gameObject);
                throw;
            }
        }

        public IReadOnlyList<Guid> GetHierarchyObjectIds(ObjectId id)
        {
            Transform root = RequireObject(id).transform;
            return objects
                .Where(pair =>
                    pair.Value != null
                    && pair.Value.gameObject != null
                    && pair.Value.transform.IsChildOf(root)
                )
                .Select(pair => pair.Key)
                .ToArray();
        }

        public IReadOnlyList<Guid> GetSceneObjectIds(SceneId id)
        {
            Scene scene = RequireSceneContainer(id).gameObject.scene;
            return objects
                .Where(pair =>
                    pair.Value != null
                    && pair.Value.gameObject != null
                    && pair.Value.gameObject.scene == scene
                )
                .Select(pair => pair.Key)
                .ToArray();
        }

        public void DestroyObject(ObjectId id)
        {
            GameObject root = RequireObject(id);
            foreach (Guid childId in GetHierarchyObjectIds(id))
            {
                if (objects.TryGetValue(childId, out BattlementIdentity identity))
                {
                    BattlementOwnedResources.Release(identity.gameObject);
                    objects.Remove(childId);
                }
            }

            root.SetActive(false);
            DestroyUnityObject(root);
        }

        public void ValidateReparent(ObjectId id, ObjectId? parentId)
        {
            GameObject gameObject = RequireObject(id);
            if (parentId is null)
            {
                return;
            }

            GameObject parent = RequireObject(parentId.Value);
            if (parent == gameObject || parent.transform.IsChildOf(gameObject.transform))
            {
                throw new BattlementWorldException(
                    CoreErrorCode.InvalidHierarchy,
                    $"Object {id} cannot be parented beneath itself."
                );
            }

            if (gameObject.scene != parent.scene)
            {
                throw new BattlementWorldException(
                    CoreErrorCode.InvalidHierarchy,
                    $"Object {id} and parent {parentId} are in different scenes."
                );
            }
        }

        public void Reparent(ObjectId id, ObjectId? parentId, bool worldPositionStays)
        {
            ValidateReparent(id, parentId);
            Transform target = RequireObject(id).transform;
            Transform? placement = target.parent;
            while (placement != null && placement.GetComponent<BattlementIdentity>() != null)
            {
                placement = placement.parent;
            }

            if (placement == null)
            {
                throw new BattlementWorldException(
                    CoreErrorCode.InvalidHierarchy,
                    $"Object {id} is not beneath a Battlement placement container."
                );
            }

            Transform parent = parentId is ObjectId value
                ? RequireObject(value).transform
                : placement;
            target.SetParent(parent, worldPositionStays);
        }

        public void SetActive(ObjectId id, bool isActive) => RequireObject(id).SetActive(isActive);

        public Transform RegisterScene(SceneId id, Scene scene)
        {
            Guid value = RequireNonzero(id.Value, nameof(id));
            if (!scene.IsValid() || !scene.isLoaded)
            {
                throw new BattlementWorldException(
                    CoreErrorCode.UnknownScene,
                    $"Scene {value} is not loaded."
                );
            }

            if (
                !usedIds.Add(value)
                && (replacementSceneIds is null || !replacementSceneIds.Remove(value))
            )
            {
                throw DuplicateId("scene", value);
            }

            var container = new GameObject("Battlement Scene");
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
                throw new BattlementWorldException(
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
            foreach (BattlementIdentity identity in objects.Values.ToArray())
            {
                if (
                    identity != null
                    && identity.gameObject != null
                    && identity.gameObject.scene == scene
                )
                {
                    ReleaseAndDestroy(identity);
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
            if (objects.TryGetValue(value, out BattlementIdentity identity))
            {
                if (identity != null && identity.gameObject != null)
                {
                    return identity.gameObject;
                }

                objects.Remove(value);
            }

            throw new BattlementWorldException(
                CoreErrorCode.UnknownObject,
                $"Object {value} does not exist."
            );
        }

        public bool TryGetObject(ObjectId id, out GameObject? gameObject)
        {
            if (
                id.Value == Guid.Empty
                || !objects.TryGetValue(id.Value, out BattlementIdentity identity)
            )
            {
                gameObject = null;
                return false;
            }

            if (identity == null || identity.gameObject == null)
            {
                gameObject = null;
                return false;
            }

            gameObject = identity.gameObject;
            return true;
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

            throw new BattlementWorldException(
                CoreErrorCode.UnknownScene,
                $"Scene {value} does not exist."
            );
        }

        public bool Contains(BattlementIdentity identity)
        {
            if (identity == null || identity.Id == Guid.Empty)
            {
                return false;
            }

            return objects.TryGetValue(identity.Id, out BattlementIdentity registered)
                && ReferenceEquals(registered, identity);
        }

        public void ConfigureInputCamera(ObjectId? id)
        {
            if (id is ObjectId objectId)
            {
                input.SetCamera(RequireObject(objectId), objectId);
            }
            else
            {
                input.SetMainCamera();
            }
            InputCameraChanged?.Invoke(input.Camera);
        }

        public void SetCameraEnabled(Camera camera, bool isEnabled)
        {
            camera.enabled = isEnabled;
            if (!isEnabled)
            {
                bool wasInputCamera = ReferenceEquals(input.Camera, camera);
                input.DisableCamera(camera);
                if (wasInputCamera)
                {
                    InputCameraChanged?.Invoke(null);
                }
            }
        }

        public void SetPointerEvents(ObjectId id, IReadOnlyList<PointerEvent> events) =>
            input.SetPointerEvents(RequireObject(id), events);

        public void SetGlobalKeys(IReadOnlyList<KeyCode> keys) => input.SetGlobalKeys(keys);

        public bool IsGlobalKeyEnabled(KeyCode key) => input.IsGlobalKeyEnabled(key);

        public void UpdateBillboards()
        {
            Camera? inputCamera = input.Camera;
            if (inputCamera == null)
            {
                return;
            }

            foreach (BattlementIdentity identity in objects.Values)
            {
                if (identity != null && identity.TryGetComponent(out BattlementImage image))
                {
                    image.UpdateBillboard(inputCamera);
                }

                if (identity != null && identity.TryGetComponent(out BattlementText text))
                {
                    text.UpdateBillboard(inputCamera);
                }
            }
        }

        public void Unregister(BattlementIdentity identity)
        {
            if (
                identity.Id != Guid.Empty
                && objects.TryGetValue(identity.Id, out BattlementIdentity registered)
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

        private void RegisterObject(
            ObjectId id,
            GameObject gameObject,
            Transform parent,
            IBattlementAssetLease? lease,
            IReadOnlyList<PointerEvent> pointerEvents,
            DragMode? dragMode,
            bool usesAutomaticPointerCollider,
            bool allowReplacement = false
        )
        {
            Guid value = RequireNonzero(id.Value, nameof(id));
            if (!usedIds.Add(value) && !allowReplacement)
            {
                throw DuplicateId("object", value);
            }

            gameObject.transform.SetParent(parent, false);
            if (lease is not null)
            {
                gameObject.AddComponent<BattlementPrefabLease>().Initialize(lease);
            }

            BattlementIdentity identity = gameObject.AddComponent<BattlementIdentity>();
            identity.Initialize(this, value, pointerEvents, dragMode, usesAutomaticPointerCollider);
            objects.Add(value, identity);
        }

        private Transform ResolveContainer(ParentScene parentScene) =>
            parentScene switch
            {
                ParentScene.Persistent => persistentContainer.transform,
                ParentScene.Specific specific => RequireSceneContainer(specific.SceneId),
                ParentScene.Primary => primarySceneId is Guid id
                    ? RequireSceneContainer(new SceneId(id))
                    : throw new BattlementWorldException(
                        CoreErrorCode.UnknownScene,
                        "The primary content scene is not loaded."
                    ),
                _ => throw new BattlementWorldException(
                    CoreErrorCode.UnknownScene,
                    "The parent scene selection is unknown."
                ),
            };

        private void DestroyOwnedObjects()
        {
            foreach (BattlementIdentity identity in objects.Values.ToArray())
            {
                if (identity != null && identity.gameObject != null)
                {
                    ReleaseAndDestroy(identity);
                }
            }

            objects.Clear();
        }

        private static void ReleaseAndDestroy(BattlementIdentity identity)
        {
            identity.gameObject.SetActive(false);
            BattlementOwnedResources.Release(identity.gameObject);
            DestroyUnityObject(identity.gameObject);
        }

        private static Guid RequireNonzero(Guid value, string parameterName)
        {
            if (value == Guid.Empty)
            {
                throw new ArgumentException("Battlement UUIDs must be nonzero.", parameterName);
            }

            return value;
        }

        private static BattlementWorldException DuplicateId(string kind, Guid id) =>
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

    internal sealed class BattlementWorldException : InvalidOperationException
    {
        public BattlementWorldException(CoreErrorCode errorCode, string message)
            : base(message) => ErrorCode = errorCode;

        public CoreErrorCode ErrorCode { get; }
    }
}

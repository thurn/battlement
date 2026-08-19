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
        private readonly GameObject persistentContainer;
        private bool isDisposed;

        public MasonryWorld(MasonryRunner runner)
        {
            persistentContainer = new GameObject("Masonry Persistent");
            SceneManager.MoveGameObjectToScene(persistentContainer, runner.gameObject.scene);
        }

        public void BeginSession()
        {
            DestroyOwnedObjects();
            foreach (GameObject container in sceneContainers.Values.ToArray())
            {
                DestroyUnityObject(container);
            }

            sceneContainers.Clear();
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
                var gameObject = new GameObject("Masonry Object");
                RegisterObject(description.Id, gameObject, container);
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
                && registered == identity;
        }

        public void Unregister(MasonryIdentity identity)
        {
            if (
                identity.Id != Guid.Empty
                && objects.TryGetValue(identity.Id, out MasonryIdentity registered)
                && registered == identity
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

        private void RegisterObject(ObjectId id, GameObject gameObject, Transform parent)
        {
            Guid value = RequireNonzero(id.Value, nameof(id));
            if (!usedIds.Add(value))
            {
                throw DuplicateId("object", value);
            }

            gameObject.transform.SetParent(parent, false);
            MasonryIdentity identity = gameObject.AddComponent<MasonryIdentity>();
            identity.Initialize(this, value);
            objects.Add(value, identity);
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
                ParentScene.Primary => throw new MasonryWorldException(
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

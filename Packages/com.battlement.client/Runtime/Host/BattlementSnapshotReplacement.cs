#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Battlement.UI;

namespace Battlement
{
    /// <summary>Owns the ordered phases of one direct snapshot replacement.</summary>
    internal sealed class BattlementSnapshotReplacement
    {
        private readonly BattlementPreparedAssets preparedAssets;
        private readonly BattlementScenes scenes;
        private readonly BattlementWorld world;
        private readonly BattlementUiDocuments uiDocuments;
        private readonly BattlementPanelInputCoordinator panelInput;
        private readonly BattlementReactantAssetCatalog reactantAssets;
        private PendingSnapshot? pending;

        public bool IsPending => pending is not null;

        internal System.Action? ApplicationProbe { get; set; }

        public BattlementSnapshotReplacement(
            BattlementPreparedAssets preparedAssets,
            BattlementScenes scenes,
            BattlementWorld world,
            BattlementUiDocuments uiDocuments,
            BattlementPanelInputCoordinator panelInput,
            BattlementReactantAssetCatalog reactantAssets
        )
        {
            this.preparedAssets = preparedAssets;
            this.scenes = scenes;
            this.world = world;
            this.uiDocuments = uiDocuments;
            this.panelInput = panelInput;
            this.reactantAssets = reactantAssets;
        }

        public void Begin(
            SessionId responseSession,
            IBattlementSnapshotView snapshot,
            bool preserveMotion
        )
        {
            if (snapshot.SessionId != responseSession)
            {
                throw Failure("A snapshot used the wrong session.");
            }

            try
            {
                if (snapshot is not BattlementFlatBufferSnapshotView flat || !flat.CanApplyDirectly)
                    throw new InvalidOperationException(
                        "Snapshot replacement requires a verified FlatBuffer view."
                    );
                IReadOnlyList<BattlementDirectSnapshotObject> directObjects = DirectOrder(flat);
                IReadOnlyList<BattlementScene> directScenes = DirectScenes(flat);
                IReadOnlyList<PhysicalKey> directGlobalKeys = flat.ReadDirectGlobalKeys();
                ControllerInputSettings? directControllerInput = flat.ReadDirectControllerInput();
                bool directWorldInput = directObjects.Any(value =>
                    value.Description is BattlementDirectUiDocumentObjectCreate document
                    && document.State.PanelSettings?.RenderMode == PanelRenderMode.WorldSpace
                );
                reactantAssets.Validate(flat);
                ValidateDirectUiDocuments(flat, directObjects);
                BattlementPanelInputCoordinator.ValidateValue(snapshot.PanelInputConfiguration);
                panelInput.ValidateBeforeReplacement(directWorldInput);
                world.PrepareReplacement(directObjects, directScenes);
                preparedAssets.BeginReplacement(flat, isAuthoritative: true);
                pending = new PendingSnapshot(
                    snapshot,
                    Array.Empty<BattlementGameObject>(),
                    directObjects,
                    directScenes,
                    directGlobalKeys,
                    directControllerInput,
                    directWorldInput,
                    preserveMotion
                );
            }
            catch (BattlementSnapshotReplacementException)
            {
                throw;
            }
            catch (Exception exception)
            {
                throw Failure($"Snapshot validation failed: {exception.Message}", exception);
            }
        }

        public bool TryComplete(out bool inputDisabled)
        {
            inputDisabled = true;
            if (pending is null)
            {
                throw new InvalidOperationException("No snapshot replacement is active.");
            }

            if (!pending.SceneReplacementStarted)
            {
                if (!preparedAssets.TryCompleteReplacement(out BattlementAssetException? error))
                {
                    return false;
                }

                if (error is not null)
                {
                    throw Failure($"Snapshot asset preparation failed: {error.Message}", error);
                }

                ValidatePreparedObjects(pending);
                BeginSceneReplacement(pending);
            }

            if (!scenes.TryCompleteReplacement(out BattlementAssetException? sceneError))
            {
                return false;
            }

            if (sceneError is not null)
            {
                throw Failure($"Snapshot scene loading failed: {sceneError.Message}", sceneError);
            }

            PendingSnapshot completed = pending;
            pending = null;
            try
            {
                world.ReplaceObjects(completed.DirectObjects!);
                var directUi = (IBattlementUiDocumentCollectionView)completed.Snapshot;
                uiDocuments.Replace(
                    directUi,
                    id => world.TryGetObject(id, out UnityEngine.GameObject? value) ? value : null,
                    completed.PreserveMotion
                );
                ApplicationProbe?.Invoke();
                world.ReplaceUiIdentities(uiDocuments.IdentityIds);
                world.ConfigureInputCamera(completed.Snapshot.InputCameraId);
                panelInput.Apply(
                    completed.Snapshot.PanelInputConfiguration,
                    completed.DirectWorldInput,
                    completed.Snapshot.InputCameraId is null,
                    world.InputCamera
                );
                world.SetGlobalKeys(completed.DirectGlobalKeys!);
                ControllerInputSettings? controllerInput = completed.DirectControllerInput;
                if (controllerInput is not null)
                {
                    world.SetControllerInput(controllerInput);
                }
                inputDisabled = completed.Snapshot.IsInputDisabled;
                return true;
            }
            catch (Exception exception)
            {
                throw Failure($"Snapshot application failed: {exception.Message}", exception);
            }
            finally
            {
                completed.Snapshot.Dispose();
            }
        }

        public void Cancel()
        {
            pending?.Snapshot.Dispose();
            pending = null;
            panelInput.Clear();
            uiDocuments.Clear();
        }

        private void ValidatePreparedObjects(PendingSnapshot replacement)
        {
            try
            {
                foreach (BattlementDirectSnapshotObject value in replacement.DirectObjects!)
                    preparedAssets.ValidateMaterialInstances(value.Placement.MaterialInstances);
                BattlementPreparedObjectValidator.Validate(
                    replacement.DirectObjects!,
                    preparedAssets,
                    replacement.Snapshot.InputCameraId
                );
            }
            catch (Exception exception)
            {
                throw Failure($"Snapshot validation failed: {exception.Message}", exception);
            }
        }

        private void BeginSceneReplacement(PendingSnapshot replacement)
        {
            try
            {
                scenes.BeginReplacement(
                    replacement.DirectScenes!,
                    replacement.Snapshot.PrimarySceneId
                );
                replacement.SceneReplacementStarted = true;
            }
            catch (Exception exception)
            {
                throw Failure($"Snapshot scene loading failed: {exception.Message}", exception);
            }
        }

        private static BattlementSnapshotReplacementException Failure(
            string message,
            Exception? innerException = null
        ) => new(message, innerException);

        private static IReadOnlyList<BattlementDirectSnapshotObject> DirectOrder(
            BattlementFlatBufferSnapshotView snapshot
        )
        {
            var values = new BattlementDirectSnapshotObject[snapshot.DirectObjectCount];
            var byId = new Dictionary<Guid, BattlementDirectSnapshotObject>(values.Length);
            for (int index = 0; index < values.Length; index++)
            {
                BattlementDirectSnapshotObject value = snapshot.ReadDirectObject(index);
                values[index] = value;
                byId.Add(value.Placement.ObjectId.Value, value);
            }
            var depths = new Dictionary<Guid, int>(values.Length);
            var visiting = new HashSet<Guid>();
            int Depth(BattlementDirectSnapshotObject value)
            {
                Guid id = value.Placement.ObjectId.Value;
                if (depths.TryGetValue(id, out int known))
                    return known;
                if (!visiting.Add(id))
                    throw new InvalidOperationException("The object hierarchy is cyclic.");
                int depth = value.Placement.ParentId is ObjectId parentId
                    ? checked(Depth(byId[parentId.Value]) + 1)
                    : 0;
                if (depth > 256)
                    throw new InvalidOperationException(
                        "The object hierarchy cannot exceed 256 levels."
                    );
                visiting.Remove(id);
                depths.Add(id, depth);
                return depth;
            }
            return values
                .Select((value, index) => (value, index, depth: Depth(value)))
                .OrderBy(item => item.depth)
                .ThenBy(item => item.index)
                .Select(item => item.value)
                .ToArray();
        }

        private static IReadOnlyList<BattlementScene> DirectScenes(
            BattlementFlatBufferSnapshotView snapshot
        )
        {
            var result = new BattlementScene[snapshot.DirectSceneCount];
            for (int index = 0; index < result.Length; index++)
                result[index] = snapshot.ReadDirectScene(index);
            return result;
        }

        private static void ValidateDirectUiDocuments(
            BattlementFlatBufferSnapshotView snapshot,
            IReadOnlyList<BattlementDirectSnapshotObject> objects
        )
        {
            var byId = objects.ToDictionary(value => value.Placement.ObjectId.Value);
            var identities = new HashSet<Guid>(byId.Keys);
            var documents = new HashSet<Guid>();
            for (int index = 0; index < snapshot.DocumentCount; index++)
            {
                IBattlementUiDocumentView document = snapshot.ReadDocument(index);
                Guid documentId = document.DocumentId.Value;
                Guid rootId = document.RootId.Value;
                if (!documents.Add(documentId) || !identities.Add(rootId))
                    throw new InvalidOperationException("A snapshot UI identity is duplicated.");
                if (
                    !byId.TryGetValue(documentId, out BattlementDirectSnapshotObject owner)
                    || owner.Description is not BattlementDirectUiDocumentObjectCreate state
                    || state.State.RootId.Value != rootId
                )
                    throw new InvalidOperationException(
                        "A UI document does not match its document GameObject and root."
                    );
                ObjectId? parentId = owner.Placement.ParentId;
                while (parentId is ObjectId parent)
                {
                    BattlementDirectSnapshotObject ancestor = byId[parent.Value];
                    if (ancestor.Description is BattlementDirectUiDocumentObjectCreate)
                        throw new InvalidOperationException(
                            "A UI document cannot be nested beneath another document."
                        );
                    parentId = ancestor.Placement.ParentId;
                }
                for (int node = 0; node < document.NodeCount; node++)
                {
                    Guid nodeId = document.ReadNodeId(node).Value;
                    if (!identities.Add(nodeId))
                        throw new InvalidOperationException(
                            "A snapshot UI identity is duplicated."
                        );
                    Battlement.UI.BattlementUiElementValidator.Validate(
                        document.ReadNodeElement(node),
                        allowUsageHints: true
                    );
                }
            }
            foreach (BattlementDirectSnapshotObject value in objects)
                if (
                    value.Description is BattlementDirectUiDocumentObjectCreate
                    && !documents.Contains(value.Placement.ObjectId.Value)
                )
                    throw new InvalidOperationException(
                        "A UI-document GameObject has no document entry."
                    );
        }

        private sealed class PendingSnapshot
        {
            public PendingSnapshot(
                IBattlementSnapshotView snapshot,
                IReadOnlyList<BattlementGameObject> objectOrder,
                IReadOnlyList<BattlementDirectSnapshotObject>? directObjects,
                IReadOnlyList<BattlementScene>? directScenes,
                IReadOnlyList<PhysicalKey>? directGlobalKeys,
                ControllerInputSettings? directControllerInput,
                bool directWorldInput,
                bool preserveMotion
            )
            {
                Snapshot = snapshot;
                ObjectOrder = objectOrder;
                DirectObjects = directObjects;
                DirectScenes = directScenes;
                DirectGlobalKeys = directGlobalKeys;
                DirectControllerInput = directControllerInput;
                DirectWorldInput = directWorldInput;
                PreserveMotion = preserveMotion;
            }

            public IBattlementSnapshotView Snapshot { get; }

            public IReadOnlyList<BattlementGameObject> ObjectOrder { get; }

            public IReadOnlyList<BattlementDirectSnapshotObject>? DirectObjects { get; }

            public IReadOnlyList<BattlementScene>? DirectScenes { get; }

            public IReadOnlyList<PhysicalKey>? DirectGlobalKeys { get; }

            public ControllerInputSettings? DirectControllerInput { get; }

            public bool DirectWorldInput { get; }

            public bool PreserveMotion { get; }

            public bool SceneReplacementStarted { get; set; }
        }
    }

    internal sealed class BattlementSnapshotReplacementException : InvalidOperationException
    {
        public BattlementSnapshotReplacementException(string message, Exception? innerException)
            : base(message, innerException) { }
    }
}

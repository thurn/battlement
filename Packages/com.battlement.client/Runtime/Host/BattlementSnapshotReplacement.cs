#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;

namespace Battlement
{
    /// <summary>Owns the ordered phases of one direct snapshot replacement.</summary>
    internal sealed class BattlementSnapshotReplacement
    {
        private readonly BattlementPreparedAssets preparedAssets;
        private readonly BattlementScenes scenes;
        private readonly BattlementWorld world;
        private readonly BattlementUiDocuments uiDocuments = new();
        private PendingSnapshot? pending;

        public BattlementSnapshotReplacement(
            BattlementPreparedAssets preparedAssets,
            BattlementScenes scenes,
            BattlementWorld world
        )
        {
            this.preparedAssets = preparedAssets;
            this.scenes = scenes;
            this.world = world;
        }

        public void Begin(SessionId responseSession, Snapshot snapshot)
        {
            if (snapshot.SessionId != responseSession)
            {
                throw Failure("A snapshot used the wrong session.");
            }

            try
            {
                IReadOnlyList<BattlementGameObject> objectOrder =
                    BattlementSnapshotValidator.Validate(snapshot);
                world.PrepareReplacement(objectOrder, snapshot.Scenes);
                preparedAssets.BeginReplacement(snapshot.PreparedAssets, isAuthoritative: true);
                pending = new PendingSnapshot(snapshot, objectOrder);
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
                world.ReplaceObjects(completed.ObjectOrder);
                uiDocuments.Replace(
                    completed.Snapshot.Ui,
                    id => world.TryGetObject(id, out UnityEngine.GameObject? value) ? value : null
                );
                world.ReplaceUiIdentities(uiDocuments.IdentityIds);
                world.ConfigureInputCamera(completed.Snapshot.InputCameraId);
                world.SetGlobalKeys(completed.Snapshot.GlobalKeys);
                if (completed.Snapshot.ControllerInput is not null)
                {
                    world.SetControllerInput(completed.Snapshot.ControllerInput);
                }
                inputDisabled = completed.Snapshot.IsInputDisabled;
                return true;
            }
            catch (Exception exception)
            {
                throw Failure($"Snapshot application failed: {exception.Message}", exception);
            }
        }

        public void Cancel()
        {
            pending = null;
            uiDocuments.Clear();
        }

        private void ValidatePreparedObjects(PendingSnapshot replacement)
        {
            try
            {
                BattlementPreparedObjectValidator.Validate(
                    replacement.ObjectOrder,
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
                    replacement.Snapshot.Scenes,
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

        private sealed class PendingSnapshot
        {
            public PendingSnapshot(
                Snapshot snapshot,
                IReadOnlyList<BattlementGameObject> objectOrder
            )
            {
                Snapshot = snapshot;
                ObjectOrder = objectOrder;
            }

            public Snapshot Snapshot { get; }

            public IReadOnlyList<BattlementGameObject> ObjectOrder { get; }

            public bool SceneReplacementStarted { get; set; }
        }
    }

    internal sealed class BattlementSnapshotReplacementException : InvalidOperationException
    {
        public BattlementSnapshotReplacementException(string message, Exception? innerException)
            : base(message, innerException) { }
    }
}

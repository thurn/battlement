#nullable enable

using System;
using System.Collections.Generic;

namespace Masonry
{
    /// <summary>Owns the ordered phases of one direct snapshot replacement.</summary>
    internal sealed class MasonrySnapshotReplacement
    {
        private readonly MasonryPreparedAssets preparedAssets;
        private readonly MasonryScenes scenes;
        private readonly MasonryWorld world;
        private PendingSnapshot? pending;

        public MasonrySnapshotReplacement(
            MasonryPreparedAssets preparedAssets,
            MasonryScenes scenes,
            MasonryWorld world
        )
        {
            this.preparedAssets = preparedAssets;
            this.scenes = scenes;
            this.world = world;
        }

        public void Begin(SessionId responseSession, Snapshot snapshot, bool isInitial)
        {
            if (snapshot.SessionId != responseSession)
            {
                throw Failure("A snapshot used the wrong session.");
            }

            try
            {
                IReadOnlyList<MasonryGameObject> objectOrder = MasonrySnapshotValidator.Validate(
                    snapshot
                );
                preparedAssets.BeginReplacement(snapshot.PreparedAssets, isAuthoritative: true);
                pending = new PendingSnapshot(snapshot, objectOrder, isInitial);
            }
            catch (MasonrySnapshotReplacementException)
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
                if (!preparedAssets.TryCompleteReplacement(out MasonryAssetException? error))
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

            if (!scenes.TryCompleteReplacement(out MasonryAssetException? sceneError))
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
                if (completed.IsInitial)
                {
                    world.CreateInitialObjects(completed.ObjectOrder);
                }

                world.ConfigureInputCamera(completed.Snapshot.InputCameraId);
                inputDisabled = completed.Snapshot.IsInputDisabled;
                return true;
            }
            catch (Exception exception)
            {
                throw Failure($"Snapshot application failed: {exception.Message}", exception);
            }
        }

        public void Cancel() => pending = null;

        private void ValidatePreparedObjects(PendingSnapshot replacement)
        {
            try
            {
                MasonryPreparedObjectValidator.Validate(
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

        private static MasonrySnapshotReplacementException Failure(
            string message,
            Exception? innerException = null
        ) => new(message, innerException);

        private sealed class PendingSnapshot
        {
            public PendingSnapshot(
                Snapshot snapshot,
                IReadOnlyList<MasonryGameObject> objectOrder,
                bool isInitial
            )
            {
                Snapshot = snapshot;
                ObjectOrder = objectOrder;
                IsInitial = isInitial;
            }

            public Snapshot Snapshot { get; }

            public IReadOnlyList<MasonryGameObject> ObjectOrder { get; }

            public bool IsInitial { get; }

            public bool SceneReplacementStarted { get; set; }
        }
    }

    internal sealed class MasonrySnapshotReplacementException : InvalidOperationException
    {
        public MasonrySnapshotReplacementException(string message, Exception? innerException)
            : base(message, innerException) { }
    }
}

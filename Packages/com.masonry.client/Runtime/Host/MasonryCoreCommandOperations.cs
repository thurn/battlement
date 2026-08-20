#nullable enable

using System;

namespace Masonry
{
    internal static class MasonryCoreCommandOperations
    {
        public static IMasonryCommandOperation ReplaceAssets(
            CommandBody.Assets.ReplaceSet command,
            MasonryPreparedAssets preparedAssets
        )
        {
            preparedAssets.BeginReplacement(command.PreparedAssets, isAuthoritative: false);
            return new PreparedAssetReplacementOperation(preparedAssets);
        }

        public static IMasonryCommandOperation LoadScene(
            CommandBody.Scene.Load command,
            MasonryScenes scenes
        )
        {
            scenes.BeginLoad(command.SceneId, command.Address, command.MakePrimary);
            return new SceneCommandOperation(scenes);
        }

        public static IMasonryCommandOperation UnloadScene(
            CommandBody.Scene.Unload command,
            MasonryScenes scenes,
            MasonryWorld world,
            MasonryOperationRegistry operations
        )
        {
            scenes.ValidateUnload(command.SceneId);
            operations.CancelObjects(world.GetSceneObjectIds(command.SceneId));
            scenes.BeginUnload(command.SceneId);
            return new SceneCommandOperation(scenes);
        }

        private sealed class PreparedAssetReplacementOperation : IMasonryCommandOperation
        {
            private readonly MasonryPreparedAssets preparedAssets;
            private bool isComplete;

            public PreparedAssetReplacementOperation(MasonryPreparedAssets preparedAssets) =>
                this.preparedAssets = preparedAssets;

            public bool IsInfinite => false;

            public bool IsComplete(TimeSpan now)
            {
                if (isComplete)
                {
                    return true;
                }

                if (!preparedAssets.TryCompleteReplacement(out MasonryAssetException? error))
                {
                    return false;
                }

                isComplete = true;
                if (error is not null)
                {
                    throw new MasonryCommandException(error.ErrorCode, error.Message, error);
                }

                return true;
            }

            public void Cancel()
            {
                if (isComplete)
                {
                    return;
                }

                preparedAssets.CancelPending();
                isComplete = true;
            }
        }

        private sealed class SceneCommandOperation : IMasonryCommandOperation
        {
            private readonly MasonryScenes scenes;
            private bool isComplete;

            public SceneCommandOperation(MasonryScenes scenes) => this.scenes = scenes;

            public bool IsInfinite => false;

            public bool IsComplete(TimeSpan now)
            {
                if (isComplete)
                {
                    return true;
                }

                if (!scenes.TryCompleteReplacement(out MasonryAssetException? error))
                {
                    return false;
                }

                isComplete = true;
                if (error is not null)
                {
                    throw new MasonryCommandException(error.ErrorCode, error.Message, error);
                }

                return true;
            }

            public void Cancel()
            {
                if (isComplete)
                {
                    return;
                }

                scenes.CancelPendingCommand();
                isComplete = true;
            }
        }
    }
}

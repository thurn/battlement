#nullable enable

using System;

namespace Battlement
{
    internal static class BattlementCoreCommandOperations
    {
        public static IBattlementCommandOperation ReplaceAssets(
            BattlementDirectAssetSet command,
            BattlementPreparedAssets preparedAssets
        )
        {
            preparedAssets.BeginReplacement(command);
            return new PreparedAssetReplacementOperation(preparedAssets);
        }

        public static IBattlementCommandOperation LoadScene(
            BattlementDirectSceneCommand command,
            BattlementScenes scenes
        )
        {
            scenes.BeginLoad(
                command.SceneId,
                new SceneAddress(
                    command.Address
                        ?? throw new BattlementCommandException(
                            CoreErrorCode.InvalidProperty,
                            "A scene-load address is absent."
                        )
                ),
                command.MakePrimary
            );
            return new SceneCommandOperation(scenes);
        }

        public static IBattlementCommandOperation UnloadScene(
            BattlementDirectSceneCommand command,
            BattlementScenes scenes,
            BattlementWorld world,
            BattlementOperationRegistry operations
        )
        {
            scenes.ValidateUnload(command.SceneId);
            operations.CancelObjects(world.GetSceneObjectIds(command.SceneId));
            scenes.BeginUnload(command.SceneId);
            return new SceneCommandOperation(scenes);
        }

        private sealed class PreparedAssetReplacementOperation : IBattlementCommandOperation
        {
            private readonly BattlementPreparedAssets preparedAssets;
            private bool isComplete;

            public PreparedAssetReplacementOperation(BattlementPreparedAssets preparedAssets) =>
                this.preparedAssets = preparedAssets;

            public bool IsInfinite => false;

            public bool IsComplete(TimeSpan now)
            {
                if (isComplete)
                {
                    return true;
                }

                if (!preparedAssets.TryCompleteReplacement(out BattlementAssetException? error))
                {
                    return false;
                }

                isComplete = true;
                if (error is not null)
                {
                    throw new BattlementCommandException(error.ErrorCode, error.Message, error);
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

        private sealed class SceneCommandOperation : IBattlementCommandOperation
        {
            private readonly BattlementScenes scenes;
            private bool isComplete;

            public SceneCommandOperation(BattlementScenes scenes) => this.scenes = scenes;

            public bool IsInfinite => false;

            public bool IsComplete(TimeSpan now)
            {
                if (isComplete)
                {
                    return true;
                }

                if (!scenes.TryCompleteReplacement(out BattlementAssetException? error))
                {
                    return false;
                }

                isComplete = true;
                if (error is not null)
                {
                    throw new BattlementCommandException(error.ErrorCode, error.Message, error);
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

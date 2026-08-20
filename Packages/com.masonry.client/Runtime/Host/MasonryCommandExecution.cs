#nullable enable

using System;

namespace Masonry
{
    internal sealed class MasonryCommandExecutor
    {
        private readonly MasonryWorld world;
        private readonly MasonryPreparedAssets preparedAssets;
        private readonly MasonryScenes scenes;
        private readonly MasonryOperationRegistry operations;
        private readonly MasonryTweenAdapter tweens;
        private readonly Action<bool> setInputEnabled;

        public MasonryCommandExecutor(
            MasonryWorld world,
            MasonryPreparedAssets preparedAssets,
            MasonryScenes scenes,
            MasonryOperationRegistry operations,
            MasonryTweenAdapter tweens,
            Action<bool> setInputEnabled
        )
        {
            this.world = world;
            this.preparedAssets = preparedAssets;
            this.scenes = scenes;
            this.operations = operations;
            this.tweens = tweens;
            this.setInputEnabled = setInputEnabled;
        }

        public IMasonryCommandOperation? Launch(Command command, TimeSpan now)
        {
            try
            {
                if (
                    command.IsBlocking
                    && MasonryTweenAdapter.IsForever(MasonryTweenAdapter.For(command.Body))
                )
                {
                    throw new MasonryCommandException(
                        CoreErrorCode.InvalidProperty,
                        "A forever tween must be nonblocking."
                    );
                }

                return command.Body switch
                {
                    CommandBody.Assets.ReplaceSet assets =>
                        MasonryCoreCommandOperations.ReplaceAssets(assets, preparedAssets),
                    CommandBody.Scene.Load scene => MasonryCoreCommandOperations.LoadScene(
                        scene,
                        scenes
                    ),
                    CommandBody.Scene.Unload scene => MasonryCoreCommandOperations.UnloadScene(
                        scene,
                        scenes,
                        world,
                        operations
                    ),
                    CommandBody.Scene.SetPrimary scene => scenes.SetPrimary(scene.SceneId),
                    CommandBody.Time.Wait wait => MasonryTimeCommands.Wait(wait, now),
                    CommandBody.Object.Create create => MasonryObjectCommands.Create(create, world),
                    CommandBody.Object.Destroy destroy => MasonryObjectCommands.Destroy(
                        destroy,
                        world,
                        operations
                    ),
                    CommandBody.Object.SetActive active => MasonryObjectCommands.SetActive(
                        active,
                        world
                    ),
                    CommandBody.Object.Reparent reparent => MasonryObjectCommands.Reparent(
                        reparent,
                        world,
                        operations
                    ),
                    CommandBody.Transform.SetLocalPosition position =>
                        MasonryTransformCommands.SetLocalPosition(position, world),
                    CommandBody.Transform.SetWorldPosition position =>
                        MasonryTransformCommands.SetWorldPosition(position, world),
                    CommandBody.Transform.TweenLocalPosition position =>
                        MasonryTransformCommands.TweenLocalPosition(position, world, tweens, now),
                    CommandBody.Transform.TweenWorldPosition position =>
                        MasonryTransformCommands.TweenWorldPosition(position, world, tweens, now),
                    CommandBody.Transform.SetLocalRotation rotation =>
                        MasonryTransformCommands.SetLocalRotation(rotation, world),
                    CommandBody.Transform.SetWorldRotation rotation =>
                        MasonryTransformCommands.SetWorldRotation(rotation, world),
                    CommandBody.Transform.TweenLocalRotation rotation =>
                        MasonryTransformCommands.TweenLocalRotation(rotation, world, tweens, now),
                    CommandBody.Transform.TweenWorldRotation rotation =>
                        MasonryTransformCommands.TweenWorldRotation(rotation, world, tweens, now),
                    CommandBody.Transform.SetLocalScale scale =>
                        MasonryTransformCommands.SetLocalScale(scale, world),
                    CommandBody.Transform.TweenLocalScale scale =>
                        MasonryTransformCommands.TweenLocalScale(scale, world, tweens, now),
                    CommandBody.Renderer.SetMaterial material => MasonryObjectCommands.SetMaterial(
                        material,
                        world,
                        preparedAssets
                    ),
                    CommandBody.Input.SetEnabled input => MasonryInputCommands.SetEnabled(
                        input,
                        setInputEnabled
                    ),
                    CommandBody.Input.SetCamera input => MasonryInputCommands.SetCamera(
                        input,
                        world
                    ),
                    CommandBody.Input.SetPointerEvents input =>
                        MasonryInputCommands.SetPointerEvents(input, world),
                    CommandBody.Input.SetGlobalKeys input => MasonryInputCommands.SetGlobalKeys(
                        input,
                        world
                    ),
                    _ => throw new MasonryCommandException(
                        CoreErrorCode.InvalidProperty,
                        $"Command {command.Body.GetType().Name} is not implemented yet."
                    ),
                };
            }
            catch (MasonryWorldException exception)
            {
                throw new MasonryCommandException(
                    exception.ErrorCode,
                    exception.Message,
                    exception
                );
            }
            catch (MasonryAssetException exception)
            {
                throw new MasonryCommandException(
                    exception.ErrorCode,
                    exception.Message,
                    exception
                );
            }
        }
    }

    internal sealed class MasonryCommandException : InvalidOperationException
    {
        public MasonryCommandException(
            CoreErrorCode errorCode,
            string message,
            Exception? innerException = null
        )
            : base(message, innerException) => ErrorCode = errorCode;

        public CoreErrorCode ErrorCode { get; }
    }
}

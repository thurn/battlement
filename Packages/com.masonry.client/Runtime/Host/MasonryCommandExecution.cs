#nullable enable

using System;

namespace Masonry
{
    internal sealed class MasonryCommandExecutor
    {
        private readonly MasonryWorld world;
        private readonly MasonryOperationRegistry operations;
        private readonly MasonryTweenAdapter tweens;
        private readonly Action<bool> setInputEnabled;

        public MasonryCommandExecutor(
            MasonryWorld world,
            MasonryOperationRegistry operations,
            MasonryTweenAdapter tweens,
            Action<bool> setInputEnabled
        )
        {
            this.world = world;
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
                    CommandBody.Time.Wait wait => MasonryTimeCommands.Wait(wait, now),
                    CommandBody.Object.Create create => MasonryObjectCommands.Create(create, world),
                    CommandBody.Object.Destroy destroy => MasonryObjectCommands.Destroy(
                        destroy,
                        world,
                        operations
                    ),
                    CommandBody.Object.Reparent reparent => MasonryObjectCommands.Reparent(
                        reparent,
                        world,
                        operations
                    ),
                    CommandBody.Transform.SetLocalPosition position =>
                        MasonryTransformCommands.SetLocalPosition(position, world),
                    CommandBody.Transform.TweenLocalPosition position =>
                        MasonryTransformCommands.TweenLocalPosition(position, world, tweens, now),
                    CommandBody.Transform.TweenWorldPosition position =>
                        MasonryTransformCommands.TweenWorldPosition(position, world, tweens, now),
                    CommandBody.Transform.TweenLocalRotation rotation =>
                        MasonryTransformCommands.TweenLocalRotation(rotation, world, tweens, now),
                    CommandBody.Transform.TweenWorldRotation rotation =>
                        MasonryTransformCommands.TweenWorldRotation(rotation, world, tweens, now),
                    CommandBody.Transform.TweenLocalScale scale =>
                        MasonryTransformCommands.TweenLocalScale(scale, world, tweens, now),
                    CommandBody.Input.SetEnabled input => MasonryInputCommands.SetEnabled(
                        input,
                        setInputEnabled
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

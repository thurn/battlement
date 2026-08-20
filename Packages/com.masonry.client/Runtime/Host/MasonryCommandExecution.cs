#nullable enable

using System;

namespace Masonry
{
    internal interface IMasonryCommandOperation
    {
        bool IsComplete(TimeSpan now);
    }

    internal sealed class MasonryCommandExecutor
    {
        private readonly MasonryWorld world;
        private readonly Action<bool> setInputEnabled;

        public MasonryCommandExecutor(MasonryWorld world, Action<bool> setInputEnabled)
        {
            this.world = world;
            this.setInputEnabled = setInputEnabled;
        }

        public IMasonryCommandOperation? Launch(Command command, TimeSpan now)
        {
            try
            {
                return command.Body switch
                {
                    CommandBody.Time.Wait wait => MasonryTimeCommands.Wait(wait, now),
                    CommandBody.Object.Create create => MasonryObjectCommands.Create(create, world),
                    CommandBody.Transform.SetLocalPosition position =>
                        MasonryTransformCommands.SetLocalPosition(position, world),
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

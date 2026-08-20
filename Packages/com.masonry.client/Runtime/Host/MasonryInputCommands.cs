#nullable enable

using System;

namespace Masonry
{
    internal static class MasonryInputCommands
    {
        public static IMasonryCommandOperation? SetEnabled(
            CommandBody.Input.SetEnabled command,
            Action<bool> setInputEnabled
        )
        {
            setInputEnabled(command.IsEnabled);
            return null;
        }

        public static IMasonryCommandOperation? SetCamera(
            CommandBody.Input.SetCamera command,
            MasonryWorld world
        )
        {
            world.ConfigureInputCamera(command.ObjectId);
            return null;
        }

        public static IMasonryCommandOperation? SetPointerEvents(
            CommandBody.Input.SetPointerEvents command,
            MasonryWorld world
        )
        {
            world.SetPointerEvents(command.ObjectId, command.Events);
            return null;
        }

        public static IMasonryCommandOperation? SetGlobalKeys(
            CommandBody.Input.SetGlobalKeys command,
            MasonryWorld world
        )
        {
            world.SetGlobalKeys(command.Keys);
            return null;
        }
    }
}

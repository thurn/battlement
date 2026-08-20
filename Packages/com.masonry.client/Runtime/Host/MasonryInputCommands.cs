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
    }
}

#nullable enable

namespace Masonry
{
    internal static class MasonryObjectCommands
    {
        public static IMasonryCommandOperation? Create(
            CommandBody.Object.Create command,
            MasonryWorld world
        )
        {
            world.CreateObject(command.GameObject);
            return null;
        }
    }
}

#nullable enable

using System.Collections.Generic;

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

        public static IMasonryCommandOperation? Destroy(
            CommandBody.Object.Destroy command,
            MasonryWorld world,
            MasonryOperationRegistry operations
        )
        {
            IReadOnlyList<System.Guid> hierarchy = world.GetHierarchyObjectIds(command.ObjectId);
            operations.CancelObjects(hierarchy);
            world.DestroyObject(command.ObjectId);
            return null;
        }

        public static IMasonryCommandOperation? Reparent(
            CommandBody.Object.Reparent command,
            MasonryWorld world,
            MasonryOperationRegistry operations
        )
        {
            world.ValidateReparent(command.ObjectId, command.ParentId);
            operations.CancelTransform(command.ObjectId);
            world.Reparent(command.ObjectId, command.ParentId, command.WorldPositionStays);
            return null;
        }
    }
}

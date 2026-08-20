#nullable enable

using System.Collections.Generic;
using UnityEngine;

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

        public static IMasonryCommandOperation? SetActive(
            CommandBody.Object.SetActive command,
            MasonryWorld world
        )
        {
            world.SetActive(command.ObjectId, command.IsActive);
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

        public static IMasonryCommandOperation? SetMaterial(
            CommandBody.Renderer.SetMaterial command,
            MasonryWorld world,
            MasonryPreparedAssets preparedAssets
        )
        {
            GameObject gameObject = world.RequireObject(command.ObjectId);
            if (
                gameObject.GetComponent<MasonryImage>() != null
                || gameObject.GetComponent<MasonryText>() != null
            )
            {
                throw new MasonryWorldException(
                    CoreErrorCode.ComponentMissing,
                    $"Object {command.ObjectId} does not support renderer material assignment."
                );
            }

            Renderer[] renderers = gameObject.GetComponents<Renderer>();
            if (renderers.Length != 1)
            {
                throw new MasonryWorldException(
                    renderers.Length == 0
                        ? CoreErrorCode.ComponentMissing
                        : CoreErrorCode.InvalidComponentCount,
                    "Material assignment requires exactly one root Renderer; "
                        + $"found {renderers.Length}."
                );
            }

            if (!gameObject.TryGetComponent(out MasonryMaterialAssignments assignments))
            {
                assignments = gameObject.AddComponent<MasonryMaterialAssignments>();
            }

            assignments.EnsureInitialized(renderers[0], preparedAssets);
            assignments.SetMaterial(command.Address, command.Slot);
            return null;
        }
    }
}

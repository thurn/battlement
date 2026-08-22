#nullable enable

using System.Collections.Generic;
using UnityEngine;

namespace Battlement
{
    internal static class BattlementObjectCommands
    {
        public static IBattlementCommandOperation? Create(
            CommandBody.Object.Create command,
            BattlementWorld world
        )
        {
            world.CreateObject(command.GameObject);
            return null;
        }

        public static IBattlementCommandOperation? Destroy(
            CommandBody.Object.Destroy command,
            BattlementWorld world,
            BattlementOperationRegistry operations
        )
        {
            IReadOnlyList<System.Guid> hierarchy = world.GetHierarchyObjectIds(command.ObjectId);
            operations.CancelObjects(hierarchy);
            world.DestroyObject(command.ObjectId);
            return null;
        }

        public static IBattlementCommandOperation? SetActive(
            CommandBody.Object.SetActive command,
            BattlementWorld world
        )
        {
            world.SetActive(command.ObjectId, command.IsActive);
            return null;
        }

        public static IBattlementCommandOperation? Reparent(
            CommandBody.Object.Reparent command,
            BattlementWorld world,
            BattlementOperationRegistry operations
        )
        {
            world.ValidateReparent(command.ObjectId, command.ParentId);
            operations.CancelTransform(command.ObjectId);
            world.Reparent(command.ObjectId, command.ParentId, command.WorldPositionStays);
            return null;
        }

        public static IBattlementCommandOperation? SetMaterial(
            CommandBody.Renderer.SetMaterial command,
            BattlementWorld world,
            BattlementPreparedAssets preparedAssets
        )
        {
            GameObject gameObject = world.RequireObject(command.ObjectId);
            if (
                gameObject.GetComponent<BattlementImage>() != null
                || gameObject.GetComponent<BattlementText>() != null
            )
            {
                throw new BattlementWorldException(
                    CoreErrorCode.ComponentMissing,
                    $"Object {command.ObjectId} does not support renderer material assignment."
                );
            }

            Renderer[] renderers = gameObject.GetComponents<Renderer>();
            if (renderers.Length != 1)
            {
                throw new BattlementWorldException(
                    renderers.Length == 0
                        ? CoreErrorCode.ComponentMissing
                        : CoreErrorCode.InvalidComponentCount,
                    "Material assignment requires exactly one root Renderer; "
                        + $"found {renderers.Length}."
                );
            }

            if (!gameObject.TryGetComponent(out BattlementMaterialAssignments assignments))
            {
                assignments = gameObject.AddComponent<BattlementMaterialAssignments>();
            }

            assignments.EnsureInitialized(renderers[0], preparedAssets);
            assignments.SetMaterial(command.Address, command.Slot);
            return null;
        }
    }
}

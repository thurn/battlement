#nullable enable
using System.Collections.Generic;
using System.Linq;
using Battlement.UI;

namespace Battlement
{
    // Tracks actual creations: queued reconciliations can describe hosts that never existed.
    internal sealed class BattlementWorkOwnership
    {
        private readonly Dictionary<ObjectId, (ulong Scope, bool Ui)> objects = new();

        public void Clear() => objects.Clear();

        public void Record(
            BattlementCommandExecution command,
            ulong? scope,
            BattlementWorld world,
            BattlementUiDocuments ui
        )
        {
            if (command.DirectUiDestroy.HasValue || command.DirectDestroyObject.HasValue)
                Prune(world, ui);
            if (!scope.HasValue)
                return;
            if (command.DirectUiCreate is BattlementDirectUiCreate forest)
                for (int index = 0; index < forest.NodeCount; index++)
                    objects[forest.ReadNodeId(index)] = (scope.Value, true);
            ObjectId? id =
                command.DirectImageObjectCreate?.Placement.ObjectId
                ?? command.DirectPrimitiveObjectCreate?.Placement.ObjectId
                ?? command.DirectPrefabObjectCreate?.Placement.ObjectId
                ?? command.DirectEmptyObjectCreate?.Placement.ObjectId
                ?? command.DirectTextObjectCreate?.Placement.ObjectId
                ?? command.DirectCameraObjectCreate?.Placement.ObjectId
                ?? command.DirectLightObjectCreate?.Placement.ObjectId;
            if (id.HasValue)
                objects[id.Value] = (scope.Value, false);
        }

        public void Cancel(
            ulong scope,
            BattlementWorld world,
            BattlementUiDocuments ui,
            BattlementOperationRegistry operations
        )
        {
            foreach (var entry in objects.Where(entry => entry.Value.Scope == scope).ToArray())
            {
                if (entry.Value.Ui)
                {
                    if (ui.TryGet(entry.Key, out _))
                        ui.Destroy(entry.Key);
                }
                else if (world.TryGetObject(entry.Key, out _))
                {
                    operations.CancelObjects(world.GetHierarchyObjectIds(entry.Key));
                    world.DestroyObject(entry.Key);
                }
                objects.Remove(entry.Key);
            }
            Prune(world, ui);
        }

        private void Prune(BattlementWorld world, BattlementUiDocuments ui)
        {
            foreach (var entry in objects.ToArray())
                if (
                    entry.Value.Ui
                        ? !ui.TryGet(entry.Key, out _)
                        : !world.TryGetObject(entry.Key, out _)
                )
                    objects.Remove(entry.Key);
        }
    }
}

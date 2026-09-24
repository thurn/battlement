#nullable enable

using System;
using System.Collections.Generic;

namespace Battlement
{
    /// <summary>
    /// Finds UI references that race an earlier batch the referencing batch does not wait for.
    /// </summary>
    /// <remarks>
    /// Unordered batches may run in either order depending on frame timing, so such a reference is
    /// a latent missing-element failure even when the current frame happens to order it.
    /// </remarks>
    internal static class BattlementBatchOrdering
    {
        /// <summary>Describes the first racing reference in <paramref name="later"/>.</summary>
        /// <param name="unordered">Unfinished unordered batches and their next groups.</param>
        /// <param name="liveSubtree">Returns a live UI element and its logical descendants.</param>
        public static string? FindUnorderedReference(
            IBattlementBatchView later,
            IEnumerable<(IBattlementBatchView Batch, int FirstGroup)> unordered,
            Func<ObjectId, IEnumerable<ObjectId>> liveSubtree
        )
        {
            List<ObjectId> references = References(later);
            if (references.Count == 0)
                return null;
            foreach ((IBattlementBatchView Batch, int FirstGroup) earlier in unordered)
            {
                HashSet<ObjectId> changed = Changed(earlier, liveSubtree);
                foreach (ObjectId id in references)
                {
                    if (changed.Contains(id))
                        return $"Batch {later.Id} references UI element {id}, which unordered "
                            + $"batch {earlier.Batch.Id} still creates or destroys; hosts may run "
                            + "them in either order.";
                }
            }
            return null;
        }

        private static List<ObjectId> References(IBattlementBatchView batch)
        {
            var created = new HashSet<ObjectId>();
            var result = new List<ObjectId>();
            foreach (BattlementCommandExecution command in Commands(batch, 0))
            {
                if (command.DirectUiCreate is BattlementDirectUiCreate create)
                {
                    result.Add(create.ParentId);
                    AddNodes(create, created);
                }
                if (command.DirectUiPlacement is BattlementDirectVisualElementPlacement placement)
                {
                    result.Add(placement.ObjectId);
                    if (placement.ParentId is ObjectId parentId)
                        result.Add(parentId);
                }
                ObjectId? target =
                    command.DirectUiProperties?.ObjectId
                    ?? command.DirectUiScalar?.ObjectId
                    ?? command.DirectUiDestroy
                    ?? command.DirectUiAction?.ObjectId;
                if (target is ObjectId id)
                    result.Add(id);
            }
            result.RemoveAll(created.Contains);
            return result;
        }

        private static HashSet<ObjectId> Changed(
            (IBattlementBatchView Batch, int FirstGroup) earlier,
            Func<ObjectId, IEnumerable<ObjectId>> liveSubtree
        )
        {
            var result = new HashSet<ObjectId>();
            var (batch, firstGroup) = earlier;
            foreach (BattlementCommandExecution command in Commands(batch, firstGroup))
            {
                if (command.DirectUiCreate is BattlementDirectUiCreate create)
                    AddNodes(create, result);
                if (command.DirectUiDestroy is ObjectId destroyed)
                {
                    result.Add(destroyed);
                    result.UnionWith(liveSubtree(destroyed));
                }
            }
            return result;
        }

        private static IEnumerable<BattlementCommandExecution> Commands(
            IBattlementBatchView batch,
            int firstGroup
        )
        {
            for (int group = firstGroup; group < batch.GroupCount; group++)
            {
                for (int command = 0; command < batch.CommandCount(group); command++)
                    yield return batch.ReadCommand(group, command);
            }
        }

        private static void AddNodes(BattlementDirectUiCreate create, HashSet<ObjectId> result)
        {
            for (int index = 0; index < create.NodeCount; index++)
                result.Add(create.ReadNodeId(index));
        }
    }
}

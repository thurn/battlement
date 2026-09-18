#nullable enable
using System.Collections.Generic;
using System.Linq;
using Battlement.UI;
using UnityEngine;

namespace Battlement
{
    // Tracks actual creations: queued reconciliations can describe hosts that never existed.
    internal sealed class BattlementWorkOwnership
    {
        private readonly Dictionary<ObjectId, (ulong Scope, bool Ui)> objects = new();
        private readonly Dictionary<ObjectId, ulong> particleObjects = new();
        private readonly Dictionary<ObjectId, ulong> motionPlaybacks = new();
        private readonly Dictionary<ulong, HashSet<ParticleSystem>> pausedParticles = new();

        public void Clear()
        {
            objects.Clear();
            particleObjects.Clear();
            motionPlaybacks.Clear();
            pausedParticles.Clear();
        }

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
            if (command.DirectParticlePlay is BattlementDirectParticlePlay particle)
                particleObjects[particle.ObjectId] = scope.Value;
            if (
                command.DirectMotionControl is BattlementDirectMotionControlCommand motion
                && motion.Kind == MotionControlOperationKind.Start
            )
                motionPlaybacks[motion.PlaybackId] = scope.Value;
            if (
                command.DirectMotionScope is BattlementDirectMotionScopeCommand motionScope
                && motionScope.Kind == MotionScopeOperationKind.Start
            )
                motionPlaybacks[motionScope.PlaybackId] = scope.Value;
        }

        public void Pause(
            ulong scope,
            BattlementWorld world,
            BattlementUiDocuments ui,
            BattlementParticleEffects particles
        )
        {
            var objectIds = objects
                .Where(entry => entry.Value.Scope == scope && !entry.Value.Ui)
                .SelectMany(entry =>
                    world.TryGetObject(entry.Key, out _)
                        ? world.GetHierarchyObjectIds(entry.Key)
                        : System.Array.Empty<System.Guid>()
                )
                .Concat(
                    particleObjects
                        .Where(entry => entry.Value == scope)
                        .Select(entry => entry.Key.Value)
                )
                .Distinct();
            pausedParticles[scope] = particles.Pause(objectIds);
            foreach (
                ObjectId playback in motionPlaybacks
                    .Where(entry => entry.Value == scope)
                    .Select(entry => entry.Key)
            )
                ui.MotionWorld.PauseEffects(playback);
        }

        public void Resume(
            ulong scope,
            BattlementUiDocuments ui,
            BattlementParticleEffects particles
        )
        {
            if (pausedParticles.Remove(scope, out HashSet<ParticleSystem>? systems))
                particles.Resume(systems);
            foreach (
                ObjectId playback in motionPlaybacks
                    .Where(entry => entry.Value == scope)
                    .Select(entry => entry.Key)
            )
                ui.MotionWorld.ResumeEffects(playback);
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
            foreach (
                ObjectId objectId in particleObjects
                    .Where(entry => entry.Value == scope)
                    .Select(entry => entry.Key)
                    .ToArray()
            )
                particleObjects.Remove(objectId);
            foreach (
                ObjectId playback in motionPlaybacks
                    .Where(entry => entry.Value == scope)
                    .Select(entry => entry.Key)
                    .ToArray()
            )
            {
                ui.MotionWorld.CancelEffects(playback);
                motionPlaybacks.Remove(playback);
            }
            pausedParticles.Remove(scope);
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

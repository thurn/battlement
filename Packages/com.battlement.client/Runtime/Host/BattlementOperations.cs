#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Battlement
{
    /// <summary>Owns command identities and work that can outlive command launch.</summary>
    internal sealed class BattlementOperationRegistry
    {
        private readonly HashSet<Guid> executedCommands = new();
        private readonly List<TrackedOperation> operations = new();
        private readonly Action<OperationFailed<CoreErrorCode>, Exception?> reportFailure;
        private readonly Action<
            BattlementRegisteredCommandException,
            SessionId,
            BatchId,
            CommandId
        > reportCustomFailure;

        public BattlementOperationRegistry(
            Action<OperationFailed<CoreErrorCode>, Exception?> reportFailure,
            Action<
                BattlementRegisteredCommandException,
                SessionId,
                BatchId,
                CommandId
            > reportCustomFailure
        ) => (this.reportFailure, this.reportCustomFailure) = (reportFailure, reportCustomFailure);

        public void BeginSession()
        {
            CancelAll();
            executedCommands.Clear();
        }

        public int FiniteOperationCount =>
            operations.Count(operation => !operation.IsInfinite && !operation.IsHeld);

        public int HeldOperationCount => operations.Count(operation => operation.IsHeld);

        public int InfiniteOperationCount => operations.Count(operation => operation.IsInfinite);

        public bool HasFiniteOperations => FiniteOperationCount != 0;

        public bool HasInfiniteOperations => InfiniteOperationCount != 0;

        public IBattlementCommandOperation? Launch(
            SessionId sessionId,
            BatchId batchId,
            BattlementCommandExecution command,
            TimeSpan now,
            Func<TimeSpan, IBattlementCommandOperation?> launch,
            ulong? workScope = null
        )
        {
            if (!executedCommands.Add(command.Id.Value))
            {
                throw new BattlementCommandException(
                    CoreErrorCode.DuplicateId,
                    $"Command UUID {command.Id} was already executed in this session."
                );
            }

            if (command.DirectCancel is BattlementDirectCancel directCancel)
            {
                Cancel(directCancel.CommandId);
                return null;
            }

            BattlementConflictKey[] keys =
                command.DirectLocalPosition is BattlementDirectLocalPosition position
                    ? new[]
                    {
                        new BattlementConflictKey(
                            position.ObjectId.Value,
                            BattlementConflictKeys.Position
                        ),
                    }
                : command.DirectWorldPosition is BattlementDirectWorldPosition worldPosition
                    ? new[]
                    {
                        new BattlementConflictKey(
                            worldPosition.ObjectId.Value,
                            BattlementConflictKeys.Position
                        ),
                    }
                : command.DirectTweenLocalPosition
                    is BattlementDirectTweenLocalPosition tweenPosition
                    ? new[]
                    {
                        new BattlementConflictKey(
                            tweenPosition.ObjectId.Value,
                            BattlementConflictKeys.Position
                        ),
                    }
                : command.DirectTweenRotation is BattlementDirectTweenRotation tweenRotation
                    ? new[]
                    {
                        new BattlementConflictKey(
                            tweenRotation.ObjectId.Value,
                            BattlementConflictKeys.Rotation
                        ),
                    }
                : command.DirectTweenScale is BattlementDirectTweenScale tweenScale
                    ? new[]
                    {
                        new BattlementConflictKey(
                            tweenScale.ObjectId.Value,
                            BattlementConflictKeys.LocalScale
                        ),
                    }
                : command.DirectRotation is BattlementDirectRotation rotation
                    ? new[]
                    {
                        new BattlementConflictKey(
                            rotation.ObjectId.Value,
                            BattlementConflictKeys.Rotation
                        ),
                    }
                : command.DirectScale is BattlementDirectScale scale
                    ? new[]
                    {
                        new BattlementConflictKey(
                            scale.ObjectId.Value,
                            BattlementConflictKeys.LocalScale
                        ),
                    }
                : command.DirectSetMaterial is BattlementDirectSetMaterial material
                    ? new[]
                    {
                        new BattlementConflictKey(
                            material.ObjectId.Value,
                            "material",
                            material.Slot
                        ),
                    }
                : command.DirectAudioVolume is BattlementDirectAudioVolume audioVolume
                    ? new[]
                    {
                        new BattlementConflictKey(audioVolume.AudioCommandId.Value, "volume"),
                    }
                : command.DirectTweenAudioVolume is BattlementDirectTweenAudioVolume tweenVolume
                    ? new[]
                    {
                        new BattlementConflictKey(tweenVolume.AudioCommandId.Value, "volume"),
                    }
                : command.DirectComponent is BattlementDirectComponentCommand component
                    ? BattlementConflictKeys.For(component)
                : Array.Empty<BattlementConflictKey>();
            TrackedOperation[] conflicts = operations
                .Where(operation => operation.ConflictsWith(keys))
                .ToArray();
            if (command.DirectConflictPolicy == ConflictPolicy.Wait)
            {
                if (conflicts.Any(operation => operation.IsInfinite))
                {
                    throw new BattlementCommandException(
                        CoreErrorCode.InfiniteWait,
                        "A property command cannot wait for an infinite operation."
                    );
                }
            }
            else
            {
                foreach (TrackedOperation conflict in conflicts)
                {
                    conflict.Cancel();
                }

                conflicts = Array.Empty<TrackedOperation>();
            }

            var tracked = new TrackedOperation(
                command.Id,
                sessionId,
                batchId,
                command.IsBlocking,
                command.DirectLocalPosition is BattlementDirectLocalPosition targetPosition
                        ? targetPosition.ObjectId.Value
                    : command.DirectWorldPosition
                        is BattlementDirectWorldPosition targetWorldPosition
                        ? targetWorldPosition.ObjectId.Value
                    : command.DirectTweenLocalPosition
                        is BattlementDirectTweenLocalPosition targetTweenPosition
                        ? targetTweenPosition.ObjectId.Value
                    : command.DirectTweenRotation
                        is BattlementDirectTweenRotation targetTweenRotation
                        ? targetTweenRotation.ObjectId.Value
                    : command.DirectTweenScale is BattlementDirectTweenScale targetTweenScale
                        ? targetTweenScale.ObjectId.Value
                    : command.DirectLabelUpdate is BattlementDirectLabelUpdate targetLabel
                        ? targetLabel.ObjectId.Value
                    : command.DirectTextContent is BattlementDirectTextContent targetText
                        ? targetText.ObjectId.Value
                    : command.DirectSetMaterial is BattlementDirectSetMaterial targetMaterial
                        ? targetMaterial.ObjectId.Value
                    : command.DirectRotation is BattlementDirectRotation targetRotation
                        ? targetRotation.ObjectId.Value
                    : command.DirectScale is BattlementDirectScale targetScale
                        ? targetScale.ObjectId.Value
                    : command.DirectBoxHitRegionCreate
                        is BattlementDirectBoxHitRegionCreate targetBox
                        ? targetBox.Placement.ObjectId.Value
                    : command.DirectBoxHitRegionGeometry
                        is BattlementDirectBoxHitRegionGeometry targetBoxGeometry
                        ? targetBoxGeometry.ObjectId.Value
                    : command.DirectMaterialInstances
                        is BattlementDirectMaterialInstances targetInstances
                        ? targetInstances.ObjectId.Value
                    : command.DirectWorldPointer is BattlementDirectWorldPointer targetPointer
                        ? targetPointer.ObjectId.Value
                    : command.DirectRenderOrder is BattlementDirectRenderOrder targetOrder
                        ? targetOrder.ObjectId.Value
                    : command.DirectObjectActive is BattlementDirectObjectActive targetActive
                        ? targetActive.ObjectId.Value
                    : command.DirectPrimitiveObjectCreate
                        is BattlementDirectPrimitiveObjectCreate targetPrimitive
                        ? targetPrimitive.Placement.ObjectId.Value
                    : command.DirectMeshObjectCreate is BattlementDirectMeshObjectCreate targetMesh
                        ? targetMesh.Placement.ObjectId.Value
                    : command.DirectPrefabObjectCreate
                        is BattlementDirectPrefabObjectCreate targetPrefab
                        ? targetPrefab.Placement.ObjectId.Value
                    : command.DirectEmptyObjectCreate
                        is BattlementDirectEmptyObjectCreate targetEmpty
                        ? targetEmpty.Placement.ObjectId.Value
                    : command.DirectTextObjectCreate
                        is BattlementDirectTextObjectCreate targetCreatedText
                        ? targetCreatedText.Placement.ObjectId.Value
                    : command.DirectCameraObjectCreate
                        is BattlementDirectCameraObjectCreate targetCamera
                        ? targetCamera.Placement.ObjectId.Value
                    : command.DirectLightObjectCreate
                        is BattlementDirectLightObjectCreate targetLight
                        ? targetLight.Placement.ObjectId.Value
                    : command.DirectObjectReparent is BattlementDirectObjectReparent targetReparent
                        ? targetReparent.ObjectId.Value
                    : command.DirectParticleSpawn is BattlementDirectParticleSpawn targetParticle
                        ? targetParticle.ObjectId?.Value
                    : command.DirectParticlePlay is BattlementDirectParticlePlay targetParticlePlay
                        ? targetParticlePlay.ObjectId.Value
                    : command.DirectParticleStop is BattlementDirectParticleStop targetParticleStop
                        ? targetParticleStop.ObjectId.Value
                    : command.DirectComponent is BattlementDirectComponentCommand targetComponent
                        ? targetComponent.ObjectId.Value
                    : command.DirectAnimator is BattlementDirectAnimatorCommand targetAnimator
                        ? targetAnimator.ObjectId.Value
                    : null,
                keys,
                conflicts,
                launch,
                Remove
            )
            {
                WorkScope = workScope,
            };
            operations.Add(tracked);
            if (conflicts.Length == 0 && tracked.IsComplete(now))
            {
                return null;
            }

            return tracked;
        }

        public void AdvanceNonblocking(TimeSpan now)
        {
            foreach (
                TrackedOperation operation in operations.Where(item => !item.IsBlocking).ToArray()
            )
            {
                try
                {
                    operation.IsComplete(now);
                }
                catch (BattlementRegisteredCommandException exception)
                {
                    reportCustomFailure(
                        exception,
                        operation.SessionId,
                        operation.BatchId,
                        operation.Id
                    );
                }
                catch (BattlementCommandException exception)
                {
                    Report(
                        operation,
                        exception.ErrorCode,
                        exception.Message,
                        exception.DeveloperException
                    );
                }
                catch (Exception exception)
                {
                    Report(operation, CoreErrorCode.UnityException, exception.Message, exception);
                }
            }
        }

        public void CancelScope(ulong scope)
        {
            foreach (
                TrackedOperation operation in operations
                    .Where(item => item.WorkScope == scope)
                    .ToArray()
            )
                operation.Cancel();
        }

        public void PauseScope(ulong scope, TimeSpan now)
        {
            foreach (
                TrackedOperation operation in operations.Where(item => item.WorkScope == scope)
            )
                operation.Pause(now);
        }

        public void ResumeScope(ulong scope, TimeSpan now)
        {
            foreach (
                TrackedOperation operation in operations.Where(item => item.WorkScope == scope)
            )
                operation.Resume(now);
        }

        public void CancelAll()
        {
            foreach (TrackedOperation operation in operations.ToArray())
            {
                operation.Cancel();
            }
        }

        public void CancelObjects(IEnumerable<Guid> objectIds)
        {
            var targets = new HashSet<Guid>(objectIds);
            foreach (TrackedOperation operation in operations.ToArray())
            {
                if (operation.TargetObjectId is Guid target && targets.Contains(target))
                {
                    operation.Cancel();
                }
            }
        }

        public void CancelTransform(ObjectId objectId)
        {
            foreach (TrackedOperation operation in operations.ToArray())
            {
                if (operation.ControlsTransform(objectId.Value))
                {
                    operation.Cancel();
                }
            }
        }

        private void Cancel(CommandId commandId)
        {
            TrackedOperation? operation = operations.FirstOrDefault(item => item.Id == commandId);
            if (operation is not null)
            {
                operation.Cancel();
                return;
            }

            if (!executedCommands.Contains(commandId.Value))
            {
                throw new BattlementCommandException(
                    CoreErrorCode.UnknownCommand,
                    $"Command UUID {commandId} has not been executed in this session."
                );
            }
        }

        private void Report(
            TrackedOperation operation,
            CoreErrorCode errorCode,
            string message,
            Exception? exception = null
        ) =>
            reportFailure(
                new OperationFailed<CoreErrorCode>(
                    operation.SessionId,
                    operation.BatchId,
                    operation.Id,
                    errorCode,
                    message
                ),
                exception
            );

        private void Remove(TrackedOperation operation) => operations.Remove(operation);

        private sealed class TrackedOperation
            : IBattlementHeldCommandOperation,
                IBattlementPausableCommandOperation
        {
            private BattlementConflictKey[] keys;
            private readonly TrackedOperation[] blockers;
            private readonly Func<TimeSpan, IBattlementCommandOperation?> launch;
            private readonly Action<TrackedOperation> remove;
            private IBattlementCommandOperation? inner;
            private bool hasStarted;
            private bool isComplete;
            private bool isPaused;

            public TrackedOperation(
                CommandId id,
                SessionId sessionId,
                BatchId batchId,
                bool isBlocking,
                Guid? targetObjectId,
                BattlementConflictKey[] keys,
                TrackedOperation[] blockers,
                Func<TimeSpan, IBattlementCommandOperation?> launch,
                Action<TrackedOperation> remove
            ) =>
                (
                    Id,
                    SessionId,
                    BatchId,
                    IsBlocking,
                    TargetObjectId,
                    this.keys,
                    this.blockers,
                    this.launch,
                    this.remove
                ) = (
                    id,
                    sessionId,
                    batchId,
                    isBlocking,
                    targetObjectId,
                    keys,
                    blockers,
                    launch,
                    remove
                );

            public CommandId Id { get; }

            public SessionId SessionId { get; }

            public BatchId BatchId { get; }

            public ulong? WorkScope { get; set; }

            public bool IsBlocking { get; private set; }

            public Guid? TargetObjectId { get; private set; }

            public bool IsHeld =>
                !isComplete
                && (
                    isPaused
                    || inner is IBattlementHeldCommandOperation { IsHeld: true }
                    || blockers.Any(blocker => blocker.IsHeld)
                );

            public bool IsInfinite =>
                !isComplete
                && (inner?.IsInfinite == true || blockers.Any(blocker => blocker.IsInfinite));

            public bool IsComplete(TimeSpan now)
            {
                if (isComplete)
                {
                    return true;
                }

                if (isPaused)
                    return false;

                try
                {
                    if (!hasStarted)
                    {
                        if (blockers.Any(blocker => !blocker.IsComplete(now)))
                        {
                            return false;
                        }

                        hasStarted = true;
                        inner = launch(now);
                        ApplyScope();
                        if (WorkScope.HasValue && inner?.IsInfinite == true)
                            IsBlocking = false;
                    }

                    if (inner is not null && !inner.IsComplete(now))
                    {
                        return false;
                    }

                    Complete();
                    return true;
                }
                catch
                {
                    Complete();
                    throw;
                }
            }

            public void Cancel()
            {
                if (isComplete)
                {
                    return;
                }

                inner?.Cancel();
                Complete();
            }

            public void Pause(TimeSpan now)
            {
                if (isComplete || isPaused)
                    return;
                isPaused = true;
                if (inner is IBattlementPausableCommandOperation pausable)
                    pausable.Pause(now);
            }

            public void Resume(TimeSpan now)
            {
                if (isComplete || !isPaused)
                    return;
                if (inner is IBattlementPausableCommandOperation pausable)
                    pausable.Resume(now);
                isPaused = false;
            }

            public bool ConflictsWith(IReadOnlyList<BattlementConflictKey> requested) =>
                !isComplete && keys.Any(key => requested.Any(key.ConflictsWith));

            public bool ControlsTransform(Guid objectId) =>
                !isComplete && keys.Any(key => key.TargetId == objectId && key.IsTransform);

            private void Complete()
            {
                if (isComplete)
                {
                    return;
                }

                isComplete = true;
                remove(this);
            }

            private void ApplyScope()
            {
                if (inner is not IBattlementScopedCommandOperation scoped)
                {
                    return;
                }

                TargetObjectId = scoped.TargetObjectId.Value;
                if (scoped.ControlsTransform)
                {
                    keys = BattlementConflictKeys.Transform(scoped.TargetObjectId);
                }
            }
        }
    }

    internal readonly struct BattlementConflictKey
    {
        public BattlementConflictKey(Guid targetId, string property, uint? slot = null) =>
            (TargetId, Property, Slot) = (targetId, property, slot);

        public Guid TargetId { get; }

        public string Property { get; }

        public uint? Slot { get; }

        public bool IsTransform =>
            Property
                is BattlementConflictKeys.Position
                    or BattlementConflictKeys.Rotation
                    or BattlementConflictKeys.LocalScale;

        public bool ConflictsWith(BattlementConflictKey other) =>
            TargetId == other.TargetId
            && Property == other.Property
            && (Slot is null || other.Slot is null || Slot == other.Slot);
    }

    internal static class BattlementConflictKeys
    {
        public const string Position = "position";
        public const string Rotation = "rotation";
        public const string LocalScale = "localScale";

        public static BattlementConflictKey[] Transform(ObjectId id) =>
            new[]
            {
                new BattlementConflictKey(id.Value, Position),
                new BattlementConflictKey(id.Value, Rotation),
                new BattlementConflictKey(id.Value, LocalScale),
            };

        public static BattlementConflictKey[] For(BattlementDirectComponentCommand command) =>
            command.Kind switch
            {
                BattlementDirectComponentCommandKind.CameraSetPerspective => CameraProjection(
                    command.ObjectId
                ),
                BattlementDirectComponentCommandKind.CameraSetOrthographic => CameraProjection(
                    command.ObjectId
                ),
                BattlementDirectComponentCommandKind.CameraTweenFieldOfView => Object(
                    command.ObjectId,
                    "camera.fieldOfView"
                ),
                BattlementDirectComponentCommandKind.CameraTweenOrthographicSize => Object(
                    command.ObjectId,
                    "camera.orthographicSize"
                ),
                BattlementDirectComponentCommandKind.LightSetColor => Object(
                    command.ObjectId,
                    "light.color"
                ),
                BattlementDirectComponentCommandKind.LightTweenColor => Object(
                    command.ObjectId,
                    "light.color"
                ),
                BattlementDirectComponentCommandKind.LightSetIntensity => Object(
                    command.ObjectId,
                    "light.intensity"
                ),
                BattlementDirectComponentCommandKind.LightTweenIntensity => Object(
                    command.ObjectId,
                    "light.intensity"
                ),
                BattlementDirectComponentCommandKind.ImageSetTint => Object(
                    command.ObjectId,
                    "image.tint"
                ),
                BattlementDirectComponentCommandKind.ImageTweenTint => Object(
                    command.ObjectId,
                    "image.tint"
                ),
                BattlementDirectComponentCommandKind.ImageSetOpacity => Object(
                    command.ObjectId,
                    "image.opacity"
                ),
                BattlementDirectComponentCommandKind.ImageTweenOpacity => Object(
                    command.ObjectId,
                    "image.opacity"
                ),
                BattlementDirectComponentCommandKind.TextSetColor => Object(
                    command.ObjectId,
                    "text.color"
                ),
                BattlementDirectComponentCommandKind.TextTweenColor => Object(
                    command.ObjectId,
                    "text.color"
                ),
                BattlementDirectComponentCommandKind.TextSetSize => Object(
                    command.ObjectId,
                    "text.size"
                ),
                BattlementDirectComponentCommandKind.TextTweenSize => Object(
                    command.ObjectId,
                    "text.size"
                ),
                _ => Array.Empty<BattlementConflictKey>(),
            };

        private static BattlementConflictKey[] CameraProjection(ObjectId id) =>
            new[]
            {
                new BattlementConflictKey(id.Value, "camera.fieldOfView"),
                new BattlementConflictKey(id.Value, "camera.orthographicSize"),
            };

        private static BattlementConflictKey[] Object(ObjectId id, string property) =>
            new[] { new BattlementConflictKey(id.Value, property) };
    }
}

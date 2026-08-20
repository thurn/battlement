#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Masonry
{
    /// <summary>Work started by a command that Masonry can poll and cancel.</summary>
    public interface IMasonryCommandOperation
    {
        /// <summary>Gets whether the operation has no natural completion.</summary>
        bool IsInfinite { get; }

        /// <summary>Advances the operation and reports whether it has completed.</summary>
        bool IsComplete(TimeSpan now);

        /// <summary>Cancels the operation without firing completion behavior.</summary>
        void Cancel();
    }

    /// <summary>Owns command identities and work that can outlive command launch.</summary>
    internal sealed class MasonryOperationRegistry
    {
        private readonly HashSet<Guid> executedCommands = new();
        private readonly List<TrackedOperation> operations = new();
        private readonly Action<OperationFailed<CoreErrorCode>> reportFailure;
        private readonly Action<
            MasonryRegisteredCommandException,
            SessionId,
            BatchId,
            CommandId
        > reportCustomFailure;

        public MasonryOperationRegistry(
            Action<OperationFailed<CoreErrorCode>> reportFailure,
            Action<
                MasonryRegisteredCommandException,
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

        public IMasonryCommandOperation? Launch(
            SessionId sessionId,
            BatchId batchId,
            ICommand command,
            TimeSpan now,
            Func<TimeSpan, IMasonryCommandOperation?> launch
        )
        {
            if (!executedCommands.Add(command.Id.Value))
            {
                throw new MasonryCommandException(
                    CoreErrorCode.DuplicateId,
                    $"Command UUID {command.Id} was already executed in this session."
                );
            }

            if (command is Command { Body: CommandBody.Operation.Cancel cancel })
            {
                Cancel(cancel.CommandId);
                return null;
            }

            MasonryConflictKey[] keys = command is Command core
                ? MasonryConflictKeys.For(core.Body)
                : Array.Empty<MasonryConflictKey>();
            TrackedOperation[] conflicts = operations
                .Where(operation => operation.ConflictsWith(keys))
                .ToArray();
            if (
                command is Command
                {
                    Body: IPropertyCommandBody { OnConflict: ConflictPolicy.Wait }
                }
            )
            {
                if (conflicts.Any(operation => operation.IsInfinite))
                {
                    throw new MasonryCommandException(
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
                command is Command target ? MasonryOperationTargets.For(target.Body) : null,
                keys,
                conflicts,
                launch,
                Remove
            );
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
                catch (MasonryRegisteredCommandException exception)
                {
                    reportCustomFailure(
                        exception,
                        operation.SessionId,
                        operation.BatchId,
                        operation.Id
                    );
                }
                catch (MasonryCommandException exception)
                {
                    Report(operation, exception.ErrorCode, exception.Message);
                }
                catch (Exception exception)
                {
                    Report(operation, CoreErrorCode.UnityException, exception.Message);
                }
            }
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
                throw new MasonryCommandException(
                    CoreErrorCode.UnknownCommand,
                    $"Command UUID {commandId} has not been executed in this session."
                );
            }
        }

        private void Report(TrackedOperation operation, CoreErrorCode errorCode, string message) =>
            reportFailure(
                new OperationFailed<CoreErrorCode>(
                    operation.SessionId,
                    operation.BatchId,
                    operation.Id,
                    errorCode,
                    message
                )
            );

        private void Remove(TrackedOperation operation) => operations.Remove(operation);

        private sealed class TrackedOperation : IMasonryCommandOperation
        {
            private MasonryConflictKey[] keys;
            private readonly TrackedOperation[] blockers;
            private readonly Func<TimeSpan, IMasonryCommandOperation?> launch;
            private readonly Action<TrackedOperation> remove;
            private IMasonryCommandOperation? inner;
            private bool hasStarted;
            private bool isComplete;

            public TrackedOperation(
                CommandId id,
                SessionId sessionId,
                BatchId batchId,
                bool isBlocking,
                Guid? targetObjectId,
                MasonryConflictKey[] keys,
                TrackedOperation[] blockers,
                Func<TimeSpan, IMasonryCommandOperation?> launch,
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

            public bool IsBlocking { get; }

            public Guid? TargetObjectId { get; private set; }

            public bool IsInfinite =>
                !isComplete
                && (inner?.IsInfinite == true || blockers.Any(blocker => blocker.IsInfinite));

            public bool IsComplete(TimeSpan now)
            {
                if (isComplete)
                {
                    return true;
                }

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

            public bool ConflictsWith(IReadOnlyList<MasonryConflictKey> requested) =>
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
                if (inner is not IMasonryScopedCommandOperation scoped)
                {
                    return;
                }

                TargetObjectId = scoped.TargetObjectId.Value;
                if (scoped.ControlsTransform)
                {
                    keys = MasonryConflictKeys.Transform(scoped.TargetObjectId);
                }
            }
        }
    }

    internal readonly struct MasonryConflictKey
    {
        public MasonryConflictKey(Guid targetId, string property, uint? slot = null) =>
            (TargetId, Property, Slot) = (targetId, property, slot);

        public Guid TargetId { get; }

        public string Property { get; }

        public uint? Slot { get; }

        public bool IsTransform =>
            Property
                is MasonryConflictKeys.Position
                    or MasonryConflictKeys.Rotation
                    or MasonryConflictKeys.LocalScale;

        public bool ConflictsWith(MasonryConflictKey other) =>
            TargetId == other.TargetId
            && Property == other.Property
            && (Slot is null || other.Slot is null || Slot == other.Slot);
    }

    internal static class MasonryConflictKeys
    {
        public const string Position = "position";
        public const string Rotation = "rotation";
        public const string LocalScale = "localScale";

        public static MasonryConflictKey[] For(CommandBody body) =>
            body switch
            {
                CommandBody.Transform.SetLocalPosition value => Object(value.ObjectId, Position),
                CommandBody.Transform.SetWorldPosition value => Object(value.ObjectId, Position),
                CommandBody.Transform.TweenLocalPosition value => Object(value.ObjectId, Position),
                CommandBody.Transform.TweenWorldPosition value => Object(value.ObjectId, Position),
                CommandBody.Transform.SetLocalRotation value => Object(value.ObjectId, Rotation),
                CommandBody.Transform.SetWorldRotation value => Object(value.ObjectId, Rotation),
                CommandBody.Transform.TweenLocalRotation value => Object(value.ObjectId, Rotation),
                CommandBody.Transform.TweenWorldRotation value => Object(value.ObjectId, Rotation),
                CommandBody.Transform.SetLocalScale value => Object(value.ObjectId, LocalScale),
                CommandBody.Transform.TweenLocalScale value => Object(value.ObjectId, LocalScale),
                CommandBody.Renderer.SetMaterial value => new[]
                {
                    new MasonryConflictKey(value.ObjectId.Value, "material", value.Slot),
                },
                CommandBody.Camera.SetPerspective value => CameraProjection(value.ObjectId),
                CommandBody.Camera.SetOrthographic value => CameraProjection(value.ObjectId),
                CommandBody.Camera.TweenFieldOfView value => Object(
                    value.ObjectId,
                    "camera.fieldOfView"
                ),
                CommandBody.Camera.TweenOrthographicSize value => Object(
                    value.ObjectId,
                    "camera.orthographicSize"
                ),
                CommandBody.Light.SetColor value => Object(value.ObjectId, "light.color"),
                CommandBody.Light.TweenColor value => Object(value.ObjectId, "light.color"),
                CommandBody.Light.SetIntensity value => Object(value.ObjectId, "light.intensity"),
                CommandBody.Light.TweenIntensity value => Object(value.ObjectId, "light.intensity"),
                CommandBody.Image.SetTint value => Object(value.ObjectId, "image.tint"),
                CommandBody.Image.TweenTint value => Object(value.ObjectId, "image.tint"),
                CommandBody.Image.SetOpacity value => Object(value.ObjectId, "image.opacity"),
                CommandBody.Image.TweenOpacity value => Object(value.ObjectId, "image.opacity"),
                CommandBody.Text.SetColor value => Object(value.ObjectId, "text.color"),
                CommandBody.Text.TweenColor value => Object(value.ObjectId, "text.color"),
                CommandBody.Text.SetSize value => Object(value.ObjectId, "text.size"),
                CommandBody.Text.TweenSize value => Object(value.ObjectId, "text.size"),
                CommandBody.Audio.SetVolume value => Audio(value.AudioCommandId),
                CommandBody.Audio.TweenVolume value => Audio(value.AudioCommandId),
                _ => Array.Empty<MasonryConflictKey>(),
            };

        public static MasonryConflictKey[] Transform(ObjectId id) =>
            new[]
            {
                new MasonryConflictKey(id.Value, Position),
                new MasonryConflictKey(id.Value, Rotation),
                new MasonryConflictKey(id.Value, LocalScale),
            };

        private static MasonryConflictKey[] CameraProjection(ObjectId id) =>
            new[]
            {
                new MasonryConflictKey(id.Value, "camera.fieldOfView"),
                new MasonryConflictKey(id.Value, "camera.orthographicSize"),
            };

        private static MasonryConflictKey[] Audio(CommandId id) =>
            new[] { new MasonryConflictKey(id.Value, "volume") };

        private static MasonryConflictKey[] Object(ObjectId id, string property) =>
            new[] { new MasonryConflictKey(id.Value, property) };
    }

    internal static class MasonryOperationTargets
    {
        public static Guid? For(CommandBody body) =>
            body switch
            {
                CommandBody.Transform.SetLocalPosition value => value.ObjectId.Value,
                CommandBody.Transform.SetWorldPosition value => value.ObjectId.Value,
                CommandBody.Transform.TweenLocalPosition value => value.ObjectId.Value,
                CommandBody.Transform.TweenWorldPosition value => value.ObjectId.Value,
                CommandBody.Transform.SetLocalRotation value => value.ObjectId.Value,
                CommandBody.Transform.SetWorldRotation value => value.ObjectId.Value,
                CommandBody.Transform.TweenLocalRotation value => value.ObjectId.Value,
                CommandBody.Transform.TweenWorldRotation value => value.ObjectId.Value,
                CommandBody.Transform.SetLocalScale value => value.ObjectId.Value,
                CommandBody.Transform.TweenLocalScale value => value.ObjectId.Value,
                CommandBody.Camera.SetPerspective value => value.ObjectId.Value,
                CommandBody.Camera.TweenFieldOfView value => value.ObjectId.Value,
                CommandBody.Camera.SetOrthographic value => value.ObjectId.Value,
                CommandBody.Camera.TweenOrthographicSize value => value.ObjectId.Value,
                CommandBody.Light.SetColor value => value.ObjectId.Value,
                CommandBody.Light.TweenColor value => value.ObjectId.Value,
                CommandBody.Light.SetIntensity value => value.ObjectId.Value,
                CommandBody.Light.TweenIntensity value => value.ObjectId.Value,
                CommandBody.Image.SetTint value => value.ObjectId.Value,
                CommandBody.Image.TweenTint value => value.ObjectId.Value,
                CommandBody.Image.SetOpacity value => value.ObjectId.Value,
                CommandBody.Image.TweenOpacity value => value.ObjectId.Value,
                CommandBody.Text.SetColor value => value.ObjectId.Value,
                CommandBody.Text.TweenColor value => value.ObjectId.Value,
                CommandBody.Text.SetSize value => value.ObjectId.Value,
                CommandBody.Text.TweenSize value => value.ObjectId.Value,
                CommandBody.Animator.Play value => value.ObjectId.Value,
                CommandBody.Animator.CrossFade value => value.ObjectId.Value,
                CommandBody.Particle.Play value => value.ObjectId.Value,
                _ => null,
            };
    }
}

#nullable enable

using System;
using System.Collections.Generic;
using System.Threading;
using UnityEngine;

namespace Battlement
{
    /// <summary>Looks up Battlement-controlled objects by their protocol identity.</summary>
    public interface IBattlementObjectLookup
    {
        /// <summary>Returns the live Unity object for an identity when it exists.</summary>
        bool TryGetObject(ObjectId id, out GameObject? gameObject);
    }

    /// <summary>Creates Battlement-owned tween operations for custom handlers.</summary>
    public interface IBattlementTweenHelpers
    {
        /// <summary>Interpolates a scalar value with Battlement tween semantics.</summary>
        IBattlementCommandOperation? Float(
            Transform lifetime,
            float start,
            float end,
            Tween settings,
            Action<float> apply
        );

        /// <summary>Interpolates a vector value with Battlement tween semantics.</summary>
        IBattlementCommandOperation? Vector(
            Transform lifetime,
            UnityEngine.Vector3 start,
            UnityEngine.Vector3 end,
            Tween settings,
            Action<UnityEngine.Vector3> apply
        );

        /// <summary>Interpolates a color value with Battlement tween semantics.</summary>
        IBattlementCommandOperation? Color(
            Transform lifetime,
            UnityEngine.Color start,
            UnityEngine.Color end,
            Tween settings,
            Action<UnityEngine.Color> apply
        );
    }

    /// <summary>Services and cancellation owned by one custom-command invocation.</summary>
    public sealed record BattlementCommandContext(
        CancellationToken Cancellation,
        IBattlementLogger Logger,
        IBattlementObjectLookup Objects,
        IBattlementPreparedAssetLookup PreparedAssets,
        IBattlementTweenHelpers Tweens
    )
    {
        /// <summary>
        /// Marks tracked work as targeting an object so destruction can cancel it.
        /// </summary>
        public IBattlementCommandOperation ForObject(
            ObjectId objectId,
            IBattlementCommandOperation operation,
            bool controlsTransform = false
        ) =>
            new BattlementScopedCommandOperation(
                objectId,
                Preconditions.CheckNotNull(operation, nameof(operation)),
                controlsTransform
            );
    }

    /// <summary>Runs a trusted, explicitly registered game command.</summary>
    public interface IBattlementCommandHandler<TPayload>
    {
        /// <summary>Runs the command and optionally returns work Battlement should track.</summary>
        IBattlementCommandOperation? Execute(
            CustomCommand<TPayload> command,
            BattlementCommandContext context
        );
    }

    /// <summary>
    /// Runs a generated custom-command payload view while its response lease is live.
    /// </summary>
    public interface IBattlementFlatBufferCommandHandler<TPayloadView>
    {
        /// <summary>
        /// Runs synchronously. The generated payload view must not be retained after this call.
        /// </summary>
        IBattlementCommandOperation? Execute(
            BattlementFlatBufferCommand<TPayloadView> command,
            BattlementCommandContext context
        );
    }

    /// <summary>Metadata and a generated borrowed payload for one custom command.</summary>
    public readonly struct BattlementFlatBufferCommand<TPayloadView>
    {
        public BattlementFlatBufferCommand(
            CommandId id,
            string type,
            TPayloadView payload,
            bool isBlocking
        ) => (Id, Type, Payload, IsBlocking) = (id, type, payload, isBlocking);

        public CommandId Id { get; }

        public string Type { get; }

        public TPayloadView Payload { get; }

        public bool IsBlocking { get; }
    }

    /// <summary>
    /// Dispatches a generated payload view without reconstructing an owned protocol command.
    /// </summary>
    public interface IBattlementFlatBufferCustomCommandDispatcher
    {
        IBattlementCommandOperation? Launch<TPayloadView>(
            CommandId id,
            string type,
            bool isBlocking,
            TPayloadView payload,
            TimeSpan now
        );
    }

    /// <summary>A protocol-visible game-specific custom-command failure.</summary>
    public sealed class BattlementCommandFailureException<TError> : Exception
    {
        /// <summary>Creates a failure with a game-owned stable error code.</summary>
        public BattlementCommandFailureException(
            TError errorCode,
            string message,
            Exception? innerException = null
        )
            : base(message, innerException) => ErrorCode = errorCode;

        public TError ErrorCode { get; }
    }

    internal sealed class BattlementCustomCommands : IBattlementFlatBufferCustomCommandDispatcher
    {
        private readonly Dictionary<string, IBattlementCommandRegistration> registrations = new(
            StringComparer.Ordinal
        );
        private readonly Func<TimeSpan, BattlementCommandContext> createContext;

        public BattlementCustomCommands(Func<TimeSpan, BattlementCommandContext> createContext) =>
            this.createContext = createContext;

        public IReadOnlyCollection<string> Types => registrations.Keys;

        public void Register<TPayload, TError>(
            string type,
            IBattlementCommandHandler<TPayload> handler
        )
        {
            RequireNamespaced(type);
            if (registrations.ContainsKey(type))
            {
                throw new InvalidOperationException(
                    $"A custom command handler is already registered for {type}."
                );
            }

            registrations.Add(
                type,
                new BattlementCommandRegistration<TPayload, TError>(
                    type,
                    Preconditions.CheckNotNull(handler, nameof(handler)),
                    createContext
                )
            );
        }

        public void RegisterFlatBuffer<TPayloadView, TError>(
            string type,
            IBattlementFlatBufferCommandHandler<TPayloadView> handler
        )
        {
            RequireNamespaced(type);
            if (registrations.ContainsKey(type))
            {
                throw new InvalidOperationException(
                    $"A custom command handler is already registered for {type}."
                );
            }

            registrations.Add(
                type,
                new BattlementFlatBufferCommandRegistration<TPayloadView, TError>(
                    type,
                    Preconditions.CheckNotNull(handler, nameof(handler)),
                    createContext
                )
            );
        }

        public IBattlementCommandOperation? Launch(ICommand command, TimeSpan now)
        {
            switch (command)
            {
                case BattlementUnknownCustomCommand unknown:
                    throw new BattlementCommandException(
                        CoreErrorCode.HandlerNotRegistered,
                        $"No custom command handler is registered for {unknown.Type}."
                    );
                case BattlementInvalidCustomCommand invalid:
                    throw new BattlementCommandException(
                        CoreErrorCode.InvalidEncoding,
                        $"Custom command {invalid.Type} payload could not be decoded: "
                            + invalid.Error.Message,
                        invalid.Error
                    );
                default:
                    break;
            }

            string type =
                (command as ICustomCommand)?.Type
                ?? throw new BattlementCommandException(
                    CoreErrorCode.HandlerNotRegistered,
                    "The custom command did not expose a command type."
                );
            if (!registrations.TryGetValue(type, out IBattlementCommandRegistration registration))
            {
                throw new BattlementCommandException(
                    CoreErrorCode.HandlerNotRegistered,
                    $"No custom command handler is registered for {type}."
                );
            }

            return registration.Launch(command, now);
        }

        public IBattlementCommandOperation? Launch<TPayloadView>(
            CommandId id,
            string type,
            bool isBlocking,
            TPayloadView payload,
            TimeSpan now
        )
        {
            if (!registrations.TryGetValue(type, out IBattlementCommandRegistration registration))
            {
                throw new BattlementCommandException(
                    CoreErrorCode.HandlerNotRegistered,
                    $"No custom command handler is registered for {type}."
                );
            }
            if (registration is not IBattlementFlatBufferCommandRegistration<TPayloadView> direct)
            {
                throw new BattlementCommandException(
                    CoreErrorCode.InvalidEncoding,
                    $"Custom command {type} used the wrong generated payload view."
                );
            }
            return direct.LaunchFlatBuffer(id, type, isBlocking, payload, now);
        }

        public bool TryGet(string type, out IBattlementCommandRegistration registration) =>
            registrations.TryGetValue(type, out registration!);

        public static void RequireNamespaced(string type)
        {
            string value = type ?? string.Empty;
            bool invalidOwner =
                string.IsNullOrWhiteSpace(value)
                || value.StartsWith("battlement.", StringComparison.Ordinal);
            bool invalidSeparator =
                value.IndexOf('.') <= 0 || value.EndsWith(".", StringComparison.Ordinal);
            if (invalidOwner || invalidSeparator)
            {
                throw new ArgumentException(
                    "A custom command type must be a non-Battlement namespaced string.",
                    nameof(type)
                );
            }
        }
    }

    internal interface IBattlementCommandRegistration
    {
        IBattlementCommandOperation? Launch(ICommand command, TimeSpan now);

        ReadOnlyMemory<byte> SerializeFlatBufferBatchFailure(
            IBattlementFlatBufferClientSchema schema,
            SessionId sessionId,
            BatchId batchId,
            CommandId? commandId,
            object errorCode,
            string message
        );

        ReadOnlyMemory<byte> SerializeFlatBufferOperationFailure(
            IBattlementFlatBufferClientSchema schema,
            SessionId sessionId,
            BatchId batchId,
            CommandId commandId,
            object errorCode,
            string message
        );
    }

    internal interface IBattlementFlatBufferCommandRegistration<TPayloadView>
        : IBattlementCommandRegistration
    {
        IBattlementCommandOperation? LaunchFlatBuffer(
            CommandId id,
            string type,
            bool isBlocking,
            TPayloadView payload,
            TimeSpan now
        );
    }

    internal sealed class BattlementCommandRegistration<TPayload, TError>
        : IBattlementCommandRegistration
    {
        private readonly string type;
        private readonly IBattlementCommandHandler<TPayload> handler;
        private readonly Func<TimeSpan, BattlementCommandContext> createContext;

        public BattlementCommandRegistration(
            string type,
            IBattlementCommandHandler<TPayload> handler,
            Func<TimeSpan, BattlementCommandContext> createContext
        ) => (this.type, this.handler, this.createContext) = (type, handler, createContext);

        public IBattlementCommandOperation? Launch(ICommand command, TimeSpan now)
        {
            var typed =
                command as CustomCommand<TPayload>
                ?? throw new BattlementCommandException(
                    CoreErrorCode.InvalidEncoding,
                    $"Custom command {type} used the wrong payload type."
                );
            var cancellation = new CancellationTokenSource();
            try
            {
                IBattlementCommandOperation? operation = handler.Execute(
                    typed,
                    createContext(now) with
                    {
                        Cancellation = cancellation.Token,
                    }
                );
                if (operation is null)
                {
                    cancellation.Dispose();
                    return null;
                }

                var custom = new BattlementCustomOperation<TError>(operation, cancellation, this);
                return operation is IBattlementScopedCommandOperation scoped
                    ? new BattlementScopedCommandOperation(
                        scoped.TargetObjectId,
                        custom,
                        scoped.ControlsTransform
                    )
                    : custom;
            }
            catch (BattlementCommandFailureException<TError> exception)
            {
                cancellation.Dispose();
                throw new BattlementRegisteredCommandException(
                    this,
                    exception.ErrorCode!,
                    exception.Message,
                    exception
                );
            }
            catch (Exception exception)
            {
                cancellation.Dispose();
                throw new BattlementCommandException(
                    CoreErrorCode.HandlerFailed,
                    exception.Message,
                    exception
                );
            }
        }

        public ReadOnlyMemory<byte> SerializeFlatBufferBatchFailure(
            IBattlementFlatBufferClientSchema schema,
            SessionId sessionId,
            BatchId batchId,
            CommandId? commandId,
            object errorCode,
            string message
        ) =>
            schema.SerializeBatchFailure(
                new BatchFailed<TError>(sessionId, batchId, (TError)errorCode, message, commandId)
            );

        public ReadOnlyMemory<byte> SerializeFlatBufferOperationFailure(
            IBattlementFlatBufferClientSchema schema,
            SessionId sessionId,
            BatchId batchId,
            CommandId commandId,
            object errorCode,
            string message
        ) =>
            schema.SerializeOperationFailure(
                new OperationFailed<TError>(
                    sessionId,
                    batchId,
                    commandId,
                    (TError)errorCode,
                    message
                )
            );
    }

    internal sealed class BattlementFlatBufferCommandRegistration<TPayloadView, TError>
        : IBattlementFlatBufferCommandRegistration<TPayloadView>
    {
        private readonly string type;
        private readonly IBattlementFlatBufferCommandHandler<TPayloadView> handler;
        private readonly Func<TimeSpan, BattlementCommandContext> createContext;

        public BattlementFlatBufferCommandRegistration(
            string type,
            IBattlementFlatBufferCommandHandler<TPayloadView> handler,
            Func<TimeSpan, BattlementCommandContext> createContext
        ) => (this.type, this.handler, this.createContext) = (type, handler, createContext);

        public IBattlementCommandOperation? Launch(ICommand command, TimeSpan now) =>
            throw new BattlementCommandException(
                CoreErrorCode.InvalidEncoding,
                $"Custom command {type} requires its generated FlatBuffer payload view."
            );

        public IBattlementCommandOperation? LaunchFlatBuffer(
            CommandId id,
            string commandType,
            bool isBlocking,
            TPayloadView payload,
            TimeSpan now
        )
        {
            var cancellation = new CancellationTokenSource();
            try
            {
                IBattlementCommandOperation? operation = handler.Execute(
                    new BattlementFlatBufferCommand<TPayloadView>(
                        id,
                        commandType,
                        payload,
                        isBlocking
                    ),
                    createContext(now) with
                    {
                        Cancellation = cancellation.Token,
                    }
                );
                if (operation is null)
                {
                    cancellation.Dispose();
                    return null;
                }
                var custom = new BattlementCustomOperation<TError>(operation, cancellation, this);
                return operation is IBattlementScopedCommandOperation scoped
                    ? new BattlementScopedCommandOperation(
                        scoped.TargetObjectId,
                        custom,
                        scoped.ControlsTransform
                    )
                    : custom;
            }
            catch (BattlementCommandFailureException<TError> exception)
            {
                cancellation.Dispose();
                throw new BattlementRegisteredCommandException(
                    this,
                    exception.ErrorCode!,
                    exception.Message,
                    exception
                );
            }
            catch (Exception exception)
            {
                cancellation.Dispose();
                throw new BattlementCommandException(
                    CoreErrorCode.HandlerFailed,
                    exception.Message,
                    exception
                );
            }
        }

        public ReadOnlyMemory<byte> SerializeFlatBufferBatchFailure(
            IBattlementFlatBufferClientSchema schema,
            SessionId sessionId,
            BatchId batchId,
            CommandId? commandId,
            object errorCode,
            string message
        ) =>
            schema.SerializeBatchFailure(
                new BatchFailed<TError>(sessionId, batchId, (TError)errorCode, message, commandId)
            );

        public ReadOnlyMemory<byte> SerializeFlatBufferOperationFailure(
            IBattlementFlatBufferClientSchema schema,
            SessionId sessionId,
            BatchId batchId,
            CommandId commandId,
            object errorCode,
            string message
        ) =>
            schema.SerializeOperationFailure(
                new OperationFailed<TError>(
                    sessionId,
                    batchId,
                    commandId,
                    (TError)errorCode,
                    message
                )
            );
    }

    internal sealed class BattlementRegisteredCommandException : Exception
    {
        public BattlementRegisteredCommandException(
            IBattlementCommandRegistration registration,
            object errorCode,
            string message,
            Exception innerException
        )
            : base(message, innerException) =>
            (Registration, ErrorCode) = (registration, errorCode);

        public IBattlementCommandRegistration Registration { get; }

        public object ErrorCode { get; }
    }

    internal sealed record BattlementUnknownCustomCommand(
        CommandId Id,
        string Type,
        bool IsBlocking
    ) : ICustomCommand;

    internal sealed record BattlementInvalidCustomCommand(
        CommandId Id,
        string Type,
        bool IsBlocking,
        Exception Error
    ) : ICustomCommand;

    internal sealed class BattlementCustomOperation<TError> : IBattlementCommandOperation
    {
        private readonly IBattlementCommandOperation operation;
        private readonly CancellationTokenSource cancellation;
        private readonly IBattlementCommandRegistration registration;
        private bool isFinished;

        public BattlementCustomOperation(
            IBattlementCommandOperation operation,
            CancellationTokenSource cancellation,
            IBattlementCommandRegistration registration
        ) =>
            (this.operation, this.cancellation, this.registration) = (
                operation,
                cancellation,
                registration
            );

        public bool IsInfinite => operation.IsInfinite;

        public bool IsComplete(TimeSpan now)
        {
            try
            {
                bool complete = operation.IsComplete(now);
                if (complete)
                {
                    Finish();
                }

                return complete;
            }
            catch (BattlementCommandFailureException<TError> exception)
            {
                Finish();
                throw new BattlementRegisteredCommandException(
                    registration,
                    exception.ErrorCode!,
                    exception.Message,
                    exception
                );
            }
            catch
            {
                Finish();
                throw;
            }
        }

        public void Cancel()
        {
            if (isFinished)
            {
                return;
            }

            cancellation.Cancel();
            operation.Cancel();
            Finish();
        }

        private void Finish()
        {
            if (isFinished)
            {
                return;
            }

            isFinished = true;
            cancellation.Dispose();
        }
    }

    internal interface IBattlementScopedCommandOperation
    {
        ObjectId TargetObjectId { get; }

        bool ControlsTransform { get; }
    }

    internal sealed class BattlementScopedCommandOperation
        : IBattlementCommandOperation,
            IBattlementScopedCommandOperation
    {
        private readonly IBattlementCommandOperation operation;

        public BattlementScopedCommandOperation(
            ObjectId targetObjectId,
            IBattlementCommandOperation operation,
            bool controlsTransform
        ) =>
            (TargetObjectId, this.operation, ControlsTransform) = (
                targetObjectId,
                operation,
                controlsTransform
            );

        public ObjectId TargetObjectId { get; }

        public bool ControlsTransform { get; }

        public bool IsInfinite => operation.IsInfinite;

        public bool IsComplete(TimeSpan now) => operation.IsComplete(now);

        public void Cancel() => operation.Cancel();
    }

    internal sealed class BattlementTweenHelpers : IBattlementTweenHelpers
    {
        private readonly BattlementTweenAdapter adapter;
        private readonly TimeSpan now;

        public BattlementTweenHelpers(BattlementTweenAdapter adapter, TimeSpan now) =>
            (this.adapter, this.now) = (adapter, now);

        public IBattlementCommandOperation? Float(
            Transform lifetime,
            float start,
            float end,
            Tween settings,
            Action<float> apply
        ) => adapter.Float(lifetime, start, end, settings, now, apply);

        public IBattlementCommandOperation? Vector(
            Transform lifetime,
            UnityEngine.Vector3 start,
            UnityEngine.Vector3 end,
            Tween settings,
            Action<UnityEngine.Vector3> apply
        ) => adapter.Vector(lifetime, start, end, settings, now, (_, value) => apply(value));

        public IBattlementCommandOperation? Color(
            Transform lifetime,
            UnityEngine.Color start,
            UnityEngine.Color end,
            Tween settings,
            Action<UnityEngine.Color> apply
        ) => adapter.Color(lifetime, start, end, settings, now, apply);
    }
}

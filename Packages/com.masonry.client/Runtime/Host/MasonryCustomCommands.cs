#nullable enable

using System;
using System.Collections.Generic;
using System.Threading;
using MessagePack;
using MessagePack.Formatters;
using UnityEngine;

namespace Masonry
{
    /// <summary>Looks up Masonry-controlled objects by their protocol identity.</summary>
    public interface IMasonryObjectLookup
    {
        /// <summary>Returns the live Unity object for an identity when it exists.</summary>
        bool TryGetObject(ObjectId id, out GameObject? gameObject);
    }

    /// <summary>Creates Masonry-owned tween operations for custom handlers.</summary>
    public interface IMasonryTweenHelpers
    {
        /// <summary>Interpolates a scalar value with Masonry tween semantics.</summary>
        IMasonryCommandOperation? Float(
            Transform lifetime,
            float start,
            float end,
            Tween settings,
            Action<float> apply
        );

        /// <summary>Interpolates a vector value with Masonry tween semantics.</summary>
        IMasonryCommandOperation? Vector(
            Transform lifetime,
            UnityEngine.Vector3 start,
            UnityEngine.Vector3 end,
            Tween settings,
            Action<UnityEngine.Vector3> apply
        );

        /// <summary>Interpolates a color value with Masonry tween semantics.</summary>
        IMasonryCommandOperation? Color(
            Transform lifetime,
            UnityEngine.Color start,
            UnityEngine.Color end,
            Tween settings,
            Action<UnityEngine.Color> apply
        );
    }

    /// <summary>Services and cancellation owned by one custom-command invocation.</summary>
    public sealed record MasonryCommandContext(
        CancellationToken Cancellation,
        IMasonryLogger Logger,
        IMasonryObjectLookup Objects,
        IMasonryPreparedAssetLookup PreparedAssets,
        IMasonryTweenHelpers Tweens
    )
    {
        /// <summary>
        /// Marks tracked work as targeting an object so destruction can cancel it.
        /// </summary>
        public IMasonryCommandOperation ForObject(
            ObjectId objectId,
            IMasonryCommandOperation operation,
            bool controlsTransform = false
        ) =>
            new MasonryScopedCommandOperation(
                objectId,
                Errors.CheckNotNull(operation, nameof(operation)),
                controlsTransform
            );
    }

    /// <summary>Runs a trusted, explicitly registered game command.</summary>
    public interface IMasonryCommandHandler<TPayload>
    {
        /// <summary>Runs the command and optionally returns work Masonry should track.</summary>
        IMasonryCommandOperation? Execute(
            CustomCommand<TPayload> command,
            MasonryCommandContext context
        );
    }

    /// <summary>A protocol-visible game-specific custom-command failure.</summary>
    public sealed class MasonryCommandFailureException<TError> : Exception
    {
        /// <summary>Creates a failure with a game-owned stable error code.</summary>
        public MasonryCommandFailureException(
            TError errorCode,
            string message,
            Exception? innerException = null
        )
            : base(message, innerException) => ErrorCode = errorCode;

        public TError ErrorCode { get; }
    }

    internal sealed class MasonryCustomCommands
    {
        internal static readonly MessagePackSerializerOptions Options =
            MessagePackSerializerOptions.Standard.WithSecurity(MessagePackSecurity.UntrustedData);

        private readonly Dictionary<string, IMasonryCommandRegistration> registrations = new(
            StringComparer.Ordinal
        );
        private readonly Func<TimeSpan, MasonryCommandContext> createContext;

        public MasonryCustomCommands(Func<TimeSpan, MasonryCommandContext> createContext) =>
            this.createContext = createContext;

        public IReadOnlyCollection<string> Types => registrations.Keys;

        public void Register<TPayload, TError>(
            string type,
            IMasonryCommandHandler<TPayload> handler,
            IMessagePackFormatter<TPayload> payloadFormatter,
            IMessagePackFormatter<TError> errorFormatter
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
                new MasonryCommandRegistration<TPayload, TError>(
                    type,
                    Errors.CheckNotNull(handler, nameof(handler)),
                    Errors.CheckNotNull(payloadFormatter, nameof(payloadFormatter)),
                    Errors.CheckNotNull(errorFormatter, nameof(errorFormatter)),
                    createContext
                )
            );
        }

        public ICommand Read(
            CommandId id,
            string type,
            bool isBlocking,
            ReadOnlyMemory<byte> payload
        )
        {
            if (!registrations.TryGetValue(type, out IMasonryCommandRegistration registration))
            {
                return new MasonryUnknownCustomCommand(id, type, isBlocking);
            }

            try
            {
                return registration.Deserialize(id, isBlocking, payload);
            }
            catch (Exception exception)
            {
                return new MasonryInvalidCustomCommand(id, type, isBlocking, exception);
            }
        }

        public IMasonryCommandOperation? Launch(ICommand command, TimeSpan now)
        {
            switch (command)
            {
                case MasonryUnknownCustomCommand unknown:
                    throw new MasonryCommandException(
                        CoreErrorCode.HandlerNotRegistered,
                        $"No custom command handler is registered for {unknown.Type}."
                    );
                case MasonryInvalidCustomCommand invalid:
                    throw new MasonryCommandException(
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
                ?? throw new MasonryCommandException(
                    CoreErrorCode.HandlerNotRegistered,
                    "The custom command did not expose a command type."
                );
            if (!registrations.TryGetValue(type, out IMasonryCommandRegistration registration))
            {
                throw new MasonryCommandException(
                    CoreErrorCode.HandlerNotRegistered,
                    $"No custom command handler is registered for {type}."
                );
            }

            return registration.Launch(command, now);
        }

        public bool TryGet(string type, out IMasonryCommandRegistration registration) =>
            registrations.TryGetValue(type, out registration!);

        public static void RequireNamespaced(string type)
        {
            string value = type ?? string.Empty;
            bool invalidOwner =
                string.IsNullOrWhiteSpace(value)
                || value.StartsWith("masonry.", StringComparison.Ordinal);
            bool invalidSeparator =
                value.IndexOf('.') <= 0 || value.EndsWith(".", StringComparison.Ordinal);
            if (invalidOwner || invalidSeparator)
            {
                throw new ArgumentException(
                    "A custom command type must be a non-Masonry namespaced string.",
                    nameof(type)
                );
            }
        }
    }

    internal interface IMasonryCommandRegistration
    {
        ICommand Deserialize(CommandId id, bool isBlocking, ReadOnlyMemory<byte> payload);

        IMasonryCommandOperation? Launch(ICommand command, TimeSpan now);

        byte[] SerializeBatchFailure(
            IMasonryExtensionProtocolCodec codec,
            SessionId sessionId,
            BatchId batchId,
            CommandId? commandId,
            object errorCode,
            string message
        );

        byte[] SerializeOperationFailure(
            IMasonryExtensionProtocolCodec codec,
            SessionId sessionId,
            BatchId batchId,
            CommandId commandId,
            object errorCode,
            string message
        );
    }

    internal sealed class MasonryCommandRegistration<TPayload, TError> : IMasonryCommandRegistration
    {
        private readonly string type;
        private readonly IMasonryCommandHandler<TPayload> handler;
        private readonly IMessagePackFormatter<TPayload> payloadFormatter;
        private readonly IMessagePackFormatter<TError> errorFormatter;
        private readonly Func<TimeSpan, MasonryCommandContext> createContext;

        public MasonryCommandRegistration(
            string type,
            IMasonryCommandHandler<TPayload> handler,
            IMessagePackFormatter<TPayload> payloadFormatter,
            IMessagePackFormatter<TError> errorFormatter,
            Func<TimeSpan, MasonryCommandContext> createContext
        ) =>
            (
                this.type,
                this.handler,
                this.payloadFormatter,
                this.errorFormatter,
                this.createContext
            ) = (type, handler, payloadFormatter, errorFormatter, createContext);

        public ICommand Deserialize(CommandId id, bool isBlocking, ReadOnlyMemory<byte> payload)
        {
            var reader = new MessagePackReader(payload);
            TPayload value = payloadFormatter.Deserialize(
                ref reader,
                MasonryCustomCommands.Options
            );
            if (!reader.End)
            {
                throw new MessagePackSerializationException(
                    "A custom command payload must contain one MessagePack value."
                );
            }

            return new CustomCommand<TPayload>(id, type, value, isBlocking);
        }

        public IMasonryCommandOperation? Launch(ICommand command, TimeSpan now)
        {
            var typed =
                command as CustomCommand<TPayload>
                ?? throw new MasonryCommandException(
                    CoreErrorCode.InvalidEncoding,
                    $"Custom command {type} used the wrong payload type."
                );
            var cancellation = new CancellationTokenSource();
            try
            {
                IMasonryCommandOperation? operation = handler.Execute(
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

                var custom = new MasonryCustomOperation<TError>(operation, cancellation, this);
                return operation is IMasonryScopedCommandOperation scoped
                    ? new MasonryScopedCommandOperation(
                        scoped.TargetObjectId,
                        custom,
                        scoped.ControlsTransform
                    )
                    : custom;
            }
            catch (MasonryCommandFailureException<TError> exception)
            {
                cancellation.Dispose();
                throw new MasonryRegisteredCommandException(
                    this,
                    exception.ErrorCode!,
                    exception.Message,
                    exception
                );
            }
            catch (Exception exception)
            {
                cancellation.Dispose();
                throw new MasonryCommandException(
                    CoreErrorCode.HandlerFailed,
                    exception.Message,
                    exception
                );
            }
        }

        public byte[] SerializeBatchFailure(
            IMasonryExtensionProtocolCodec codec,
            SessionId sessionId,
            BatchId batchId,
            CommandId? commandId,
            object errorCode,
            string message
        ) =>
            codec.SerializeBatchFailure(
                new BatchFailed<TError>(sessionId, batchId, (TError)errorCode, message, commandId),
                errorFormatter
            );

        public byte[] SerializeOperationFailure(
            IMasonryExtensionProtocolCodec codec,
            SessionId sessionId,
            BatchId batchId,
            CommandId commandId,
            object errorCode,
            string message
        ) =>
            codec.SerializeOperationFailure(
                new OperationFailed<TError>(
                    sessionId,
                    batchId,
                    commandId,
                    (TError)errorCode,
                    message
                ),
                errorFormatter
            );
    }

    internal sealed class MasonryRegisteredCommandException : Exception
    {
        public MasonryRegisteredCommandException(
            IMasonryCommandRegistration registration,
            object errorCode,
            string message,
            Exception innerException
        )
            : base(message, innerException) =>
            (Registration, ErrorCode) = (registration, errorCode);

        public IMasonryCommandRegistration Registration { get; }

        public object ErrorCode { get; }
    }

    internal sealed record MasonryUnknownCustomCommand(CommandId Id, string Type, bool IsBlocking)
        : ICustomCommand;

    internal sealed record MasonryInvalidCustomCommand(
        CommandId Id,
        string Type,
        bool IsBlocking,
        Exception Error
    ) : ICustomCommand;

    internal sealed class MasonryCustomOperation<TError> : IMasonryCommandOperation
    {
        private readonly IMasonryCommandOperation operation;
        private readonly CancellationTokenSource cancellation;
        private readonly IMasonryCommandRegistration registration;
        private bool isFinished;

        public MasonryCustomOperation(
            IMasonryCommandOperation operation,
            CancellationTokenSource cancellation,
            IMasonryCommandRegistration registration
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
            catch (MasonryCommandFailureException<TError> exception)
            {
                Finish();
                throw new MasonryRegisteredCommandException(
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

    internal interface IMasonryScopedCommandOperation
    {
        ObjectId TargetObjectId { get; }

        bool ControlsTransform { get; }
    }

    internal sealed class MasonryScopedCommandOperation
        : IMasonryCommandOperation,
            IMasonryScopedCommandOperation
    {
        private readonly IMasonryCommandOperation operation;

        public MasonryScopedCommandOperation(
            ObjectId targetObjectId,
            IMasonryCommandOperation operation,
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

    internal sealed class MasonryTweenHelpers : IMasonryTweenHelpers
    {
        private readonly MasonryTweenAdapter adapter;
        private readonly TimeSpan now;

        public MasonryTweenHelpers(MasonryTweenAdapter adapter, TimeSpan now) =>
            (this.adapter, this.now) = (adapter, now);

        public IMasonryCommandOperation? Float(
            Transform lifetime,
            float start,
            float end,
            Tween settings,
            Action<float> apply
        ) => adapter.Float(lifetime, start, end, settings, now, apply);

        public IMasonryCommandOperation? Vector(
            Transform lifetime,
            UnityEngine.Vector3 start,
            UnityEngine.Vector3 end,
            Tween settings,
            Action<UnityEngine.Vector3> apply
        ) => adapter.Vector(lifetime, start, end, settings, now, (_, value) => apply(value));

        public IMasonryCommandOperation? Color(
            Transform lifetime,
            UnityEngine.Color start,
            UnityEngine.Color end,
            Tween settings,
            Action<UnityEngine.Color> apply
        ) => adapter.Color(lifetime, start, end, settings, now, apply);
    }
}

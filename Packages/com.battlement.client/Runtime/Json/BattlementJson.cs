#nullable enable

using System;
using Newtonsoft.Json;

namespace Battlement
{
    /// <summary>Encodes and decodes Battlement protocol values as JSON.</summary>
    public sealed class BattlementJson : IBattlementExtensionProtocolCodec
    {
        public static BattlementJson Instance { get; } = new();

        private BattlementJson() { }

        /// <summary>Encodes a connection message.</summary>
        public static byte[] SerializeConnect(Connect value) => JsonProtocol.Serialize(value);

        /// <summary>Encodes one core batch-failure submission.</summary>
        public static byte[] SerializeBatchFailure(BatchFailed<CoreErrorCode> value) =>
            JsonProtocol.Serialize(
                new ClientMessage<CoreErrorCode, object>.BatchFailedMessage(value)
            );

        /// <summary>Encodes one core operation-failure submission.</summary>
        public static byte[] SerializeOperationFailure(OperationFailed<CoreErrorCode> value) =>
            JsonProtocol.Serialize(
                new ClientMessage<CoreErrorCode, object>.OperationFailedMessage(value)
            );

        /// <summary>Encodes one built-in pointer or keyboard action.</summary>
        public static byte[] SerializeAction(Action value) =>
            JsonProtocol.Serialize(new ClientMessage<CoreErrorCode, object>.ActionMessage(value));

        /// <summary>Decodes a connection message.</summary>
        public static Connect DeserializeConnect(ReadOnlyMemory<byte> bytes) =>
            JsonProtocol.Deserialize<Connect>(bytes);

        /// <summary>Decodes one value with an optional game-owned converter.</summary>
        public static T Deserialize<T>(
            ReadOnlyMemory<byte> bytes,
            JsonConverter<T>? converter = null
        ) =>
            converter is null
                ? JsonProtocol.Deserialize<T>(bytes)
                : JsonProtocol.Deserialize<T>(bytes, converter);

        /// <summary>Encodes a response containing only core commands.</summary>
        public static byte[] SerializeResponse(Response value) => JsonProtocol.Serialize(value);

        /// <summary>Decodes a response containing only core commands.</summary>
        public static Response DeserializeResponse(ReadOnlyMemory<byte> bytes) =>
            JsonProtocol.Deserialize<Response>(bytes);

        byte[] IBattlementProtocolCodec.SerializeConnect(Connect value) => SerializeConnect(value);

        byte[] IBattlementProtocolCodec.SerializeBatchFailure(BatchFailed<CoreErrorCode> value) =>
            SerializeBatchFailure(value);

        byte[] IBattlementProtocolCodec.SerializeOperationFailure(
            OperationFailed<CoreErrorCode> value
        ) => SerializeOperationFailure(value);

        byte[] IBattlementProtocolCodec.SerializeAction(Action value) => SerializeAction(value);

        Response IBattlementProtocolCodec.DeserializeResponse(ReadOnlyMemory<byte> bytes) =>
            DeserializeResponse(bytes);

        /// <summary>Encodes a response containing core and custom commands.</summary>
        public static byte[] SerializeResponse<TPayload>(
            Response<ICommand> value,
            JsonConverter<TPayload>? payloadConverter = null
        ) =>
            payloadConverter is null
                ? JsonProtocol.Serialize(value)
                : JsonProtocol.Serialize(value, payloadConverter);

        /// <summary>Encodes a typed client message with optional game converters.</summary>
        public static byte[] SerializeClientMessage<TError, TPayload>(
            ClientMessage<TError, TPayload> value,
            JsonConverter<TError>? errorConverter = null,
            JsonConverter<TPayload>? payloadConverter = null
        )
        {
            if (errorConverter is null && payloadConverter is null)
            {
                return JsonProtocol.Serialize(value);
            }

            if (errorConverter is null)
            {
                return JsonProtocol.Serialize(value, payloadConverter!);
            }

            if (payloadConverter is null)
            {
                return JsonProtocol.Serialize(value, errorConverter);
            }

            return JsonProtocol.Serialize(value, errorConverter, payloadConverter);
        }

        /// <summary>Decodes a typed client message with optional game converters.</summary>
        public static ClientMessage<TError, TPayload> DeserializeClientMessage<TError, TPayload>(
            ReadOnlyMemory<byte> bytes,
            JsonConverter<TError>? errorConverter = null,
            JsonConverter<TPayload>? payloadConverter = null
        )
        {
            if (errorConverter is null && payloadConverter is null)
            {
                return JsonProtocol.Deserialize<ClientMessage<TError, TPayload>>(bytes);
            }

            if (errorConverter is null)
            {
                return JsonProtocol.Deserialize<ClientMessage<TError, TPayload>>(
                    bytes,
                    payloadConverter!
                );
            }

            if (payloadConverter is null)
            {
                return JsonProtocol.Deserialize<ClientMessage<TError, TPayload>>(
                    bytes,
                    errorConverter
                );
            }

            return JsonProtocol.Deserialize<ClientMessage<TError, TPayload>>(
                bytes,
                errorConverter,
                payloadConverter
            );
        }

        /// <summary>Encodes one typed custom action.</summary>
        public static byte[] SerializeCustomAction<TPayload>(
            CustomAction<TPayload> value,
            JsonConverter<TPayload>? payloadConverter = null
        ) =>
            payloadConverter is null
                ? JsonProtocol.Serialize(
                    new ClientMessage<object, TPayload>.CustomActionMessage(value)
                )
                : JsonProtocol.Serialize(
                    new ClientMessage<object, TPayload>.CustomActionMessage(value),
                    payloadConverter
                );

        /// <summary>Encodes one game-specific batch failure.</summary>
        public static byte[] SerializeBatchFailure<TError>(
            BatchFailed<TError> value,
            JsonConverter<TError>? errorConverter = null
        ) =>
            errorConverter is null
                ? JsonProtocol.Serialize(
                    new ClientMessage<TError, object>.BatchFailedMessage(value)
                )
                : JsonProtocol.Serialize(
                    new ClientMessage<TError, object>.BatchFailedMessage(value),
                    errorConverter
                );

        /// <summary>Encodes one game-specific late operation failure.</summary>
        public static byte[] SerializeOperationFailure<TError>(
            OperationFailed<TError> value,
            JsonConverter<TError>? errorConverter = null
        ) =>
            errorConverter is null
                ? JsonProtocol.Serialize(
                    new ClientMessage<TError, object>.OperationFailedMessage(value)
                )
                : JsonProtocol.Serialize(
                    new ClientMessage<TError, object>.OperationFailedMessage(value),
                    errorConverter
                );

        public static Response<ICommand> DeserializeResponse(
            ReadOnlyMemory<byte> bytes,
            Func<CommandId, string, bool, ReadOnlyMemory<byte>, ICommand> decodeCustomCommand
        ) =>
            JsonProtocol.Deserialize<Response<ICommand>>(
                bytes,
                new CustomCommandJsonConverter(decodeCustomCommand)
            );

        Response<ICommand> IBattlementExtensionProtocolCodec.DeserializeResponse(
            ReadOnlyMemory<byte> bytes,
            Func<CommandId, string, bool, ReadOnlyMemory<byte>, ICommand> decodeCustomCommand
        ) => DeserializeResponse(bytes, decodeCustomCommand);

        byte[] IBattlementExtensionProtocolCodec.SerializeCustomAction<TPayload>(
            CustomAction<TPayload> value,
            JsonConverter<TPayload>? payloadConverter
        ) => SerializeCustomAction(value, payloadConverter);

        byte[] IBattlementExtensionProtocolCodec.SerializeBatchFailure<TError>(
            BatchFailed<TError> value,
            JsonConverter<TError>? errorConverter
        ) => SerializeBatchFailure(value, errorConverter);

        byte[] IBattlementExtensionProtocolCodec.SerializeOperationFailure<TError>(
            OperationFailed<TError> value,
            JsonConverter<TError>? errorConverter
        ) => SerializeOperationFailure(value, errorConverter);
    }
}

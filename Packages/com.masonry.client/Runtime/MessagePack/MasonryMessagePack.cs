#nullable enable

using System;
using System.Buffers;
using MessagePack;
using MessagePack.Formatters;

namespace Masonry
{
    /// <summary>Encodes and decodes Masonry protocol values as MessagePack.</summary>
    public sealed class MasonryMessagePack : IMasonryExtensionProtocolCodec
    {
        internal static readonly MessagePackSerializerOptions SerializerOptions =
            MessagePackSerializerOptions.Standard.WithSecurity(MessagePackSecurity.UntrustedData);

        public static MasonryMessagePack Instance { get; } = new();

        private MasonryMessagePack() { }

        /// <summary>Encodes a connection message.</summary>
        public static byte[] SerializeConnect(Connect value) =>
            Serialize(
                (ref MessagePackWriter writer) => ProtocolFormat.WriteConnect(ref writer, value)
            );

        /// <summary>Encodes one core batch-failure submission.</summary>
        public static byte[] SerializeBatchFailure(BatchFailed<CoreErrorCode> value) =>
            Serialize(
                (ref MessagePackWriter writer) =>
                    ProtocolFormat.WriteBatchFailureClientMessage(ref writer, value)
            );

        /// <summary>Encodes one core operation-failure submission.</summary>
        public static byte[] SerializeOperationFailure(OperationFailed<CoreErrorCode> value) =>
            Serialize(
                (ref MessagePackWriter writer) =>
                    ProtocolFormat.WriteOperationFailureClientMessage(ref writer, value)
            );

        /// <summary>Encodes one built-in pointer or keyboard action.</summary>
        public static byte[] SerializeAction(Action value) =>
            Serialize(
                (ref MessagePackWriter writer) =>
                    ProtocolFormat.WriteActionClientMessage(ref writer, value)
            );

        /// <summary>Decodes a connection message.</summary>
        public static Connect DeserializeConnect(ReadOnlyMemory<byte> bytes) =>
            Deserialize(bytes, ProtocolFormat.ReadConnect);

        /// <summary>Encodes a response containing only core commands.</summary>
        public static byte[] SerializeResponse(Response value) =>
            Serialize(
                (ref MessagePackWriter writer) => ProtocolFormat.WriteResponse(ref writer, value)
            );

        /// <summary>Decodes a response containing only core commands.</summary>
        public static Response DeserializeResponse(ReadOnlyMemory<byte> bytes) =>
            Deserialize(bytes, ProtocolFormat.ReadResponse);

        byte[] IMasonryProtocolCodec.SerializeConnect(Connect value) => SerializeConnect(value);

        byte[] IMasonryProtocolCodec.SerializeBatchFailure(BatchFailed<CoreErrorCode> value) =>
            SerializeBatchFailure(value);

        byte[] IMasonryProtocolCodec.SerializeOperationFailure(
            OperationFailed<CoreErrorCode> value
        ) => SerializeOperationFailure(value);

        byte[] IMasonryProtocolCodec.SerializeAction(Action value) => SerializeAction(value);

        Response IMasonryProtocolCodec.DeserializeResponse(ReadOnlyMemory<byte> bytes) =>
            DeserializeResponse(bytes);

        /// <summary>Encodes a response containing core and custom commands.</summary>
        public static byte[] SerializeResponse<TPayload>(
            Response<ICommand> value,
            IMessagePackFormatter<TPayload> payloadFormatter
        ) =>
            Serialize(
                (ref MessagePackWriter writer) =>
                    ProtocolFormat.WriteResponse(
                        ref writer,
                        value,
                        payloadFormatter,
                        SerializerOptions
                    )
            );

        /// <summary>Decodes a response containing core and custom commands.</summary>
        public static Response<ICommand> DeserializeResponse<TPayload>(
            ReadOnlyMemory<byte> bytes,
            IMessagePackFormatter<TPayload> payloadFormatter
        ) =>
            Deserialize(
                bytes,
                (ref MessagePackReader reader) =>
                    ProtocolFormat.ReadResponse(ref reader, payloadFormatter, SerializerOptions)
            );

        /// <summary>Encodes a client message with typed game extensions.</summary>
        public static byte[] SerializeClientMessage<TError, TCustomActionPayload>(
            ClientMessage<TError, TCustomActionPayload> value,
            IMessagePackFormatter<TError> errorFormatter,
            IMessagePackFormatter<TCustomActionPayload> payloadFormatter
        ) =>
            Serialize(
                (ref MessagePackWriter writer) =>
                    ProtocolFormat.WriteClientMessage(
                        ref writer,
                        value,
                        errorFormatter,
                        payloadFormatter,
                        SerializerOptions
                    )
            );

        /// <summary>Decodes a client message with typed game extensions.</summary>
        public static ClientMessage<TError, TCustomActionPayload> DeserializeClientMessage<
            TError,
            TCustomActionPayload
        >(
            ReadOnlyMemory<byte> bytes,
            IMessagePackFormatter<TError> errorFormatter,
            IMessagePackFormatter<TCustomActionPayload> payloadFormatter
        ) =>
            Deserialize(
                bytes,
                (ref MessagePackReader reader) =>
                    ProtocolFormat.ReadClientMessage(
                        ref reader,
                        errorFormatter,
                        payloadFormatter,
                        SerializerOptions
                    )
            );

        /// <summary>Encodes one typed custom action.</summary>
        public static byte[] SerializeCustomAction<TPayload>(
            CustomAction<TPayload> value,
            IMessagePackFormatter<TPayload> payloadFormatter
        ) =>
            Serialize(
                (ref MessagePackWriter writer) =>
                    ProtocolFormat.WriteCustomActionClientMessage(
                        ref writer,
                        value,
                        payloadFormatter,
                        SerializerOptions
                    )
            );

        /// <summary>Encodes one game-specific batch failure.</summary>
        public static byte[] SerializeBatchFailure<TError>(
            BatchFailed<TError> value,
            IMessagePackFormatter<TError> errorFormatter
        ) =>
            Serialize(
                (ref MessagePackWriter writer) =>
                    ProtocolFormat.WriteBatchFailureClientMessage(
                        ref writer,
                        value,
                        errorFormatter,
                        SerializerOptions
                    )
            );

        /// <summary>Encodes one game-specific late operation failure.</summary>
        public static byte[] SerializeOperationFailure<TError>(
            OperationFailed<TError> value,
            IMessagePackFormatter<TError> errorFormatter
        ) =>
            Serialize(
                (ref MessagePackWriter writer) =>
                    ProtocolFormat.WriteOperationFailureClientMessage(
                        ref writer,
                        value,
                        errorFormatter,
                        SerializerOptions
                    )
            );

        internal static Response<ICommand> DeserializeResponse(
            ReadOnlyMemory<byte> bytes,
            Func<CommandId, string, bool, ReadOnlyMemory<byte>, ICommand> decodeCustomCommand
        ) =>
            Deserialize(
                bytes,
                (ref MessagePackReader reader) =>
                    ProtocolFormat.ReadResponse(ref reader, decodeCustomCommand)
            );

        Response<ICommand> IMasonryExtensionProtocolCodec.DeserializeResponse(
            ReadOnlyMemory<byte> bytes,
            Func<CommandId, string, bool, ReadOnlyMemory<byte>, ICommand> decodeCustomCommand
        ) => DeserializeResponse(bytes, decodeCustomCommand);

        byte[] IMasonryExtensionProtocolCodec.SerializeCustomAction<TPayload>(
            CustomAction<TPayload> value,
            IMessagePackFormatter<TPayload> payloadFormatter
        ) => SerializeCustomAction(value, payloadFormatter);

        byte[] IMasonryExtensionProtocolCodec.SerializeBatchFailure<TError>(
            BatchFailed<TError> value,
            IMessagePackFormatter<TError> errorFormatter
        ) => SerializeBatchFailure(value, errorFormatter);

        byte[] IMasonryExtensionProtocolCodec.SerializeOperationFailure<TError>(
            OperationFailed<TError> value,
            IMessagePackFormatter<TError> errorFormatter
        ) => SerializeOperationFailure(value, errorFormatter);

        private static byte[] Serialize(WriterAction action)
        {
            var buffer = new ArrayBufferWriter<byte>();
            var writer = new MessagePackWriter(buffer);
            action(ref writer);
            writer.Flush();
            return buffer.WrittenSpan.ToArray();
        }

        private static T Deserialize<T>(ReadOnlyMemory<byte> bytes, ReaderFunc<T> read)
        {
            try
            {
                var reader = new MessagePackReader(bytes);
                T value = read(ref reader);
                if (!reader.End)
                {
                    throw new MessagePackSerializationException(
                        "A Masonry buffer must contain exactly one MessagePack value."
                    );
                }

                return value;
            }
            catch (MessagePackSerializationException)
            {
                throw;
            }
            catch (Exception exception)
            {
                throw new MessagePackSerializationException(
                    "The buffer is not a valid Masonry MessagePack value.",
                    exception
                );
            }
        }

        private delegate void WriterAction(ref MessagePackWriter writer);

        private delegate T ReaderFunc<T>(ref MessagePackReader reader);
    }
}

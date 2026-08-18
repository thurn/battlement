#nullable enable

using System;
using System.Buffers;
using MessagePack;
using MessagePack.Formatters;

namespace Masonry
{
    /// <summary>Encodes and decodes Masonry protocol values as MessagePack.</summary>
    public static class MasonryMessagePack
    {
        private static readonly MessagePackSerializerOptions Options =
            MessagePackSerializerOptions.Standard.WithSecurity(MessagePackSecurity.UntrustedData);

        /// <summary>Encodes a connection message.</summary>
        public static byte[] SerializeConnect(Connect value) =>
            Serialize(
                (ref MessagePackWriter writer) => ProtocolFormat.WriteConnect(ref writer, value)
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

        /// <summary>Encodes a response containing core and custom commands.</summary>
        public static byte[] SerializeResponse<TPayload>(
            Response<ICommand> value,
            IMessagePackFormatter<TPayload> payloadFormatter
        ) =>
            Serialize(
                (ref MessagePackWriter writer) =>
                    ProtocolFormat.WriteResponse(ref writer, value, payloadFormatter, Options)
            );

        /// <summary>Decodes a response containing core and custom commands.</summary>
        public static Response<ICommand> DeserializeResponse<TPayload>(
            ReadOnlyMemory<byte> bytes,
            IMessagePackFormatter<TPayload> payloadFormatter
        ) =>
            Deserialize(
                bytes,
                (ref MessagePackReader reader) =>
                    ProtocolFormat.ReadResponse(ref reader, payloadFormatter, Options)
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
                        Options
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
                        Options
                    )
            );

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

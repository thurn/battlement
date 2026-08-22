#nullable enable

using System;
using System.Buffers;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using MessagePack;
using NUnit.Framework;

namespace Masonry.Tests
{
    public sealed class MessagePackInteropTests
    {
        private static readonly string FixtureDirectory = Path.Combine(
            "Packages",
            "com.masonry.client",
            "Tests",
            "Fixtures"
        );

        [Test]
        public void CsharpCorpusMatchesTheCurrentEncoder()
        {
            var fixtures = new Dictionary<string, byte[]>
            {
                ["csharp-connect.msgpack"] = MasonryMessagePack.SerializeConnect(
                    MessagePackFixtureData.Connect()
                ),
                ["csharp-response.msgpack"] = MasonryMessagePack.SerializeResponse(
                    MessagePackFixtureData.ComprehensiveResponse()
                ),
                ["csharp-custom-response.msgpack"] = MasonryMessagePack.SerializeResponse(
                    MessagePackFixtureData.CustomResponse(),
                    new MessagePackFixtureData.SamplePayloadFormatter()
                ),
            };
            foreach (
                KeyValuePair<string, byte[]> fixture in MessagePackFixtureData.ClientMessages()
            )
            {
                fixtures.Add(fixture.Key, fixture.Value);
            }

            foreach (KeyValuePair<string, byte[]> fixture in fixtures)
            {
                Assert.That(fixture.Value, Is.EqualTo(ReadFixture(fixture.Key)), fixture.Key);
            }
        }

        [Test]
        public void RustCoreCorpusDecodesEveryCommandAndReproducesTheBytes()
        {
            byte[] bytes = ReadFixture("rust-response.msgpack");
            Response response = MasonryMessagePack.DeserializeResponse(bytes);
            var batch = (ResponseMessage<Command>.BatchMessage)response.Messages[1];
            Command[] commands = batch.Batch.Groups.SelectMany(group => group.Commands).ToArray();

            Assert.That(
                response.Messages[0],
                Is.TypeOf<ResponseMessage<Command>.SnapshotMessage>()
            );
            Assert.That(commands, Has.Length.EqualTo(74));
            Assert.That(
                commands.Select(command => command.Body.GetType()).Distinct().Count(),
                Is.EqualTo(MessagePackFixtureData.ConcreteCommandTypes().Length)
            );
            Assert.That(MasonryMessagePack.SerializeResponse(response), Is.EqualTo(bytes));
        }

        [Test]
        public void RustCustomCorpusUsesTheSuppliedPayloadFormatter()
        {
            byte[] bytes = ReadFixture("rust-custom-response.msgpack");
            var formatter = new MessagePackFixtureData.SamplePayloadFormatter();
            Response<ICommand> response = MasonryMessagePack.DeserializeResponse(bytes, formatter);
            var batch = (ResponseMessage<ICommand>.BatchMessage)response.Messages[0];
            var command =
                (CustomCommand<MessagePackFixtureData.SamplePayload>)
                    batch.Batch.Groups[0].Commands[0];

            Assert.That(command.Type, Is.EqualTo("cards.reveal"));
            Assert.That(
                command.Payload,
                Is.EqualTo(new MessagePackFixtureData.SamplePayload("queen", 2))
            );
            Assert.That(
                MasonryMessagePack.SerializeResponse(response, formatter),
                Is.EqualTo(bytes)
            );
        }

        [Test]
        public void SameLanguageRoundTripsPreserveValues()
        {
            Connect connect = MessagePackFixtureData.Connect();
            Connect decoded = MasonryMessagePack.DeserializeConnect(
                MasonryMessagePack.SerializeConnect(connect)
            );

            Assert.That(decoded.Platform, Is.EqualTo(connect.Platform));
            Assert.That(decoded.UnityVersion, Is.EqualTo(connect.UnityVersion));
            Assert.That(decoded.Screen, Is.EqualTo(connect.Screen));
            Assert.That(decoded.CustomCommandTypes, Is.EqualTo(connect.CustomCommandTypes));
            Assert.That(
                MasonryMessagePack.SerializeConnect(decoded),
                Is.EqualTo(MasonryMessagePack.SerializeConnect(connect))
            );
        }

        [Test]
        public void MainCameraSnapshotRoundTripsWithoutAnObjectId()
        {
            Response fixture = MessagePackFixtureData.ComprehensiveResponse();
            var snapshotMessage = (ResponseMessage<Command>.SnapshotMessage)fixture.Messages[0];
            Response response = fixture with
            {
                Messages = new ResponseMessage<Command>[]
                {
                    new ResponseMessage<Command>.SnapshotMessage(
                        snapshotMessage.Snapshot with
                        {
                            InputCameraId = null,
                        }
                    ),
                },
            };

            Response decoded = MasonryMessagePack.DeserializeResponse(
                MasonryMessagePack.SerializeResponse(response)
            );
            var decodedSnapshot = (ResponseMessage<Command>.SnapshotMessage)decoded.Messages[0];

            Assert.That(decodedSnapshot.Snapshot.InputCameraId, Is.Null);
        }

        [Test]
        public void MalformedMessagesAreRejected()
        {
            byte[] connect = ReadFixture("csharp-connect.msgpack");
            byte[] truncated = connect.Take(connect.Length - 1).ToArray();
            byte[] trailing = connect.Concat(new byte[] { 0xc0 }).ToArray();
            byte[] wrongLength = { 0x95 };
            byte[] overflow = InvalidConnectWithOverflowingWidth();
            byte[] invalidUuid = ReadFixture("rust-response.msgpack");
            Array.Clear(invalidUuid, 3, 16);
            byte[] unknownVariant = ReadFixture("rust-response.msgpack");
            ReplaceAscii(unknownVariant, "Snapshot", "Snapshox");
            byte[] excessiveNesting = Enumerable
                .Repeat((byte)0x91, 129)
                .Append((byte)0xc0)
                .ToArray();

            Assert.Throws<MessagePackSerializationException>(() =>
                MasonryMessagePack.DeserializeConnect(truncated)
            );
            Assert.Throws<MessagePackSerializationException>(() =>
                MasonryMessagePack.DeserializeConnect(trailing)
            );
            Assert.Throws<MessagePackSerializationException>(() =>
                MasonryMessagePack.DeserializeConnect(wrongLength)
            );
            Assert.Throws<MessagePackSerializationException>(() =>
                MasonryMessagePack.DeserializeConnect(overflow)
            );
            Assert.Throws<MessagePackSerializationException>(() =>
                MasonryMessagePack.DeserializeResponse(invalidUuid)
            );
            Assert.Throws<MessagePackSerializationException>(() =>
                MasonryMessagePack.DeserializeResponse(unknownVariant)
            );
            Assert.Throws<MessagePackSerializationException>(() =>
                MasonryMessagePack.DeserializeConnect(excessiveNesting)
            );
            Assert.Throws<MessagePackSerializationException>(() =>
                MasonryMessagePack.SerializeResponse(
                    new Response(new SessionId(Guid.Empty), Array.Empty<ResponseMessage<Command>>())
                )
            );
        }

        private static byte[] InvalidConnectWithOverflowingWidth()
        {
            var buffer = new ArrayBufferWriter<byte>();
            var writer = new MessagePackWriter(buffer);
            writer.WriteArrayHeader(6);
            writer.Write("platform");
            writer.Write("version");
            writer.WriteArrayHeader(2);
            writer.Write(ulong.MaxValue);
            writer.Write(1u);
            writer.WriteArrayHeader(0);
            writer.WriteNil();
            writer.WriteNil();
            writer.Flush();
            return buffer.WrittenSpan.ToArray();
        }

        private static byte[] ReadFixture(string name) =>
            File.ReadAllBytes(Path.Combine(FixtureDirectory, name));

        private static void ReplaceAscii(byte[] bytes, string value, string replacement)
        {
            byte[] search = System.Text.Encoding.ASCII.GetBytes(value);
            int offset = bytes.AsSpan().IndexOf(search);
            Assert.That(offset, Is.GreaterThanOrEqualTo(0));
            System.Text.Encoding.ASCII.GetBytes(replacement).CopyTo(bytes, offset);
        }
    }
}

#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;

namespace Masonry.Tests
{
    public sealed class MasonryResponseProcessingTests
    {
        private const int MaximumResponseBytes = 16 * 1024 * 1024;

        [Test]
        public void ConnectAndSubmitParseSynchronouslyOnTheCallingThread()
        {
            var codec = new RecordingCodec();
            using MasonryTestHarness harness = MasonryTestHarness.Create(protocolCodec: codec);
            int callingThread = Environment.CurrentManagedThreadId;
            byte[] submitted = { 1, 2, 3, 4 };
            SessionId session = new(Guid.NewGuid());

            harness.Transport.EnqueueConnect(
                new MasonryTransportResult(MasonryTransportStatus.Success, new byte[17])
            );
            codec.EnqueueResponse(Response(session, inputDisabled: false));
            harness.Runner.Connect();

            harness.Transport.EnqueueSubmit(
                new MasonryTransportResult(MasonryTransportStatus.Success, new byte[31])
            );
            codec.EnqueueResponse(Response(session, inputDisabled: true));
            harness.Runner.Submit(submitted);

            Assert.That(codec.PayloadSizes, Is.EqualTo(new[] { 17, 31 }));
            Assert.That(codec.ThreadIds, Is.All.EqualTo(callingThread));
            Assert.That(harness.Transport.SubmitMessages.Single(), Is.EqualTo(submitted));
            Assert.That(harness.Runner.IsInputAvailable, Is.False);
        }

        [Test]
        public void ResponseAtTheLimitParsesButLargerResponseStopsBeforeParsing()
        {
            var codec = new RecordingCodec();
            using MasonryTestHarness harness = MasonryTestHarness.Create(protocolCodec: codec);
            codec.EnqueueResponse(Response(new SessionId(Guid.NewGuid()), inputDisabled: false));
            harness.Transport.EnqueueConnect(
                new MasonryTransportResult(
                    MasonryTransportStatus.Success,
                    new byte[MaximumResponseBytes]
                )
            );

            harness.Runner.Connect();
            Assert.That(codec.PayloadSizes, Is.EqualTo(new[] { MaximumResponseBytes }));
            Assert.That(harness.Runner.IsInputAvailable, Is.True);

            harness.Transport.EnqueueSubmit(
                new MasonryTransportResult(
                    MasonryTransportStatus.Success,
                    new byte[MaximumResponseBytes + 1]
                )
            );
            harness.Runner.Submit(new byte[] { 1 });

            Assert.That(codec.PayloadSizes, Has.Count.EqualTo(1));
            Assert.That(harness.Runner.IsInputAvailable, Is.False);
            Assert.That(
                harness.Transport.Calls.TakeLast(2),
                Is.EqualTo(new[] { "submit", "stop" })
            );
            Assert.That(
                harness.Logger.Records.Last().EventName,
                Is.EqualTo("masonry.session.failed")
            );
        }

        [Test]
        public void NestedSubmitParsesImmediatelyButAppliesAfterTheOuterResponse()
        {
            var codec = new RecordingCodec();
            using MasonryTestHarness harness = MasonryTestHarness.Create(protocolCodec: codec);
            SessionId session = new(Guid.NewGuid());
            codec.EnqueueResponse(Response(session, inputDisabled: false));
            harness.Runner.Connect();

            codec.EnqueueResponse(Response(session, inputDisabled: true));
            codec.EnqueueResponse(Response(session, inputDisabled: false));
            harness.Transport.EnqueueSubmit(
                new MasonryTransportResult(MasonryTransportStatus.Success, new byte[] { 10 })
            );
            harness.Transport.EnqueueSubmit(
                new MasonryTransportResult(MasonryTransportStatus.Success, new byte[] { 20 })
            );
            codec.BeforeSecondDecode = () =>
            {
                harness.Runner.Submit(new byte[] { 2 });
                Assert.That(
                    harness.Runner.IsInputAvailable,
                    Is.True,
                    "A nested return must not apply while the outer response is parsing."
                );
            };

            harness.Runner.Submit(new byte[] { 1 });

            Assert.That(codec.PayloadSizes, Has.Count.EqualTo(3));
            Assert.That(codec.PayloadSizes.TakeLast(2), Is.EqualTo(new[] { 1, 1 }));
            Assert.That(
                harness.Transport.Calls.TakeLast(2),
                Is.EqualTo(new[] { "submit", "submit" })
            );
            Assert.That(
                harness.Runner.IsInputAvailable,
                Is.True,
                "FIFO draining should apply the outer disabled snapshot before the "
                    + "nested enabled snapshot."
            );
        }

        private static Response Response(SessionId session, bool inputDisabled)
        {
            Snapshot snapshot = FakeMasonryTransport.CompleteSnapshot(
                session,
                inputDisabled: inputDisabled
            );
            return new Response(
                session,
                new ResponseMessage<Command>[]
                {
                    new ResponseMessage<Command>.SnapshotMessage(snapshot),
                }
            );
        }

        private sealed class RecordingCodec : IMasonryProtocolCodec
        {
            private readonly Queue<Response> responses = new();
            private int decodeCount;

            public List<int> PayloadSizes { get; } = new();

            public List<int> ThreadIds { get; } = new();

            public System.Action? BeforeSecondDecode { get; set; }

            public void EnqueueResponse(Response response) => responses.Enqueue(response);

            public byte[] SerializeConnect(Connect value) => Array.Empty<byte>();

            public byte[] SerializeBatchFailure(BatchFailed<CoreErrorCode> value) =>
                new byte[] { 30 };

            public byte[] SerializeOperationFailure(OperationFailed<CoreErrorCode> value) =>
                new byte[] { 40 };

            public Response DeserializeResponse(ReadOnlyMemory<byte> bytes)
            {
                decodeCount++;
                PayloadSizes.Add(bytes.Length);
                ThreadIds.Add(Environment.CurrentManagedThreadId);
                Response response = responses.Dequeue();
                if (decodeCount == 2)
                {
                    BeforeSecondDecode?.Invoke();
                }

                return response;
            }
        }
    }
}

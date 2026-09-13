#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class BattlementResponseProcessingTests
    {
        private const int MaximumResponseBytes = 16 * 1024 * 1024;

        [Test]
        public void DecodeRunsSynchronouslyInAdmissionOrderOnTheCallingThread()
        {
            var stream = new BattlementResponseStream();
            var decoded = new List<int>();
            int callingThread = Environment.CurrentManagedThreadId;
            var decodeThreads = new List<int>();
            SessionId session = new(Guid.NewGuid());
            Response response = new(session, Array.Empty<ResponseMessage<Command>>());
            ReadOnlyMemory<byte> payload = BattlementFlatBufferResponseFixtures.Write(response);

            BattlementResponseStream.Reservation first = stream.Reserve(
                (_, owner) =>
                {
                    decodeThreads.Add(Environment.CurrentManagedThreadId);
                    return new BattlementFlatBufferResponse(payload, owner);
                },
                decoded: _ => decoded.Add(1)
            );
            BattlementResponseStream.Reservation second = stream.Reserve(
                (_, owner) =>
                {
                    decodeThreads.Add(Environment.CurrentManagedThreadId);
                    return new BattlementFlatBufferResponse(payload, owner);
                },
                decoded: _ => decoded.Add(2)
            );
            first.Commit(payload);
            second.Commit(payload);
            Drain(stream);
            Assert.That(decoded, Is.EqualTo(new[] { 1, 2 }));
            Assert.That(decodeThreads, Is.All.EqualTo(callingThread));
        }

        [Test]
        public void ResponsePayloadOwnerIsReleasedOnRetirementClearAndRejectedCommit()
        {
            SessionId session = new(Guid.NewGuid());
            Response response = new(session, Array.Empty<ResponseMessage<Command>>());
            ReadOnlyMemory<byte> payload = BattlementFlatBufferResponseFixtures.Write(response);
            var stream = new BattlementResponseStream();
            var retired = new TrackingDisposable();
            BattlementResponseStream.Reservation first = stream.Reserve(
                (bytes, owner) => new BattlementFlatBufferResponse(bytes, owner)
            );
            first.Commit(payload, retired);
            Drain(stream);
            Assert.That(retired.IsDisposed, Is.True);

            var cleared = new TrackingDisposable();
            BattlementResponseStream.Reservation second = stream.Reserve(
                (bytes, owner) => new BattlementFlatBufferResponse(bytes, owner)
            );
            second.Commit(payload, cleared);
            stream.Clear();
            Assert.That(cleared.IsDisposed, Is.True);

            var decodeFailed = new TrackingDisposable();
            BattlementResponseStream.Reservation failed = stream.Reserve(
                (_, owner) =>
                {
                    owner?.Dispose();
                    throw new InvalidOperationException("decode failed");
                }
            );
            failed.Commit(new byte[] { 3 }, decodeFailed);
            Assert.Throws<InvalidOperationException>(() => Drain(stream));
            Assert.That(decodeFailed.IsDisposed, Is.True);

            var rejected = new TrackingDisposable();
            BattlementResponseStream.Reservation third = stream.Reserve(
                (bytes, owner) => new BattlementFlatBufferResponse(bytes, owner)
            );
            Assert.Throws<System.IO.InvalidDataException>(() =>
                third.Commit(new byte[MaximumResponseBytes + 1], rejected)
            );
            Assert.That(rejected.IsDisposed, Is.True);
            third.Release();
        }

        [Test]
        public void OutboundPayloadLimitStopsBeforeCallingTheTransport()
        {
            using BattlementTestHarness connectHarness = BattlementTestHarness.Create(
                customCommandTypes: new[] { new string('x', MaximumResponseBytes) }
            );

            connectHarness.Runner.Connect();

            Assert.That(connectHarness.Transport.Calls, Is.EqualTo(new[] { "stop" }));
            Assert.That(
                connectHarness.Logger.Records.Last().EventName,
                Is.EqualTo("battlement.session.failed")
            );

            using BattlementTestHarness submitHarness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            submitHarness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(session)
            );
            submitHarness.Runner.Connect();

            submitHarness.Runner.Submit(new byte[MaximumResponseBytes + 1]);

            Assert.That(submitHarness.Transport.SubmitMessages, Is.Empty);
            Assert.That(submitHarness.Transport.Calls.Last(), Is.EqualTo("stop"));
        }

        [Test]
        public void NestedResponseQueuesBehindTheResponseBeingDecoded()
        {
            var stream = new BattlementResponseStream();
            SessionId session = new(Guid.NewGuid());
            var applied = new List<bool>();
            ReadOnlyMemory<byte> outerPayload = BattlementFlatBufferResponseFixtures.Write(
                Response(session, inputDisabled: true)
            );
            ReadOnlyMemory<byte> nestedPayload = BattlementFlatBufferResponseFixtures.Write(
                Response(session, inputDisabled: false)
            );
            BattlementResponseStream.Reservation outer = stream.Reserve(
                (_, owner) =>
                {
                    BattlementResponseStream.Reservation nested = stream.Reserve(
                        (payload, nestedOwner) =>
                            new BattlementFlatBufferResponse(payload, nestedOwner)
                    );
                    nested.Commit(nestedPayload);
                    Assert.That(applied, Is.Empty);
                    return new BattlementFlatBufferResponse(outerPayload, owner);
                }
            );
            outer.Commit(outerPayload);
            stream.Drain(
                (_, _, _) => true,
                (_, response, index) => applied.Add(response.ReadSnapshot(index).IsInputDisabled),
                () => false,
                () => false,
                observeTiming: false
            );

            Assert.That(applied, Is.EqualTo(new[] { true, false }));
        }

        [Test]
        public void ResponseQueueRejectsTheTwoHundredFiftySeventhEntry()
        {
            var stream = new BattlementResponseStream();
            var reservations = new List<BattlementResponseStream.Reservation>();
            for (int index = 0; index < 256; index++)
                reservations.Add(stream.Reserve((_, _) => throw new InvalidOperationException()));

            InvalidDataException error = Assert.Throws<InvalidDataException>(() =>
                stream.Reserve((_, _) => throw new InvalidOperationException())
            );
            Assert.That(error.Message, Does.Contain("cannot queue more than 256 responses"));
            foreach (BattlementResponseStream.Reservation reservation in reservations)
                reservation.Release();
        }

        private static Response Response(SessionId session, bool inputDisabled)
        {
            Snapshot snapshot = FakeBattlementTransport.CompleteSnapshot(
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

        private static void Drain(BattlementResponseStream stream) =>
            stream.Drain(
                (_, _, _) => true,
                (_, _, _) => { },
                () => false,
                () => false,
                observeTiming: false
            );

        private sealed class TrackingDisposable : IDisposable
        {
            public bool IsDisposed { get; private set; }

            public void Dispose() => IsDisposed = true;
        }
    }
}

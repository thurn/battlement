#nullable enable

using System;
using System.Linq;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class BattlementGeometryFrameTests
    {
        [Test]
        public void CoalescesNativeFramesIntoOneNewestGenerationExchange()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = Connect(harness);
            GeometryObservationId observationId = ObservationId(1);
            harness.Transport.EnqueuePoll(
                Response(
                    session,
                    Update(
                        session,
                        new GeometryObservationUpdate(
                            new[]
                            {
                                new GeometryObservation(
                                    observationId,
                                    new GeometryObservationTarget.Viewport(new DisplayId(0))
                                ),
                            },
                            Array.Empty<GeometryObservationId>()
                        )
                    )
                )
            );

            harness.Runner.RunFrame();
            Sample(harness, 3);
            harness.Transport.EnqueueSubmit(Response(session));
            harness.Runner.RunFrame();

            Action geometryAction = Actions(harness).Single();
            var body = (ActionBody.GeometryObservations)geometryAction.Body;
            Assert.That(body.Value.Generation.Value, Is.EqualTo(3));
            Assert.That(body.Value.Changed, Has.Count.EqualTo(1));
            Assert.That(body.Value.Changed[0].ObservationId, Is.EqualTo(observationId));
            Assert.That(harness.Transport.Calls, Is.EqualTo(new[] { "connect", "poll", "submit" }));

            Sample(harness);
            harness.Runner.RunFrame();

            Assert.That(Actions(harness), Has.Length.EqualTo(1));
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("poll"));
        }

        [Test]
        public void RetiredPendingEpochIsDroppedBeforeTheFrameExchange()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = Connect(harness);
            GeometryObservationId observationId = ObservationId(2);
            harness.Transport.EnqueuePoll(
                Response(
                    session,
                    Update(
                        session,
                        new GeometryObservationUpdate(
                            new[]
                            {
                                new GeometryObservation(
                                    observationId,
                                    new GeometryObservationTarget.Viewport(new DisplayId(0))
                                ),
                            },
                            Array.Empty<GeometryObservationId>()
                        )
                    )
                )
            );
            harness.Runner.RunFrame();
            Sample(harness);

            byte[] input = Input(session);
            harness.Transport.EnqueueSubmit(
                Response(
                    session,
                    Update(
                        session,
                        new GeometryObservationUpdate(
                            Array.Empty<GeometryObservation>(),
                            new[] { observationId }
                        )
                    )
                )
            );
            harness.Runner.Submit(input);
            harness.Runner.RunFrame();

            Assert.That(harness.Transport.SubmitMessages, Is.EqualTo(new[] { input }));
            Assert.That(
                harness.Transport.Calls.TakeLast(2),
                Is.EqualTo(new[] { "submit", "poll" })
            );
        }

        [Test]
        public void ImmediateInputRemainsSeparateAndPrecedesPendingGeometry()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = Connect(harness);
            harness.Transport.EnqueuePoll(
                Response(
                    session,
                    Update(
                        session,
                        new GeometryObservationUpdate(
                            new[]
                            {
                                new GeometryObservation(
                                    ObservationId(3),
                                    new GeometryObservationTarget.Viewport(new DisplayId(0))
                                ),
                            },
                            Array.Empty<GeometryObservationId>()
                        )
                    )
                )
            );
            harness.Runner.RunFrame();
            Sample(harness);

            harness.Transport.EnqueueSubmit(Response(session));
            harness.Runner.Submit(Input(session));
            harness.Transport.EnqueueSubmit(Response(session));
            harness.Runner.RunFrame();

            Action[] actions = Actions(harness);
            Assert.That(actions, Has.Length.EqualTo(2));
            Assert.That(actions[0].Body, Is.TypeOf<ActionBody.KeyDown>());
            Assert.That(actions[1].Body, Is.TypeOf<ActionBody.GeometryObservations>());
            Assert.That(
                harness.Transport.Calls.TakeLast(2),
                Is.EqualTo(new[] { "submit", "submit" })
            );
        }

        [Test]
        public void ReturningToLastSubmittedValueRemovesThePendingChange()
        {
            var frames = new BattlementGeometryFrames();
            GeometryObservationId observationId = ObservationId(4);
            var first = new GeometryObservationResult.Unavailable(GeometryUnavailable.Hidden);
            var changed = new GeometryObservationResult.Unavailable(GeometryUnavailable.Detached);

            frames.Merge(Batch(1, observationId, first));
            Assert.That(frames.Take(), Is.Not.Null);
            frames.Merge(Batch(2, observationId, changed));
            frames.Merge(Batch(3, observationId, first));

            Assert.That(frames.Take(), Is.Null);
        }

        [Test]
        public void GeometrySubmitRequiresAnEngineResponse()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = Connect(harness);
            InstallViewport(harness, session, ObservationId(5));
            harness.Transport.EnqueueSubmit(
                new BattlementTransportResult(BattlementTransportStatus.NoMessage)
            );

            harness.Runner.RunFrame();

            Assert.That(
                harness.Transport.Calls.TakeLast(2),
                Is.EqualTo(new[] { "submit", "stop" })
            );
            Assert.That(harness.Runner.IsInputAvailable, Is.False);
            Assert.That(harness.Logger.Records.Last().Fields!["status"], Is.EqualTo("NoMessage"));
        }

        [Test]
        public void OversizedGeometryStopsBeforeTransportSubmission()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                protocolCodec: new OversizedActionCodec()
            );
            SessionId session = Connect(harness);
            InstallViewport(harness, session, ObservationId(6));

            harness.Runner.RunFrame();

            Assert.That(harness.Transport.SubmitMessages, Is.Empty);
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
            Assert.That(
                harness.Logger.Records.Last().Fields!["payload_bytes"],
                Is.EqualTo((BattlementProtocolLimits.MaximumMessageBytes + 1).ToString())
            );
        }

        private static SessionId Connect(BattlementTestHarness harness)
        {
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(session, session)
            );
            harness.Runner.Connect();
            return session;
        }

        private static void Sample(BattlementTestHarness harness, int count = 1)
        {
            for (int index = 0; index < count; index++)
                harness.Runner.CompleteNativeFrame();
        }

        private static void InstallViewport(
            BattlementTestHarness harness,
            SessionId session,
            GeometryObservationId observationId
        )
        {
            harness.Transport.EnqueuePoll(
                Response(
                    session,
                    Update(
                        session,
                        new GeometryObservationUpdate(
                            new[]
                            {
                                new GeometryObservation(
                                    observationId,
                                    new GeometryObservationTarget.Viewport(new DisplayId(0))
                                ),
                            },
                            Array.Empty<GeometryObservationId>()
                        )
                    )
                )
            );
            harness.Runner.RunFrame();
            Sample(harness);
        }

        private static BattlementTransportResult Response(SessionId session, params Batch[] batches)
        {
            var response = new Response(
                session,
                batches
                    .Select(batch =>
                        (ResponseMessage<Command>)new ResponseMessage<Command>.BatchMessage(batch)
                    )
                    .ToArray()
            );
            return FakeBattlementTransport.ResponseResult(response);
        }

        private static Batch Update(SessionId session, GeometryObservationUpdate update) =>
            new(
                new BatchId(Guid.NewGuid()),
                session,
                new[]
                {
                    new ParallelCommandGroup<Command>(
                        new[]
                        {
                            new Command(
                                new CommandId(Guid.NewGuid()),
                                new CommandBody.GeometryObservation(update)
                            ),
                        }
                    ),
                }
            );

        private static byte[] Input(SessionId session) =>
            BattlementJson.SerializeAction(
                new Action(
                    new ActionId(Guid.NewGuid()),
                    session,
                    new ActionBody.KeyDown(PhysicalKey.KeyA)
                )
            );

        private static Action[] Actions(BattlementTestHarness harness) =>
            harness
                .Transport.SubmitMessages.Select(bytes =>
                    BattlementJson.DeserializeClientMessage<CoreErrorCode, byte>(bytes)
                )
                .OfType<ClientMessage<CoreErrorCode, byte>.ActionMessage>()
                .Select(message => message.Action)
                .ToArray();

        private static GeometryObservationBatch Batch(
            ulong generation,
            GeometryObservationId observationId,
            GeometryObservationResult result
        ) =>
            new(
                new GeometryGeneration(generation),
                new[] { new GeometryObservationValue(observationId, result) }
            );

        private static GeometryObservationId ObservationId(int value) =>
            new(new Guid(value, 0, 0, new byte[8]));

        private sealed class OversizedActionCodec : IBattlementProtocolCodec
        {
            public byte[] SerializeConnect(Connect value) => BattlementJson.SerializeConnect(value);

            public byte[] SerializeBatchFailure(BatchFailed<CoreErrorCode> value) =>
                BattlementJson.SerializeBatchFailure(value);

            public byte[] SerializeOperationFailure(OperationFailed<CoreErrorCode> value) =>
                BattlementJson.SerializeOperationFailure(value);

            public byte[] SerializeAction(Action value) =>
                new byte[BattlementProtocolLimits.MaximumMessageBytes + 1];

            public Response DeserializeResponse(ReadOnlyMemory<byte> bytes) =>
                BattlementJson.DeserializeResponse(bytes);
        }
    }
}

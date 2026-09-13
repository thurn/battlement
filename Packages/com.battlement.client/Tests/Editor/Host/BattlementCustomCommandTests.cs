#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Battlement.CustomFixtures;
using Battlement.Errors;
using Google.FlatBuffers;
using NUnit.Framework;
using UnityEngine;
using FixtureWire = Battlement.FlatBuffers.FixtureGenerated;
using ProtocolVector3 = Battlement.Vector3;

namespace Battlement.Tests
{
    public sealed class BattlementCustomCommandTests
    {
        private const string CommandType = "fixture.character.flash";

        [Test]
        public void RegistrationAdvertisesAndRunsThroughPublicServices()
        {
            using BattlementTestHarness harness = CreateHarness();
            var handler = new FixtureHandler();
            Register(harness, handler);
            Assert.Throws<InvalidOperationException>(() => Register(harness, handler));
            Assert.Throws<ArgumentException>(() =>
                harness.Runner.RegisterFlatBufferCommand<FixtureWire.FlashPayload, FixtureError>(
                    "battlement.private",
                    handler
                )
            );

            SessionId session = new(Guid.NewGuid());
            ObjectId targetId = new(Guid.NewGuid());
            Connect(harness, session, targetId);
            harness.Transport.EnqueuePoll(
                Result(Response(session, Batch(session, Custom(targetId, scale: 2f))))
            );

            harness.Runner.RunFrame();

            Connect connect = harness.Transport.ConnectValues.Single();
            Assert.That(connect.CustomCommandTypes, Does.Contain(CommandType));
            Assert.That(
                handler.InvocationCount,
                Is.EqualTo(1),
                string.Join("\n", harness.Logger.Records.Select(record => record.Message))
            );
            Assert.That(handler.InvocationThreadId, Is.EqualTo(Environment.CurrentManagedThreadId));
            Assert.That(handler.LastContext!.Logger, Is.SameAs(harness.Logger));
            Assert.That(handler.LastContext.PreparedAssets, Is.Not.Null);
            Assert.That(handler.LastContext.Tweens, Is.Not.Null);
            Assert.That(harness.Runner.TryGetObject(targetId, out GameObject? target), Is.True);
            Assert.That(target!.transform.localScale, Is.EqualTo(UnityEngine.Vector3.one * 2f));
        }

        [Test]
        public void UnregisteredPayloadFailsOnlyItsBatch()
        {
            using BattlementTestHarness harness = CreateHarness();
            Register(harness, new FixtureHandler());
            SessionId session = new(Guid.NewGuid());
            ObjectId targetId = new(Guid.NewGuid());
            Connect(harness, session, targetId);

            harness.Transport.EnqueueSubmit(EmptyResult(session));
            harness.Transport.EnqueuePoll(
                Result(
                    Response(
                        session,
                        Batch(
                            session,
                            new CustomCommand<FlashPayload>(
                                new CommandId(Guid.NewGuid()),
                                "fixture.unregistered",
                                new FlashPayload(targetId, 2f)
                            )
                        )
                    )
                )
            );
            harness.Runner.RunFrame();

            Assert.That(
                harness
                    .Logger.Records.Last(record => record.EventName == "battlement.batch.failed")
                    .Fields!["error_code"],
                Is.EqualTo(nameof(CoreErrorCode.HandlerNotRegistered))
            );

            Assert.That(harness.Runner.IsInputAvailable, Is.True);
        }

        [Test]
        public void ImmediateExceptionsUseCoreOrGameNamespacedFailures()
        {
            using BattlementTestHarness harness = CreateHarness();
            var handler = new FixtureHandler(FixtureHandlerMode.Throw);
            Register(harness, handler);
            SessionId session = new(Guid.NewGuid());
            ObjectId targetId = new(Guid.NewGuid());
            Connect(harness, session, targetId);

            harness.Transport.EnqueueSubmit(EmptyResult(session));
            harness.Transport.EnqueuePoll(
                Result(Response(session, Batch(session, Custom(targetId))))
            );
            harness.Runner.RunFrame();
            Assert.That(
                harness
                    .Logger.Records.Last(record => record.EventName == "battlement.batch.failed")
                    .Fields!["error_code"],
                Is.EqualTo(nameof(CoreErrorCode.HandlerFailed))
            );
            var sink = (FakeBattlementErrorSink)harness.ErrorSink;
            Assert.That(sink.Errors, Has.Count.EqualTo(1));
            Assert.That(sink.Errors[0].Type, Is.EqualTo(BattlementErrorType.CommandFailed));
            Assert.That(sink.Errors[0].Exception, Is.TypeOf<InvalidOperationException>());
            Assert.That(
                sink.Errors[0].Exception!.StackTrace,
                Does.Contain(nameof(FixtureHandler.Execute))
            );

            handler.Mode = FixtureHandlerMode.Reject;
            harness.Transport.EnqueueSubmit(EmptyResult(session));
            harness.Transport.EnqueuePoll(
                Result(Response(session, Batch(session, Custom(targetId))))
            );
            harness.Runner.RunFrame();

            FixtureWire.FixtureBatchFailed failed = DecodeCustom(
                    harness.Transport.SubmitMessages.Last()
                )
                .BodyAsFixtureBatchFailed();
            Assert.That(failed.Error, Is.EqualTo(FixtureWire.FixtureError.Rejected));
            Assert.That(failed.CommandId.HasValue, Is.True);
        }

        [Test]
        public void TrackedWorkBlocksFailsLateAndReceivesCancellation()
        {
            using BattlementTestHarness harness = CreateHarness();
            var handler = new FixtureHandler(FixtureHandlerMode.Track);
            Register(harness, handler);
            SessionId session = new(Guid.NewGuid());
            ObjectId targetId = new(Guid.NewGuid());
            Connect(harness, session, targetId);

            CustomCommand<FlashPayload> blocking = Custom(targetId);
            Command later = new(
                new CommandId(Guid.NewGuid()),
                new CommandBody.Transform.SetLocalScale(targetId, new ProtocolVector3(4f, 4f, 4f))
            );
            harness.Transport.EnqueuePoll(
                Result(Response(session, Batch(session, blocking, later)))
            );
            harness.Runner.RunFrame();
            Assert.That(harness.Runner.TryGetObject(targetId, out GameObject? target), Is.True);
            Assert.That(target!.transform.localScale, Is.EqualTo(UnityEngine.Vector3.one));

            handler.Operation!.ShouldFail = true;
            harness.Transport.EnqueueSubmit(EmptyResult(session));
            harness.Runner.RunFrame();
            FixtureWire.FixtureBatchFailed blockingFailure = DecodeCustom(
                    harness.Transport.SubmitMessages.Last()
                )
                .BodyAsFixtureBatchFailed();
            Assert.That(blockingFailure.Error, Is.EqualTo(FixtureWire.FixtureError.Delayed));
            Assert.That(target.transform.localScale, Is.EqualTo(UnityEngine.Vector3.one));

            handler.Mode = FixtureHandlerMode.Track;
            CustomCommand<FlashPayload> nonblocking = Custom(targetId).Nonblocking();
            harness.Transport.EnqueuePoll(Result(Response(session, Batch(session, nonblocking))));
            harness.Runner.RunFrame();
            FixtureOperation late = handler.Operation!;
            late.ShouldFail = true;
            harness.Transport.EnqueueSubmit(EmptyResult(session));
            harness.Runner.RunFrame();
            FixtureWire.FixtureOperationFailed operationFailure = DecodeCustom(
                    harness.Transport.SubmitMessages.Last()
                )
                .BodyAsFixtureOperationFailed();
            Assert.That(operationFailure.Error, Is.EqualTo(FixtureWire.FixtureError.Delayed));

            harness.Transport.EnqueuePoll(
                Result(Response(session, Batch(session, Custom(targetId).Nonblocking())))
            );
            harness.Runner.RunFrame();
            FixtureOperation cancelled = handler.Operation!;
            harness.Transport.EnqueuePoll(FakeBattlementTransport.SnapshotResponse(session));
            harness.Runner.RunFrame();
            Assert.That(cancelled.WasCancelled, Is.True);
            Assert.That(cancelled.CancellationWasRequested, Is.True);
        }

        [Test]
        public void TypedActionNestedReturnWaitsForCurrentCommandStep()
        {
            using BattlementTestHarness harness = CreateHarness();
            var handler = new FixtureHandler(FixtureHandlerMode.EmitNestedAction, harness.Runner);
            Register(harness, handler);
            SessionId session = new(Guid.NewGuid());
            ObjectId targetId = new(Guid.NewGuid());
            Connect(harness, session, targetId);

            Command nestedScale = new(
                new CommandId(Guid.NewGuid()),
                new CommandBody.Transform.SetLocalScale(targetId, new ProtocolVector3(3f, 3f, 3f))
            );
            harness.Transport.EnqueueSubmit(Result(Response(session, Batch(session, nestedScale))));
            harness.Transport.EnqueuePoll(
                Result(Response(session, Batch(session, Custom(targetId, scale: 2f))))
            );

            harness.Runner.RunFrame();

            FixtureWire.FixtureAction action = DecodeCustom(
                    harness.Transport.SubmitMessages.Single()
                )
                .BodyAsFixtureAction();
            Assert.That(action.ActionType, Is.EqualTo("fixture.flash.completed"));
            Assert.That(
                BattlementFlatBufferCore.ReadUuid(action.SessionId, "session"),
                Is.EqualTo(session.Value)
            );
            Assert.That(harness.Runner.TryGetObject(targetId, out GameObject? target), Is.True);
            Assert.That(
                target!.transform.localScale,
                Is.EqualTo(UnityEngine.Vector3.one * 3f),
                "The nested return must apply after the emitting handler finishes."
            );
        }

        [Test]
        public void ObjectDestructionCancelsScopedCustomWork()
        {
            using BattlementTestHarness harness = CreateHarness();
            var handler = new FixtureHandler(FixtureHandlerMode.Track);
            Register(harness, handler);
            SessionId session = new(Guid.NewGuid());
            ObjectId targetId = new(Guid.NewGuid());
            Connect(harness, session, targetId);

            harness.Transport.EnqueuePoll(
                Result(Response(session, Batch(session, Custom(targetId).Nonblocking())))
            );
            harness.Runner.RunFrame();
            FixtureOperation operation = handler.Operation!;
            Command destroy = new(
                new CommandId(Guid.NewGuid()),
                new CommandBody.Object.Destroy(targetId)
            );
            harness.Transport.EnqueuePoll(Result(Response(session, Batch(session, destroy))));

            harness.Runner.RunFrame();

            Assert.That(operation.WasCancelled, Is.True);
            Assert.That(operation.CancellationWasRequested, Is.True);
            Assert.That(harness.Runner.TryGetObject(targetId, out _), Is.False);
        }

        private static void Register(BattlementTestHarness harness, FixtureHandler handler) =>
            harness.Runner.RegisterFlatBufferCommand<FixtureWire.FlashPayload, FixtureError>(
                CommandType,
                handler
            );

        private static void Connect(
            BattlementTestHarness harness,
            SessionId session,
            ObjectId targetId
        )
        {
            var target = new BattlementGameObject(
                targetId,
                new GameObjectKind.Empty(),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(session, objects: new[] { target })
            );
            harness.Runner.Connect();
        }

        private static CustomCommand<FlashPayload> Custom(ObjectId targetId, float scale = 2f) =>
            new(new CommandId(Guid.NewGuid()), CommandType, new FlashPayload(targetId, scale));

        private static Batch<ICommand> Batch(SessionId session, params ICommand[] commands)
        {
            var groups = new List<ParallelCommandGroup<ICommand>>();
            foreach (ICommand command in commands)
            {
                groups.Add(new ParallelCommandGroup<ICommand>(new[] { command }));
            }

            return new Batch<ICommand>(new BatchId(Guid.NewGuid()), session, groups);
        }

        private static Response<ICommand> Response(SessionId session, Batch<ICommand> batch) =>
            new(
                session,
                new ResponseMessage<ICommand>[]
                {
                    new ResponseMessage<ICommand>.BatchMessage(batch),
                }
            );

        private static BattlementTransportResult Result(Response<ICommand> response) =>
            new BattlementTransportResult(
                BattlementTransportStatus.Success,
                BattlementFlatBufferResponseFixtures.Write(response)
            );

        private static BattlementTransportResult EmptyResult(SessionId session) =>
            Result(new Response<ICommand>(session, Array.Empty<ResponseMessage<ICommand>>()));

        private static BattlementTestHarness CreateHarness()
        {
            var schema = new FixtureFlatBufferResponseSchema(
                error => (byte)(FixtureError)error,
                payload =>
                {
                    var value = (FlashPayload)payload;
                    return (value.ObjectId, value.Scale);
                }
            );
            return BattlementTestHarness.Create(
                flatBufferResponseSchema: schema,
                flatBufferClientSchema: schema
            );
        }

        private static FixtureWire.FixtureClientMessage DecodeCustom(byte[] message)
        {
            var bytes = new ByteBuffer(message);
            Assert.That(
                new Verifier(bytes).VerifyBuffer(
                    "BTCM",
                    true,
                    FixtureWire.FixtureClientMessageVerify.Verify
                ),
                Is.True
            );
            bytes.Position = FlatBufferConstants.SizePrefixLength;
            return FixtureWire.FixtureClientMessage.GetRootAsFixtureClientMessage(bytes);
        }
    }
}

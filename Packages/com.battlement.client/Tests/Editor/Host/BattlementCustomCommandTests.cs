#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Battlement.CustomFixtures;
using Battlement.Errors;
using NUnit.Framework;
using UnityEngine;
using ProtocolVector3 = Battlement.Vector3;

namespace Battlement.Tests
{
    public sealed class BattlementCustomCommandTests
    {
        private const string CommandType = "fixture.character.flash";

        private static readonly FlashPayloadFormatter PayloadFormatter = new();
        private static readonly FixtureErrorFormatter ErrorFormatter = new();

        [Test]
        public void RegistrationAdvertisesAndRunsThroughPublicServices()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var handler = new FixtureHandler();
            Register(harness, handler);
            Assert.Throws<InvalidOperationException>(() => Register(harness, handler));
            Assert.Throws<ArgumentException>(() =>
                harness.Runner.RegisterCommand(
                    "battlement.private",
                    handler,
                    PayloadFormatter,
                    ErrorFormatter
                )
            );

            SessionId session = new(Guid.NewGuid());
            ObjectId targetId = new(Guid.NewGuid());
            Connect(harness, session, targetId);
            harness.Transport.EnqueuePoll(
                Result(Response(session, Batch(session, Custom(targetId, scale: 2f))))
            );

            harness.Runner.RunFrame();

            Connect connect = BattlementJson.DeserializeConnect(
                harness.Transport.ConnectMessages.Single()
            );
            Assert.That(connect.CustomCommandTypes, Does.Contain(CommandType));
            Assert.That(handler.InvocationCount, Is.EqualTo(1));
            Assert.That(handler.InvocationThreadId, Is.EqualTo(Environment.CurrentManagedThreadId));
            Assert.That(handler.LastContext!.Logger, Is.SameAs(harness.Logger));
            Assert.That(handler.LastContext.PreparedAssets, Is.Not.Null);
            Assert.That(handler.LastContext.Tweens, Is.Not.Null);
            Assert.That(harness.Runner.TryGetObject(targetId, out GameObject? target), Is.True);
            Assert.That(target!.transform.localScale, Is.EqualTo(UnityEngine.Vector3.one * 2f));
        }

        [Test]
        public void UnregisteredAndMalformedPayloadsFailOnlyTheirBatches()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
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

            using BattlementTestHarness malformed = BattlementTestHarness.Create();
            malformed.Runner.RegisterCommand(
                CommandType,
                new FixtureHandler(),
                new RejectingFlashPayloadFormatter(),
                ErrorFormatter
            );
            Connect(malformed, session, targetId);
            malformed.Transport.EnqueueSubmit(EmptyResult(session));
            malformed.Transport.EnqueuePoll(
                Result(Response(session, Batch(session, Custom(targetId, scale: 3f))))
            );
            malformed.Runner.RunFrame();

            Assert.That(
                malformed
                    .Logger.Records.Last(record => record.EventName == "battlement.batch.failed")
                    .Fields!["error_code"],
                Is.EqualTo(nameof(CoreErrorCode.InvalidEncoding))
            );
            Assert.That(malformed.Runner.IsInputAvailable, Is.True);
        }

        [Test]
        public void ImmediateExceptionsUseCoreOrGameNamespacedFailures()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
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

            ClientMessage<FixtureError, FlashPayload> submitted = DecodeCustom(
                harness.Transport.SubmitMessages.Last()
            );
            var failed = (ClientMessage<FixtureError, FlashPayload>.BatchFailedMessage)submitted;
            Assert.That(failed.Failure.ErrorCode, Is.EqualTo(FixtureError.Rejected));
            Assert.That(failed.Failure.CommandId, Is.Not.Null);
        }

        [Test]
        public void TrackedWorkBlocksFailsLateAndReceivesCancellation()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
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
            var blockingFailure = (ClientMessage<
                FixtureError,
                FlashPayload
            >.BatchFailedMessage)DecodeCustom(harness.Transport.SubmitMessages.Last());
            Assert.That(blockingFailure.Failure.ErrorCode, Is.EqualTo(FixtureError.Delayed));
            Assert.That(target.transform.localScale, Is.EqualTo(UnityEngine.Vector3.one));

            handler.Mode = FixtureHandlerMode.Track;
            CustomCommand<FlashPayload> nonblocking = Custom(targetId).Nonblocking();
            harness.Transport.EnqueuePoll(Result(Response(session, Batch(session, nonblocking))));
            harness.Runner.RunFrame();
            FixtureOperation late = handler.Operation!;
            late.ShouldFail = true;
            harness.Transport.EnqueueSubmit(EmptyResult(session));
            harness.Runner.RunFrame();
            var operationFailure = (ClientMessage<
                FixtureError,
                FlashPayload
            >.OperationFailedMessage)DecodeCustom(harness.Transport.SubmitMessages.Last());
            Assert.That(operationFailure.Failure.ErrorCode, Is.EqualTo(FixtureError.Delayed));

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
            using BattlementTestHarness harness = BattlementTestHarness.Create();
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

            var action = (ClientMessage<
                FixtureError,
                FlashPayload
            >.CustomActionMessage)DecodeCustom(harness.Transport.SubmitMessages.Single());
            Assert.That(action.Action.Type, Is.EqualTo("fixture.flash.completed"));
            Assert.That(action.Action.SessionId, Is.EqualTo(session));
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
            using BattlementTestHarness harness = BattlementTestHarness.Create();
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
            harness.Runner.RegisterCommand(CommandType, handler, PayloadFormatter, ErrorFormatter);

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

        private static BattlementTransportResult Result(Response<ICommand> response)
        {
            byte[] bytes = BattlementJson.SerializeResponse(response, PayloadFormatter);
            return new(BattlementTransportStatus.Success, bytes);
        }

        private static BattlementTransportResult EmptyResult(SessionId session) =>
            Result(new Response<ICommand>(session, Array.Empty<ResponseMessage<ICommand>>()));

        private static ClientMessage<FixtureError, FlashPayload> DecodeCustom(byte[] message) =>
            BattlementJson.DeserializeClientMessage(message, ErrorFormatter, PayloadFormatter);
    }
}

#nullable enable

using System;
using Google.FlatBuffers;
using NUnit.Framework;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    public sealed class BattlementCoreClientMessageWriterTests
    {
        private static readonly ActionId ActionId = new(
            Guid.Parse("00112233-4455-6677-8899-aabbccddeeff")
        );
        private static readonly SessionId SessionId = new(
            Guid.Parse("10213243-5465-7687-98a9-bacbdcedfe0f")
        );
        private static readonly ObjectId ObjectId = new(
            Guid.Parse("ffeeddcc-bbaa-4988-8776-655443322110")
        );

        [Test]
        public void WritesVerifiedPointerActionWithoutLosingDoublePrecision()
        {
            var writer = new BattlementCoreClientMessageWriter();
            byte[] bytes = writer
                .WriteAction(
                    new Action(
                        ActionId,
                        SessionId,
                        new ActionBody.PointerClick(
                            ObjectId,
                            new ScreenPosition(1.234567890123, -9.876543210987),
                            new Vector3(123456789.125, -0.000000000123, 42.5),
                            7,
                            PointerButton.Right
                        )
                    )
                )
                .ToArray();

            Assert.That(Verify(bytes), Is.True);
            var buffer = new ByteBuffer(bytes) { Position = 4 };
            Wire.CoreAction action = Wire
                .CoreClientMessage.GetRootAsCoreClientMessage(buffer)
                .BodyAsCoreAction();
            Assert.That(action.Kind, Is.EqualTo(Wire.CoreActionKind.PointerClick));
            Assert.That(action.BodyType, Is.EqualTo(Wire.CoreActionBody.PointerButtonAction));
            Wire.PointerButtonAction pointer = action.BodyAsPointerButtonAction();
            Assert.That(pointer.PointerId, Is.EqualTo(7));
            Assert.That(pointer.ScreenPosition!.Value.X, Is.EqualTo(1.234567890123));
            Assert.That(pointer.ScreenPosition!.Value.Y, Is.EqualTo(-9.876543210987));
            Assert.That(pointer.WorldHit!.Value.X, Is.EqualTo(123456789.125));
            Assert.That(pointer.WorldHit!.Value.Y, Is.EqualTo(-0.000000000123));
            Assert.That(pointer.Button!.Value.Kind, Is.EqualTo(Wire.PointerButtonKind.Right));
        }

        [Test]
        public void WritesStructuredFailuresAndPreservesOptionalCommandIdentity()
        {
            var writer = new BattlementCoreClientMessageWriter();
            var batchId = new BatchId(Guid.Parse("aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee"));

            byte[] withoutCommand = writer
                .WriteBatchFailure(
                    new BatchFailed<CoreErrorCode>(
                        SessionId,
                        batchId,
                        CoreErrorCode.InvalidProperty,
                        "日本語 failure"
                    )
                )
                .ToArray();
            Assert.That(Verify(withoutCommand), Is.True);
            var first = new ByteBuffer(withoutCommand) { Position = 4 };
            Wire.BatchFailed batch = Wire
                .CoreClientMessage.GetRootAsCoreClientMessage(first)
                .BodyAsBatchFailed();
            Assert.That(batch.CommandId, Is.Null);
            Assert.That(batch.ErrorCode, Is.EqualTo(Wire.CoreErrorCode.InvalidProperty));
            Assert.That(batch.Message, Is.EqualTo("日本語 failure"));

            var commandId = new CommandId(Guid.Parse("bbbbbbbb-cccc-4ddd-8eee-ffffffffffff"));
            byte[] operation = writer
                .WriteOperationFailure(
                    new OperationFailed<CoreErrorCode>(
                        SessionId,
                        batchId,
                        commandId,
                        CoreErrorCode.HandlerFailed,
                        "late"
                    )
                )
                .ToArray();
            Assert.That(Verify(operation), Is.True);
            var second = new ByteBuffer(operation) { Position = 4 };
            Wire.OperationFailed late = Wire
                .CoreClientMessage.GetRootAsCoreClientMessage(second)
                .BodyAsOperationFailed();
            Assert.That(late.CommandId, Is.Not.Null);
            Assert.That(late.ErrorCode, Is.EqualTo(Wire.CoreErrorCode.HandlerFailed));
        }

        [Test]
        public void WritesTypedGeometryResultsInOneVerifiedMessage()
        {
            var writer = new BattlementCoreClientMessageWriter();
            var firstId = new GeometryObservationId(
                Guid.Parse("11111111-2222-4333-8444-555555555555")
            );
            var secondId = new GeometryObservationId(
                Guid.Parse("22222222-3333-4444-8555-666666666666")
            );
            var changed = new GeometryObservationValue[]
            {
                new(
                    firstId,
                    new GeometryObservationResult.Unavailable(GeometryUnavailable.Detached)
                ),
                new(
                    secondId,
                    new GeometryObservationResult.Current(
                        new GeometryValue.WorldPoint(
                            new WorldPointGeometry(
                                new ViewportPoint(12.5, 23.75, new DisplayId(4)),
                                -9.125,
                                true
                            )
                        )
                    )
                ),
            };
            byte[] bytes = writer
                .WriteAction(
                    Action(
                        new ActionBody.GeometryObservations(
                            new GeometryObservationBatch(new GeometryGeneration(7), changed)
                        )
                    )
                )
                .ToArray();

            Assert.That(Verify(bytes), Is.True);
            var buffer = new ByteBuffer(bytes) { Position = 4 };
            Wire.CoreAction action = Wire
                .CoreClientMessage.GetRootAsCoreClientMessage(buffer)
                .BodyAsCoreAction();
            Assert.That(action.Kind, Is.EqualTo(Wire.CoreActionKind.GeometryObservations));
            Wire.GeometryObservationBatch batch = action.BodyAsGeometryAction().Value!.Value;
            Assert.That(batch.Generation, Is.EqualTo(7));
            Assert.That(batch.ChangedLength, Is.EqualTo(2));
            Assert.That(
                batch.Changed(0)!.Value.ResultType,
                Is.EqualTo(Wire.GeometryResult.UnavailableGeometry)
            );
            Wire.WorldPointGeometry point = batch
                .Changed(1)!
                .Value.ResultAsCurrentGeometry()
                .ValueAsWorldPointGeometry();
            Assert.That(point.Point!.Value.X, Is.EqualTo(12.5));
            Assert.That(point.Point!.Value.DisplayId, Is.EqualTo(4));
            Assert.That(point.Depth, Is.EqualTo(-9.125));
        }

        [Test]
        public void RejectsUnsupportedOrNoncanonicalValuesBeforeSubmission()
        {
            var writer = new BattlementCoreClientMessageWriter();
            Assert.Throws<System.IO.InvalidDataException>(() =>
                writer.WriteAction(
                    Action(
                        new ActionBody.PointerEnter(
                            ObjectId,
                            new ScreenPosition(double.NaN, 0),
                            Vector3.Zero
                        )
                    )
                )
            );
            Assert.Throws<System.IO.InvalidDataException>(() =>
                writer.WriteAction(
                    Action(
                        new ActionBody.MotionEvents(
                            new MotionEventBatch(
                                2,
                                1,
                                Array.Empty<MotionLifecycleEvent>(),
                                Array.Empty<MotionPresentationSample>()
                            )
                        )
                    )
                )
            );
        }

        private static Action Action(ActionBody body) => new(ActionId, SessionId, body);

        private static bool Verify(byte[] bytes)
        {
            return new Verifier(new ByteBuffer(bytes), new Options()).VerifyBuffer(
                "BTCM",
                true,
                Wire.CoreClientMessageVerify.Verify
            );
        }
    }
}

#nullable enable

using System;
using MessagePack;
using MessagePack.Formatters;

namespace Masonry.Performance
{
    internal sealed class PerformanceSmokeTransport : IMasonryTransport
    {
        internal static readonly ObjectId TargetId = new(
            Guid.Parse("00000000-0000-0000-0000-000000003802")
        );

        private static readonly SessionId Session = new(
            Guid.Parse("00000000-0000-0000-0000-000000003800")
        );
        private static readonly SceneId Scene = new(
            Guid.Parse("00000000-0000-0000-0000-000000003803")
        );
        private static readonly ObjectId CameraId = new(
            Guid.Parse("00000000-0000-0000-0000-000000003801")
        );
        private static readonly SceneAddress SceneAddress = new("masonry/integration/scene");
        private static readonly CoreErrorFormatter ErrorFormatter = new();
        private static readonly UnusedPayloadFormatter PayloadFormatter = new();

        public MasonryTransportKind Kind => MasonryTransportKind.Native;

        public int ClickCount { get; private set; }

        public MasonryTransportResult Connect(ReadOnlyMemory<byte> messagePack) =>
            Result(new Response(Session, new[] { SnapshotMessage() }));

        public MasonryTransportResult Submit(ReadOnlyMemory<byte> messagePack)
        {
            ClientMessage<CoreErrorCode, byte> message =
                MasonryMessagePack.DeserializeClientMessage(
                    messagePack,
                    ErrorFormatter,
                    PayloadFormatter
                );
            if (
                message is ClientMessage<CoreErrorCode, byte>.ActionMessage action
                && action.Action.Body is ActionBody.PointerClick click
                && click.ObjectId == TargetId
            )
            {
                ClickCount++;
                return Result(TweenResponse(action.Action.Id));
            }

            return Result(new Response(Session, Array.Empty<ResponseMessage<Command>>()));
        }

        public MasonryTransportResult Poll() => new(MasonryTransportStatus.NoMessage);

        public void Stop() { }

        public void Dispose() { }

        private static ResponseMessage<Command> SnapshotMessage()
        {
            var camera = new MasonryGameObject(
                CameraId,
                new GameObjectKind.Camera(
                    new CameraState() with
                    {
                        Projection = CameraProjection.Orthographic,
                        OrthographicSize = 3,
                    }
                ),
                new ParentScene.Persistent(),
                null,
                true,
                new LocalTransform(new Vector3(0, 0, -10), Quaternion.Identity, Vector3.One),
                Array.Empty<PointerEvent>()
            );
            var target = new MasonryGameObject(
                TargetId,
                new GameObjectKind.Cube(),
                new ParentScene.Specific(Scene),
                null,
                true,
                LocalTransform.Identity,
                new[] { PointerEvent.Down, PointerEvent.Up, PointerEvent.Click }
            );
            var snapshot = new Snapshot(
                Session,
                new PreparedAsset[] { new PreparedAsset.Scene(SceneAddress) },
                new[] { new MasonryScene(Scene, SceneAddress) },
                new[] { camera, target },
                CameraId
            );
            return new ResponseMessage<Command>.SnapshotMessage(snapshot);
        }

        private static Response TweenResponse(ActionId actionId)
        {
            var command = new Command(
                new CommandId(Guid.Parse("00000000-0000-0000-0000-000000003804")),
                new CommandBody.Transform.TweenLocalPosition(
                    TargetId,
                    new Vector3(2, 0, 0),
                    new Tween(TimeSpan.FromMilliseconds(500))
                )
            );
            var batch = new Batch(
                new BatchId(Guid.Parse("00000000-0000-0000-0000-000000003805")),
                Session,
                new[] { new ParallelCommandGroup<Command>(new[] { command }) },
                actionId
            );
            return new Response(
                Session,
                new ResponseMessage<Command>[] { new ResponseMessage<Command>.BatchMessage(batch) }
            );
        }

        private static MasonryTransportResult Result(Response response) =>
            new(MasonryTransportStatus.Success, MasonryMessagePack.SerializeResponse(response));

        private sealed class CoreErrorFormatter : IMessagePackFormatter<CoreErrorCode>
        {
            public void Serialize(
                ref MessagePackWriter writer,
                CoreErrorCode value,
                MessagePackSerializerOptions options
            ) => throw new NotSupportedException();

            public CoreErrorCode Deserialize(
                ref MessagePackReader reader,
                MessagePackSerializerOptions options
            ) => throw new NotSupportedException();
        }

        private sealed class UnusedPayloadFormatter : IMessagePackFormatter<byte>
        {
            public void Serialize(
                ref MessagePackWriter writer,
                byte value,
                MessagePackSerializerOptions options
            ) => throw new NotSupportedException();

            public byte Deserialize(
                ref MessagePackReader reader,
                MessagePackSerializerOptions options
            ) => throw new NotSupportedException();
        }
    }
}

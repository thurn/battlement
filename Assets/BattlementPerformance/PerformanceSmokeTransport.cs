#nullable enable

using System;

namespace Battlement.Performance
{
    internal sealed class PerformanceSmokeTransport : IBattlementTransport
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
        private static readonly SceneAddress SceneAddress = new("battlement/integration/scene");

        public int ClickCount { get; private set; }

        public BattlementTransportResult Connect(ReadOnlyMemory<byte> json) =>
            Result(new Response(Session, new[] { SnapshotMessage() }));

        public BattlementTransportResult Submit(ReadOnlyMemory<byte> json)
        {
            ClientMessage<CoreErrorCode, byte> message = BattlementJson.DeserializeClientMessage<
                CoreErrorCode,
                byte
            >(json);
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

        public BattlementUiEventTransportResult SubmitUiEvent(ReadOnlyMemory<byte> json)
        {
            UiEventAction action = BattlementJson.Deserialize<UiEventAction>(json);
            return new BattlementUiEventTransportResult(
                BattlementTransportStatus.Success,
                action.Event.DefaultPrevented
                    ? UiEventDisposition.PreventDefault
                    : UiEventDisposition.Continue,
                BattlementJson.SerializeResponse(
                    new Response(Session, Array.Empty<ResponseMessage<Command>>())
                )
            );
        }

        public BattlementTransportResult Poll() => new(BattlementTransportStatus.NoMessage);

        public void Stop() { }

        public void Dispose() { }

        private static ResponseMessage<Command> SnapshotMessage()
        {
            var camera = new BattlementGameObject(
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
            var target = new BattlementGameObject(
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
                new[] { new BattlementScene(Scene, SceneAddress) },
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

        private static BattlementTransportResult Result(Response response) =>
            new(BattlementTransportStatus.Success, BattlementJson.SerializeResponse(response));
    }
}

#nullable enable

using System;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementCameraLightCommandTests
    {
        [Test]
        public void CameraCommandsApplyProjectionClippingClearAndEnabledState()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var cameraId = new ObjectId(Guid.NewGuid());
            SessionId session = Connect(harness, CameraObject(cameraId), cameraId);
            Camera camera = Find(cameraId).GetComponent<Camera>();

            Submit(
                harness,
                session,
                Body(new CommandBody.Camera.SetOrthographic(cameraId, 7)),
                Body(new CommandBody.Camera.SetClipping(cameraId, 0.5, 400))
            );
            Assert.That(camera.orthographic, Is.True);
            Assert.That(camera.orthographicSize, Is.EqualTo(7).Within(0.0001f));
            Assert.That(camera.nearClipPlane, Is.EqualTo(0.5f).Within(0.0001f));
            Assert.That(camera.farClipPlane, Is.EqualTo(400).Within(0.0001f));

            (CameraClearMode Mode, CameraClearFlags Flags)[] modes =
            {
                (CameraClearMode.Skybox, CameraClearFlags.Skybox),
                (CameraClearMode.Depth, CameraClearFlags.Depth),
                (CameraClearMode.Nothing, CameraClearFlags.Nothing),
            };
            foreach ((CameraClearMode mode, CameraClearFlags flags) in modes)
            {
                Submit(harness, session, Body(new CommandBody.Camera.SetClear(cameraId, mode)));
                Assert.That(camera.clearFlags, Is.EqualTo(flags));
            }

            var clearColor = new Color(0.1, 0.25, 0.75, 0.5);
            Submit(
                harness,
                session,
                Body(
                    new CommandBody.Camera.SetClear(
                        cameraId,
                        CameraClearMode.SolidColor,
                        clearColor
                    )
                ),
                Body(new CommandBody.Camera.SetPerspective(cameraId, 72)),
                Body(new CommandBody.Camera.SetEnabled(cameraId, false))
            );

            Assert.That(camera.clearFlags, Is.EqualTo(CameraClearFlags.SolidColor));
            AssertColor(camera.backgroundColor, clearColor);
            Assert.That(camera.orthographic, Is.False);
            Assert.That(camera.fieldOfView, Is.EqualTo(72).Within(0.0001f));
            Assert.That(camera.enabled, Is.False);
        }

        [Test]
        public void ProjectionSwitchesCancelBothKeysAndWrongProjectionTweensFail()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            var cameraId = new ObjectId(Guid.NewGuid());
            SessionId session = Connect(harness, CameraObject(cameraId), cameraId);
            Camera camera = Find(cameraId).GetComponent<Camera>();
            Tween linear = LinearTween();

            Submit(
                harness,
                session,
                Body(new CommandBody.Camera.TweenFieldOfView(cameraId, 120, linear)).Nonblocking()
            );
            Advance(harness, 250);
            float displayedFieldOfView = camera.fieldOfView;
            Submit(harness, session, Body(new CommandBody.Camera.SetOrthographic(cameraId, 8)));
            Advance(harness, 1_000);
            Assert.That(camera.fieldOfView, Is.EqualTo(displayedFieldOfView).Within(0.001f));
            Assert.That(camera.orthographicSize, Is.EqualTo(8).Within(0.001f));

            Submit(
                harness,
                session,
                Body(new CommandBody.Camera.TweenFieldOfView(cameraId, 90, linear)),
                reportsFailure: true
            );
            Assert.That(
                Failures(harness).Last().ErrorCode,
                Is.EqualTo(CoreErrorCode.InvalidProperty)
            );

            Submit(
                harness,
                session,
                Body(new CommandBody.Camera.TweenOrthographicSize(cameraId, 20, linear))
                    .Nonblocking()
            );
            Advance(harness, 300);
            float displayedSize = camera.orthographicSize;
            Submit(harness, session, Body(new CommandBody.Camera.SetPerspective(cameraId, 65)));
            Advance(harness, 1_000);
            Assert.That(camera.orthographicSize, Is.EqualTo(displayedSize).Within(0.001f));

            Submit(
                harness,
                session,
                Body(new CommandBody.Camera.TweenOrthographicSize(cameraId, 10, linear)),
                reportsFailure: true
            );
            Assert.That(
                Failures(harness).Last().ErrorCode,
                Is.EqualTo(CoreErrorCode.InvalidProperty)
            );
        }

        [Test]
        public void DisablingTheInputCameraStopsBillboardUpdatesUntilReselected()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var cameraId = new ObjectId(Guid.NewGuid());
            var imageId = new ObjectId(Guid.NewGuid());
            var textureAddress = new TextureAddress("game/camera-disable-image");
            var texture = new Texture2D(2, 2);
            harness.AssetStorage.EnqueueValue(texture);
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    preparedAssets: new PreparedAsset[]
                    {
                        new PreparedAsset.Texture(textureAddress),
                    },
                    objects: new[]
                    {
                        CameraObject(cameraId, new Vector3(0, 0, 10)),
                        ImageObject(imageId, textureAddress),
                    },
                    inputCameraId: cameraId
                )
            );
            harness.Runner.Connect();
            harness.Runner.RunFrame();
            Camera camera = Find(cameraId).GetComponent<Camera>();
            Transform image = Find(imageId).transform;

            Submit(harness, session, Body(new CommandBody.Camera.SetEnabled(cameraId, false)));
            UnityEngine.Quaternion retained = image.rotation;
            camera.transform.position = new UnityEngine.Vector3(10, 0, 0);
            harness.Runner.RunFrame();
            Assert.That(
                UnityEngine.Quaternion.Angle(image.rotation, retained),
                Is.Zero.Within(0.001f)
            );

            Submit(harness, session, Body(new CommandBody.Camera.SetEnabled(cameraId, true)));
            Submit(harness, session, Body(new CommandBody.Input.SetCamera(cameraId)));
            harness.Runner.RunFrame();
            Assert.That(UnityEngine.Quaternion.Angle(image.rotation, retained), Is.GreaterThan(1));
            Object.DestroyImmediate(texture);
        }

        [Test]
        public void LightCommandsApplyEveryTypeShadowAndBoundaryValue()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var lightId = new ObjectId(Guid.NewGuid());
            SessionId session = Connect(harness, LightObject(lightId));
            Light light = Find(lightId).GetComponent<Light>();

            Submit(
                harness,
                session,
                Body(new CommandBody.Light.SetEnabled(lightId, false)),
                Body(new CommandBody.Light.SetType(lightId, LightType.Directional)),
                Body(new CommandBody.Light.SetColor(lightId, new Color(0.2, 0.4, 0.6, 0.8))),
                Body(new CommandBody.Light.SetIntensity(lightId, 0))
            );
            Assert.That(light.enabled, Is.False);
            Assert.That(light.type, Is.EqualTo(UnityEngine.LightType.Directional));
            AssertColor(light.color, new Color(0.2, 0.4, 0.6, 0.8));
            Assert.That(light.intensity, Is.Zero);

            Submit(harness, session, Body(new CommandBody.Light.SetType(lightId, LightType.Point)));
            Submit(harness, session, Body(new CommandBody.Light.SetRange(lightId, 0.01)));
            Assert.That(light.type, Is.EqualTo(UnityEngine.LightType.Point));
            Assert.That(light.range, Is.EqualTo(0.01f).Within(0.0001f));

            Submit(harness, session, Body(new CommandBody.Light.SetType(lightId, LightType.Spot)));
            Submit(
                harness,
                session,
                Body(new CommandBody.Light.SetSpotAngle(lightId, 178.99, 178.99))
            );
            Assert.That(light.spotAngle, Is.EqualTo(178.99f).Within(0.001f));
            Assert.That(light.innerSpotAngle, Is.EqualTo(178.99f).Within(0.001f));

            (ShadowMode Mode, LightShadows Shadows)[] shadows =
            {
                (ShadowMode.None, LightShadows.None),
                (ShadowMode.Hard, LightShadows.Hard),
                (ShadowMode.Soft, LightShadows.Soft),
            };
            foreach ((ShadowMode mode, LightShadows expected) in shadows)
            {
                Submit(harness, session, Body(new CommandBody.Light.SetShadows(lightId, mode)));
                Assert.That(light.shadows, Is.EqualTo(expected));
            }
        }

        [Test]
        public void LightColorAndIntensityTweensRemainIndependent()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            var lightId = new ObjectId(Guid.NewGuid());
            SessionId session = Connect(harness, LightObject(lightId));
            Light light = Find(lightId).GetComponent<Light>();
            Tween linear = LinearTween();

            Submit(
                harness,
                session,
                Body(new CommandBody.Light.TweenColor(lightId, new Color(0, 0.5, 1, 0.5), linear))
                    .Nonblocking(),
                Body(new CommandBody.Light.TweenIntensity(lightId, 5, linear)).Nonblocking()
            );
            Advance(harness, 400);
            AssertColor(light.color, new Color(0.6, 0.8, 1, 0.8), 0.001f);
            Assert.That(light.intensity, Is.EqualTo(2.6f).Within(0.001f));

            Submit(
                harness,
                session,
                Body(new CommandBody.Light.SetColor(lightId, new Color(1, 0, 0, 1)))
            );
            Advance(harness, 600);
            AssertColor(light.color, new Color(1, 0, 0, 1));
            Assert.That(light.intensity, Is.EqualTo(5).Within(0.001f));
        }

        [TestCase("camera-fov-low")]
        [TestCase("camera-fov-high")]
        [TestCase("camera-size")]
        [TestCase("camera-near")]
        [TestCase("camera-clipping")]
        [TestCase("camera-clear-missing")]
        [TestCase("camera-clear-extra")]
        [TestCase("light-color")]
        [TestCase("light-intensity")]
        [TestCase("light-range-zero")]
        [TestCase("light-range-directional")]
        [TestCase("light-spot-zero")]
        [TestCase("light-spot-high")]
        [TestCase("light-spot-inner-negative")]
        [TestCase("light-spot-inner-high")]
        [TestCase("light-spot-wrong-type")]
        public void InvalidExecutionTimeValuesFailWithoutApplying(string invalidCase)
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var cameraId = new ObjectId(Guid.NewGuid());
            var lightId = new ObjectId(Guid.NewGuid());
            SessionId session = Connect(
                harness,
                new[] { CameraObject(cameraId), LightObject(lightId) },
                cameraId
            );
            if (invalidCase == "light-range-zero")
            {
                Submit(
                    harness,
                    session,
                    Body(new CommandBody.Light.SetType(lightId, LightType.Point))
                );
            }
            if (invalidCase.StartsWith("light-spot-") && invalidCase != "light-spot-wrong-type")
            {
                Submit(
                    harness,
                    session,
                    Body(new CommandBody.Light.SetType(lightId, LightType.Spot))
                );
            }

            Command command = Body(InvalidCommand(invalidCase, cameraId, lightId));

            Submit(harness, session, command, reportsFailure: true);

            BatchFailed<CoreErrorCode> failure = Failures(harness).Single();
            Assert.That(failure.CommandId, Is.EqualTo(command.Id));
            Assert.That(failure.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
        }

        private static CommandBody InvalidCommand(
            string invalidCase,
            ObjectId cameraId,
            ObjectId lightId
        ) =>
            invalidCase switch
            {
                "camera-fov-low" => new CommandBody.Camera.SetPerspective(cameraId, 1),
                "camera-fov-high" => new CommandBody.Camera.SetPerspective(cameraId, 179),
                "camera-size" => new CommandBody.Camera.SetOrthographic(cameraId, 0),
                "camera-near" => new CommandBody.Camera.SetClipping(cameraId, 0, 100),
                "camera-clipping" => new CommandBody.Camera.SetClipping(cameraId, 10, 10),
                "camera-clear-missing" => new CommandBody.Camera.SetClear(
                    cameraId,
                    CameraClearMode.SolidColor
                ),
                "camera-clear-extra" => new CommandBody.Camera.SetClear(
                    cameraId,
                    CameraClearMode.Depth,
                    Color.Black
                ),
                "light-color" => new CommandBody.Light.SetColor(lightId, new Color(1.01, 0, 0, 1)),
                "light-intensity" => new CommandBody.Light.SetIntensity(lightId, -0.01),
                "light-range-zero" => new CommandBody.Light.SetRange(lightId, 0),
                "light-range-directional" => new CommandBody.Light.SetRange(lightId, 5),
                "light-spot-zero" => Spot(lightId, 0, 0),
                "light-spot-high" => Spot(lightId, 179, 0),
                "light-spot-inner-negative" => Spot(lightId, 30, -0.01),
                "light-spot-inner-high" => Spot(lightId, 30, 30.01),
                "light-spot-wrong-type" => new CommandBody.Light.SetSpotAngle(lightId, 30, 0),
                _ => throw new ArgumentOutOfRangeException(nameof(invalidCase)),
            };

        private static CommandBody Spot(ObjectId lightId, double outer, double inner) =>
            new CommandBody.Light.SetSpotAngle(lightId, outer, inner);

        private static SessionId Connect(
            BattlementTestHarness harness,
            BattlementGameObject gameObject,
            ObjectId? inputCameraId = null
        ) => Connect(harness, new[] { gameObject }, inputCameraId);

        private static SessionId Connect(
            BattlementTestHarness harness,
            BattlementGameObject[] gameObjects,
            ObjectId? inputCameraId = null
        )
        {
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    objects: gameObjects,
                    inputCameraId: inputCameraId
                )
            );
            harness.Runner.Connect();
            return session;
        }

        private static BattlementGameObject CameraObject(ObjectId id, Vector3? position = null) =>
            new(
                id,
                new GameObjectKind.Camera(new CameraState()),
                new ParentScene.Persistent(),
                null,
                true,
                position is Vector3 value
                    ? new LocalTransform(value, Quaternion.Identity, Vector3.One)
                    : LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );

        private static BattlementGameObject LightObject(ObjectId id) =>
            new(
                id,
                new GameObjectKind.Light(
                    new LightState(
                        true,
                        LightType.Directional,
                        Color.White,
                        1,
                        10,
                        30,
                        0,
                        ShadowMode.None
                    )
                ),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );

        private static BattlementGameObject ImageObject(ObjectId id, TextureAddress address) =>
            new(
                id,
                new GameObjectKind.Image(
                    new ImageState(address, 2, 2, ImageFit.Stretch, RgbColor.White, 1, true)
                ),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );

        private static Tween LinearTween() =>
            new(TimeSpan.FromSeconds(1), TimeSpan.Zero, Easing.Linear, new TweenRepeat.Once());

        private static Command Body(CommandBody body) => new(new CommandId(Guid.NewGuid()), body);

        private static void Submit(
            BattlementTestHarness harness,
            SessionId session,
            params Command[] commands
        ) => Submit(harness, session, commands, false);

        private static void Submit(
            BattlementTestHarness harness,
            SessionId session,
            Command command,
            bool reportsFailure
        ) => Submit(harness, session, new[] { command }, reportsFailure);

        private static void Submit(
            BattlementTestHarness harness,
            SessionId session,
            Command[] commands,
            bool reportsFailure
        )
        {
            var batch = new Batch(
                new BatchId(Guid.NewGuid()),
                session,
                new[] { new ParallelCommandGroup<Command>(commands) }
            );
            var response = new Response(
                session,
                new ResponseMessage<Command>[] { new ResponseMessage<Command>.BatchMessage(batch) }
            );
            harness.Transport.EnqueueSubmit(FakeBattlementTransport.ResponseResult(response));
            if (reportsFailure)
            {
                harness.Transport.EnqueueSubmit(
                    FakeBattlementTransport.ResponseResult(
                        new Response(session, Array.Empty<ResponseMessage<Command>>())
                    )
                );
            }

            harness.Runner.Submit(new byte[] { 1 });
        }

        private static void Advance(BattlementTestHarness harness, double milliseconds)
        {
            harness.Clock.Advance(TimeSpan.FromMilliseconds(milliseconds));
            harness.Runner.RunFrame();
        }

        private static GameObject Find(ObjectId id) =>
            Object
                .FindObjectsByType<BattlementIdentity>(FindObjectsInactive.Include)
                .Single(value => value.Id == id.Value)
                .gameObject;

        private static void AssertColor(
            UnityEngine.Color actual,
            Color expected,
            float tolerance = 0.0001f
        )
        {
            Assert.That(actual.r, Is.EqualTo(expected.Red).Within(tolerance));
            Assert.That(actual.g, Is.EqualTo(expected.Green).Within(tolerance));
            Assert.That(actual.b, Is.EqualTo(expected.Blue).Within(tolerance));
            Assert.That(actual.a, Is.EqualTo(expected.Alpha).Within(tolerance));
        }

        private static BatchFailed<CoreErrorCode>[] Failures(BattlementTestHarness harness) =>
            harness
                .Transport.SubmitMessages.Where(bytes => bytes.Length > 1)
                .Select(bytes =>
                    BattlementJson.DeserializeClientMessage<CoreErrorCode, byte>(bytes)
                )
                .OfType<ClientMessage<CoreErrorCode, byte>.BatchFailedMessage>()
                .Select(message => message.Failure)
                .ToArray();
    }
}

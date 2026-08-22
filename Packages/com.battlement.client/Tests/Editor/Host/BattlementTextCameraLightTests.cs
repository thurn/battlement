#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;
using TMPro;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementTextCameraLightTests
    {
        [Test]
        public void SnapshotConstructsTextCameraAndLightComponentState()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            TMP_FontAsset font = FontAsset();
            var fontAddress = new FontAddress("game/display-font");
            var inputCameraId = NewObjectId();
            ObjectId[] textIds = Enumerable.Range(0, 4).Select(_ => NewObjectId()).ToArray();
            ObjectId[] cameraIds = Enumerable.Range(0, 4).Select(_ => NewObjectId()).ToArray();
            ObjectId[] lightIds = Enumerable.Range(0, 3).Select(_ => NewObjectId()).ToArray();
            harness.AssetStorage.EnqueueValue(font);
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    preparedAssets: new PreparedAsset[] { new PreparedAsset.Font(fontAddress) },
                    objects: TextObjects(textIds, fontAddress)
                        .Concat(CameraObjects(inputCameraId, cameraIds))
                        .Concat(LightObjects(lightIds))
                        .ToArray(),
                    inputCameraId: inputCameraId
                )
            );

            harness.Runner.Connect();

            HorizontalAlignmentOptions[] horizontal =
            {
                HorizontalAlignmentOptions.Left,
                HorizontalAlignmentOptions.Center,
                HorizontalAlignmentOptions.Right,
                HorizontalAlignmentOptions.Justified,
            };
            VerticalAlignmentOptions[] vertical =
            {
                VerticalAlignmentOptions.Top,
                VerticalAlignmentOptions.Middle,
                VerticalAlignmentOptions.Bottom,
                VerticalAlignmentOptions.Top,
            };
            for (int index = 0; index < textIds.Length; index++)
            {
                GameObject gameObject = Identity(textIds[index]).gameObject;
                TextMeshPro text = gameObject.GetComponent<TextMeshPro>();
                Assert.That(text.text, Is.EqualTo($"Text {index}"));
                Assert.That(text.font, Is.SameAs(font));
                Assert.That(text.fontSize, Is.EqualTo(1 + index));
                Assert.That(text.horizontalAlignment, Is.EqualTo(horizontal[index]));
                Assert.That(text.verticalAlignment, Is.EqualTo(vertical[index]));
                Assert.That(text.richText, Is.EqualTo(index % 2 == 0));
                Assert.That(gameObject.GetComponent<Collider>(), Is.Null);
            }

            TextMeshPro unwrapped = Identity(textIds[0]).GetComponent<TextMeshPro>();
            TextMeshPro wrapped = Identity(textIds[1]).GetComponent<TextMeshPro>();
            Assert.That(unwrapped.textWrappingMode, Is.EqualTo(TextWrappingModes.NoWrap));
            Assert.That(wrapped.textWrappingMode, Is.EqualTo(TextWrappingModes.Normal));
            Assert.That(wrapped.rectTransform.rect.width, Is.EqualTo(6).Within(0.0001f));
            AssertColor(unwrapped.color, 0.1f, 0.2f, 0.3f, 0.4f);

            Camera inputCamera = Identity(inputCameraId).GetComponent<Camera>();
            Assert.That(inputCamera.enabled, Is.True);
            Assert.That(inputCamera.orthographic, Is.False);
            Assert.That(inputCamera.fieldOfView, Is.EqualTo(72).Within(0.0001f));
            Assert.That(inputCamera.nearClipPlane, Is.EqualTo(0.2f).Within(0.0001f));
            Assert.That(inputCamera.farClipPlane, Is.EqualTo(500).Within(0.0001f));
            Assert.That(inputCamera.clearFlags, Is.EqualTo(CameraClearFlags.Skybox));
            Assert.That(inputCamera.GetComponent<Collider>(), Is.Null);

            CameraClearFlags[] clearFlags =
            {
                CameraClearFlags.SolidColor,
                CameraClearFlags.Depth,
                CameraClearFlags.Nothing,
                CameraClearFlags.Skybox,
            };
            for (int index = 0; index < cameraIds.Length; index++)
            {
                Camera camera = Identity(cameraIds[index]).GetComponent<Camera>();
                Assert.That(camera.orthographic, Is.EqualTo(index % 2 == 0));
                Assert.That(camera.clearFlags, Is.EqualTo(clearFlags[index]));
                Assert.That(camera.enabled, Is.EqualTo(index != 3));
                Assert.That(camera.orthographicSize, Is.EqualTo(3 + index));
                Assert.That(camera.GetComponent<Collider>(), Is.Null);
            }

            UnityEngine.LightType[] types =
            {
                UnityEngine.LightType.Directional,
                UnityEngine.LightType.Point,
                UnityEngine.LightType.Spot,
            };
            LightShadows[] shadows = { LightShadows.None, LightShadows.Hard, LightShadows.Soft };
            for (int index = 0; index < lightIds.Length; index++)
            {
                Light light = Identity(lightIds[index]).GetComponent<Light>();
                Assert.That(light.type, Is.EqualTo(types[index]));
                Assert.That(light.shadows, Is.EqualTo(shadows[index]));
                Assert.That(light.enabled, Is.EqualTo(index != 1));
                Assert.That(light.intensity, Is.EqualTo(0.5f + index).Within(0.0001f));
                Assert.That(light.range, Is.EqualTo(5 + index).Within(0.0001f));
                Assert.That(light.spotAngle, Is.EqualTo(30 + index).Within(0.0001f));
                Assert.That(light.innerSpotAngle, Is.EqualTo(10 + index).Within(0.0001f));
                Assert.That(light.GetComponent<Collider>(), Is.Null);
            }
        }

        [Test]
        public void TextBillboardsAndSnapshotReplacementReleasesItsFontLease()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            TMP_FontAsset font = FontAsset();
            SessionId session = new(Guid.NewGuid());
            var address = new FontAddress("game/billboard-font");
            var textId = NewObjectId();
            var cameraId = NewObjectId();
            harness.AssetStorage.EnqueueValue(font);
            harness.Transport.EnqueueConnect(
                Response(
                    session,
                    new PreparedAsset[] { new PreparedAsset.Font(address) },
                    new[]
                    {
                        Describe(
                            textId,
                            new GameObjectKind.Text(
                                new TextState(
                                    "Facing text",
                                    address,
                                    1,
                                    Color.White,
                                    HorizontalAlignment.Center,
                                    VerticalAlignment.Middle,
                                    null,
                                    false,
                                    true
                                )
                            )
                        ),
                        Describe(
                            cameraId,
                            new GameObjectKind.Camera(new CameraState()),
                            new LocalTransform(
                                new Battlement.Vector3(0, 0, 10),
                                new Battlement.Quaternion(0, 0, 0.2588190451, 0.9659258263),
                                Battlement.Vector3.One
                            )
                        ),
                    },
                    cameraId
                )
            );
            harness.Runner.Connect();
            FakeAssetHandle handle = harness.AssetStorage.Handles.Single(value =>
                value.Asset == new PreparedAsset.Font(address)
            );

            harness.Runner.RunFrame();

            Transform text = Identity(textId).transform;
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            Assert.That(
                UnityEngine.Vector3.Angle(text.forward, camera.transform.position - text.position),
                Is.LessThan(0.001f)
            );
            UnityEngine.Vector3 expectedUp = UnityEngine
                .Vector3.ProjectOnPlane(camera.transform.up, text.forward)
                .normalized;
            Assert.That(UnityEngine.Vector3.Angle(text.up, expectedUp), Is.LessThan(0.001f));
            harness.Transport.EnqueueSubmit(
                Response(session, Array.Empty<PreparedAsset>(), Array.Empty<BattlementGameObject>())
            );
            harness.Runner.Submit(new byte[] { 1 });
            Assert.That(handle.IsDisposed, Is.True);
        }

        [TestCase("text-size")]
        [TestCase("text-wrap")]
        [TestCase("camera-clipping")]
        [TestCase("camera-field-of-view")]
        [TestCase("light-range")]
        [TestCase("light-spot")]
        public void InvalidComponentRangesStopTheSession(string invalidCase)
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var fontAddress = new FontAddress("game/invalid-font-state");
            BattlementGameObject invalid = InvalidObject(invalidCase, fontAddress);
            PreparedAsset[] assets =
                invalid.Kind is GameObjectKind.Text
                    ? new PreparedAsset[] { new PreparedAsset.Font(fontAddress) }
                    : Array.Empty<PreparedAsset>();
            if (assets.Length > 0)
            {
                harness.AssetStorage.EnqueueValue(FontAsset());
            }

            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    preparedAssets: assets,
                    objects: new[] { invalid }
                )
            );

            harness.Runner.Connect();

            Assert.That(Identities(), Is.Empty);
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
            Assert.That(harness.Logger.Records.Last().Message, Does.Contain("Snapshot validation"));
        }

        [Test]
        public void MissingFontAndDisabledInputCameraStopTheSession()
        {
            using BattlementTestHarness missingFontHarness = BattlementTestHarness.Create();
            missingFontHarness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    objects: new[]
                    {
                        Describe(
                            NewObjectId(),
                            new GameObjectKind.Text(
                                new TextState("Missing", new FontAddress("game/missing-font"))
                            )
                        ),
                    }
                )
            );

            missingFontHarness.Runner.Connect();

            Assert.That(missingFontHarness.Transport.Calls.Last(), Is.EqualTo("stop"));
            Assert.That(
                missingFontHarness.Logger.Records.Last().Message,
                Does.Contain("not in the prepared set")
            );

            using BattlementTestHarness disabledCameraHarness = BattlementTestHarness.Create();
            var cameraId = NewObjectId();
            disabledCameraHarness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    objects: new[]
                    {
                        Describe(
                            cameraId,
                            new GameObjectKind.Camera(new CameraState() with { IsEnabled = false })
                        ),
                    },
                    inputCameraId: cameraId
                )
            );

            disabledCameraHarness.Runner.Connect();

            Assert.That(disabledCameraHarness.Transport.Calls.Last(), Is.EqualTo("stop"));
            Assert.That(
                disabledCameraHarness.Logger.Records.Last().Message,
                Does.Contain("must be enabled and active")
            );
        }

        private static IEnumerable<BattlementGameObject> TextObjects(
            ObjectId[] ids,
            FontAddress address
        )
        {
            HorizontalAlignment[] horizontal =
            {
                HorizontalAlignment.Left,
                HorizontalAlignment.Center,
                HorizontalAlignment.Right,
                HorizontalAlignment.Justified,
            };
            VerticalAlignment[] vertical =
            {
                VerticalAlignment.Top,
                VerticalAlignment.Middle,
                VerticalAlignment.Bottom,
                VerticalAlignment.Top,
            };
            for (int index = 0; index < ids.Length; index++)
            {
                yield return Describe(
                    ids[index],
                    new GameObjectKind.Text(
                        new TextState(
                            $"Text {index}",
                            address,
                            1 + index,
                            index == 0 ? new Color(0.1, 0.2, 0.3, 0.4) : Color.White,
                            horizontal[index],
                            vertical[index],
                            index == 0 ? null : 5 + index,
                            index % 2 == 0,
                            false
                        )
                    )
                );
            }
        }

        private static IEnumerable<BattlementGameObject> CameraObjects(
            ObjectId inputId,
            ObjectId[] ids
        )
        {
            yield return Describe(
                inputId,
                new GameObjectKind.Camera(
                    new CameraState(
                        true,
                        CameraProjection.Perspective,
                        72,
                        5,
                        0.2,
                        500,
                        CameraClearMode.Skybox,
                        Color.Black
                    )
                )
            );
            CameraClearMode[] modes =
            {
                CameraClearMode.SolidColor,
                CameraClearMode.Depth,
                CameraClearMode.Nothing,
                CameraClearMode.Skybox,
            };
            for (int index = 0; index < ids.Length; index++)
            {
                yield return Describe(
                    ids[index],
                    new GameObjectKind.Camera(
                        new CameraState(
                            index != 3,
                            index % 2 == 0
                                ? CameraProjection.Orthographic
                                : CameraProjection.Perspective,
                            60 + index,
                            3 + index,
                            0.3,
                            100 + index,
                            modes[index],
                            new Color(0.2, 0.3, 0.4, 0.5)
                        )
                    )
                );
            }
        }

        private static IEnumerable<BattlementGameObject> LightObjects(ObjectId[] ids)
        {
            LightType[] types = { LightType.Directional, LightType.Point, LightType.Spot };
            ShadowMode[] shadows = { ShadowMode.None, ShadowMode.Hard, ShadowMode.Soft };
            for (int index = 0; index < ids.Length; index++)
            {
                yield return Describe(
                    ids[index],
                    new GameObjectKind.Light(
                        new LightState(
                            index != 1,
                            types[index],
                            new Color(0.8, 0.7, 0.6),
                            0.5 + index,
                            5 + index,
                            30 + index,
                            10 + index,
                            shadows[index]
                        )
                    )
                );
            }
        }

        private static BattlementGameObject InvalidObject(string invalidCase, FontAddress font) =>
            invalidCase switch
            {
                "text-size" => Describe(
                    NewObjectId(),
                    new GameObjectKind.Text(new TextState("Bad", font) with { Size = 0 })
                ),
                "text-wrap" => Describe(
                    NewObjectId(),
                    new GameObjectKind.Text(new TextState("Bad", font) with { WrapWidth = -1 })
                ),
                "camera-clipping" => Describe(
                    NewObjectId(),
                    new GameObjectKind.Camera(new CameraState() with { NearClip = 3, FarClip = 2 })
                ),
                "camera-field-of-view" => Describe(
                    NewObjectId(),
                    new GameObjectKind.Camera(new CameraState() with { FieldOfView = 180 })
                ),
                "light-range" => Describe(
                    NewObjectId(),
                    new GameObjectKind.Light(new LightState() with { Range = 0 })
                ),
                "light-spot" => Describe(
                    NewObjectId(),
                    new GameObjectKind.Light(
                        new LightState() with
                        {
                            InnerSpotAngle = 31,
                            OuterSpotAngle = 30,
                        }
                    )
                ),
                _ => throw new ArgumentOutOfRangeException(nameof(invalidCase)),
            };

        private static BattlementGameObject Describe(
            ObjectId id,
            GameObjectKind kind,
            LocalTransform? transform = null
        ) =>
            new(
                id,
                kind,
                new ParentScene.Persistent(),
                null,
                true,
                transform ?? LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );

        private static TMP_FontAsset FontAsset()
        {
            TMP_FontAsset font = Resources.Load<TMP_FontAsset>(
                "Fonts & Materials/LiberationSans SDF"
            );
            return font != null
                ? font
                : throw new InvalidOperationException("The TMP test font is unavailable.");
        }

        private static ObjectId NewObjectId() => new(Guid.NewGuid());

        private static BattlementIdentity Identity(ObjectId id) =>
            Identities().Single(identity => identity.Id == id.Value);

        private static BattlementIdentity[] Identities() =>
            Object
                .FindObjectsByType<BattlementIdentity>(FindObjectsInactive.Include)
                .Where(identity => !FakeBattlementTransport.IsFixtureIdentity(identity))
                .ToArray();

        private static void AssertColor(
            UnityEngine.Color actual,
            float red,
            float green,
            float blue,
            float alpha
        )
        {
            Assert.That(actual.r, Is.EqualTo(red).Within(0.0001f));
            Assert.That(actual.g, Is.EqualTo(green).Within(0.0001f));
            Assert.That(actual.b, Is.EqualTo(blue).Within(0.0001f));
            Assert.That(actual.a, Is.EqualTo(alpha).Within(0.0001f));
        }

        private static BattlementTransportResult Response(
            SessionId session,
            IReadOnlyList<PreparedAsset> assets,
            IReadOnlyList<BattlementGameObject> objects,
            ObjectId? inputCameraId = null
        ) =>
            FakeBattlementTransport.ResponseResult(
                new Response(
                    session,
                    new ResponseMessage<Command>[]
                    {
                        new ResponseMessage<Command>.SnapshotMessage(
                            FakeBattlementTransport.CompleteSnapshot(
                                session,
                                preparedAssets: assets,
                                objects: objects,
                                inputCameraId: inputCameraId
                            )
                        ),
                    }
                )
            );
    }
}

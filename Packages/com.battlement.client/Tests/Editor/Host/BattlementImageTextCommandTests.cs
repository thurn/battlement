#nullable enable

using System;
using System.Linq;
using NUnit.Framework;
using TMPro;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementImageTextCommandTests
    {
        [Test]
        public void ImageCommandsMutateGeometryAssetsAndIndependentColorChannels()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            Texture2D initialTexture = Texture(400, 200);
            Texture2D replacementTexture = Texture(100, 400);
            var initialAddress = new TextureAddress("game/image-initial");
            var replacementAddress = new TextureAddress("game/image-replacement");
            var imageId = new ObjectId(Guid.NewGuid());
            SessionId session = Connect(
                harness,
                new PreparedAsset[]
                {
                    new PreparedAsset.Texture(initialAddress),
                    new PreparedAsset.Texture(replacementAddress),
                },
                new[] { Image(imageId, initialAddress) },
                initialTexture,
                replacementTexture
            );
            GameObject image = Find(imageId);
            Tween linear = LinearTween();

            Submit(
                harness,
                session,
                Body(new CommandBody.Image.SetTexture(imageId, replacementAddress)),
                Body(new CommandBody.Image.SetSize(imageId, 2, 2)),
                Body(new CommandBody.Image.SetFit(imageId, ImageFit.Cover)),
                Body(new CommandBody.Image.TweenTint(imageId, new RgbColor(0, 0, 0), linear))
                    .Nonblocking(),
                Body(new CommandBody.Image.TweenOpacity(imageId, 0, linear)).Nonblocking()
            );
            Advance(harness, 500);

            Mesh mesh = image.GetComponent<MeshFilter>().sharedMesh;
            Assert.That(mesh.bounds.size.x, Is.EqualTo(2).Within(0.001f));
            Assert.That(mesh.bounds.size.y, Is.EqualTo(2).Within(0.001f));
            Assert.That(mesh.uv.Min(value => value.y), Is.EqualTo(0.375f).Within(0.001f));
            Assert.That(mesh.uv.Max(value => value.y), Is.EqualTo(0.625f).Within(0.001f));
            Assert.That(image.GetComponent<BoxCollider>().size.x, Is.EqualTo(2).Within(0.001f));
            Material material = image.GetComponent<MeshRenderer>().sharedMaterial;
            Assert.That(material.GetTexture("_BaseMap"), Is.SameAs(replacementTexture));
            AssertColor(material.GetColor("_BaseColor"), 0.5f, 0.5f, 0.5f, 0.5f);

            Submit(
                harness,
                session,
                Body(new CommandBody.Image.SetTint(imageId, new RgbColor(1, 0.25, 0)))
            );
            Advance(harness, 250);

            AssertColor(material.GetColor("_BaseColor"), 1, 0.25f, 0, 0.25f);
        }

        [Test]
        public void TextCommandsMutateTmpStateAndTweenFromDisplayedValues()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            TMP_FontAsset initialFont = FontAsset();
            TMP_FontAsset replacementFont = FontAsset();
            var initialAddress = new TextMeshProFontAddress("game/font-initial");
            var replacementAddress = new TextMeshProFontAddress("game/font-replacement");
            var textId = new ObjectId(Guid.NewGuid());
            SessionId session = Connect(
                harness,
                new PreparedAsset[]
                {
                    new PreparedAsset.TextMeshProFont(initialAddress),
                    new PreparedAsset.TextMeshProFont(replacementAddress),
                },
                new[] { Text(textId, initialAddress) },
                initialFont,
                replacementFont
            );
            TextMeshPro text = Find(textId).GetComponent<TextMeshPro>();
            Tween linear = LinearTween();

            Submit(
                harness,
                session,
                Body(new CommandBody.Text.SetContent(textId, "<b>Changed</b>")),
                Body(new CommandBody.Text.SetFont(textId, replacementAddress)),
                Body(
                    new CommandBody.Text.SetAlignment(
                        textId,
                        HorizontalAlignment.Justified,
                        VerticalAlignment.Bottom
                    )
                ),
                Body(new CommandBody.Text.SetWrapping(textId, 6)),
                Body(new CommandBody.Text.SetRichText(textId, true)),
                Body(new CommandBody.Text.TweenSize(textId, 3, linear)).Nonblocking(),
                Body(new CommandBody.Text.TweenColor(textId, new Color(0, 0.5, 1, 0), linear))
                    .Nonblocking()
            );
            Advance(harness, 500);

            Assert.That(text.text, Is.EqualTo("<b>Changed</b>"));
            Assert.That(text.font, Is.SameAs(replacementFont));
            Assert.That(text.horizontalAlignment, Is.EqualTo(HorizontalAlignmentOptions.Justified));
            Assert.That(text.verticalAlignment, Is.EqualTo(VerticalAlignmentOptions.Bottom));
            Assert.That(text.textWrappingMode, Is.EqualTo(TextWrappingModes.Normal));
            Assert.That(text.rectTransform.rect.width, Is.EqualTo(6).Within(0.001f));
            Assert.That(text.richText, Is.True);
            Assert.That(text.fontSize, Is.EqualTo(2).Within(0.001f));
            AssertColor(text.color, 0.5f, 0.75f, 1, 0.5f);

            Submit(harness, session, Body(new CommandBody.Text.SetSize(textId, 4)));
            Advance(harness, 500);

            Assert.That(text.fontSize, Is.EqualTo(4).Within(0.001f));
            AssertColor(text.color, 0, 0.5f, 1, 0);

            Submit(harness, session, Body(new CommandBody.Text.SetWrapping(textId, null)));
            Assert.That(text.textWrappingMode, Is.EqualTo(TextWrappingModes.NoWrap));
        }

        [Test]
        public void FaceCameraCommandsEnableBillboardsUsingTheInputCameraRoll()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            Texture2D texture = Texture(100, 100);
            TMP_FontAsset font = FontAsset();
            var textureAddress = new TextureAddress("game/billboard-image");
            var fontAddress = new TextMeshProFontAddress("game/billboard-text");
            var imageId = new ObjectId(Guid.NewGuid());
            var textId = new ObjectId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            SessionId session = Connect(
                harness,
                new PreparedAsset[]
                {
                    new PreparedAsset.Texture(textureAddress),
                    new PreparedAsset.TextMeshProFont(fontAddress),
                },
                new[]
                {
                    Image(imageId, textureAddress),
                    Text(textId, fontAddress),
                    CameraObject(cameraId),
                },
                texture,
                font,
                cameraId
            );

            Submit(
                harness,
                session,
                Body(new CommandBody.Image.SetFaceCamera(imageId, true)),
                Body(new CommandBody.Text.SetFaceCamera(textId, true))
            );
            harness.Runner.RunFrame();

            Camera camera = Find(cameraId).GetComponent<Camera>();
            AssertBillboard(Find(imageId).transform, camera, true);
            AssertBillboard(Find(textId).transform, camera, false);
        }

        private static SessionId Connect(
            BattlementTestHarness harness,
            PreparedAsset[] assets,
            BattlementGameObject[] objects,
            object firstValue,
            object secondValue,
            ObjectId? inputCameraId = null
        )
        {
            harness.AssetStorage.EnqueueValue(firstValue);
            harness.AssetStorage.EnqueueValue(secondValue);
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    preparedAssets: assets,
                    objects: objects,
                    inputCameraId: inputCameraId
                )
            );
            harness.Runner.Connect();
            return session;
        }

        private static BattlementGameObject Image(ObjectId id, TextureAddress address) =>
            Describe(
                id,
                new GameObjectKind.Image(new ImageState(address, 4, 2)),
                new[] { PointerEvent.Click }
            );

        private static BattlementGameObject Text(ObjectId id, TextMeshProFontAddress address) =>
            Describe(
                id,
                new GameObjectKind.Text(
                    new TextState("Initial", address) with
                    {
                        Size = 1,
                        Color = Color.White,
                    }
                )
            );

        private static BattlementGameObject CameraObject(ObjectId id) =>
            new(
                id,
                new GameObjectKind.Camera(new CameraState()),
                new ParentScene.Persistent(),
                null,
                true,
                new LocalTransform(
                    new Vector3(0, 0, 10),
                    new Quaternion(0, 0, 0.3826834324, 0.9238795325),
                    Vector3.One
                ),
                Array.Empty<PointerEvent>()
            );

        private static BattlementGameObject Describe(
            ObjectId id,
            GameObjectKind kind,
            PointerEvent[]? events = null
        ) =>
            new(
                id,
                kind,
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                events ?? Array.Empty<PointerEvent>()
            );

        private static Command Body(CommandBody body) => new(new CommandId(Guid.NewGuid()), body);

        private static Tween LinearTween() =>
            new(TimeSpan.FromSeconds(1), TimeSpan.Zero, Easing.Linear, new TweenRepeat.Once());

        private static void Submit(
            BattlementTestHarness harness,
            SessionId session,
            params Command[] commands
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
                .Single(identity => identity.Id == id.Value)
                .gameObject;

        private static Texture2D Texture(int width, int height) =>
            new(width, height, TextureFormat.RGBA32, false, true);

        private static TMP_FontAsset FontAsset() =>
            Resources.Load<TMP_FontAsset>("Fonts & Materials/LiberationSans SDF");

        private static void AssertBillboard(Transform target, Camera camera, bool frontIsNegativeZ)
        {
            UnityEngine.Vector3 visibleFace = frontIsNegativeZ ? -target.forward : target.forward;
            Assert.That(
                UnityEngine.Vector3.Angle(visibleFace, camera.transform.position - target.position),
                Is.LessThan(0.001f)
            );
            UnityEngine.Vector3 expectedUp = UnityEngine
                .Vector3.ProjectOnPlane(camera.transform.up, visibleFace)
                .normalized;
            Assert.That(UnityEngine.Vector3.Angle(target.up, expectedUp), Is.LessThan(0.001f));
        }

        private static void AssertColor(
            UnityEngine.Color actual,
            float red,
            float green,
            float blue,
            float alpha
        )
        {
            Assert.That(actual.r, Is.EqualTo(red).Within(0.001f));
            Assert.That(actual.g, Is.EqualTo(green).Within(0.001f));
            Assert.That(actual.b, Is.EqualTo(blue).Within(0.001f));
            Assert.That(actual.a, Is.EqualTo(alpha).Within(0.001f));
        }
    }
}

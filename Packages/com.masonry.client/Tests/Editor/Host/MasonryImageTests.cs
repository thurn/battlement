#nullable enable

using System;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.Rendering;
using Object = UnityEngine.Object;

namespace Masonry.Tests
{
    public sealed class MasonryImageTests
    {
        [Test]
        public void SnapshotConstructsOwnedImageMaterialsAndAllFitModes()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            Texture2D texture = Texture();
            var address = new TextureAddress("game/card-art");
            var asset = new PreparedAsset.Texture(address);
            var stretchId = NewObjectId();
            var containId = NewObjectId();
            var coverId = NewObjectId();
            harness.AssetStorage.EnqueueValue(texture);
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    preparedAssets: new PreparedAsset[] { asset },
                    objects: new[]
                    {
                        Image(
                            stretchId,
                            new ImageState(
                                address,
                                4,
                                4,
                                ImageFit.Stretch,
                                new RgbColor(0.25, 0.5, 0.75),
                                0.4,
                                false
                            ),
                            PointerEvent.Click
                        ),
                        Image(
                            containId,
                            new ImageState(
                                address,
                                4,
                                4,
                                ImageFit.Contain,
                                RgbColor.White,
                                1,
                                false
                            )
                        ),
                        Image(
                            coverId,
                            new ImageState(address, 2, 4, ImageFit.Cover, RgbColor.White, 1, false)
                        ),
                    }
                )
            );

            harness.Runner.Connect();

            MeshRenderer stretchRenderer = Identity(stretchId).GetComponent<MeshRenderer>();
            MeshRenderer containRenderer = Identity(containId).GetComponent<MeshRenderer>();
            MeshRenderer coverRenderer = Identity(coverId).GetComponent<MeshRenderer>();
            Mesh stretch = stretchRenderer.GetComponent<MeshFilter>().sharedMesh;
            Mesh contain = containRenderer.GetComponent<MeshFilter>().sharedMesh;
            Mesh cover = coverRenderer.GetComponent<MeshFilter>().sharedMesh;
            AssertSize(stretch.bounds.size, 4, 4);
            AssertSize(contain.bounds.size, 4, 2);
            AssertSize(cover.bounds.size, 2, 4);
            AssertUv(stretch.uv, 0, 0, 1, 1);
            AssertUv(contain.uv, 0, 0, 1, 1);
            AssertUv(cover.uv, 0.375f, 0, 0.625f, 1);
            Assert.That(stretch.normals.All(value => value.z > 0.999f), Is.True);

            Material material = stretchRenderer.sharedMaterial;
            Assert.That(material.shader.name, Is.EqualTo("Universal Render Pipeline/Unlit"));
            Assert.That(
                Resources
                    .Load<Material>("MasonryImage")
                    .IsKeywordEnabled("_SURFACE_TYPE_TRANSPARENT"),
                Is.True
            );
            Assert.That(material.GetTexture("_BaseMap"), Is.SameAs(texture));
            Assert.That(
                material.GetFloat("_DstBlendAlpha"),
                Is.EqualTo((float)BlendMode.OneMinusSrcAlpha)
            );
            Assert.That(material.GetShaderPassEnabled("DepthOnly"), Is.False);
            UnityEngine.Color color = material.GetColor("_BaseColor");
            Assert.That(color.r, Is.EqualTo(0.25f).Within(0.0001f));
            Assert.That(color.g, Is.EqualTo(0.5f).Within(0.0001f));
            Assert.That(color.b, Is.EqualTo(0.75f).Within(0.0001f));
            Assert.That(color.a, Is.EqualTo(0.4f).Within(0.0001f));
            Assert.That(material.renderQueue, Is.EqualTo(3000));
            Assert.That(containRenderer.sharedMaterial, Is.Not.SameAs(material));
            Assert.That(texture.filterMode, Is.EqualTo(FilterMode.Point));
            Assert.That(texture.wrapMode, Is.EqualTo(TextureWrapMode.Repeat));

            BoxCollider collider = Identity(stretchId).GetComponent<BoxCollider>();
            AssertSize(collider.size, 4, 4, 0.01f);
            Assert.That(Identity(containId).GetComponent<Collider>(), Is.Null);
        }

        [Test]
        public void ImageMaterialRendersOrientedRgbaPixelsWithoutEchoes()
        {
            if (SystemInfo.graphicsDeviceType == UnityEngine.Rendering.GraphicsDeviceType.Null)
            {
                Assert.Pass("Framebuffer validation requires a graphics device.");
            }
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            var texture = new Texture2D(2, 2, TextureFormat.RGBA32, false, true)
            {
                filterMode = FilterMode.Point,
                wrapMode = TextureWrapMode.Clamp,
            };
            texture.SetPixels32(
                new[]
                {
                    new Color32(255, 0, 0, 255),
                    new Color32(0, 255, 0, 255),
                    new Color32(0, 0, 255, 255),
                    new Color32(0, 0, 0, 0),
                }
            );
            texture.Apply();
            var address = new TextureAddress("game/rendered-image");
            harness.AssetStorage.EnqueueValue(texture);
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    preparedAssets: new PreparedAsset[] { new PreparedAsset.Texture(address) },
                    objects: new[] { Image(NewObjectId(), new ImageState(address, 2, 2)) }
                )
            );
            harness.Runner.Connect();
            var cameraObject = new GameObject("Image render test camera");
            Camera camera = cameraObject.AddComponent<Camera>();
            camera.transform.position = new UnityEngine.Vector3(0, 0, -10);
            camera.orthographic = true;
            camera.orthographicSize = 1;
            camera.clearFlags = CameraClearFlags.SolidColor;
            camera.backgroundColor = UnityEngine.Color.black;
            var target = new RenderTexture(64, 64, 24, RenderTextureFormat.ARGB32);
            camera.targetTexture = target;
            Assert.That(target.Create(), Is.True);

            camera.Render();

            RenderTexture previous = RenderTexture.active;
            RenderTexture.active = target;
            var rendered = new Texture2D(64, 64, TextureFormat.RGBA32, false, true);
            rendered.ReadPixels(new Rect(0, 0, 64, 64), 0, 0);
            rendered.Apply();
            RenderTexture.active = previous;
            AssertPrimary(rendered.GetPixel(16, 16), 0);
            AssertPrimary(rendered.GetPixel(48, 16), 1);
            AssertPrimary(rendered.GetPixel(16, 48), 2);
            UnityEngine.Color transparentRegion = rendered.GetPixel(48, 48);
            Assert.That(transparentRegion.maxColorComponent, Is.LessThan(0.05f));
            Assert.That(transparentRegion.a, Is.GreaterThan(0.95f));
            Object.DestroyImmediate(rendered);
            camera.targetTexture = null;
            target.Release();
            Object.DestroyImmediate(target);
            Object.DestroyImmediate(cameraObject);
        }

        [Test]
        public void SnapshotReplacementReleasesDestroyedImageTextureLease()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            Texture2D texture = Texture();
            SessionId session = new(Guid.NewGuid());
            var address = new TextureAddress("game/leased-image");
            var asset = new PreparedAsset.Texture(address);
            harness.AssetStorage.EnqueueValue(texture);
            harness.Transport.EnqueueConnect(
                Response(
                    session,
                    new[] { asset },
                    new[] { Image(NewObjectId(), new ImageState(address, 2, 1)) }
                )
            );
            harness.Runner.Connect();
            FakeAssetHandle handle = harness.AssetStorage.Handles.Single(value =>
                value.Asset == asset
            );
            harness.Transport.EnqueueSubmit(
                Response(session, Array.Empty<PreparedAsset>(), Array.Empty<MasonryGameObject>())
            );

            harness.Runner.Submit(new byte[] { 1 });

            Assert.That(handle.IsDisposed, Is.True);
            Assert.That(harness.Runner.TryGetPreparedAsset(asset, out _), Is.False);
        }

        [Test]
        public void FaceCameraUsesRolledCameraAndRetainsRotationWhenCoincident()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            Texture2D texture = Texture();
            GameObject cameraPrefab = new("Input camera prefab");
            cameraPrefab.SetActive(false);
            cameraPrefab.AddComponent<Camera>();
            var textureAddress = new TextureAddress("game/billboard");
            var cameraAddress = new PrefabAddress("game/input-camera");
            var imageId = NewObjectId();
            var cameraId = NewObjectId();
            harness.AssetStorage.EnqueueValue(texture);
            harness.AssetStorage.EnqueueValue(cameraPrefab);
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    preparedAssets: new PreparedAsset[]
                    {
                        new PreparedAsset.Texture(textureAddress),
                        new PreparedAsset.Prefab(cameraAddress),
                    },
                    objects: new[]
                    {
                        Image(
                            imageId,
                            new ImageState(
                                textureAddress,
                                2,
                                1,
                                ImageFit.Contain,
                                RgbColor.White,
                                1,
                                true
                            )
                        ),
                        new MasonryGameObject(
                            cameraId,
                            new GameObjectKind.Prefab(cameraAddress),
                            new ParentScene.Persistent(),
                            null,
                            true,
                            new LocalTransform(
                                new Masonry.Vector3(0, 0, 10),
                                new Masonry.Quaternion(0, 0, 0.2588190451, 0.9659258263),
                                Masonry.Vector3.One
                            ),
                            Array.Empty<PointerEvent>()
                        ),
                    },
                    inputCameraId: cameraId
                )
            );
            harness.Runner.Connect();

            harness.Runner.RunFrame();

            Transform image = Identity(imageId).transform;
            Camera inputCamera = Identity(cameraId).GetComponent<Camera>();
            Assert.That(
                UnityEngine.Vector3.Angle(
                    -image.forward,
                    inputCamera.transform.position - image.position
                ),
                Is.LessThan(0.001f)
            );
            UnityEngine.Vector3 expectedUp = UnityEngine
                .Vector3.ProjectOnPlane(inputCamera.transform.up, image.forward)
                .normalized;
            Assert.That(UnityEngine.Vector3.Angle(image.up, expectedUp), Is.LessThan(0.001f));
            UnityEngine.Quaternion retained = image.rotation;
            inputCamera.transform.SetPositionAndRotation(
                image.position,
                UnityEngine.Quaternion.Euler(45, 30, 90)
            );

            harness.Runner.RunFrame();

            Assert.That(image.rotation, Is.EqualTo(retained));
            inputCamera.transform.SetPositionAndRotation(
                new UnityEngine.Vector3(0, 10, 0),
                UnityEngine.Quaternion.identity
            );

            harness.Runner.RunFrame();

            Assert.That(
                UnityEngine.Vector3.Angle(-image.forward, UnityEngine.Vector3.up),
                Is.LessThan(0.001f)
            );
            Assert.That(
                UnityEngine.Vector3.Angle(image.up, UnityEngine.Vector3.right),
                Is.LessThan(0.001f)
            );
        }

        [TestCase(0, 1)]
        [TestCase(1, double.NaN)]
        [TestCase(double.MaxValue, 1)]
        public void InvalidImageSizeStopsTheSession(double width, double height)
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            Texture2D texture = Texture();
            var address = new TextureAddress("game/invalid-image");
            harness.AssetStorage.EnqueueValue(texture);
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    preparedAssets: new PreparedAsset[] { new PreparedAsset.Texture(address) },
                    objects: new[] { Image(NewObjectId(), new ImageState(address, width, height)) }
                )
            );

            harness.Runner.Connect();

            Assert.That(Identities(), Is.Empty);
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
            Assert.That(harness.Logger.Records.Last().Message, Does.Contain("finite and positive"));
        }

        [Test]
        public void WrongPreparedTextureTypeStopsTheSession()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            var address = new TextureAddress("game/not-a-texture");
            harness.AssetStorage.EnqueueValue(new GameObject("Wrong texture type"));
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    preparedAssets: new PreparedAsset[] { new PreparedAsset.Texture(address) },
                    objects: new[] { Image(NewObjectId(), new ImageState(address, 1, 1)) }
                )
            );

            harness.Runner.Connect();

            Assert.That(Identities(), Is.Empty);
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
            Assert.That(harness.Logger.Records.Last().Message, Does.Contain("not a Unity Texture"));
        }

        private static MasonryGameObject Image(
            ObjectId id,
            ImageState state,
            params PointerEvent[] pointerEvents
        ) =>
            new(
                id,
                new GameObjectKind.Image(state),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                pointerEvents
            );

        private static Texture2D Texture()
        {
            var texture = new Texture2D(400, 200, TextureFormat.RGBA32, false, true)
            {
                filterMode = FilterMode.Point,
                wrapMode = TextureWrapMode.Repeat,
            };
            return texture;
        }

        private static ObjectId NewObjectId() => new(Guid.NewGuid());

        private static MasonryIdentity Identity(ObjectId id) =>
            Identities().Single(identity => identity.Id == id.Value);

        private static MasonryIdentity[] Identities() =>
            Object
                .FindObjectsByType<MasonryIdentity>(FindObjectsInactive.Include)
                .Where(identity => !FakeMasonryTransport.IsFixtureIdentity(identity))
                .ToArray();

        private static void AssertSize(UnityEngine.Vector3 actual, float x, float y, float z = 0)
        {
            Assert.That(actual.x, Is.EqualTo(x).Within(0.0001f));
            Assert.That(actual.y, Is.EqualTo(y).Within(0.0001f));
            Assert.That(actual.z, Is.EqualTo(z).Within(0.0001f));
        }

        private static void AssertUv(
            UnityEngine.Vector2[] uv,
            float minimumX,
            float minimumY,
            float maximumX,
            float maximumY
        )
        {
            Assert.That(uv.Min(value => value.x), Is.EqualTo(minimumX).Within(0.0001f));
            Assert.That(uv.Min(value => value.y), Is.EqualTo(minimumY).Within(0.0001f));
            Assert.That(uv.Max(value => value.x), Is.EqualTo(maximumX).Within(0.0001f));
            Assert.That(uv.Max(value => value.y), Is.EqualTo(maximumY).Within(0.0001f));
        }

        private static void AssertPrimary(UnityEngine.Color color, int channel)
        {
            Assert.That(color[channel], Is.GreaterThan(0.75f));
            Assert.That(color[(channel + 1) % 3], Is.LessThan(0.1f));
            Assert.That(color[(channel + 2) % 3], Is.LessThan(0.1f));
        }

        private static MasonryTransportResult Response(
            SessionId session,
            PreparedAsset[] assets,
            MasonryGameObject[] objects
        ) =>
            FakeMasonryTransport.ResponseResult(
                new Response(
                    session,
                    new ResponseMessage<Command>[]
                    {
                        new ResponseMessage<Command>.SnapshotMessage(
                            FakeMasonryTransport.CompleteSnapshot(
                                session,
                                preparedAssets: assets,
                                objects: objects
                            )
                        ),
                    }
                )
            );
    }
}

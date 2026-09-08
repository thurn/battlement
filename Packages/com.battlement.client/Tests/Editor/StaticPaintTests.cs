#nullable enable

using System;
using System.Collections;
using System.Collections.Generic;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UiView = Battlement.UiElement.VisualElement;
using UnityColor = UnityEngine.Color;

namespace Battlement.Tests
{
    public sealed class StaticPaintTests
    {
        [TestCase(PaintBlendMode.Screen)]
        [TestCase(PaintBlendMode.Additive)]
        public void BlendingRejectsMaterialMotionBeforeMutatingHost(PaintBlendMode blend)
        {
            using var fixture = new Fixture();
            MotionDescriptor material = fixture.Descriptor with
            {
                Generation = 2,
                Slots = Array.Empty<MotionSlotDescriptor>(),
                Initial = new MotionTargetDescriptor(
                    Array.Empty<MotionPropertyTrack>(),
                    new[]
                    {
                        new MotionPropertyValue(
                            MotionProperty.UnityMaterial,
                            new MotionValue.Discrete(Newtonsoft.Json.Linq.JValue.CreateNull())
                        ),
                    }
                ),
            };
            var error = Assert.Throws<BattlementUiException>(() =>
                fixture.Update(
                    new UiView
                    {
                        Name = "must-not-apply",
                        Paint = new PaintStyle(BlendMode: blend),
                        Motion = material,
                    }
                )
            );
            Assert.That(error!.Message, Does.Contain("owns the host material"));
            Assert.That(fixture.Target.name, Is.Not.EqualTo("must-not-apply"));
            Assert.That(fixture.Child.parent, Is.SameAs(fixture.Target));

            fixture.Update(new UiView { Motion = material });
            error = Assert.Throws<BattlementUiException>(() =>
                fixture.Update(
                    new UiView { Name = "must-not-apply", Paint = new PaintStyle(BlendMode: blend) }
                )
            );
            Assert.That(error!.Message, Does.Contain("owns the host material"));
            Assert.That(fixture.Target.name, Is.Not.EqualTo("must-not-apply"));
        }

        [UnityTest]
        public IEnumerator BlendRemovalReleasesMaterialOwnership()
        {
            RequireGraphics();
            using var fixture = new Fixture();
            StyleMaterialDefinition original = fixture.Target.style.unityMaterial;
            fixture.Update(new UiView { Paint = new PaintStyle(BlendMode: PaintBlendMode.Screen) });
            MotionDescriptor material = fixture.Descriptor with
            {
                Generation = 2,
                Slots = Array.Empty<MotionSlotDescriptor>(),
                Initial = new MotionTargetDescriptor(
                    Array.Empty<MotionPropertyTrack>(),
                    new[]
                    {
                        new MotionPropertyValue(
                            MotionProperty.UnityMaterial,
                            new MotionValue.Discrete(Newtonsoft.Json.Linq.JValue.CreateNull())
                        ),
                    }
                ),
            };
            Assert.Throws<BattlementUiException>(() =>
                fixture.Update(new UiView { Motion = material })
            );
            Assert.Throws<BattlementUiException>(() =>
                BattlementMotionPropertyWriter.Write(
                    fixture.Target,
                    MotionProperty.UnityMaterial,
                    new MotionValue.Discrete(Newtonsoft.Json.Linq.JValue.CreateNull())
                )
            );
            Assert.That(fixture.Child.parent, Is.Not.SameAs(fixture.Target));
            fixture.Update(new UiView { Paint = Prop<PaintStyle>.Reset(), Motion = material });
            Assert.That(fixture.Child.parent, Is.SameAs(fixture.Target));
            Assert.That(fixture.Target.style.unityMaterial, Is.EqualTo(original));
            BattlementMotionPropertyWriter.Write(
                fixture.Target,
                MotionProperty.UnityMaterial,
                new MotionValue.Discrete(Newtonsoft.Json.Linq.JValue.CreateNull())
            );
            Assert.That(fixture.Target.style.unityMaterial.keyword, Is.EqualTo(StyleKeyword.None));
            yield return null;
            LogAssert.NoUnexpectedReceived();
        }

        [TestCase(PaintBlendMode.Screen)]
        [TestCase(PaintBlendMode.Additive)]
        public void NonViewBlendIsRejectedDuringProtocolValidation(PaintBlendMode blend)
        {
            var error = Assert.Throws<BattlementUiException>(() =>
                BattlementUiElementValidator.Validate(
                    new UiElement.Box { Paint = new PaintStyle(BlendMode: blend) },
                    true
                )
            );
            Assert.That(error!.Message, Does.Contain("decorative View"));
        }

        [UnityTest]
        public IEnumerator RemovingMotionPreservesCompositionUntilPaintIsRemoved()
        {
            RequireGraphics();
            using var fixture = new Fixture();
            fixture.Update(new UiView { Paint = new PaintStyle(BlendMode: PaintBlendMode.Screen) });
            fixture.Update(new UiView { Motion = Prop<MotionDescriptor>.Reset() });
            Assert.That(fixture.Child.parent, Is.Not.SameAs(fixture.Target));
            fixture.Update(new UiView { Paint = new PaintStyle(BlendMode: PaintBlendMode.Normal) });
            var material = new Material(Resources.Load<Shader>("BattlementComposite"));
            try
            {
                fixture.Target.style.unityMaterial = material;
                StyleMaterialDefinition latest = fixture.Target.style.unityMaterial;
                fixture.Update(new UiView { Paint = Prop<PaintStyle>.Reset() });
                Assert.That(fixture.Target.style.unityMaterial, Is.EqualTo(latest));
            }
            finally
            {
                fixture.Target.style.unityMaterial = StyleKeyword.None;
                Object.DestroyImmediate(material);
            }
            yield return null;
            LogAssert.NoUnexpectedReceived();
        }

        [UnityTest]
        public IEnumerator OrderedShadowsFollowTransparentArtwork()
        {
            RequireGraphics();
            using var fixture = new Fixture();
            var texture = new RenderTexture(128, 128, 24);
            texture.Create();
            PanelSettings panel = fixture.Owned.GetComponent<UIDocument>().panelSettings;
            panel.targetTexture = texture;
            panel.clearColor = true;
            panel.colorClearValue = UnityColor.black;
            try
            {
                fixture.Update(
                    new UiView
                    {
                        Paint = new PaintStyle(
                            Background: new PaintFill.Gradient(
                                new Gradient.Radial(
                                    new[] { 0.5, 0.5 },
                                    new[] { 0.1, 0.1 },
                                    new[]
                                    {
                                        new GradientStop(new Color(1, 1, 1, 1), 0.7),
                                        new GradientStop(new Color(1, 1, 1, 0), 1),
                                    }
                                )
                            ),
                            PaintFilter: new UiFilterFunction[]
                            {
                                new UiFilterFunction.DropShadow(
                                    new Shadow(20, 0, 0, 0, new Color(1, 0, 0, 1), false)
                                ),
                                new UiFilterFunction.DropShadow(
                                    new Shadow(0, 20, 0, 0, new Color(0, 0, 1, 1), false)
                                ),
                            }
                        ),
                    }
                );
                for (int frame = 0; frame < 8; frame++)
                {
                    UnityEditor.EditorApplication.QueuePlayerLoopUpdate();
                    yield return null;
                }
                LogAssert.NoUnexpectedReceived();
                if (SystemInfo.graphicsDeviceType == UnityEngine.Rendering.GraphicsDeviceType.Null)
                    yield break;
                Texture2D pixels = Read(texture);
                try
                {
                    Assert.That(pixels.GetPixel(50, 128 - 50).g, Is.GreaterThan(0.9));
                    Assert.That(pixels.GetPixel(70, 128 - 50).r, Is.GreaterThan(0.9));
                    Assert.That(pixels.GetPixel(50, 128 - 70).b, Is.GreaterThan(0.9));
                    Assert.That(
                        pixels.GetPixel(70, 128 - 70).b,
                        Is.GreaterThan(0.9),
                        "The second shadow must include the first shadow's alpha."
                    );
                    Assert.That(
                        pixels.GetPixel(30, 128 - 30).maxColorComponent,
                        Is.LessThan(0.05),
                        "Transparent artwork must not cast an element-sized shadow."
                    );
                }
                finally
                {
                    Object.DestroyImmediate(pixels);
                }
            }
            finally
            {
                panel.targetTexture = null;
                texture.Release();
                Object.DestroyImmediate(texture);
            }
        }

        [UnityTest]
        public IEnumerator CompoundClipMasksDescendantsAndScreensOverBackdrop()
        {
            RequireGraphics();
            using var fixture = new Fixture();
            var texture = new RenderTexture(128, 128, 24);
            texture.Create();
            PanelSettings panel = fixture.Owned.GetComponent<UIDocument>().panelSettings;
            panel.targetTexture = texture;
            panel.clearColor = true;
            panel.colorClearValue = UnityColor.blue;
            try
            {
                fixture.Target.style.overflow = Overflow.Hidden;
                fixture.Target.parent.style.backgroundColor = UnityColor.blue;
                fixture.Child.style.backgroundColor = UnityColor.red;
                fixture.Update(
                    new UiView
                    {
                        Paint = new PaintStyle(
                            SubtreeClip: new PaintClipPath(
                                new[] { Square(0, 100), Square(30, 70) },
                                PaintFillRule.EvenOdd
                            ),
                            BlendMode: PaintBlendMode.Screen
                        ),
                    }
                );
                for (int frame = 0; frame < 12; frame++)
                {
                    UnityEditor.EditorApplication.QueuePlayerLoopUpdate();
                    yield return null;
                }
                LogAssert.NoUnexpectedReceived();
                if (SystemInfo.graphicsDeviceType == UnityEngine.Rendering.GraphicsDeviceType.Null)
                    yield break;
                Texture2D pixels = Read(texture);
                try
                {
                    UnityColor ring = pixels.GetPixel(15, 128 - 50);
                    UnityColor hole = pixels.GetPixel(50, 128 - 50);
                    Assert.That(ring.r, Is.GreaterThan(0.9), "Child must survive inside the ring.");
                    Assert.That(
                        ring.b,
                        Is.GreaterThan(0.9),
                        "Screen must preserve the blue backdrop."
                    );
                    Assert.That(
                        hole.r,
                        Is.LessThan(0.1),
                        "The compound hole must remove child paint."
                    );
                    Assert.That(hole.b, Is.GreaterThan(0.9));
                }
                finally
                {
                    Object.DestroyImmediate(pixels);
                }
            }
            finally
            {
                panel.targetTexture = null;
                texture.Release();
                Object.DestroyImmediate(texture);
            }
        }

        private static void RequireGraphics()
        {
            if (SystemInfo.graphicsDeviceType == UnityEngine.Rendering.GraphicsDeviceType.Null)
                Assert.Pass("Pixel assertions require a graphics-enabled Unity test run.");
        }

        private static IReadOnlyList<IReadOnlyList<UiLength>> Square(double low, double high) =>
            new[]
            {
                new[] { UiLength.FromComponents(low, 0), UiLength.FromComponents(low, 0) },
                new[] { UiLength.FromComponents(high, 0), UiLength.FromComponents(low, 0) },
                new[] { UiLength.FromComponents(high, 0), UiLength.FromComponents(high, 0) },
                new[] { UiLength.FromComponents(low, 0), UiLength.FromComponents(high, 0) },
            };

        [UnityTest]
        public IEnumerator EllipticalAlphaFalloffUsesBothRadii()
        {
            RequireGraphics();
            using var fixture = new Fixture();
            var texture = new RenderTexture(128, 128, 24);
            texture.Create();
            PanelSettings panel = fixture.Owned.GetComponent<UIDocument>().panelSettings;
            panel.targetTexture = texture;
            panel.clearColor = true;
            panel.colorClearValue = UnityColor.black;
            try
            {
                fixture.Target.style.height = 40;
                fixture.Update(
                    new UiView
                    {
                        Paint = new PaintStyle(
                            Background: new PaintFill.Gradient(
                                new Gradient.Radial(
                                    new[] { 0.5, 0.5 },
                                    new[] { 0.5, 0.5 },
                                    new[]
                                    {
                                        new GradientStop(new Color(1, 1, 1, 1), 0),
                                        new GradientStop(new Color(0, 0, 0, 0), 1),
                                    }
                                )
                            )
                        ),
                    }
                );
                for (int frame = 0; frame < 8; frame++)
                {
                    UnityEditor.EditorApplication.QueuePlayerLoopUpdate();
                    yield return null;
                }
                LogAssert.NoUnexpectedReceived();
                if (SystemInfo.graphicsDeviceType == UnityEngine.Rendering.GraphicsDeviceType.Null)
                    yield break;
                Texture2D pixels = Read(texture);
                try
                {
                    float across = pixels.GetPixel(25, 128 - 20).r;
                    float down = pixels.GetPixel(50, 128 - 10).r;
                    Assert.That(
                        across,
                        Is.EqualTo(down).Within(0.04),
                        "Equal elliptical radii must have equal opacity."
                    );
                    Assert.That(across, Is.GreaterThan(0.2));
                    Assert.That(pixels.GetPixel(50, 128 - 20).r, Is.GreaterThan(across + 0.15));
                    Assert.That(pixels.GetPixel(3, 128 - 20).r, Is.LessThan(across - 0.15));
                }
                finally
                {
                    Object.DestroyImmediate(pixels);
                }
            }
            finally
            {
                panel.targetTexture = null;
                texture.Release();
                Object.DestroyImmediate(texture);
            }
        }

        [UnityTest]
        public IEnumerator StaticFillUsesBorderBoxAndSurvivesPaddingUpdates()
        {
            bool hasGraphics =
                SystemInfo.graphicsDeviceType != UnityEngine.Rendering.GraphicsDeviceType.Null;
            using var fixture = new Fixture();
            var texture = new RenderTexture(128, 128, 24);
            texture.Create();
            PanelSettings panel = fixture.Owned.GetComponent<UIDocument>().panelSettings;
            panel.targetTexture = texture;
            panel.clearColor = true;
            panel.colorClearValue = UnityColor.black;
            try
            {
                foreach (int padding in new[] { 8, 20 })
                {
                    fixture.Update(new UiView { Style = Padding(padding) });
                    for (int frame = 0; frame < 8; frame++)
                    {
                        UnityEditor.EditorApplication.QueuePlayerLoopUpdate();
                        yield return null;
                    }
                    Assert.That(fixture.Target.layout.width, Is.EqualTo(100).Within(0.01));
                    Assert.That(fixture.Child.layout.x, Is.EqualTo(padding).Within(0.01));
                    Assert.That(
                        fixture.Child.layout.width,
                        Is.EqualTo(100 - 2 * padding).Within(0.01)
                    );
                    AssertColor(fixture.Target, 1, 0, 0);
                    if (!hasGraphics)
                        continue;
                    Texture2D pixels = Read(texture);
                    try
                    {
                        Assert.That(pixels.GetPixel(50, 128 - 3).r, Is.GreaterThan(0.9));
                        Assert.That(pixels.GetPixel(3, 128 - 3).r, Is.LessThan(0.1));
                        Assert.That(pixels.GetPixel(50, 128 - 50).r, Is.GreaterThan(0.9));
                    }
                    finally
                    {
                        Object.DestroyImmediate(pixels);
                    }
                }
            }
            finally
            {
                panel.targetTexture = null;
                texture.Release();
                Object.DestroyImmediate(texture);
            }
        }

        [UnityTest]
        public IEnumerator TwoStopGradientLayerUsesNativeGradientFill()
        {
            using var fixture = new Fixture();
            var texture = new RenderTexture(128, 128, 24);
            texture.Create();
            PanelSettings panel = fixture.Owned.GetComponent<UIDocument>().panelSettings;
            panel.targetTexture = texture;
            try
            {
                fixture.Update(
                    new UiView
                    {
                        Paint = new PaintStyle(
                            Layers: new[]
                            {
                                new PaintLayer(
                                    new PaintFill.Gradient(
                                        new Gradient.Linear(
                                            90,
                                            new[]
                                            {
                                                new GradientStop(new Color(1, 0, 0, 1), 0),
                                                new GradientStop(new Color(0, 0, 1, 1), 1),
                                            }
                                        )
                                    )
                                ),
                            }
                        ),
                    }
                );
                for (int frame = 0; frame < 8; frame++)
                {
                    UnityEditor.EditorApplication.QueuePlayerLoopUpdate();
                    yield return null;
                }
                LogAssert.NoUnexpectedReceived();
                if (SystemInfo.graphicsDeviceType != UnityEngine.Rendering.GraphicsDeviceType.Null)
                {
                    Texture2D pixels = Read(texture);
                    try
                    {
                        Assert.That(pixels.GetPixel(50, 128 - 3).r, Is.GreaterThan(0.95));
                        Assert.That(pixels.GetPixel(50, 128 - 97).b, Is.GreaterThan(0.95));
                    }
                    finally
                    {
                        Object.DestroyImmediate(pixels);
                    }
                }
            }
            finally
            {
                panel.targetTexture = null;
                texture.Release();
                Object.DestroyImmediate(texture);
            }
        }

        [UnityTest]
        public IEnumerator InsetShadowStaysInsideItsLayerAndHonorsSharpOffset()
        {
            using var fixture = new Fixture();
            var texture = new RenderTexture(128, 128, 24);
            texture.Create();
            PanelSettings panel = fixture.Owned.GetComponent<UIDocument>().panelSettings;
            panel.targetTexture = texture;
            panel.clearColor = true;
            panel.colorClearValue = UnityColor.black;
            try
            {
                fixture.Update(
                    new UiView
                    {
                        Paint = new PaintStyle(
                            Background: new PaintFill.Color(new Color(1, 0, 0, 1)),
                            Layers: new[]
                            {
                                new PaintLayer(
                                    new PaintFill.Color(new Color(0, 0, 1, 1)),
                                    BoundsInset: Insets(20),
                                    BoxShadow: new[]
                                    {
                                        new Shadow(0, 0, 12, 0, new Color(0, 0, 0, 0.5), true),
                                        new Shadow(0, -3, 0, 0, new Color(0, 1, 0, 1), true),
                                    }
                                ),
                            }
                        ),
                    }
                );
                for (int frame = 0; frame < 8; frame++)
                {
                    UnityEditor.EditorApplication.QueuePlayerLoopUpdate();
                    yield return null;
                }
                LogAssert.NoUnexpectedReceived();
                if (SystemInfo.graphicsDeviceType == UnityEngine.Rendering.GraphicsDeviceType.Null)
                    yield break;
                Texture2D pixels = Read(texture);
                try
                {
                    UnityColor outsideLayer = pixels.GetPixel(16, 128 - 50);
                    Assert.That(
                        outsideLayer.r,
                        Is.GreaterThan(0.95),
                        "Inset shadow covered its border"
                    );
                    UnityColor middle = pixels.GetPixel(50, 128 - 50);
                    Assert.That(
                        middle.b,
                        Is.GreaterThan(0.95),
                        "Blur darkened the distant interior"
                    );
                    UnityColor bottom = pixels.GetPixel(50, 128 - 78);
                    Assert.That(
                        bottom.g,
                        Is.GreaterThan(0.9),
                        "Negative Y must paint a sharp bottom edge"
                    );
                    UnityColor aboveBottom = pixels.GetPixel(50, 128 - 73);
                    Assert.That(
                        aboveBottom.g,
                        Is.LessThan(0.1),
                        "Zero blur must not paint a thick stroke"
                    );
                }
                finally
                {
                    Object.DestroyImmediate(pixels);
                }
            }
            finally
            {
                panel.targetTexture = null;
                texture.Release();
                Object.DestroyImmediate(texture);
            }
        }

        [Test]
        public void StaticPaintChangesUnderFocusAndRestoresLatestOrdinaryFill()
        {
            using var fixture = new Fixture();

            fixture.Sample();
            AssertColor(fixture.Target, 1, 0, 0);
            fixture.Focus(true);
            fixture.Sample();
            AssertColor(fixture.Target, 0, 0, 1);
            PaintStyle green = Paint(0, 1, 0);
            fixture.Update(new UiView { Paint = green });
            fixture.Sample();
            AssertColor(fixture.Target, 0, 0, 1);
            fixture.Focus(false);
            fixture.Sample();
            AssertColor(fixture.Target, 0, 1, 0);
            fixture.Update(
                new UiView
                {
                    Style = new UiStyle(BackgroundColor: UiStyle.Set(new Color(1, 1, 0, 1))),
                }
            );
            fixture.Sample();
            AssertColor(fixture.Target, 0, 1, 0);
            fixture.Update(new UiView { Paint = Prop<PaintStyle>.Reset() });
            fixture.Sample();
            AssertColor(fixture.Target, 1, 1, 0);
            fixture.Focus(true);
            fixture.Sample();
            AssertColor(fixture.Target, 0, 0, 1);
            fixture.Focus(false);
            fixture.Sample();
            AssertColor(fixture.Target, 1, 1, 0);
        }

        [Test]
        public void EmptyPaintFilterResetsAnOrdinaryHost()
        {
            var target = new VisualElement();

            Assert.DoesNotThrow(() =>
                BattlementMotionPropertyWriter.Write(
                    target,
                    MotionProperty.PaintFilter,
                    new MotionValue.FilterList(Array.Empty<UiFilterFunction>())
                )
            );
        }

        [Test]
        public void PaintValidationRejectsPolygonsWithFewerThanThreeVertices()
        {
            IReadOnlyList<IReadOnlyList<UiLength>> polygon = new IReadOnlyList<UiLength>[]
            {
                new[] { UiLength.FromComponents(0, 0), UiLength.FromComponents(0, 0) },
                new[] { UiLength.FromComponents(0, 100), UiLength.FromComponents(0, 100) },
            };

            Assert.Throws<BattlementUiException>(() =>
                BattlementPaintProperties.Validate(
                    Prop<PaintStyle>.Set(
                        new PaintStyle(
                            Background: new PaintFill.Color(new Color(1, 0, 0, 1)),
                            ClipPolygon: polygon
                        )
                    )
                )
            );
            Assert.Throws<BattlementUiException>(() =>
                BattlementPaintProperties.Validate(
                    Prop<PaintStyle>.Set(
                        new PaintStyle(
                            Layers: new[]
                            {
                                new PaintLayer(
                                    new PaintFill.Color(new Color(1, 0, 0, 1)),
                                    ClipPolygon: polygon
                                ),
                            }
                        )
                    )
                )
            );
        }

        [Test]
        public void LayerOnlyPaintPreservesTheOrdinaryBackground()
        {
            using var fixture = new Fixture();
            fixture.Update(
                new UiView
                {
                    Style = new UiStyle(BackgroundColor: UiStyle.Set(new Color(0, 1, 0, 1))),
                    Paint = new PaintStyle(
                        Layers: new[]
                        {
                            new PaintLayer(
                                new PaintFill.Color(new Color(1, 0, 0, 1)),
                                BoundsInset: Insets(20)
                            ),
                        }
                    ),
                }
            );

            Assert.That(fixture.Target.style.backgroundColor.value, Is.EqualTo(UnityColor.green));
            Assert.That(BattlementAdvancedPaint.TryGet(fixture.Target, out var paint), Is.True);
            Assert.That(paint.HasStaticPaint, Is.True);
        }

        private static void AssertColor(VisualElement target, double red, double green, double blue)
        {
            var color = (MotionValue.Color)
                BattlementMotionPropertyWriter.Read(target, MotionProperty.BackgroundColor);
            Assert.That(color.Value.Red, Is.EqualTo(red).Within(0.001));
            Assert.That(color.Value.Green, Is.EqualTo(green).Within(0.001));
            Assert.That(color.Value.Blue, Is.EqualTo(blue).Within(0.001));
        }

        private static UiStyle Padding(int value) =>
            new(
                PaddingTop: UiStyle.Set<UiLength>(new UiLength.Px(value)),
                PaddingRight: UiStyle.Set<UiLength>(new UiLength.Px(value)),
                PaddingBottom: UiStyle.Set<UiLength>(new UiLength.Px(value)),
                PaddingLeft: UiStyle.Set<UiLength>(new UiLength.Px(value))
            );

        private static IReadOnlyList<UiLength> Insets(int value) =>
            new[]
            {
                UiLength.FromComponents(value, 0),
                UiLength.FromComponents(value, 0),
                UiLength.FromComponents(value, 0),
                UiLength.FromComponents(value, 0),
            };

        private static Texture2D Read(RenderTexture texture)
        {
            RenderTexture previous = RenderTexture.active;
            try
            {
                RenderTexture.active = texture;
                var pixels = new Texture2D(
                    texture.width,
                    texture.height,
                    TextureFormat.RGBA32,
                    false
                );
                pixels.ReadPixels(new UnityEngine.Rect(0, 0, texture.width, texture.height), 0, 0);
                pixels.Apply();
                return pixels;
            }
            finally
            {
                RenderTexture.active = previous;
            }
        }

        private static PaintStyle Paint(double red, double green, double blue) =>
            new(
                Background: new PaintFill.Color(new Color(red, green, blue, 1)),
                ClipPolygon: new IReadOnlyList<UiLength>[]
                {
                    new[] { UiLength.FromComponents(0, 20), UiLength.FromComponents(0, 0) },
                    new[] { UiLength.FromComponents(0, 80), UiLength.FromComponents(0, 0) },
                    new[] { UiLength.FromComponents(0, 100), UiLength.FromComponents(0, 50) },
                    new[] { UiLength.FromComponents(0, 80), UiLength.FromComponents(0, 100) },
                    new[] { UiLength.FromComponents(0, 20), UiLength.FromComponents(0, 100) },
                    new[] { UiLength.FromComponents(0, 0), UiLength.FromComponents(0, 50) },
                }
            );

        private sealed class Fixture : IDisposable
        {
            public readonly ObjectId Host = new(Guid.NewGuid());
            public readonly GameObject Owned;
            public readonly BattlementUiDocuments Documents = new();
            public readonly MotionDescriptor Descriptor;
            public readonly VisualElement Target;
            public readonly VisualElement Child;

            public Fixture()
            {
                ObjectId document = new(Guid.NewGuid());
                ObjectId root = new(Guid.NewGuid());
                ObjectId child = new(Guid.NewGuid());
                Owned = BattlementUiDocuments.CreateGameObject(
                    new GameObjectKind.UiDocumentState(
                        root,
                        new PanelSettingsValue(ScaleMode: PanelScaleMode.ConstantPixelSize)
                    )
                );
                Descriptor = new MotionDescriptor(
                    Host,
                    Host,
                    1,
                    false,
                    new[]
                    {
                        new MotionSlotDescriptor(
                            1,
                            1,
                            MotionLayer.FocusVisible,
                            new MotionTargetDescriptor(
                                new[]
                                {
                                    new MotionPropertyTrack(
                                        MotionProperty.BackgroundColor,
                                        new MotionValue[]
                                        {
                                            new MotionValue.Color(new Color(0, 0, 1, 1)),
                                        },
                                        new TransitionDefinition(
                                            new TransitionGenerator.Tween(
                                                1,
                                                new MotionEasing[] { new MotionEasing.Linear() },
                                                null
                                            ),
                                            0,
                                            new MotionRepeat.None(),
                                            0,
                                            MotionRepeatType.Loop
                                        )
                                    ),
                                },
                                Array.Empty<MotionPropertyValue>()
                            ),
                            new MotionCallbackSubscriptions(
                                false,
                                false,
                                false,
                                false,
                                false,
                                false
                            )
                        ),
                    },
                    new MotionClockSource.Controlled(Host),
                    ReducedMotionPolicy.Never
                );
                Documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            document,
                            root,
                            Children: new[]
                            {
                                new UiNode(
                                    Host,
                                    new UiView
                                    {
                                        Focusable = true,
                                        Motion = Descriptor,
                                        Paint = Paint(1, 0, 0),
                                        Style = new UiStyle(
                                            Width: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(100)
                                            ),
                                            Height: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(100)
                                            )
                                        ),
                                    },
                                    Children: new[]
                                    {
                                        new UiNode(
                                            child,
                                            new UiView
                                            {
                                                Style = new UiStyle(
                                                    Width: UiStyle.Set<UiLengthOrAuto>(
                                                        new UiLengthOrAuto.Percent(100)
                                                    ),
                                                    Height: UiStyle.Set<UiLengthOrAuto>(
                                                        new UiLengthOrAuto.Percent(100)
                                                    )
                                                ),
                                            }
                                        ),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == document ? Owned : null
                );
                Documents.TryGet(Host, out VisualElement? target);
                Documents.TryGet(child, out VisualElement? inner);
                Target = target!;
                Child = inner!;
            }

            public void Update(UiView value) =>
                Documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(Host, value)
                    )
                );

            public void Focus(bool value)
            {
                Documents.PerformAction(
                    new CommandBody.VisualElement.PerformAction(
                        Host,
                        value ? new VisualElementAction.Focus() : new VisualElementAction.Blur()
                    )
                );
                if (!value)
                    return;
                using KeyDownEvent key = KeyDownEvent.GetPooled(
                    '\0',
                    KeyCode.LeftArrow,
                    EventModifiers.None
                );
                key.target = Target;
                Target.SendEvent(key);
            }

            public void Sample()
            {
                Documents.MotionWorldForTests.AdvanceControlledClock(Host, 100_000);
                Documents.MotionWorldForTests.PreLayout();
                Documents.MotionWorldForTests.PostLayout();
            }

            public void Dispose()
            {
                Documents.Dispose();
                Object.DestroyImmediate(Owned);
            }
        }
    }
}

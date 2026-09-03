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
    public sealed class RoundedPaintTests
    {
        [UnityTest]
        public IEnumerator PaintFollowsRoundedStyleUpdatesAndExplicitContours()
        {
            if (SystemInfo.graphicsDeviceType == UnityEngine.Rendering.GraphicsDeviceType.Null)
                Assert.Pass("Rendered contour assertions require a graphics device.");
            using var fixture = new Fixture();
            foreach (bool gradient in new[] { false, true })
            {
                fixture.Update(100, 32, gradient);
                yield return Settle();
                fixture.Check(
                    (3, 3, false),
                    (50, 3, true),
                    (3, 50, true),
                    (50, 50, true),
                    (96, 96, false)
                );
                fixture.Update(100, 0, gradient);
                yield return Settle();
                fixture.Check((3, 3, true), (96, 96, true));
                fixture.Update(100, 1000, gradient);
                yield return Settle();
                fixture.Check((3, 3, false), (50, 3, true), (50, 50, true), (96, 96, false));
                fixture.Update(4, 32, gradient);
                yield return Settle();
                fixture.Check((1, 50, true), (8, 50, false));
                fixture.Update(0, 32, gradient);
                yield return Settle();
                fixture.Check((1, 50, false), (50, 50, false));
            }
            fixture.Update(100, 0, true, topLeft: 32);
            yield return Settle();
            fixture.Check((3, 3, false), (96, 3, true), (3, 96, true), (96, 96, true));
            fixture.Update(100, 32, true, polygon: true);
            yield return Settle();
            fixture.Check((3, 3, true), (50, 3, true), (3, 50, true), (96, 96, false));
            fixture.Update(100, 32, true, inset: true);
            yield return Settle();
            fixture.Check((3, 50, false), (12, 50, true), (12, 12, true), (96, 50, false));
        }

        private static IEnumerator Settle()
        {
            for (int frame = 0; frame < 8; frame++)
            {
                UnityEditor.EditorApplication.QueuePlayerLoopUpdate();
                yield return null;
            }
        }

        private sealed class Fixture : IDisposable
        {
            private readonly ObjectId host = new(Guid.NewGuid());
            private readonly BattlementUiDocuments documents = new();
            private readonly GameObject owned;
            private readonly RenderTexture texture = new(128, 128, 24);
            private readonly PanelSettings panel;
            private (bool Gradient, bool Inset, bool Polygon)? paintState;

            public Fixture()
            {
                ObjectId document = new(Guid.NewGuid());
                ObjectId root = new(Guid.NewGuid());
                owned = BattlementUiDocuments.CreateGameObject(
                    new GameObjectKind.UiDocumentState(
                        root,
                        new PanelSettingsValue(ScaleMode: PanelScaleMode.ConstantPixelSize)
                    )
                );
                texture.Create();
                panel = owned.GetComponent<UIDocument>().panelSettings;
                panel.targetTexture = texture;
                panel.clearColor = true;
                panel.colorClearValue = UnityColor.black;
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            document,
                            root,
                            Children: new[] { new UiNode(host, new UiView()) }
                        ),
                    },
                    id => id == document ? owned : null
                );
            }

            public void Update(
                int width,
                int radius,
                bool gradient,
                int? topLeft = null,
                bool inset = false,
                bool polygon = false
            )
            {
                PaintFill background = gradient
                    ? new PaintFill.Gradient(
                        new Gradient.Linear(
                            0,
                            new[]
                            {
                                new GradientStop(new Color(1, 0, 0, 1), 0),
                                new GradientStop(new Color(0, 0, 1, 1), 1),
                            }
                        )
                    )
                    : new PaintFill.Color(new Color(1, 0, 0, 1));
                IReadOnlyList<UiLength>? clipInset = inset
                    ? new[]
                    {
                        UiLength.FromComponents(10, 0),
                        UiLength.FromComponents(10, 0),
                        UiLength.FromComponents(10, 0),
                        UiLength.FromComponents(10, 0),
                    }
                    : null;
                IReadOnlyList<IReadOnlyList<UiLength>>? clipPolygon = polygon
                    ? new IReadOnlyList<UiLength>[]
                    {
                        new[] { UiLength.FromComponents(0, 0), UiLength.FromComponents(0, 0) },
                        new[] { UiLength.FromComponents(0, 100), UiLength.FromComponents(0, 0) },
                        new[] { UiLength.FromComponents(0, 0), UiLength.FromComponents(0, 100) },
                    }
                    : null;
                bool paintChanged = paintState != (gradient, inset, polygon);
                paintState = (gradient, inset, polygon);
                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            host,
                            new UiView
                            {
                                Style = new UiStyle(
                                    Width: UiStyle.Set<UiLengthOrAuto>(
                                        new UiLengthOrAuto.Px(width)
                                    ),
                                    Height: UiStyle.Set<UiLengthOrAuto>(new UiLengthOrAuto.Px(100)),
                                    BorderTopLeftRadius: UiStyle.Set<UiLength>(
                                        new UiLength.Px(topLeft ?? radius)
                                    ),
                                    BorderTopRightRadius: UiStyle.Set<UiLength>(
                                        new UiLength.Px(radius)
                                    ),
                                    BorderBottomRightRadius: UiStyle.Set<UiLength>(
                                        new UiLength.Px(radius)
                                    ),
                                    BorderBottomLeftRadius: UiStyle.Set<UiLength>(
                                        new UiLength.Px(radius)
                                    )
                                ),
                                Paint = paintChanged
                                    ? new PaintStyle(background, clipPolygon, null, clipInset)
                                    : default(Prop<PaintStyle>),
                            }
                        )
                    )
                );
                documents.MotionWorldForTests.PreLayout();
                documents.MotionWorldForTests.PostLayout();
            }

            public void Check(params (int X, int Y, bool Filled)[] probes)
            {
                RenderTexture previous = RenderTexture.active;
                var pixels = new Texture2D(128, 128, TextureFormat.RGBA32, false);
                try
                {
                    RenderTexture.active = texture;
                    pixels.ReadPixels(new UnityEngine.Rect(0, 0, 128, 128), 0, 0);
                    pixels.Apply();
                    foreach (var probe in probes)
                    {
                        UnityColor color = pixels.GetPixel(probe.X, 127 - probe.Y);
                        float brightness = Mathf.Max(color.r, color.b);
                        string expected = probe.Filled ? "filled" : "clear";
                        Assert.That(
                            brightness,
                            probe.Filled ? Is.GreaterThan(0.2f) : Is.LessThan(0.05f),
                            $"Pixel ({probe.X}, {probe.Y}) should be {expected}."
                        );
                    }
                }
                finally
                {
                    RenderTexture.active = previous;
                    Object.DestroyImmediate(pixels);
                }
            }

            public void Dispose()
            {
                documents.Dispose();
                panel.targetTexture = null;
                texture.Release();
                Object.DestroyImmediate(texture);
                Object.DestroyImmediate(owned);
            }
        }
    }
}

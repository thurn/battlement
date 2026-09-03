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

        private static Texture2D Read(RenderTexture texture)
        {
            RenderTexture previous = RenderTexture.active;
            try
            {
                RenderTexture.active = texture;
                var pixels = new Texture2D(128, 128, TextureFormat.RGBA32, false);
                pixels.ReadPixels(new UnityEngine.Rect(0, 0, 128, 128), 0, 0);
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

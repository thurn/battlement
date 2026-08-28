#nullable enable

using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UiBox = Battlement.UiElement.Box;

namespace Battlement.Tests
{
    public sealed class BattlementGeometrySamplerTests
    {
        [UnityTest]
        public IEnumerator SamplesScaledElementViewportAndAvailabilityInOnePass()
        {
            ObjectId documentId = Id(1);
            ObjectId rootId = Id(2);
            ObjectId elementId = Id(3);
            ObjectId detachedId = Id(4);
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(
                    rootId,
                    new PanelSettingsValue(ScaleMode: PanelScaleMode.ConstantPixelSize, Scale: 2)
                )
            );
            var documents = new BattlementUiDocuments();
            var displays = new FakeDisplays();
            displays.Set(
                0,
                new BattlementDisplayGeometry(
                    800,
                    600,
                    new UnityEngine.Rect(10, 20, 780, 550),
                    1.25,
                    144,
                    DisplayOrientation.Landscape
                )
            );
            displays.Set(
                1,
                new BattlementDisplayGeometry(
                    1024,
                    768,
                    new UnityEngine.Rect(0, 0, 1024, 768),
                    1,
                    null,
                    DisplayOrientation.LandscapeFlipped
                )
            );
            try
            {
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new[]
                            {
                                new UiNode(
                                    elementId,
                                    new UiBox
                                    {
                                        Style = new UiStyle(
                                            Position: UiStyle.Set(UiPosition.Absolute),
                                            Left: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(12)
                                            ),
                                            Top: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(18)
                                            ),
                                            Width: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(40)
                                            ),
                                            Height: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(30)
                                            )
                                        ),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                yield return null;
                yield return null;

                var sampler = new BattlementGeometrySampler(documents, displays);
                sampler.Apply(
                    new GeometryObservationUpdate(
                        new[]
                        {
                            Observation(10, new GeometryObservationTarget.UiElement(elementId)),
                            Observation(11, new GeometryObservationTarget.UiElement(detachedId)),
                            Observation(
                                12,
                                new GeometryObservationTarget.Viewport(new DisplayId(0))
                            ),
                            Observation(
                                13,
                                new GeometryObservationTarget.Viewport(new DisplayId(1))
                            ),
                            Observation(
                                14,
                                new GeometryObservationTarget.Viewport(new DisplayId(2))
                            ),
                        },
                        Array.Empty<GeometryObservationId>()
                    )
                );

                GeometryObservationBatch first = sampler.Sample()!;
                Assert.That(first.Generation.Value, Is.EqualTo(1));
                Assert.That(first.Changed, Has.Count.EqualTo(5));
                var element = (GeometryValue.Element)
                    ((GeometryObservationResult.Current)Value(first, 10).Result).Value;
                Assert.That(element.Value.Layout, Is.EqualTo(new Battlement.Rect(12, 18, 40, 30)));
                Assert.That(element.Value.ViewportBound.DisplayId, Is.EqualTo(new DisplayId(0)));
                Assert.That(element.Value.ViewportBound.Width, Is.EqualTo(80).Within(0.01));
                Assert.That(element.Value.ViewportBound.Height, Is.EqualTo(60).Within(0.01));
                Assert.That(element.Value.ViewportFromLocal.M11, Is.EqualTo(2).Within(0.01));
                Assert.That(element.Value.ViewportFromLocal.M22, Is.EqualTo(2).Within(0.01));
                Assert.That(element.Value.PanelId, Is.EqualTo(rootId));

                var viewport = (GeometryValue.Viewport)
                    ((GeometryObservationResult.Current)Value(first, 12).Result).Value;
                Assert.That(
                    viewport.Value.Viewport,
                    Is.EqualTo(new ViewportRect(0, 0, 800, 600, new DisplayId(0)))
                );
                Assert.That(
                    viewport.Value.SafeArea,
                    Is.EqualTo(new ViewportRect(10, 30, 780, 550, new DisplayId(0)))
                );
                Assert.That(viewport.Value.Scale, Is.EqualTo(1.25));
                Assert.That(viewport.Value.Dpi, Is.EqualTo(144));
                AssertUnavailable(first, 11, GeometryUnavailable.Detached);
                AssertUnavailable(first, 14, GeometryUnavailable.DisplayUnavailable);
                Assert.That(sampler.Sample(), Is.Null);

                displays.Set(
                    0,
                    new BattlementDisplayGeometry(
                        800,
                        600,
                        new UnityEngine.Rect(20, 40, 760, 520),
                        1.25,
                        144,
                        DisplayOrientation.Landscape
                    )
                );
                GeometryObservationBatch safeAreaChange = sampler.Sample()!;
                Assert.That(safeAreaChange.Generation.Value, Is.EqualTo(3));
                Assert.That(safeAreaChange.Changed, Has.Count.EqualTo(1));
                Assert.That(safeAreaChange.Changed[0].ObservationId, Is.EqualTo(ObservationId(12)));

                Assert.That(documents.TryGet(elementId, out VisualElement? target), Is.True);
                target!.style.display = DisplayStyle.None;
                yield return null;
                GeometryObservationBatch hidden = sampler.Sample()!;
                Assert.That(hidden.Changed, Has.Count.EqualTo(1));
                AssertUnavailable(hidden, 10, GeometryUnavailable.Hidden);

                target.style.display = DisplayStyle.Flex;
                target.style.scale = new Scale(new UnityEngine.Vector3(0, 1, 1));
                yield return null;
                GeometryObservationBatch singular = sampler.Sample()!;
                Assert.That(singular.Changed, Has.Count.EqualTo(1));
                AssertUnavailable(singular, 10, GeometryUnavailable.ProjectionUnavailable);

                target.style.scale = new Scale(UnityEngine.Vector3.one);
                UnityEngine.UIElements.PanelSettings panel = owned
                    .GetComponent<UIDocument>()
                    .panelSettings;
                panel.targetDisplay = 1;
                yield return null;
                GeometryObservationBatch mapped = sampler.Sample()!;
                Assert.That(mapped.Changed, Has.Count.EqualTo(1));
                var mappedElement = (GeometryValue.Element)
                    ((GeometryObservationResult.Current)Value(mapped, 10).Result).Value;
                Assert.That(
                    mappedElement.Value.ViewportBound.DisplayId,
                    Is.EqualTo(new DisplayId(1))
                );
                Assert.That(
                    mappedElement.Value.ViewportFromLocal.M11,
                    Is.EqualTo(target.panel.scaledPixelsPerPoint).Within(0.01)
                );

                panel.targetDisplay = 7;
                yield return null;
                GeometryObservationBatch missingDisplay = sampler.Sample()!;
                Assert.That(missingDisplay.Changed, Has.Count.EqualTo(1));
                AssertUnavailable(missingDisplay, 10, GeometryUnavailable.DisplayUnavailable);
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        private static GeometryObservation Observation(
            int value,
            GeometryObservationTarget target
        ) => new(ObservationId(value), target);

        private static GeometryObservationId ObservationId(int value) =>
            new(new Guid(value, 0, 0, new byte[8]));

        private static ObjectId Id(int value) => new(new Guid(value, 0, 0, new byte[8]));

        private static GeometryObservationValue Value(GeometryObservationBatch batch, int id) =>
            batch.Changed.Single(value => value.ObservationId.Equals(ObservationId(id)));

        private static void AssertUnavailable(
            GeometryObservationBatch batch,
            int id,
            GeometryUnavailable expected
        ) =>
            Assert.That(
                ((GeometryObservationResult.Unavailable)Value(batch, id).Result).Reason,
                Is.EqualTo(expected)
            );

        private sealed class FakeDisplays : IBattlementGeometryDisplaySource
        {
            private readonly Dictionary<DisplayId, BattlementDisplayGeometry> values = new();

            public void Set(uint id, BattlementDisplayGeometry value) =>
                values[new DisplayId(id)] = value;

            public bool TryGet(DisplayId id, out BattlementDisplayGeometry geometry) =>
                values.TryGetValue(id, out geometry);
        }
    }
}

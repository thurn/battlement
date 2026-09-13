#nullable enable

using System;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class GeometryProtocolTests
    {
        private static readonly ObjectId ObjectId = new(GuidAt(1));
        private static readonly ObjectId PanelId = new(GuidAt(2));

        [Test]
        public void InvalidBatchesDoNotAdvanceTheAcceptedGeneration()
        {
            var registry = new GeometryRegistry();
            registry.Apply(
                new GeometryObservationUpdate(Targets(), Array.Empty<GeometryObservationId>())
            );
            GeometryObservationValue valid = Values()[0];

            Assert.Throws<ArgumentException>(() =>
                registry.Accept(
                    new GeometryObservationBatch(new GeometryGeneration(1), new[] { valid, valid })
                )
            );
            Assert.Throws<ArgumentException>(() =>
                registry.Accept(
                    new GeometryObservationBatch(
                        new GeometryGeneration(1),
                        new[]
                        {
                            new GeometryObservationValue(
                                Targets()[0].ObservationId,
                                new GeometryObservationResult.Current(
                                    new GeometryValue.Element(
                                        new ElementGeometry(
                                            new Rect(1, 2, 30, 40),
                                            ViewportRect(),
                                            OverflowingProjective(),
                                            Identity(),
                                            PanelId
                                        )
                                    )
                                )
                            ),
                        }
                    )
                )
            );
            Assert.Throws<ArgumentException>(() =>
                registry.Accept(
                    new GeometryObservationBatch(
                        new GeometryGeneration(1),
                        new[]
                        {
                            new GeometryObservationValue(
                                Targets()[0].ObservationId,
                                new GeometryObservationResult.Current(
                                    new GeometryValue.Viewport(Viewport())
                                )
                            ),
                        }
                    )
                )
            );
            Assert.Throws<ArgumentException>(() =>
                registry.Accept(
                    new GeometryObservationBatch(
                        new GeometryGeneration(1),
                        new[]
                        {
                            new GeometryObservationValue(
                                Targets()[2].ObservationId,
                                new GeometryObservationResult.Current(
                                    new GeometryValue.WorldPoint(
                                        new WorldPointGeometry(
                                            new ViewportPoint(double.NaN, 2, new DisplayId(0)),
                                            3,
                                            true
                                        )
                                    )
                                )
                            ),
                        }
                    )
                )
            );

            Assert.DoesNotThrow(() =>
                registry.Accept(new GeometryObservationBatch(new GeometryGeneration(1), Values()))
            );
            Assert.Throws<ArgumentException>(() =>
                registry.Accept(
                    new GeometryObservationBatch(
                        new GeometryGeneration(1),
                        Array.Empty<GeometryObservationValue>()
                    )
                )
            );
        }

        [Test]
        public void MalformedGenerationAndDuplicateRegistryUpdateRejectAtomically()
        {
            var registry = new GeometryRegistry();
            GeometryObservation duplicate = Targets()[0];
            Assert.Throws<ArgumentException>(() =>
                registry.Apply(
                    new GeometryObservationUpdate(
                        new[] { duplicate, duplicate },
                        Array.Empty<GeometryObservationId>()
                    )
                )
            );
            Assert.That(registry.Targets, Is.Empty);

            registry.Apply(
                new GeometryObservationUpdate(
                    new[] { duplicate },
                    Array.Empty<GeometryObservationId>()
                )
            );
            Assert.Throws<ArgumentException>(() =>
                registry.Apply(
                    new GeometryObservationUpdate(
                        new[]
                        {
                            new GeometryObservation(
                                duplicate.ObservationId,
                                new GeometryObservationTarget.Viewport(new DisplayId(0))
                            ),
                        },
                        new[] { duplicate.ObservationId }
                    )
                )
            );
            Assert.That(
                registry.Targets[duplicate.ObservationId],
                Is.TypeOf<GeometryObservationTarget.UiElement>()
            );
        }

        private static GeometryObservation[] Targets() =>
            new[]
            {
                Observation(0, new GeometryObservationTarget.UiElement(ObjectId)),
                Observation(1, new GeometryObservationTarget.Viewport(new DisplayId(0))),
                Observation(
                    2,
                    new GeometryObservationTarget.WorldOrigin(ObjectId, new CameraTarget.Input())
                ),
                Observation(
                    3,
                    new GeometryObservationTarget.WorldAnchor(
                        ObjectId,
                        new AnchorName("head"),
                        new CameraTarget.Object(PanelId)
                    )
                ),
                Observation(
                    4,
                    new GeometryObservationTarget.WorldRenderedBounds(
                        ObjectId,
                        new CameraTarget.Input()
                    )
                ),
            };

        private static GeometryObservationValue[] Values() =>
            new[]
            {
                Value(
                    0,
                    new GeometryValue.Element(
                        new ElementGeometry(
                            new Rect(1, 2, 30, 40),
                            ViewportRect(),
                            Identity(),
                            Identity(),
                            PanelId
                        )
                    )
                ),
                Value(1, new GeometryValue.Viewport(Viewport())),
                Value(
                    2,
                    new GeometryValue.WorldPoint(
                        new WorldPointGeometry(new ViewportPoint(1, 2, new DisplayId(0)), 3, true)
                    )
                ),
                Value(
                    3,
                    new GeometryValue.WorldPoint(
                        new WorldPointGeometry(new ViewportPoint(4, 5, new DisplayId(0)), 6, false)
                    )
                ),
                Value(
                    4,
                    new GeometryValue.WorldBounds(
                        new WorldBoundsGeometry(ViewportRect(), 1, 8, true)
                    )
                ),
            };

        private static GeometryObservation Observation(
            int index,
            GeometryObservationTarget target
        ) => new(new GeometryObservationId(GuidAt(10 + index)), target);

        private static GeometryObservationValue Value(int index, GeometryValue value) =>
            new(
                new GeometryObservationId(GuidAt(10 + index)),
                new GeometryObservationResult.Current(value)
            );

        private static ViewportGeometry Viewport() =>
            new(ViewportRect(), ViewportRect(), 2, 144, DisplayOrientation.Landscape);

        private static ViewportRect ViewportRect() => new(0, 0, 1920, 1080, new DisplayId(0));

        private static Projective2 Identity() => new(1, 0, 0, 0, 1, 0, 0, 0, 1);

        private static Projective2 OverflowingProjective() =>
            new(1e308, 1e308, 1e308, 1e308, 1e308, 1e308, 1e308, 1e308, 1e308);

        private static Guid GuidAt(int value) => Guid.Parse($"10000000-0000-0000-0000-{value:D12}");
    }
}

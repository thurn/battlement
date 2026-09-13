#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    internal static partial class BattlementFlatBufferMaterializer
    {
        private sealed partial class UiProperties
        {
            private Prop<PaintStyle> PaintProperty(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.PaintStylePropertyValue,
                    property => PaintStyle(property.ValueAsPaintStylePropertyValue())
                );

            private static PaintStyle PaintStyle(Wire.PaintStylePropertyValue value)
            {
                var layers = new PaintLayer[value.LayersLength];
                for (int index = 0; index < layers.Length; index++)
                    layers[index] = PaintLayer(value.Layers(index) ?? throw Missing("paint layer"));
                return new PaintStyle(
                    value.Background.HasValue ? PaintFill(value.Background.Value) : null,
                    value.FiltersLength == 0
                        ? null
                        : PaintFilters(value.FiltersLength, value.Filters),
                    value.ClipPolygonLength == 0
                        ? null
                        : PaintPolygon(value.ClipPolygonLength, value.ClipPolygon),
                    value.BoxShadowsLength == 0
                        ? null
                        : PaintShadows(value.BoxShadowsLength, value.BoxShadows),
                    value.ClipInsets.HasValue ? PaintInsets(value.ClipInsets.Value) : null,
                    layers,
                    value.SubtreeClip.HasValue ? PaintClipPath(value.SubtreeClip.Value) : null,
                    value.HasBlendMode ? (PaintBlendMode?)(byte)value.BlendMode : null
                );
            }

            private static PaintLayer PaintLayer(Wire.PaintLayerValue value) =>
                new(
                    PaintFill(value.Background ?? throw Missing("paint layer background")),
                    value.FiltersLength == 0
                        ? null
                        : PaintFilters(value.FiltersLength, value.Filters),
                    value.ClipPolygonLength == 0
                        ? null
                        : PaintPolygon(value.ClipPolygonLength, value.ClipPolygon),
                    value.BoxShadowsLength == 0
                        ? null
                        : PaintShadows(value.BoxShadowsLength, value.BoxShadows),
                    value.ClipInsets.HasValue ? PaintInsets(value.ClipInsets.Value) : null,
                    value.BoundsInsets.HasValue ? PaintInsets(value.BoundsInsets.Value) : null
                );

            private static PaintFill PaintFill(Wire.PaintFillValue value) =>
                value.Kind switch
                {
                    Wire.PaintFillKind.Color when value.Color.HasValue => new PaintFill.Color(
                        Rgba(value.Color.Value)
                    ),
                    Wire.PaintFillKind.Gradient when value.Gradient.HasValue =>
                        new PaintFill.Gradient(PaintGradient(value.Gradient.Value)),
                    _ => throw new InvalidDataException(
                        "Paint fill kind and payload do not match."
                    ),
                };

            private static Gradient PaintGradient(Wire.PaintGradientValue value)
            {
                var stops = new GradientStop[value.StopsLength];
                for (int index = 0; index < stops.Length; index++)
                {
                    Wire.PaintGradientStop stop =
                        value.Stops(index) ?? throw Missing("paint gradient stop");
                    stops[index] = new GradientStop(
                        Rgba(stop.Color ?? throw Missing("paint gradient stop color")),
                        stop.Position
                    );
                }
                return value.Kind switch
                {
                    Wire.PaintGradientKind.Linear => new Gradient.Linear(value.Angle, stops),
                    Wire.PaintGradientKind.Radial => new Gradient.Radial(
                        Point(value.Center, "paint radial center"),
                        Point(value.Radius, "paint radial radius"),
                        stops
                    ),
                    _ => throw new InvalidDataException("Unknown paint gradient kind."),
                };
            }

            private static IReadOnlyList<double> Point(Wire.F32Vector2? value, string field)
            {
                Wire.F32Vector2 point = value ?? throw Missing(field);
                return new double[] { point.X, point.Y };
            }

            private static IReadOnlyList<UiFilterFunction> PaintFilters(
                int length,
                Func<int, Wire.PaintFilterValue?> read
            )
            {
                var result = new UiFilterFunction[length];
                for (int index = 0; index < length; index++)
                {
                    Wire.PaintFilterValue item = read(index) ?? throw Missing("paint filter");
                    result[index] = item.Kind switch
                    {
                        Wire.PaintFilterKind.Brightness => new UiFilterFunction.Brightness(
                            item.Amount
                        ),
                        Wire.PaintFilterKind.DropShadow when item.Shadow.HasValue =>
                            new UiFilterFunction.DropShadow(PaintShadow(item.Shadow.Value)),
                        _ => throw new InvalidDataException(
                            "Paint filter kind and payload do not match."
                        ),
                    };
                }
                return result;
            }

            private static IReadOnlyList<Shadow> PaintShadows(
                int length,
                Func<int, Wire.PaintShadowValue?> read
            )
            {
                var result = new Shadow[length];
                for (int index = 0; index < length; index++)
                    result[index] = PaintShadow(read(index) ?? throw Missing("paint shadow"));
                return result;
            }

            private static Shadow PaintShadow(Wire.PaintShadowValue value) =>
                new(
                    value.X,
                    value.Y,
                    value.Blur,
                    value.Spread,
                    Rgba(value.Color ?? throw Missing("paint shadow color")),
                    value.Inset
                );

            private static IReadOnlyList<IReadOnlyList<UiLength>> PaintPolygon(
                int length,
                Func<int, Wire.PaintLengthPoint?> read
            )
            {
                var result = new IReadOnlyList<UiLength>[length];
                for (int index = 0; index < length; index++)
                    result[index] = PaintPoint(read(index) ?? throw Missing("paint point"));
                return result;
            }

            private static IReadOnlyList<UiLength> PaintPoint(Wire.PaintLengthPoint value) =>
                new[]
                {
                    UiLength(value.X ?? throw Missing("paint point x"), false),
                    UiLength(value.Y ?? throw Missing("paint point y"), false),
                };

            private static IReadOnlyList<UiLength> PaintInsets(Wire.PaintInsetsValue value) =>
                new[]
                {
                    UiLength(value.Top ?? throw Missing("paint inset top"), false),
                    UiLength(value.Right ?? throw Missing("paint inset right"), false),
                    UiLength(value.Bottom ?? throw Missing("paint inset bottom"), false),
                    UiLength(value.Left ?? throw Missing("paint inset left"), false),
                };

            private static PaintClipPath PaintClipPath(Wire.PaintClipPathValue value)
            {
                var contours = new IReadOnlyList<IReadOnlyList<UiLength>>[value.ContoursLength];
                for (int contourIndex = 0; contourIndex < contours.Length; contourIndex++)
                {
                    Wire.PaintContour contour =
                        value.Contours(contourIndex) ?? throw Missing("paint clip contour");
                    var points = new IReadOnlyList<UiLength>[contour.PointsLength];
                    for (int pointIndex = 0; pointIndex < points.Length; pointIndex++)
                        points[pointIndex] = PaintPoint(
                            contour.Points(pointIndex) ?? throw Missing("paint clip point")
                        );
                    contours[contourIndex] = points;
                }
                return new PaintClipPath(contours, (PaintFillRule)(byte)value.FillRule);
            }
        }
    }
}

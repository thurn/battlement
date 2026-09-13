#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    internal sealed class BattlementMotionValueWriter
    {
        private const int MaximumCollectionValues = 262_144;
        private readonly FlatBufferBuilder builder;
        private Offset<Wire.MotionTransform>[] transformOffsets = Array.Empty<
            Offset<Wire.MotionTransform>
        >();
        private Offset<Wire.MotionFilter>[] filterOffsets = Array.Empty<
            Offset<Wire.MotionFilter>
        >();
        private Offset<Wire.MotionPropertyValue>[] propertyOffsets = Array.Empty<
            Offset<Wire.MotionPropertyValue>
        >();

        internal BattlementMotionValueWriter(FlatBufferBuilder builder) => this.builder = builder;

        internal MotionValueOffset Write(MotionValue value)
        {
            return value switch
            {
                MotionValue.Scalar scalar => Scalar(scalar.Value, false),
                MotionValue.Length length => Length(length.Value),
                MotionValue.Color color => Color(color.Value),
                MotionValue.Vector2 vector => Vector(vector.Value, 2),
                MotionValue.Vector3 vector => Vector(vector.Value, 3),
                MotionValue.Angle angle => Scalar(angle.Value, true),
                MotionValue.TransformList transforms => Transforms(transforms.Value),
                MotionValue.FilterList filters => Filters(filters.Value),
                MotionValue.ShadowList shadows => Shadows(shadows.Value),
                MotionValue.Gradient gradient => Gradient(gradient.Value),
                MotionValue.ClipInset inset => ClipInset(inset.Value),
                MotionValue.ClipPolygon polygon => ClipPolygon(polygon.Value),
                MotionValue.Discrete discrete => Discrete(discrete.Value),
                _ => throw new InvalidDataException("Unknown Motion value."),
            };
        }

        internal VectorOffset WritePropertyValues(IReadOnlyList<MotionPropertyValue> values)
        {
            Limit(values.Count);
            if (propertyOffsets.Length < values.Count)
                Array.Resize(ref propertyOffsets, values.Count);
            for (int index = 0; index < values.Count; index++)
            {
                MotionPropertyValue value = values[index];
                if ((uint)value.Property > (uint)MotionProperty.Layout)
                    throw new InvalidDataException("Unknown Motion property.");
                MotionValueOffset encoded = Write(value.Value);
                propertyOffsets[index] = Wire.MotionPropertyValue.CreateMotionPropertyValue(
                    builder,
                    (Wire.MotionProperty)value.Property,
                    encoded.Type,
                    encoded.Offset
                );
            }
            Wire.MotionPresentationSample.StartValuesVector(builder, values.Count);
            for (int index = values.Count - 1; index >= 0; index--)
                builder.AddOffset(propertyOffsets[index].Value);
            return builder.EndVector();
        }

        private MotionValueOffset Scalar(double value, bool angle)
        {
            float encoded = Float(value);
            if (angle)
            {
                Offset<Wire.AngleMotionValue> result = Wire.AngleMotionValue.CreateAngleMotionValue(
                    builder,
                    encoded
                );
                return new(Wire.MotionValue.AngleMotionValue, result.Value);
            }
            Offset<Wire.ScalarMotionValue> scalar = Wire.ScalarMotionValue.CreateScalarMotionValue(
                builder,
                encoded
            );
            return new(Wire.MotionValue.ScalarMotionValue, scalar.Value);
        }

        private MotionValueOffset Length(UiLength value)
        {
            (float pixels, float percentage) = Components(value);
            Wire.LengthMotionValue.StartLengthMotionValue(builder);
            Wire.LengthMotionValue.AddValue(
                builder,
                Wire.MotionLength.CreateMotionLength(builder, pixels, percentage)
            );
            return new(
                Wire.MotionValue.LengthMotionValue,
                Wire.LengthMotionValue.EndLengthMotionValue(builder).Value
            );
        }

        private MotionValueOffset Color(Battlement.Color value)
        {
            Validate(value);
            Wire.ColorMotionValue.StartColorMotionValue(builder);
            Wire.ColorMotionValue.AddValue(
                builder,
                Wire.MotionColor.CreateMotionColor(
                    builder,
                    value.Red,
                    value.Green,
                    value.Blue,
                    value.Alpha
                )
            );
            return new(
                Wire.MotionValue.ColorMotionValue,
                Wire.ColorMotionValue.EndColorMotionValue(builder).Value
            );
        }

        private MotionValueOffset Vector(IReadOnlyList<double> values, int expected)
        {
            if (values.Count != expected)
                throw new InvalidDataException($"A Motion vector must have {expected} channels.");
            float x = Float(values[0]);
            float y = Float(values[1]);
            if (expected == 2)
            {
                Wire.Vector2MotionValue.StartVector2MotionValue(builder);
                Wire.Vector2MotionValue.AddValue(
                    builder,
                    Wire.MotionVector2.CreateMotionVector2(builder, x, y)
                );
                return new(
                    Wire.MotionValue.Vector2MotionValue,
                    Wire.Vector2MotionValue.EndVector2MotionValue(builder).Value
                );
            }
            float z = Float(values[2]);
            Wire.Vector3MotionValue.StartVector3MotionValue(builder);
            Wire.Vector3MotionValue.AddValue(
                builder,
                Wire.MotionVector3.CreateMotionVector3(builder, x, y, z)
            );
            return new(
                Wire.MotionValue.Vector3MotionValue,
                Wire.Vector3MotionValue.EndVector3MotionValue(builder).Value
            );
        }

        private MotionValueOffset Transforms(IReadOnlyList<TransformOperation> values)
        {
            Limit(values.Count);
            if (transformOffsets.Length < values.Count)
                Array.Resize(ref transformOffsets, values.Count);
            for (int index = 0; index < values.Count; index++)
            {
                transformOffsets[index] = values[index] switch
                {
                    TransformOperation.Translate value => TransformLengths(
                        Wire.MotionTransformKind.Translate,
                        value.Value
                    ),
                    TransformOperation.Rotate value => TransformScalars(
                        Wire.MotionTransformKind.Rotate,
                        value.Value
                    ),
                    TransformOperation.Skew value => TransformScalars(
                        Wire.MotionTransformKind.Skew,
                        value.Value
                    ),
                    TransformOperation.Scale value => TransformScalars(
                        Wire.MotionTransformKind.Scale,
                        value.Value
                    ),
                    _ => throw new InvalidDataException("Unknown Motion transform."),
                };
            }
            Wire.TransformListMotionValue.StartValuesVector(builder, values.Count);
            for (int index = values.Count - 1; index >= 0; index--)
                builder.AddOffset(transformOffsets[index].Value);
            Offset<Wire.TransformListMotionValue> result =
                Wire.TransformListMotionValue.CreateTransformListMotionValue(
                    builder,
                    builder.EndVector()
                );
            return new(Wire.MotionValue.TransformListMotionValue, result.Value);
        }

        private Offset<Wire.MotionTransform> TransformLengths(
            Wire.MotionTransformKind kind,
            IReadOnlyList<UiLength> values
        )
        {
            if (values.Count != 3)
                throw new InvalidDataException("A Motion translation requires three channels.");
            Limit(values.Count);
            Wire.MotionTransform.StartLengthsVector(builder, values.Count);
            for (int index = values.Count - 1; index >= 0; index--)
            {
                (float pixels, float percentage) = Components(values[index]);
                Wire.MotionLength.CreateMotionLength(builder, pixels, percentage);
            }
            return Wire.MotionTransform.CreateMotionTransform(builder, kind, builder.EndVector());
        }

        private Offset<Wire.MotionTransform> TransformScalars(
            Wire.MotionTransformKind kind,
            IReadOnlyList<double> values
        )
        {
            int expected = kind == Wire.MotionTransformKind.Skew ? 2 : 3;
            if (values.Count != expected)
                throw new InvalidDataException(
                    $"A Motion transform requires {expected} scalar channels."
                );
            Limit(values.Count);
            Wire.MotionTransform.StartScalarsVector(builder, values.Count);
            for (int index = values.Count - 1; index >= 0; index--)
                builder.AddFloat(Float(values[index]));
            return Wire.MotionTransform.CreateMotionTransform(
                builder,
                kind,
                default,
                builder.EndVector()
            );
        }

        private MotionValueOffset Filters(IReadOnlyList<UiFilterFunction> values)
        {
            Limit(values.Count);
            if (filterOffsets.Length < values.Count)
                Array.Resize(ref filterOffsets, values.Count);
            for (int index = 0; index < values.Count; index++)
            {
                filterOffsets[index] = values[index] switch
                {
                    UiFilterFunction.Brightness brightness => BrightnessFilter(brightness.Value),
                    UiFilterFunction.DropShadow shadow => ShadowFilter(shadow.Value),
                    _ => throw new InvalidDataException("Unknown Motion filter."),
                };
            }
            Wire.FilterListMotionValue.StartValuesVector(builder, values.Count);
            for (int index = values.Count - 1; index >= 0; index--)
                builder.AddOffset(filterOffsets[index].Value);
            Offset<Wire.FilterListMotionValue> result =
                Wire.FilterListMotionValue.CreateFilterListMotionValue(
                    builder,
                    builder.EndVector()
                );
            return new(Wire.MotionValue.FilterListMotionValue, result.Value);
        }

        private Offset<Wire.MotionFilter> BrightnessFilter(double value)
        {
            Wire.MotionFilter.StartMotionFilter(builder);
            Wire.MotionFilter.AddBrightness(builder, Float(Nonnegative(value)));
            Wire.MotionFilter.AddKind(builder, Wire.MotionFilterKind.Brightness);
            return Wire.MotionFilter.EndMotionFilter(builder);
        }

        private Offset<Wire.MotionFilter> ShadowFilter(Shadow value)
        {
            Validate(value);
            Wire.MotionFilter.StartMotionFilter(builder);
            Wire.MotionFilter.AddShadow(builder, WriteShadow(value));
            Wire.MotionFilter.AddKind(builder, Wire.MotionFilterKind.DropShadow);
            return Wire.MotionFilter.EndMotionFilter(builder);
        }

        private MotionValueOffset Shadows(IReadOnlyList<Shadow> values)
        {
            Limit(values.Count);
            Wire.ShadowListMotionValue.StartValuesVector(builder, values.Count);
            for (int index = values.Count - 1; index >= 0; index--)
            {
                Validate(values[index]);
                WriteShadow(values[index]);
            }
            Offset<Wire.ShadowListMotionValue> result =
                Wire.ShadowListMotionValue.CreateShadowListMotionValue(
                    builder,
                    builder.EndVector()
                );
            return new(Wire.MotionValue.ShadowListMotionValue, result.Value);
        }

        private MotionValueOffset Gradient(Battlement.Gradient value)
        {
            IReadOnlyList<GradientStop> stops;
            Wire.MotionGradientKind kind;
            float angle = 0;
            float centerX = 0;
            float centerY = 0;
            float radiusX = 0;
            float radiusY = 0;
            switch (value)
            {
                case Battlement.Gradient.Linear linear:
                    kind = Wire.MotionGradientKind.Linear;
                    angle = Float(linear.Angle);
                    stops = linear.Stops;
                    break;
                case Battlement.Gradient.Radial radial:
                    kind = Wire.MotionGradientKind.Radial;
                    if (radial.Center.Count != 2 || radial.Radius.Count != 2)
                        throw new InvalidDataException(
                            "A radial Motion gradient requires two center and radius channels."
                        );
                    centerX = Float(radial.Center[0]);
                    centerY = Float(radial.Center[1]);
                    radiusX = Float(radial.Radius[0]);
                    radiusY = Float(radial.Radius[1]);
                    stops = radial.Stops;
                    break;
                default:
                    throw new InvalidDataException("Unknown Motion gradient.");
            }
            if (stops.Count == 0)
                throw new InvalidDataException("A Motion gradient must contain a stop.");
            Limit(stops.Count);
            Wire.GradientMotionValue.StartStopsVector(builder, stops.Count);
            for (int index = stops.Count - 1; index >= 0; index--)
            {
                GradientStop stop = stops[index];
                Validate(stop.Color);
                Wire.MotionGradientStop.CreateMotionGradientStop(
                    builder,
                    stop.Color.Red,
                    stop.Color.Green,
                    stop.Color.Blue,
                    stop.Color.Alpha,
                    Float(stop.Position)
                );
            }
            VectorOffset stopVector = builder.EndVector();
            Wire.GradientMotionValue.StartGradientMotionValue(builder);
            Wire.GradientMotionValue.AddStops(builder, stopVector);
            if (kind == Wire.MotionGradientKind.Radial)
            {
                Wire.GradientMotionValue.AddRadius(
                    builder,
                    Wire.MotionVector2.CreateMotionVector2(builder, radiusX, radiusY)
                );
                Wire.GradientMotionValue.AddCenter(
                    builder,
                    Wire.MotionVector2.CreateMotionVector2(builder, centerX, centerY)
                );
            }
            else
            {
                Wire.GradientMotionValue.AddAngle(builder, angle);
            }
            Wire.GradientMotionValue.AddKind(builder, kind);
            return new(
                Wire.MotionValue.GradientMotionValue,
                Wire.GradientMotionValue.EndGradientMotionValue(builder).Value
            );
        }

        private MotionValueOffset ClipInset(IReadOnlyList<UiLength> values)
        {
            if (values.Count != 4)
                throw new InvalidDataException("A Motion clip inset requires four lengths.");
            Wire.ClipInsetMotionValue.StartValuesVector(builder, values.Count);
            for (int index = values.Count - 1; index >= 0; index--)
            {
                (float pixels, float percentage) = Components(values[index]);
                Wire.MotionLength.CreateMotionLength(builder, pixels, percentage);
            }
            Offset<Wire.ClipInsetMotionValue> result =
                Wire.ClipInsetMotionValue.CreateClipInsetMotionValue(builder, builder.EndVector());
            return new(Wire.MotionValue.ClipInsetMotionValue, result.Value);
        }

        private MotionValueOffset ClipPolygon(IReadOnlyList<IReadOnlyList<UiLength>> values)
        {
            if (values.Count == 0)
                throw new InvalidDataException("A Motion clip polygon must contain a point.");
            Limit(values.Count);
            Wire.ClipPolygonMotionValue.StartValuesVector(builder, values.Count);
            for (int index = values.Count - 1; index >= 0; index--)
            {
                IReadOnlyList<UiLength> point = values[index];
                if (point.Count != 2)
                    throw new InvalidDataException("A Motion clip point requires two lengths.");
                (float xPixels, float xPercentage) = Components(point[0]);
                (float yPixels, float yPercentage) = Components(point[1]);
                Wire.MotionLengthPoint.CreateMotionLengthPoint(
                    builder,
                    xPixels,
                    xPercentage,
                    yPixels,
                    yPercentage
                );
            }
            Offset<Wire.ClipPolygonMotionValue> result =
                Wire.ClipPolygonMotionValue.CreateClipPolygonMotionValue(
                    builder,
                    builder.EndVector()
                );
            return new(Wire.MotionValue.ClipPolygonMotionValue, result.Value);
        }

        private MotionValueOffset Discrete(MotionDiscreteValue value)
        {
            Offset<Wire.DiscreteMotionValue> result;
            if (value is MotionDiscreteValue.Null)
            {
                result = Wire.DiscreteMotionValue.CreateDiscreteMotionValue(
                    builder,
                    Wire.MotionDiscreteKind.Null
                );
            }
            else if (value is MotionDiscreteValue.String textValue)
            {
                StringOffset encoded = builder.CreateString(textValue.Value);
                result = Wire.DiscreteMotionValue.CreateDiscreteMotionValue(
                    builder,
                    Wire.MotionDiscreteKind.String,
                    encoded
                );
            }
            else
            {
                throw new InvalidDataException("A discrete Motion value must be a string or null.");
            }
            return new(Wire.MotionValue.DiscreteMotionValue, result.Value);
        }

        private Offset<Wire.MotionShadow> WriteShadow(Shadow value)
        {
            return Wire.MotionShadow.CreateMotionShadow(
                builder,
                Float(value.X),
                Float(value.Y),
                Nonnegative(value.Blur),
                Float(value.Spread),
                value.Color.Red,
                value.Color.Green,
                value.Color.Blue,
                value.Color.Alpha,
                value.Inset
            );
        }

        private static (float Pixels, float Percentage) Components(UiLength value)
        {
            return value switch
            {
                UiLength.Px pixels => (Finite(pixels.Value), 0),
                UiLength.Percent percentage => (0, Finite(percentage.Value)),
                UiLength.Calc calc => (
                    Finite(calc.PixelComponent),
                    Finite(calc.PercentageComponent)
                ),
                _ => throw new InvalidDataException("Unknown UI length."),
            };
        }

        private static void Validate(Battlement.Color value)
        {
            if (
                !double.IsFinite(value.Red)
                || !double.IsFinite(value.Green)
                || !double.IsFinite(value.Blue)
                || !double.IsFinite(value.Alpha)
            )
                throw new InvalidDataException("Motion colors must be finite.");
        }

        private static void Validate(Shadow value)
        {
            _ = Float(value.X);
            _ = Float(value.Y);
            _ = Nonnegative(value.Blur);
            _ = Float(value.Spread);
            Validate(value.Color);
        }

        private static float Nonnegative(double value)
        {
            float result = Float(value);
            if (result < 0)
                throw new InvalidDataException("Motion value must be nonnegative.");
            return result;
        }

        private static float Finite(float value) =>
            float.IsFinite(value)
                ? value
                : throw new InvalidDataException("Motion numbers must be finite.");

        private static float Float(double value)
        {
            if (!double.IsFinite(value) || value > float.MaxValue || value < float.MinValue)
                throw new InvalidDataException("Motion number is outside finite float range.");
            return (float)value;
        }

        private static void Limit(int count)
        {
            if (count > MaximumCollectionValues)
                throw new InvalidDataException("A Motion collection has too many values.");
        }
    }

    internal readonly struct MotionValueOffset
    {
        internal MotionValueOffset(Wire.MotionValue type, int offset) =>
            (Type, Offset) = (type, offset);

        internal Wire.MotionValue Type { get; }
        internal int Offset { get; }
    }
}

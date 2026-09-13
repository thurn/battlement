#nullable enable

using System.Collections.Generic;
using System.IO;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    internal static partial class BattlementFlatBufferMaterializer
    {
        internal static MotionSelector MotionSelector(Wire.MotionSelector value) =>
            value.Kind switch
            {
                Wire.MotionSelectorKind.Element when value.ObjectId.HasValue =>
                    new MotionSelector.Element(ObjectId(value.ObjectId)),
                Wire.MotionSelectorKind.Name => new MotionSelector.Name(
                    Required(value.Name, "motion selector name")
                ),
                Wire.MotionSelectorKind.ScopeRoot => new MotionSelector.ScopeRoot(),
                Wire.MotionSelectorKind.Children => new MotionSelector.Children(),
                Wire.MotionSelectorKind.Descendants => new MotionSelector.Descendants(),
                _ => throw new InvalidDataException(
                    "Motion selector kind and payload do not match."
                ),
            };

        internal static MotionValue MotionValue(Wire.MotionValueOperation value) =>
            value.ValueType switch
            {
                Wire.MotionValue.ScalarMotionValue => Scalar(value.ValueAsScalarMotionValue()),
                Wire.MotionValue.LengthMotionValue => Length(value.ValueAsLengthMotionValue()),
                Wire.MotionValue.ColorMotionValue => MotionColor(value.ValueAsColorMotionValue()),
                Wire.MotionValue.Vector2MotionValue => Vector2(value.ValueAsVector2MotionValue()),
                Wire.MotionValue.Vector3MotionValue => Vector3(value.ValueAsVector3MotionValue()),
                Wire.MotionValue.AngleMotionValue => Angle(value.ValueAsAngleMotionValue()),
                Wire.MotionValue.TransformListMotionValue => TransformList(
                    value.ValueAsTransformListMotionValue()
                ),
                Wire.MotionValue.FilterListMotionValue => FilterList(
                    value.ValueAsFilterListMotionValue()
                ),
                Wire.MotionValue.ShadowListMotionValue => ShadowList(
                    value.ValueAsShadowListMotionValue()
                ),
                Wire.MotionValue.GradientMotionValue => Gradient(
                    value.ValueAsGradientMotionValue()
                ),
                Wire.MotionValue.ClipInsetMotionValue => ClipInset(
                    value.ValueAsClipInsetMotionValue()
                ),
                Wire.MotionValue.ClipPolygonMotionValue => ClipPolygon(
                    value.ValueAsClipPolygonMotionValue()
                ),
                Wire.MotionValue.DiscreteMotionValue => Discrete(
                    value.ValueAsDiscreteMotionValue()
                ),
                _ => throw new InvalidDataException("Unknown motion value kind."),
            };

        private static MotionValue MotionValue(Wire.MotionValueEntry value) =>
            value.ValueType switch
            {
                Wire.MotionValue.ScalarMotionValue => Scalar(value.ValueAsScalarMotionValue()),
                Wire.MotionValue.LengthMotionValue => Length(value.ValueAsLengthMotionValue()),
                Wire.MotionValue.ColorMotionValue => MotionColor(value.ValueAsColorMotionValue()),
                Wire.MotionValue.Vector2MotionValue => Vector2(value.ValueAsVector2MotionValue()),
                Wire.MotionValue.Vector3MotionValue => Vector3(value.ValueAsVector3MotionValue()),
                Wire.MotionValue.AngleMotionValue => Angle(value.ValueAsAngleMotionValue()),
                Wire.MotionValue.TransformListMotionValue => TransformList(
                    value.ValueAsTransformListMotionValue()
                ),
                Wire.MotionValue.FilterListMotionValue => FilterList(
                    value.ValueAsFilterListMotionValue()
                ),
                Wire.MotionValue.ShadowListMotionValue => ShadowList(
                    value.ValueAsShadowListMotionValue()
                ),
                Wire.MotionValue.GradientMotionValue => Gradient(
                    value.ValueAsGradientMotionValue()
                ),
                Wire.MotionValue.ClipInsetMotionValue => ClipInset(
                    value.ValueAsClipInsetMotionValue()
                ),
                Wire.MotionValue.ClipPolygonMotionValue => ClipPolygon(
                    value.ValueAsClipPolygonMotionValue()
                ),
                Wire.MotionValue.DiscreteMotionValue => Discrete(
                    value.ValueAsDiscreteMotionValue()
                ),
                _ => throw new InvalidDataException("Unknown motion value kind."),
            };

        private static MotionValue MotionValue(Wire.MotionPropertyValue value) =>
            value.ValueType switch
            {
                Wire.MotionValue.ScalarMotionValue => Scalar(value.ValueAsScalarMotionValue()),
                Wire.MotionValue.LengthMotionValue => Length(value.ValueAsLengthMotionValue()),
                Wire.MotionValue.ColorMotionValue => MotionColor(value.ValueAsColorMotionValue()),
                Wire.MotionValue.Vector2MotionValue => Vector2(value.ValueAsVector2MotionValue()),
                Wire.MotionValue.Vector3MotionValue => Vector3(value.ValueAsVector3MotionValue()),
                Wire.MotionValue.AngleMotionValue => Angle(value.ValueAsAngleMotionValue()),
                Wire.MotionValue.TransformListMotionValue => TransformList(
                    value.ValueAsTransformListMotionValue()
                ),
                Wire.MotionValue.FilterListMotionValue => FilterList(
                    value.ValueAsFilterListMotionValue()
                ),
                Wire.MotionValue.ShadowListMotionValue => ShadowList(
                    value.ValueAsShadowListMotionValue()
                ),
                Wire.MotionValue.GradientMotionValue => Gradient(
                    value.ValueAsGradientMotionValue()
                ),
                Wire.MotionValue.ClipInsetMotionValue => ClipInset(
                    value.ValueAsClipInsetMotionValue()
                ),
                Wire.MotionValue.ClipPolygonMotionValue => ClipPolygon(
                    value.ValueAsClipPolygonMotionValue()
                ),
                Wire.MotionValue.DiscreteMotionValue => Discrete(
                    value.ValueAsDiscreteMotionValue()
                ),
                _ => throw new InvalidDataException("Unknown motion value kind."),
            };

        private static MotionValue Scalar(Wire.ScalarMotionValue value) =>
            new MotionValue.Scalar(value.Value);

        private static MotionValue Length(Wire.LengthMotionValue value) =>
            new MotionValue.Length(MotionLength(value.Value ?? throw Missing("motion length")));

        private static MotionValue MotionColor(Wire.ColorMotionValue value) =>
            new MotionValue.Color(MotionColor(value.Value ?? throw Missing("motion color")));

        private static MotionValue Vector2(Wire.Vector2MotionValue value)
        {
            Wire.MotionVector2 vector = value.Value ?? throw Missing("motion vector2");
            return new MotionValue.Vector2(new double[] { vector.X, vector.Y });
        }

        private static MotionValue Vector3(Wire.Vector3MotionValue value)
        {
            Wire.MotionVector3 vector = value.Value ?? throw Missing("motion vector3");
            return new MotionValue.Vector3(new double[] { vector.X, vector.Y, vector.Z });
        }

        private static MotionValue Angle(Wire.AngleMotionValue value) =>
            new MotionValue.Angle(value.Value);

        private static UiLength MotionLength(Wire.MotionLength value) =>
            UiLength.FromComponents(value.Pixels, value.Percentage);

        private static Color MotionColor(Wire.MotionColor value) =>
            new(value.Red, value.Green, value.Blue, value.Alpha);

        private static MotionValue TransformList(Wire.TransformListMotionValue value)
        {
            var result = new TransformOperation[value.ValuesLength];
            for (int index = 0; index < result.Length; index++)
            {
                Wire.MotionTransform item =
                    value.Values(index) ?? throw Missing("motion transform");
                if (item.Kind == Wire.MotionTransformKind.Translate)
                {
                    var lengths = new UiLength[item.LengthsLength];
                    for (int component = 0; component < lengths.Length; component++)
                        lengths[component] = MotionLength(
                            item.Lengths(component) ?? throw Missing("motion transform length")
                        );
                    result[index] = new TransformOperation.Translate(lengths);
                }
                else
                {
                    var scalars = new double[item.ScalarsLength];
                    for (int component = 0; component < scalars.Length; component++)
                        scalars[component] = item.Scalars(component);
                    result[index] = item.Kind switch
                    {
                        Wire.MotionTransformKind.Rotate => new TransformOperation.Rotate(scalars),
                        Wire.MotionTransformKind.Skew => new TransformOperation.Skew(scalars),
                        Wire.MotionTransformKind.Scale => new TransformOperation.Scale(scalars),
                        _ => throw new InvalidDataException("Unknown motion transform kind."),
                    };
                }
            }
            return new MotionValue.TransformList(result);
        }

        private static MotionValue FilterList(Wire.FilterListMotionValue value)
        {
            var result = new UiFilterFunction[value.ValuesLength];
            for (int index = 0; index < result.Length; index++)
            {
                Wire.MotionFilter item = value.Values(index) ?? throw Missing("motion filter");
                result[index] = item.Kind switch
                {
                    Wire.MotionFilterKind.Brightness => new UiFilterFunction.Brightness(
                        item.Brightness
                    ),
                    Wire.MotionFilterKind.DropShadow when item.Shadow.HasValue =>
                        new UiFilterFunction.DropShadow(MotionShadow(item.Shadow.Value)),
                    _ => throw new InvalidDataException(
                        "Motion filter kind and payload do not match."
                    ),
                };
            }
            return new MotionValue.FilterList(result);
        }

        private static MotionValue ShadowList(Wire.ShadowListMotionValue value)
        {
            var result = new Shadow[value.ValuesLength];
            for (int index = 0; index < result.Length; index++)
                result[index] = MotionShadow(value.Values(index) ?? throw Missing("motion shadow"));
            return new MotionValue.ShadowList(result);
        }

        private static Shadow MotionShadow(Wire.MotionShadow value) =>
            new(value.X, value.Y, value.Blur, value.Spread, MotionColor(value.Color), value.Inset);

        private static MotionValue Gradient(Wire.GradientMotionValue value)
        {
            var stops = new GradientStop[value.StopsLength];
            for (int index = 0; index < stops.Length; index++)
            {
                Wire.MotionGradientStop stop =
                    value.Stops(index) ?? throw Missing("motion gradient stop");
                stops[index] = new GradientStop(MotionColor(stop.Color), stop.Position);
            }
            Battlement.Gradient result = value.Kind switch
            {
                Wire.MotionGradientKind.Linear => new Battlement.Gradient.Linear(
                    value.Angle,
                    stops
                ),
                Wire.MotionGradientKind.Radial
                    when value.Center.HasValue && value.Radius.HasValue =>
                    new Battlement.Gradient.Radial(
                        new double[] { value.Center.Value.X, value.Center.Value.Y },
                        new double[] { value.Radius.Value.X, value.Radius.Value.Y },
                        stops
                    ),
                _ => throw new InvalidDataException(
                    "Motion gradient kind and payload do not match."
                ),
            };
            return new MotionValue.Gradient(result);
        }

        private static MotionValue ClipInset(Wire.ClipInsetMotionValue value)
        {
            var result = new UiLength[value.ValuesLength];
            for (int index = 0; index < result.Length; index++)
                result[index] = MotionLength(
                    value.Values(index) ?? throw Missing("motion clip inset")
                );
            return new MotionValue.ClipInset(result);
        }

        private static MotionValue ClipPolygon(Wire.ClipPolygonMotionValue value)
        {
            var result = new IReadOnlyList<UiLength>[value.ValuesLength];
            for (int index = 0; index < result.Length; index++)
            {
                Wire.MotionLengthPoint point =
                    value.Values(index) ?? throw Missing("motion clip point");
                result[index] = new UiLength[] { MotionLength(point.X), MotionLength(point.Y) };
            }
            return new MotionValue.ClipPolygon(result);
        }

        private static MotionValue Discrete(Wire.DiscreteMotionValue value) =>
            new MotionValue.Discrete(
                value.Kind switch
                {
                    Wire.MotionDiscreteKind.Null when value.StringValue is null =>
                        new MotionDiscreteValue.Null(),
                    Wire.MotionDiscreteKind.String => new MotionDiscreteValue.String(
                        Required(value.StringValue, "discrete motion string")
                    ),
                    _ => throw new InvalidDataException(
                        "Discrete motion kind and payload do not match."
                    ),
                }
            );

        internal static MotionTargetDescriptor MotionTarget(Wire.MotionTargetDescriptor value)
        {
            var tracks = new MotionPropertyTrack[value.TracksLength];
            for (int index = 0; index < tracks.Length; index++)
            {
                Wire.MotionPropertyTrack track =
                    value.Tracks(index) ?? throw Missing("motion property track");
                var values = new MotionValue[track.ValuesLength];
                for (int valueIndex = 0; valueIndex < values.Length; valueIndex++)
                    values[valueIndex] = MotionValue(track.Values(valueIndex)!.Value);
                double[]? times = null;
                if (track.TimesLength > 0)
                {
                    times = new double[track.TimesLength];
                    for (int timeIndex = 0; timeIndex < times.Length; timeIndex++)
                        times[timeIndex] = track.Times(timeIndex);
                }
                tracks[index] = new MotionPropertyTrack(
                    (MotionProperty)(ushort)track.Property,
                    values,
                    Transition(track.Transition ?? throw Missing("motion transition")),
                    times
                );
            }
            var transitionEnd = new MotionPropertyValue[value.TransitionEndLength];
            for (int index = 0; index < transitionEnd.Length; index++)
            {
                Wire.MotionPropertyValue item =
                    value.TransitionEnd(index) ?? throw Missing("motion transition-end property");
                transitionEnd[index] = new MotionPropertyValue(
                    (MotionProperty)(ushort)item.Property,
                    MotionValue(item)
                );
            }
            return new MotionTargetDescriptor(tracks, transitionEnd);
        }

        internal static TransitionDefinition Transition(Wire.TransitionDefinition value)
        {
            MotionRepeat repeat = value.Repeat switch
            {
                Wire.MotionRepeatKind.None => new MotionRepeat.None(),
                Wire.MotionRepeatKind.Count => new MotionRepeat.Count(value.RepeatCount),
                Wire.MotionRepeatKind.Forever => new MotionRepeat.Forever(),
                _ => throw new InvalidDataException("Unknown motion repeat kind."),
            };
            TransitionGenerator generator = value.Generator switch
            {
                Wire.TransitionGeneratorKind.Immediate => new TransitionGenerator.Immediate(),
                Wire.TransitionGeneratorKind.Tween => new TransitionGenerator.Tween(
                    value.DurationMicros,
                    Easings(value),
                    Times(value)
                ),
                Wire.TransitionGeneratorKind.SpringPhysical => new TransitionGenerator.Spring(
                    new SpringConfiguration.Physical(
                        value.Stiffness,
                        value.Damping,
                        value.Mass,
                        value.InitialVelocity,
                        value.RestSpeed,
                        value.RestDelta
                    )
                ),
                Wire.TransitionGeneratorKind.SpringDuration => new TransitionGenerator.Spring(
                    new SpringConfiguration.Duration(value.DurationMicros, value.Bounce, value.Mass)
                ),
                Wire.TransitionGeneratorKind.SpringVisualDuration => new TransitionGenerator.Spring(
                    new SpringConfiguration.VisualDuration(
                        value.DurationMicros,
                        value.Bounce,
                        value.Mass
                    )
                ),
                Wire.TransitionGeneratorKind.Inertia => new TransitionGenerator.Inertia(
                    value.InitialVelocity
                        ?? throw new InvalidDataException(
                            "An inertia transition requires initial velocity."
                        ),
                    value.Power,
                    value.TimeConstantMicros,
                    value.Minimum,
                    value.Maximum,
                    value.RestDelta
                        ?? throw new InvalidDataException(
                            "An inertia transition requires rest delta."
                        ),
                    value.BounceStiffness,
                    value.BounceDamping,
                    InertiaTarget(value)
                ),
                _ => throw new InvalidDataException("Unknown motion transition generator."),
            };
            return new TransitionDefinition(
                generator,
                value.DelayMicros,
                repeat,
                value.RepeatDelayMicros,
                (MotionRepeatType)(byte)value.RepeatType
            );
        }

        private static IReadOnlyList<MotionEasing> Easings(Wire.TransitionDefinition value)
        {
            var result = new MotionEasing[value.EasingsLength];
            for (int index = 0; index < result.Length; index++)
            {
                Wire.MotionEasingDefinition easing =
                    value.Easings(index) ?? throw Missing("motion easing");
                result[index] = easing.Kind switch
                {
                    Wire.MotionEasingKind.Linear => new MotionEasing.Linear(),
                    Wire.MotionEasingKind.EaseIn => new MotionEasing.EaseIn(),
                    Wire.MotionEasingKind.EaseOut => new MotionEasing.EaseOut(),
                    Wire.MotionEasingKind.EaseInOut => new MotionEasing.EaseInOut(),
                    Wire.MotionEasingKind.CubicBezier => new MotionEasing.CubicBezier(
                        FloatValues(easing)
                    ),
                    Wire.MotionEasingKind.Steps => new MotionEasing.Steps(
                        easing.StepCount,
                        (MotionStepPosition)(byte)easing.StepPosition
                    ),
                    _ => throw new InvalidDataException("Unknown motion easing kind."),
                };
            }
            return result;
        }

        private static IReadOnlyList<double> FloatValues(Wire.MotionEasingDefinition value)
        {
            var result = new double[value.CubicBezierLength];
            for (int index = 0; index < result.Length; index++)
                result[index] = value.CubicBezier(index);
            return result;
        }

        private static IReadOnlyList<double>? Times(Wire.TransitionDefinition value)
        {
            if (value.TimesLength == 0)
                return null;
            var result = new double[value.TimesLength];
            for (int index = 0; index < result.Length; index++)
                result[index] = value.Times(index);
            return result;
        }

        private static InertiaTarget InertiaTarget(Wire.TransitionDefinition value) =>
            value.InertiaTarget switch
            {
                Wire.InertiaTargetKind.Identity => new InertiaTarget.Identity(),
                Wire.InertiaTargetKind.NearestMultiple => new InertiaTarget.NearestMultiple(
                    value.InertiaTargetValue
                ),
                Wire.InertiaTargetKind.FloorMultiple => new InertiaTarget.FloorMultiple(
                    value.InertiaTargetValue
                ),
                Wire.InertiaTargetKind.CeilingMultiple => new InertiaTarget.CeilingMultiple(
                    value.InertiaTargetValue
                ),
                Wire.InertiaTargetKind.Clamp => new InertiaTarget.Clamp(
                    value.InertiaTargetValue,
                    value.InertiaTargetMaximum
                ),
                _ => throw new InvalidDataException("Unknown inertia target kind."),
            };
    }
}

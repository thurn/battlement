#nullable enable

using System;
using System.Collections.Generic;

namespace Battlement.UI
{
    internal static class BattlementMotionComposition
    {
        public static MotionValue Compose(
            MotionProperty property,
            MotionValue lower,
            MotionValue sample,
            MotionValue origin,
            MotionValue target,
            uint iteration,
            AnimationComposition composition
        )
        {
            MotionValue current =
                composition == AnimationComposition.Accumulate && iteration != 0
                    ? Accumulate(property, sample, origin, target, iteration)
                    : sample;
            return Combine(property, lower, current);
        }

        private static MotionValue Accumulate(
            MotionProperty property,
            MotionValue sample,
            MotionValue origin,
            MotionValue target,
            uint iteration
        )
        {
            if (Multiply(property))
                return MultiplyValue(sample, Ratio(target, origin), iteration);
            if (sample is MotionValue.TransformList value)
                return new MotionValue.TransformList(
                    AccumulateTransforms(value.Value, origin, target, iteration)
                );
            return Combine(property, sample, ScaleValue(Subtract(target, origin), iteration));
        }

        private static MotionValue Combine(
            MotionProperty property,
            MotionValue left,
            MotionValue right
        )
        {
            if (Multiply(property))
                return MultiplyValue(left, right, 1);
            return (left, right) switch
            {
                (MotionValue.Scalar a, MotionValue.Scalar b) => new MotionValue.Scalar(
                    a.Value + b.Value
                ),
                (MotionValue.Length a, MotionValue.Length b) => new MotionValue.Length(
                    UiLength.FromComponents(
                        a.Value.Pixels + b.Value.Pixels,
                        a.Value.Percentage + b.Value.Percentage
                    )
                ),
                (MotionValue.Angle a, MotionValue.Angle b) => new MotionValue.Angle(
                    a.Value + b.Value
                ),
                (MotionValue.Vector2 a, MotionValue.Vector2 b) => new MotionValue.Vector2(
                    Add(a.Value, b.Value)
                ),
                (MotionValue.Vector3 a, MotionValue.Vector3 b) => new MotionValue.Vector3(
                    Add(a.Value, b.Value)
                ),
                (MotionValue.TransformList a, MotionValue.TransformList b) =>
                    new MotionValue.TransformList(CombineTransforms(a.Value, b.Value)),
                _ => throw Invalid("CSS additive animation received incompatible values."),
            };
        }

        private static MotionValue Subtract(MotionValue left, MotionValue right) =>
            (left, right) switch
            {
                (MotionValue.Scalar a, MotionValue.Scalar b) => new MotionValue.Scalar(
                    a.Value - b.Value
                ),
                (MotionValue.Length a, MotionValue.Length b) => new MotionValue.Length(
                    UiLength.FromComponents(
                        a.Value.Pixels - b.Value.Pixels,
                        a.Value.Percentage - b.Value.Percentage
                    )
                ),
                (MotionValue.Angle a, MotionValue.Angle b) => new MotionValue.Angle(
                    a.Value - b.Value
                ),
                (MotionValue.Vector2 a, MotionValue.Vector2 b) => new MotionValue.Vector2(
                    Subtract(a.Value, b.Value)
                ),
                (MotionValue.Vector3 a, MotionValue.Vector3 b) => new MotionValue.Vector3(
                    Subtract(a.Value, b.Value)
                ),
                _ => throw Invalid("CSS accumulation received incompatible values."),
            };

        private static MotionValue ScaleValue(MotionValue value, uint count) =>
            value switch
            {
                MotionValue.Scalar item => new MotionValue.Scalar(item.Value * count),
                MotionValue.Length item => new MotionValue.Length(
                    UiLength.FromComponents(
                        item.Value.Pixels * count,
                        item.Value.Percentage * count
                    )
                ),
                MotionValue.Angle item => new MotionValue.Angle(item.Value * count),
                MotionValue.Vector2 item => new MotionValue.Vector2(Scale(item.Value, count)),
                MotionValue.Vector3 item => new MotionValue.Vector3(Scale(item.Value, count)),
                _ => throw Invalid("CSS accumulation received an unsupported value."),
            };

        private static MotionValue Ratio(MotionValue target, MotionValue origin) =>
            (target, origin) switch
            {
                (MotionValue.Scalar a, MotionValue.Scalar b) => new MotionValue.Scalar(
                    Ratio(a.Value, b.Value)
                ),
                (MotionValue.Vector2 a, MotionValue.Vector2 b) => new MotionValue.Vector2(
                    Ratio(a.Value, b.Value)
                ),
                _ => throw Invalid("CSS multiplicative accumulation requires scale values."),
            };

        private static MotionValue MultiplyValue(MotionValue left, MotionValue right, uint power) =>
            (left, right) switch
            {
                (MotionValue.Scalar a, MotionValue.Scalar b) => new MotionValue.Scalar(
                    a.Value * Math.Pow(b.Value, power)
                ),
                (MotionValue.Vector2 a, MotionValue.Vector2 b) => new MotionValue.Vector2(
                    Multiply(a.Value, b.Value, power)
                ),
                _ => throw Invalid("CSS multiplicative composition requires scale values."),
            };

        private static IReadOnlyList<TransformOperation> CombineTransforms(
            IReadOnlyList<TransformOperation> left,
            IReadOnlyList<TransformOperation> right
        )
        {
            if (left.Count != right.Count)
                throw Invalid("Additive transform lists must have compatible operations.");
            var result = new TransformOperation[left.Count];
            for (int index = 0; index < result.Length; index++)
                result[index] = CombineTransform(left[index], right[index]);
            return result;
        }

        private static TransformOperation CombineTransform(
            TransformOperation left,
            TransformOperation right
        ) =>
            (left, right) switch
            {
                (TransformOperation.Translate a, TransformOperation.Translate b) =>
                    new TransformOperation.Translate(AddLengths(a.Value, b.Value)),
                (TransformOperation.Rotate a, TransformOperation.Rotate b) =>
                    new TransformOperation.Rotate(Add(a.Value, b.Value)),
                (TransformOperation.Skew a, TransformOperation.Skew b) =>
                    new TransformOperation.Skew(Add(a.Value, b.Value)),
                (TransformOperation.Scale a, TransformOperation.Scale b) =>
                    new TransformOperation.Scale(Multiply(a.Value, b.Value, 1)),
                _ => throw Invalid("Additive transform lists must have compatible operations."),
            };

        private static IReadOnlyList<TransformOperation> AccumulateTransforms(
            IReadOnlyList<TransformOperation> sample,
            MotionValue origin,
            MotionValue target,
            uint iteration
        )
        {
            if (
                origin is not MotionValue.TransformList a
                || target is not MotionValue.TransformList b
            )
                throw Invalid("Transform accumulation requires transform-list endpoints.");
            if (sample.Count != a.Value.Count || sample.Count != b.Value.Count)
                throw Invalid("Accumulated transform lists must have compatible operations.");
            var result = new TransformOperation[sample.Count];
            for (int index = 0; index < result.Length; index++)
                result[index] = AccumulateTransform(
                    sample[index],
                    a.Value[index],
                    b.Value[index],
                    iteration
                );
            return result;
        }

        private static TransformOperation AccumulateTransform(
            TransformOperation sample,
            TransformOperation origin,
            TransformOperation target,
            uint iteration
        ) =>
            (sample, origin, target) switch
            {
                (
                    TransformOperation.Translate value,
                    TransformOperation.Translate a,
                    TransformOperation.Translate b
                ) => new TransformOperation.Translate(
                    AddLengths(
                        value.Value,
                        ScaleLengths(SubtractLengths(b.Value, a.Value), iteration)
                    )
                ),
                (
                    TransformOperation.Rotate value,
                    TransformOperation.Rotate a,
                    TransformOperation.Rotate b
                ) => new TransformOperation.Rotate(
                    Add(value.Value, Scale(Subtract(b.Value, a.Value), iteration))
                ),
                (
                    TransformOperation.Skew value,
                    TransformOperation.Skew a,
                    TransformOperation.Skew b
                ) => new TransformOperation.Skew(
                    Add(value.Value, Scale(Subtract(b.Value, a.Value), iteration))
                ),
                (
                    TransformOperation.Scale value,
                    TransformOperation.Scale a,
                    TransformOperation.Scale b
                ) => new TransformOperation.Scale(
                    Multiply(value.Value, Ratio(b.Value, a.Value), iteration)
                ),
                _ => throw Invalid("Accumulated transform lists must have compatible operations."),
            };

        private static bool Multiply(MotionProperty property) =>
            property is MotionProperty.Scale or MotionProperty.ScaleX or MotionProperty.ScaleY;

        private static double Ratio(double value, double origin) =>
            origin == 0
                ? throw Invalid("Scale accumulation cannot divide by a zero origin.")
                : value / origin;

        private static IReadOnlyList<double> Add(
            IReadOnlyList<double> left,
            IReadOnlyList<double> right
        ) => Zip(left, right, (a, b) => a + b);

        private static IReadOnlyList<double> Subtract(
            IReadOnlyList<double> left,
            IReadOnlyList<double> right
        ) => Zip(left, right, (a, b) => a - b);

        private static IReadOnlyList<double> Scale(IReadOnlyList<double> value, uint count)
        {
            var result = new double[value.Count];
            for (int index = 0; index < result.Length; index++)
                result[index] = value[index] * count;
            return result;
        }

        private static IReadOnlyList<double> Ratio(
            IReadOnlyList<double> left,
            IReadOnlyList<double> right
        ) => Zip(left, right, Ratio);

        private static IReadOnlyList<double> Multiply(
            IReadOnlyList<double> left,
            IReadOnlyList<double> right,
            uint power
        ) => Zip(left, right, (a, b) => a * Math.Pow(b, power));

        private static IReadOnlyList<double> Zip(
            IReadOnlyList<double> left,
            IReadOnlyList<double> right,
            Func<double, double, double> combine
        )
        {
            if (left.Count != right.Count)
                throw Invalid("Additive channels must have matching dimensions.");
            var result = new double[left.Count];
            for (int index = 0; index < result.Length; index++)
                result[index] = combine(left[index], right[index]);
            return result;
        }

        private static IReadOnlyList<UiLength> AddLengths(
            IReadOnlyList<UiLength> left,
            IReadOnlyList<UiLength> right
        ) =>
            ZipLengths(
                left,
                right,
                (a, b) => UiLength.FromComponents(a.Pixels + b.Pixels, a.Percentage + b.Percentage)
            );

        private static IReadOnlyList<UiLength> SubtractLengths(
            IReadOnlyList<UiLength> left,
            IReadOnlyList<UiLength> right
        ) =>
            ZipLengths(
                left,
                right,
                (a, b) => UiLength.FromComponents(a.Pixels - b.Pixels, a.Percentage - b.Percentage)
            );

        private static IReadOnlyList<UiLength> ScaleLengths(
            IReadOnlyList<UiLength> values,
            uint count
        )
        {
            var result = new UiLength[values.Count];
            for (int index = 0; index < result.Length; index++)
                result[index] = UiLength.FromComponents(
                    values[index].Pixels * count,
                    values[index].Percentage * count
                );
            return result;
        }

        private static IReadOnlyList<UiLength> ZipLengths(
            IReadOnlyList<UiLength> left,
            IReadOnlyList<UiLength> right,
            Func<UiLength, UiLength, UiLength> combine
        )
        {
            if (left.Count != right.Count)
                throw Invalid("Additive transform channels must have matching dimensions.");
            var result = new UiLength[left.Count];
            for (int index = 0; index < result.Length; index++)
                result[index] = combine(left[index], right[index]);
            return result;
        }

        private static InvalidOperationException Invalid(string message) => new(message);
    }
}

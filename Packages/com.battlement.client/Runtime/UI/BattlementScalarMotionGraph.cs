#nullable enable

using System;

namespace Battlement.UI
{
    internal static class BattlementScalarMotionGraph
    {
        public static double Range(MotionValueSource.Range range, double value)
        {
            int segment = range.Input.Count - 2;
            for (int index = 0; index < range.Input.Count - 1; index++)
            {
                if (value > Scalar(range.Input[index + 1]))
                    continue;
                segment = index;
                break;
            }
            double start = Scalar(range.Input[segment]);
            double end = Scalar(range.Input[segment + 1]);
            double progress = (value - start) / (end - start);
            if (range.Clamp)
                progress = Math.Clamp(progress, 0, 1);
            double outputStart = Scalar(range.Output[segment]);
            return outputStart + (Scalar(range.Output[segment + 1]) - outputStart) * progress;
        }

        public static double Expression(
            MotionExpressionOperation operation,
            double left,
            double right,
            double mix
        ) =>
            operation switch
            {
                MotionExpressionOperation.Add => left + right,
                MotionExpressionOperation.Subtract => left - right,
                MotionExpressionOperation.Multiply => left * right,
                MotionExpressionOperation.Divide => left / right,
                MotionExpressionOperation.Power value => Math.Pow(left, value.Value),
                MotionExpressionOperation.SquareRoot => Math.Sqrt(left),
                MotionExpressionOperation.Absolute => Math.Abs(left),
                MotionExpressionOperation.Minimum => Math.Min(left, right),
                MotionExpressionOperation.Maximum => Math.Max(left, right),
                MotionExpressionOperation.Clamp value => Math.Clamp(left, value.Min, value.Max),
                MotionExpressionOperation.Modulo value => Euclidean(left, value.Value),
                MotionExpressionOperation.Wrap value => value.Min
                    + Euclidean(left - value.Min, value.Max - value.Min),
                MotionExpressionOperation.ExponentialDecay value => Math.Exp(-value.Rate * left),
                MotionExpressionOperation.Mix => left + (right - left) * mix,
                _ => throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    "Unknown motion expression operation."
                ),
            };

        private static double Scalar(MotionValue value) =>
            value is MotionValue.Scalar scalar
                ? scalar.Value
                : throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    "Motion graph expected a scalar value."
                );

        private static double Euclidean(double value, double modulus) =>
            ((value % modulus) + modulus) % modulus;
    }
}

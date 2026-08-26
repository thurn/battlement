#nullable enable

using System;
using System.Collections.Generic;
using System.Text;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementUiStyleTransformProperties
    {
        private static readonly IReadOnlyDictionary<string, UiTransitionProperty> Properties =
            BuildPropertyMap();

        public static StyleRotate ToUnity(UiRotate value) =>
            new(
                new Rotate(
                    new Angle(value.Degrees, AngleUnit.Degree),
                    new UnityEngine.Vector3(value.X, value.Y, value.Z)
                )
            );

        public static StyleScale ToUnity(UiScale value) =>
            new(new Scale(new UnityEngine.Vector2(value.X, value.Y)));

        public static StyleTranslate ToUnity(UiTranslate value) =>
            new(new Translate(ToUnity(value.X), ToUnity(value.Y), value.Z));

        public static StyleTransformOrigin ToUnity(UiTransformOrigin value) =>
            new(new TransformOrigin(ToUnity(value.X), ToUnity(value.Y), value.Z));

        public static StyleList<FilterFunction> ToUnity(IReadOnlyList<UiFilterFunction> values)
        {
            var result = new List<FilterFunction>(values.Count);
            foreach (UiFilterFunction value in values)
                result.Add(ToUnity(value));
            return new StyleList<FilterFunction>(result);
        }

        public static StyleList<TimeValue> ToUnityTimes(IReadOnlyList<float> values)
        {
            var result = new List<TimeValue>(values.Count);
            foreach (float value in values)
                result.Add(new TimeValue(value, TimeUnit.Millisecond));
            return new StyleList<TimeValue>(result);
        }

        public static StyleList<StylePropertyName> ToUnity(
            IReadOnlyList<UiTransitionProperty> values
        )
        {
            var result = new List<StylePropertyName>(values.Count);
            foreach (UiTransitionProperty value in values)
                result.Add(new StylePropertyName(ToUssName(value)));
            return new StyleList<StylePropertyName>(result);
        }

        public static StyleList<EasingFunction> ToUnity(IReadOnlyList<UiEasingFunction> values)
        {
            var result = new List<EasingFunction>(values.Count);
            foreach (UiEasingFunction value in values)
                result.Add(new EasingFunction((EasingMode)value));
            return new StyleList<EasingFunction>(result);
        }

        public static bool TryFromUnity(
            StylePropertyName value,
            out UiTransitionProperty property
        ) => Properties.TryGetValue(value.ToString(), out property);

        private static FilterFunction ToUnity(UiFilterFunction value)
        {
            FilterFunction result = value switch
            {
                UiFilterFunction.Tint => new FilterFunction(FilterFunctionType.Tint),
                UiFilterFunction.Opacity => new FilterFunction(FilterFunctionType.Opacity),
                UiFilterFunction.Invert => new FilterFunction(FilterFunctionType.Invert),
                UiFilterFunction.Grayscale => new FilterFunction(FilterFunctionType.Grayscale),
                UiFilterFunction.Sepia => new FilterFunction(FilterFunctionType.Sepia),
                UiFilterFunction.Blur => new FilterFunction(FilterFunctionType.Blur),
                UiFilterFunction.Contrast => new FilterFunction(FilterFunctionType.Contrast),
                UiFilterFunction.HueRotate => new FilterFunction(FilterFunctionType.HueRotate),
                _ => throw new ArgumentOutOfRangeException(nameof(value)),
            };
            result.AddParameter(
                value is UiFilterFunction.Tint tint
                    ? new FilterParameter(ToUnity(tint.Value))
                    : new FilterParameter(FloatParameter(value))
            );
            return result;
        }

        private static float FloatParameter(UiFilterFunction value) =>
            value switch
            {
                UiFilterFunction.Opacity item => item.Value,
                UiFilterFunction.Invert item => item.Value,
                UiFilterFunction.Grayscale item => item.Value,
                UiFilterFunction.Sepia item => item.Value,
                UiFilterFunction.Blur item => item.Value,
                UiFilterFunction.Contrast item => item.Value,
                UiFilterFunction.HueRotate item => item.Value,
                _ => throw new ArgumentOutOfRangeException(nameof(value)),
            };

        private static UnityEngine.Color ToUnity(Color value) =>
            new((float)value.Red, (float)value.Green, (float)value.Blue, (float)value.Alpha);

        private static Length ToUnity(UiLength value) =>
            value switch
            {
                UiLength.Px item => new Length(item.Value, LengthUnit.Pixel),
                UiLength.Percent item => new Length(item.Value, LengthUnit.Percent),
                _ => throw new ArgumentOutOfRangeException(nameof(value)),
            };

        private static IReadOnlyDictionary<string, UiTransitionProperty> BuildPropertyMap()
        {
            var result = new Dictionary<string, UiTransitionProperty>(StringComparer.Ordinal);
            foreach (UiTransitionProperty value in Enum.GetValues(typeof(UiTransitionProperty)))
                result.Add(ToUssName(value), value);
            return result;
        }

        private static string ToUssName(UiTransitionProperty value)
        {
            string name = value.ToString();
            if (value == UiTransitionProperty.All)
                return "all";
            bool unity = name.StartsWith("Unity", StringComparison.Ordinal);
            int start = unity ? "Unity".Length : 0;
            var result = new StringBuilder(unity ? "-unity" : string.Empty);
            for (int index = start; index < name.Length; index++)
            {
                char character = name[index];
                if (char.IsUpper(character) && result.Length > 0)
                    result.Append('-');
                result.Append(char.ToLowerInvariant(character));
            }
            return result.ToString();
        }
    }
}

#nullable enable

using System;

namespace Battlement
{
    internal static class UiStyleValidator
    {
        public static void Validate(UiStyle? value, Func<string, Exception> invalid)
        {
            if (value is null)
                return;
            ValidateColor(value.BackgroundColor, invalid);
            ValidateColor(value.BorderBottomColor, invalid);
            ValidateColor(value.BorderLeftColor, invalid);
            ValidateColor(value.BorderRightColor, invalid);
            ValidateColor(value.BorderTopColor, invalid);
            ValidateColor(value.Color, invalid);
            ValidateColor(value.UnityBackgroundImageTintColor, invalid);
            if (value.FontSize is float fontSize)
                ValidateNumber(fontSize, false, invalid);
            ValidateEnum(value.AlignContent, invalid);
            ValidateEnum(value.AlignItems, invalid);
            ValidateEnum(value.AlignSelf, invalid);
            ValidateRatio(value.AspectRatio, invalid);
            ValidateLength(value.BorderBottomLeftRadius, true, invalid);
            ValidateLength(value.BorderBottomRightRadius, true, invalid);
            ValidateLength(value.BorderTopLeftRadius, true, invalid);
            ValidateLength(value.BorderTopRightRadius, true, invalid);
            ValidateLength(value.PaddingBottom, true, invalid);
            ValidateLength(value.PaddingLeft, true, invalid);
            ValidateLength(value.PaddingRight, true, invalid);
            ValidateLength(value.PaddingTop, true, invalid);
            ValidateLength(value.Bottom, false, invalid);
            ValidateLength(value.FlexBasis, false, invalid);
            ValidateLength(value.Left, false, invalid);
            ValidateLength(value.MarginBottom, false, invalid);
            ValidateLength(value.MarginLeft, false, invalid);
            ValidateLength(value.MarginRight, false, invalid);
            ValidateLength(value.MarginTop, false, invalid);
            ValidateLength(value.Right, false, invalid);
            ValidateLength(value.Top, false, invalid);
            ValidateLength(value.Height, true, invalid);
            ValidateLength(value.MaxHeight, true, invalid);
            ValidateLength(value.MaxWidth, true, invalid);
            ValidateLength(value.MinHeight, true, invalid);
            ValidateLength(value.MinWidth, true, invalid);
            ValidateLength(value.Width, true, invalid);
            ValidateFloat(value.FlexGrow, true, invalid);
            ValidateFloat(value.FlexShrink, true, invalid);
            ValidateFloat(value.BorderBottomWidth, true, invalid);
            ValidateFloat(value.BorderLeftWidth, true, invalid);
            ValidateFloat(value.BorderRightWidth, true, invalid);
            ValidateFloat(value.BorderTopWidth, true, invalid);
            ValidateRange(value.Opacity, 0, 1, invalid);
            ValidatePositive(value.UnitySliceScale, invalid);
            ValidateNonnegative(value.UnitySliceBottom, invalid);
            ValidateNonnegative(value.UnitySliceLeft, invalid);
            ValidateNonnegative(value.UnitySliceRight, invalid);
            ValidateNonnegative(value.UnitySliceTop, invalid);
            ValidateEnum(value.FlexDirection, invalid);
            ValidateEnum(value.FlexWrap, invalid);
            ValidateEnum(value.JustifyContent, invalid);
            ValidateEnum(value.Position, invalid);
            ValidateEnum(value.Display, invalid);
            ValidateEnum(value.Overflow, invalid);
            ValidateEnum(value.UnityOverflowClipBox, invalid);
            ValidateEnum(value.UnitySliceType, invalid);
            ValidateEnum(value.Visibility, invalid);
            ValidateKeyword(value.UnityMaterial?.Keyword, invalid);
        }

        private static void ValidateColor(
            UiStyleValue<Color>? value,
            Func<string, Exception> invalid
        )
        {
            if (value is null || ValidateKeyword(value.Keyword, invalid))
                return;
            foreach (
                double channel in new[]
                {
                    value.Value.Red,
                    value.Value.Green,
                    value.Value.Blue,
                    value.Value.Alpha,
                }
            )
            {
                bool invalidRange = channel < 0 || channel > 1;
                if (!double.IsFinite(channel) || invalidRange)
                    throw invalid("A UI style color is invalid.");
            }
        }

        private static void ValidateNonnegative(
            UiStyleValue<int>? value,
            Func<string, Exception> invalid
        )
        {
            if (value is null || ValidateKeyword(value.Keyword, invalid))
                return;
            if (value.Value < 0)
                throw invalid("A UI slice inset is invalid.");
        }

        private static void ValidatePositive(
            UiStyleValue<float>? value,
            Func<string, Exception> invalid
        )
        {
            if (value is null || ValidateKeyword(value.Keyword, invalid))
                return;
            if (!float.IsFinite(value.Value) || value.Value <= 0)
                throw invalid("A UI style scale is invalid.");
        }

        private static void ValidateRange(
            UiStyleValue<float>? value,
            float minimum,
            float maximum,
            Func<string, Exception> invalid
        )
        {
            if (value is null || ValidateKeyword(value.Keyword, invalid))
                return;
            if (!float.IsFinite(value.Value))
                throw invalid("A UI style number is invalid.");
            if (value.Value < minimum || value.Value > maximum)
                throw invalid("A UI style number is out of range.");
        }

        private static void ValidateRatio(
            UiStyleValue<UiAspectRatio>? value,
            Func<string, Exception> invalid
        )
        {
            if (value is null || ValidateKeyword(value.Keyword, invalid))
                return;
            if (value.Value is UiAspectRatio.Auto)
                return;
            if (value.Value is not UiAspectRatio.Ratio ratio)
                throw invalid("Unknown UI aspect ratio kind.");
            bool finite = float.IsFinite(ratio.Width) && float.IsFinite(ratio.Height);
            bool positive = ratio.Width > 0 && ratio.Height > 0;
            if (!finite || !positive || !float.IsFinite(ratio.Width / ratio.Height))
                throw invalid("A UI aspect ratio is invalid.");
        }

        private static void ValidateLength(
            UiStyleValue<UiLength>? value,
            bool nonnegative,
            Func<string, Exception> invalid
        )
        {
            if (value is null || ValidateKeyword(value.Keyword, invalid))
                return;
            float number = value.Value switch
            {
                UiLength.Px item => item.Value,
                UiLength.Percent item => item.Value,
                _ => throw invalid("Unknown UI length kind."),
            };
            ValidateNumber(number, nonnegative, invalid);
        }

        private static void ValidateLength(
            UiStyleValue<UiLengthOrAuto>? value,
            bool nonnegative,
            Func<string, Exception> invalid
        )
        {
            if (value is null || ValidateKeyword(value.Keyword, invalid))
                return;
            float? number = value.Value switch
            {
                UiLengthOrAuto.Px item => item.Value,
                UiLengthOrAuto.Percent item => item.Value,
                UiLengthOrAuto.Auto => null,
                _ => throw invalid("Unknown UI length kind."),
            };
            if (number is float concrete)
                ValidateNumber(concrete, nonnegative, invalid);
        }

        private static void ValidateFloat(
            UiStyleValue<float>? value,
            bool nonnegative,
            Func<string, Exception> invalid
        )
        {
            if (value is null || ValidateKeyword(value.Keyword, invalid))
                return;
            ValidateNumber(value.Value, nonnegative, invalid);
        }

        private static void ValidateNumber(
            float value,
            bool nonnegative,
            Func<string, Exception> invalid
        )
        {
            if (!float.IsFinite(value) || nonnegative && value < 0)
                throw invalid("A UI style number is invalid.");
        }

        private static void ValidateEnum<T>(UiStyleValue<T>? value, Func<string, Exception> invalid)
            where T : struct, Enum
        {
            if (value is null || ValidateKeyword(value.Keyword, invalid))
                return;
            if (!Enum.IsDefined(typeof(T), value.Value))
                throw invalid("A UI style enum is invalid.");
        }

        private static bool ValidateKeyword(UiInlineKeyword? value, Func<string, Exception> invalid)
        {
            if (value is null)
                return false;
            if (value != UiInlineKeyword.Initial)
                throw invalid("A UI style keyword is invalid.");
            return true;
        }
    }
}

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
            if (value.BackgroundColor is Color background)
                ValidateColor(background, invalid);
            if (value.Color is Color foreground)
                ValidateColor(foreground, invalid);
            if (value.FontSize is float fontSize)
                ValidateNumber(fontSize, false, invalid);
            ValidateEnum(value.AlignContent, invalid);
            ValidateEnum(value.AlignItems, invalid);
            ValidateEnum(value.AlignSelf, invalid);
            ValidateRatio(value.AspectRatio, invalid);
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
            ValidateEnum(value.FlexDirection, invalid);
            ValidateEnum(value.FlexWrap, invalid);
            ValidateEnum(value.JustifyContent, invalid);
            ValidateEnum(value.Position, invalid);
        }

        private static void ValidateColor(Color value, Func<string, Exception> invalid)
        {
            foreach (double channel in new[] { value.Red, value.Green, value.Blue, value.Alpha })
            {
                bool invalidRange = channel < 0 || channel > 1;
                if (!double.IsFinite(channel) || invalidRange)
                    throw invalid("A UI style color is invalid.");
            }
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

#nullable enable

using System;
using System.Collections.Generic;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    internal static partial class BattlementFlatBufferResponseFixtures
    {
        private static void AddStyle(
            FlatBufferBuilder builder,
            List<int> properties,
            UiStyle? style
        )
        {
            if (style is null)
                return;
            AddStyleLength(builder, properties, Wire.UiPropertyKey.StyleLeft, style.Left);
            AddStyleLength(builder, properties, Wire.UiPropertyKey.StyleTop, style.Top);
            AddStyleLength(builder, properties, Wire.UiPropertyKey.StyleWidth, style.Width);
            AddStyleLength(builder, properties, Wire.UiPropertyKey.StyleHeight, style.Height);
            AddStyleEnum(
                builder,
                properties,
                Wire.UiPropertyKey.StylePosition,
                Wire.UiEnumCatalog.Position,
                style.Position
            );
        }

        private static void AddStyleLength(
            FlatBufferBuilder builder,
            List<int> properties,
            Wire.UiPropertyKey key,
            Prop<UiStyleValue<UiLengthOrAuto>> prop
        )
        {
            if (prop.IsUnset)
                return;
            Wire.StyleValueKind styleKind = StyleKind(prop, out UiLengthOrAuto? value);
            int offset = 0;
            if (value is not null)
            {
                (Wire.LengthKind kind, float pixels, float percentage) = value switch
                {
                    UiLengthOrAuto.Px length => (Wire.LengthKind.Pixels, length.Value, 0f),
                    UiLengthOrAuto.Percent length => (Wire.LengthKind.Percent, 0f, length.Value),
                    UiLengthOrAuto.Auto => (Wire.LengthKind.Auto, 0f, 0f),
                    _ => throw Unsupported(value),
                };
                offset = Wire
                    .LengthPropertyValue.CreateLengthPropertyValue(
                        builder,
                        kind,
                        pixels,
                        percentage
                    )
                    .Value;
            }
            properties.Add(
                Property(
                    builder,
                    key,
                    prop.State,
                    Wire.UiPropertyValue.LengthPropertyValue,
                    offset,
                    styleKind
                )
            );
        }

        private static void AddStyleEnum<T>(
            FlatBufferBuilder builder,
            List<int> properties,
            Wire.UiPropertyKey key,
            Wire.UiEnumCatalog catalog,
            Prop<UiStyleValue<T>> prop
        )
            where T : Enum
        {
            if (prop.IsUnset)
                return;
            Wire.StyleValueKind styleKind = StyleKind(prop, out T? value);
            int offset = value is null
                ? 0
                : Wire
                    .EnumPropertyValue.CreateEnumPropertyValue(
                        builder,
                        catalog,
                        Convert.ToUInt32(value)
                    )
                    .Value;
            properties.Add(
                Property(
                    builder,
                    key,
                    prop.State,
                    Wire.UiPropertyValue.EnumPropertyValue,
                    offset,
                    styleKind
                )
            );
        }

        private static Wire.StyleValueKind StyleKind<T>(Prop<UiStyleValue<T>> prop, out T? value)
        {
            value = default;
            if (!prop.IsSet)
                return Wire.StyleValueKind.Value;
            UiStyleValue<T> style = prop.Value;
            if (style.Keyword == UiInlineKeyword.Initial)
                return Wire.StyleValueKind.Initial;
            value = style.Value;
            return Wire.StyleValueKind.Value;
        }
    }
}

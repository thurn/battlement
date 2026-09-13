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
            private UiStyle? Style()
            {
                bool present = false;
                foreach (Wire.UiPropertyKey key in values.Keys)
                {
                    if (key >= Wire.UiPropertyKey.StyleAlignContent)
                    {
                        present = true;
                        break;
                    }
                }
                if (!present)
                    return null;
                return new UiStyle(
                    AlignContent: StyleEnum<UiAlign>(
                        Wire.UiPropertyKey.StyleAlignContent,
                        Wire.UiEnumCatalog.Align
                    ),
                    AlignItems: StyleEnum<UiAlign>(
                        Wire.UiPropertyKey.StyleAlignItems,
                        Wire.UiEnumCatalog.Align
                    ),
                    AlignSelf: StyleEnum<UiAlign>(
                        Wire.UiPropertyKey.StyleAlignSelf,
                        Wire.UiEnumCatalog.Align
                    ),
                    AspectRatio: StyleAspectRatio(Wire.UiPropertyKey.StyleAspectRatio),
                    BackgroundColor: StyleColor(Wire.UiPropertyKey.StyleBackgroundColor),
                    BackgroundImage: StyleBackground(Wire.UiPropertyKey.StyleBackgroundImage),
                    BackgroundPositionX: StyleBackgroundPosition(
                        Wire.UiPropertyKey.StyleBackgroundPositionX
                    ),
                    BackgroundPositionY: StyleBackgroundPosition(
                        Wire.UiPropertyKey.StyleBackgroundPositionY
                    ),
                    BackgroundRepeat: StyleBackgroundRepeat(
                        Wire.UiPropertyKey.StyleBackgroundRepeat
                    ),
                    BackgroundSize: StyleBackgroundSize(Wire.UiPropertyKey.StyleBackgroundSize),
                    BorderBottomColor: StyleColor(Wire.UiPropertyKey.StyleBorderBottomColor),
                    BorderBottomLeftRadius: StyleLength(
                        Wire.UiPropertyKey.StyleBorderBottomLeftRadius
                    ),
                    BorderBottomRightRadius: StyleLength(
                        Wire.UiPropertyKey.StyleBorderBottomRightRadius
                    ),
                    BorderBottomWidth: StyleFloat(Wire.UiPropertyKey.StyleBorderBottomWidth),
                    BorderLeftColor: StyleColor(Wire.UiPropertyKey.StyleBorderLeftColor),
                    BorderLeftWidth: StyleFloat(Wire.UiPropertyKey.StyleBorderLeftWidth),
                    BorderRightColor: StyleColor(Wire.UiPropertyKey.StyleBorderRightColor),
                    BorderRightWidth: StyleFloat(Wire.UiPropertyKey.StyleBorderRightWidth),
                    BorderTopColor: StyleColor(Wire.UiPropertyKey.StyleBorderTopColor),
                    BorderTopLeftRadius: StyleLength(Wire.UiPropertyKey.StyleBorderTopLeftRadius),
                    BorderTopRightRadius: StyleLength(Wire.UiPropertyKey.StyleBorderTopRightRadius),
                    BorderTopWidth: StyleFloat(Wire.UiPropertyKey.StyleBorderTopWidth),
                    Bottom: StyleLengthOrAuto(Wire.UiPropertyKey.StyleBottom),
                    Color: StyleColor(Wire.UiPropertyKey.StyleColor),
                    Cursor: StyleCursor(Wire.UiPropertyKey.StyleCursor),
                    Display: StyleEnum<UiDisplay>(
                        Wire.UiPropertyKey.StyleDisplay,
                        Wire.UiEnumCatalog.Display
                    ),
                    FlexBasis: StyleLengthOrAuto(Wire.UiPropertyKey.StyleFlexBasis),
                    FlexDirection: StyleEnum<UiFlexDirection>(
                        Wire.UiPropertyKey.StyleFlexDirection,
                        Wire.UiEnumCatalog.FlexDirection
                    ),
                    FlexGrow: StyleFloat(Wire.UiPropertyKey.StyleFlexGrow),
                    FlexShrink: StyleFloat(Wire.UiPropertyKey.StyleFlexShrink),
                    FlexWrap: StyleEnum<UiFlexWrap>(
                        Wire.UiPropertyKey.StyleFlexWrap,
                        Wire.UiEnumCatalog.FlexWrap
                    ),
                    FontSize: StyleLength(Wire.UiPropertyKey.StyleFontSize),
                    Height: StyleLengthOrAuto(Wire.UiPropertyKey.StyleHeight),
                    JustifyContent: StyleEnum<UiJustify>(
                        Wire.UiPropertyKey.StyleJustifyContent,
                        Wire.UiEnumCatalog.Justify
                    ),
                    LetterSpacing: StyleLength(Wire.UiPropertyKey.StyleLetterSpacing),
                    Left: StyleLengthOrAuto(Wire.UiPropertyKey.StyleLeft),
                    MarginBottom: StyleLengthOrAuto(Wire.UiPropertyKey.StyleMarginBottom),
                    MarginLeft: StyleLengthOrAuto(Wire.UiPropertyKey.StyleMarginLeft),
                    MarginRight: StyleLengthOrAuto(Wire.UiPropertyKey.StyleMarginRight),
                    MarginTop: StyleLengthOrAuto(Wire.UiPropertyKey.StyleMarginTop),
                    MaxHeight: StyleLengthOrAuto(Wire.UiPropertyKey.StyleMaxHeight),
                    MaxWidth: StyleLengthOrAuto(Wire.UiPropertyKey.StyleMaxWidth),
                    MinHeight: StyleLengthOrAuto(Wire.UiPropertyKey.StyleMinHeight),
                    MinWidth: StyleLengthOrAuto(Wire.UiPropertyKey.StyleMinWidth),
                    Opacity: StyleFloat(Wire.UiPropertyKey.StyleOpacity),
                    Overflow: StyleEnum<UiOverflow>(
                        Wire.UiPropertyKey.StyleOverflow,
                        Wire.UiEnumCatalog.Overflow
                    ),
                    PaddingBottom: StyleLength(Wire.UiPropertyKey.StylePaddingBottom),
                    PaddingLeft: StyleLength(Wire.UiPropertyKey.StylePaddingLeft),
                    PaddingRight: StyleLength(Wire.UiPropertyKey.StylePaddingRight),
                    PaddingTop: StyleLength(Wire.UiPropertyKey.StylePaddingTop),
                    Position: StyleEnum<UiPosition>(
                        Wire.UiPropertyKey.StylePosition,
                        Wire.UiEnumCatalog.Position
                    ),
                    Right: StyleLengthOrAuto(Wire.UiPropertyKey.StyleRight),
                    Rotate: StyleRotate(Wire.UiPropertyKey.StyleRotate),
                    Scale: StyleScale(Wire.UiPropertyKey.StyleScale),
                    TextOverflow: StyleEnum<UiTextOverflow>(
                        Wire.UiPropertyKey.StyleTextOverflow,
                        Wire.UiEnumCatalog.TextOverflow
                    ),
                    TextShadow: StyleTextShadow(Wire.UiPropertyKey.StyleTextShadow),
                    Top: StyleLengthOrAuto(Wire.UiPropertyKey.StyleTop),
                    TransformOrigin: StyleTransformOrigin(Wire.UiPropertyKey.StyleTransformOrigin),
                    TransitionDelay: StyleFloatList(Wire.UiPropertyKey.StyleTransitionDelay),
                    TransitionDuration: StyleFloatList(Wire.UiPropertyKey.StyleTransitionDuration),
                    TransitionProperty: StyleEnumList<UiTransitionProperty>(
                        Wire.UiPropertyKey.StyleTransitionProperty,
                        Wire.UiEnumCatalog.TransitionProperty
                    ),
                    TransitionTimingFunction: StyleEnumList<UiEasingFunction>(
                        Wire.UiPropertyKey.StyleTransitionTimingFunction,
                        Wire.UiEnumCatalog.EasingFunction
                    ),
                    Translate: StyleTranslate(Wire.UiPropertyKey.StyleTranslate),
                    UnityBackgroundImageTintColor: StyleColor(
                        Wire.UiPropertyKey.StyleUnityBackgroundImageTintColor
                    ),
                    UnityEditorTextRenderingMode: StyleEnum<UiEditorTextRenderingMode>(
                        Wire.UiPropertyKey.StyleUnityEditorTextRenderingMode,
                        Wire.UiEnumCatalog.EditorTextRenderingMode
                    ),
                    UnityFontDefinition: StyleFont(Wire.UiPropertyKey.StyleUnityFontDefinition),
                    UnityFontStyleAndWeight: StyleEnum<UiFontStyle>(
                        Wire.UiPropertyKey.StyleUnityFontStyleAndWeight,
                        Wire.UiEnumCatalog.FontStyle
                    ),
                    UnityMaterial: StyleMaterial(Wire.UiPropertyKey.StyleUnityMaterial),
                    UnityOverflowClipBox: StyleEnum<UiOverflowClipBox>(
                        Wire.UiPropertyKey.StyleUnityOverflowClipBox,
                        Wire.UiEnumCatalog.OverflowClipBox
                    ),
                    UnityParagraphSpacing: StyleLength(
                        Wire.UiPropertyKey.StyleUnityParagraphSpacing
                    ),
                    UnitySliceBottom: StyleInt(Wire.UiPropertyKey.StyleUnitySliceBottom),
                    UnitySliceLeft: StyleInt(Wire.UiPropertyKey.StyleUnitySliceLeft),
                    UnitySliceRight: StyleInt(Wire.UiPropertyKey.StyleUnitySliceRight),
                    UnitySliceScale: StyleFloat(Wire.UiPropertyKey.StyleUnitySliceScale),
                    UnitySliceTop: StyleInt(Wire.UiPropertyKey.StyleUnitySliceTop),
                    UnitySliceType: StyleEnum<UiSliceType>(
                        Wire.UiPropertyKey.StyleUnitySliceType,
                        Wire.UiEnumCatalog.SliceType
                    ),
                    UnityTextAlign: StyleEnum<UiTextAnchor>(
                        Wire.UiPropertyKey.StyleUnityTextAlign,
                        Wire.UiEnumCatalog.TextAnchor
                    ),
                    UnityTextAutoSize: StyleTextAutoSize(Wire.UiPropertyKey.StyleUnityTextAutoSize),
                    UnityTextGenerator: StyleEnum<UiTextGenerator>(
                        Wire.UiPropertyKey.StyleUnityTextGenerator,
                        Wire.UiEnumCatalog.TextGenerator
                    ),
                    UnityTextOutlineColor: StyleColor(
                        Wire.UiPropertyKey.StyleUnityTextOutlineColor
                    ),
                    UnityTextOutlineWidth: StyleFloat(
                        Wire.UiPropertyKey.StyleUnityTextOutlineWidth
                    ),
                    UnityTextOverflowPosition: StyleEnum<UiTextOverflowPosition>(
                        Wire.UiPropertyKey.StyleUnityTextOverflowPosition,
                        Wire.UiEnumCatalog.TextOverflowPosition
                    ),
                    Visibility: StyleEnum<UiVisibility>(
                        Wire.UiPropertyKey.StyleVisibility,
                        Wire.UiEnumCatalog.Visibility
                    ),
                    WhiteSpace: StyleEnum<UiWhiteSpace>(
                        Wire.UiPropertyKey.StyleWhiteSpace,
                        Wire.UiEnumCatalog.WhiteSpace
                    ),
                    Width: StyleLengthOrAuto(Wire.UiPropertyKey.StyleWidth),
                    WordSpacing: StyleLength(Wire.UiPropertyKey.StyleWordSpacing)
                );
            }

            private Prop<UiStyleValue<T>> StyleValue<T>(
                Wire.UiPropertyKey key,
                Wire.UiPropertyValue expected,
                Func<Wire.UiProperty, T> decode
            )
            {
                if (!values.Remove(key, out Wire.UiProperty value))
                    return default;
                if (
                    value.State == Wire.PropState.Reset
                    && value.ValueType == Wire.UiPropertyValue.NONE
                )
                    return Prop<UiStyleValue<T>>.Reset();
                if (value.State != Wire.PropState.Set)
                    throw new InvalidDataException("A style property has an invalid state.");
                if (value.StyleValueKind == Wire.StyleValueKind.Initial)
                {
                    if (value.ValueType != Wire.UiPropertyValue.NONE)
                        throw new InvalidDataException("An initial style keyword carries a value.");
                    return Prop<UiStyleValue<T>>.Set(
                        new UiStyleValue<T>(default!, UiInlineKeyword.Initial)
                    );
                }
                if (
                    value.StyleValueKind != Wire.StyleValueKind.Value
                    || value.ValueType != expected
                )
                    throw new InvalidDataException("A style property kind and value do not match.");
                return Prop<UiStyleValue<T>>.Set(new UiStyleValue<T>(decode(value)));
            }

            private Prop<UiStyleValue<T>> StyleEnum<T>(
                Wire.UiPropertyKey key,
                Wire.UiEnumCatalog catalog
            )
                where T : struct, Enum =>
                StyleValue(
                    key,
                    Wire.UiPropertyValue.EnumPropertyValue,
                    value =>
                    {
                        Wire.EnumPropertyValue item = value.ValueAsEnumPropertyValue();
                        if (item.Catalog != catalog)
                            throw new InvalidDataException("A style enum uses the wrong catalog.");
                        return (T)System.Enum.ToObject(typeof(T), item.Value);
                    }
                );

            private Prop<UiStyleValue<BackgroundSource>> StyleBackground(Wire.UiPropertyKey key) =>
                StyleValue<BackgroundSource>(
                    key,
                    Wire.UiPropertyValue.AssetPropertyValue,
                    value =>
                    {
                        Wire.AssetPropertyValue item = value.ValueAsAssetPropertyValue();
                        string address = Required(item.Address, "background asset address");
                        return item.Kind switch
                        {
                            Wire.AssetSourceKind.Texture => new BackgroundSource.Texture(
                                new TextureAddress(address)
                            ),
                            Wire.AssetSourceKind.Sprite => new BackgroundSource.Sprite(
                                new SpriteAddress(address)
                            ),
                            Wire.AssetSourceKind.VectorImage => new BackgroundSource.VectorImage(
                                new VectorImageAddress(address)
                            ),
                            Wire.AssetSourceKind.RenderTexture =>
                                new BackgroundSource.RenderTexture(
                                    new RenderTextureAddress(address)
                                ),
                            _ => throw new InvalidDataException("Invalid background asset kind."),
                        };
                    }
                );

            private Prop<UiStyleValue<UiFontAddress>> StyleFont(Wire.UiPropertyKey key) =>
                StyleValue(
                    key,
                    Wire.UiPropertyValue.AssetPropertyValue,
                    value =>
                    {
                        Wire.AssetPropertyValue item = value.ValueAsAssetPropertyValue();
                        if (item.Kind != Wire.AssetSourceKind.UiFont)
                            throw new InvalidDataException(
                                "A UI font property has the wrong asset kind."
                            );
                        return new UiFontAddress(Required(item.Address, "UI font address"));
                    }
                );

            private Prop<UiStyleValue<MaterialAddress>> StyleMaterial(Wire.UiPropertyKey key) =>
                StyleValue(
                    key,
                    Wire.UiPropertyValue.AssetPropertyValue,
                    value =>
                    {
                        Wire.AssetPropertyValue item = value.ValueAsAssetPropertyValue();
                        if (item.Kind != Wire.AssetSourceKind.Material)
                            throw new InvalidDataException(
                                "A material property has the wrong asset kind."
                            );
                        return new MaterialAddress(Required(item.Address, "material address"));
                    }
                );

            private Prop<UiStyleValue<UiCursor>> StyleCursor(Wire.UiPropertyKey key) =>
                StyleValue<UiCursor>(
                    key,
                    Wire.UiPropertyValue.CursorPropertyValue,
                    value =>
                    {
                        Wire.CursorPropertyValue item = value.ValueAsCursorPropertyValue();
                        return item.Kind switch
                        {
                            Wire.CursorKind.Default => new UiCursor.Default(),
                            Wire.CursorKind.Texture when item.Hotspot.HasValue =>
                                new UiCursor.Texture(
                                    new TextureAddress(Required(item.Address, "cursor texture")),
                                    new UiCursorHotspot(
                                        (float)item.Hotspot.Value.X,
                                        (float)item.Hotspot.Value.Y
                                    )
                                ),
                            _ => throw new InvalidDataException("Invalid cursor kind."),
                        };
                    }
                );

            private Prop<UiStyleValue<UiRotate>> StyleRotate(Wire.UiPropertyKey key) =>
                StyleValue(
                    key,
                    Wire.UiPropertyValue.RotatePropertyValue,
                    value =>
                    {
                        Wire.RotatePropertyValue item = value.ValueAsRotatePropertyValue();
                        return new UiRotate(item.X, item.Y, item.Z, item.Degrees);
                    }
                );

            private Prop<UiStyleValue<UiScale>> StyleScale(Wire.UiPropertyKey key) =>
                StyleValue(
                    key,
                    Wire.UiPropertyValue.ScalePropertyValue,
                    value =>
                    {
                        Wire.ScalePropertyValue item = value.ValueAsScalePropertyValue();
                        return new UiScale(item.X, item.Y);
                    }
                );

            private Prop<UiStyleValue<UiTextShadow>> StyleTextShadow(Wire.UiPropertyKey key) =>
                StyleValue(
                    key,
                    Wire.UiPropertyValue.TextShadowPropertyValue,
                    value =>
                    {
                        Wire.TextShadowPropertyValue item = value.ValueAsTextShadowPropertyValue();
                        return new UiTextShadow(
                            item.X,
                            item.Y,
                            item.BlurRadius,
                            Rgba(item.Color ?? throw Missing("text-shadow color"))
                        );
                    }
                );

            private Prop<UiStyleValue<UiTransformOrigin>> StyleTransformOrigin(
                Wire.UiPropertyKey key
            ) =>
                StyleValue(
                    key,
                    Wire.UiPropertyValue.TransformOriginPropertyValue,
                    value =>
                    {
                        Wire.TransformOriginPropertyValue item =
                            value.ValueAsTransformOriginPropertyValue();
                        return new UiTransformOrigin(
                            UiLength(item.X ?? throw Missing("transform-origin x"), false),
                            UiLength(item.Y ?? throw Missing("transform-origin y"), false),
                            item.Z
                        );
                    }
                );

            private Prop<UiStyleValue<UiTranslate>> StyleTranslate(Wire.UiPropertyKey key) =>
                StyleValue(
                    key,
                    Wire.UiPropertyValue.TranslatePropertyValue,
                    value =>
                    {
                        Wire.TranslatePropertyValue item = value.ValueAsTranslatePropertyValue();
                        return new UiTranslate(
                            UiLength(item.X ?? throw Missing("translate x"), false),
                            UiLength(item.Y ?? throw Missing("translate y"), false),
                            item.Z
                        );
                    }
                );

            private Prop<UiStyleValue<IReadOnlyList<float>>> StyleFloatList(
                Wire.UiPropertyKey key
            ) =>
                StyleValue<IReadOnlyList<float>>(
                    key,
                    Wire.UiPropertyValue.FloatListPropertyValue,
                    value =>
                    {
                        Wire.FloatListPropertyValue list = value.ValueAsFloatListPropertyValue();
                        var result = new float[list.ValuesLength];
                        for (int index = 0; index < result.Length; index++)
                            result[index] = list.Values(index);
                        return result;
                    }
                );

            private Prop<UiStyleValue<IReadOnlyList<T>>> StyleEnumList<T>(
                Wire.UiPropertyKey key,
                Wire.UiEnumCatalog catalog
            )
                where T : struct, Enum =>
                StyleValue<IReadOnlyList<T>>(
                    key,
                    Wire.UiPropertyValue.EnumListPropertyValue,
                    value =>
                    {
                        Wire.EnumListPropertyValue list = value.ValueAsEnumListPropertyValue();
                        if (list.Catalog != catalog)
                            throw new InvalidDataException(
                                "A style enum list uses the wrong catalog."
                            );
                        var result = new T[list.ValuesLength];
                        for (int index = 0; index < result.Length; index++)
                            result[index] = (T)System.Enum.ToObject(typeof(T), list.Values(index));
                        return result;
                    }
                );

            private Prop<UiStyleValue<UiTextAutoSize>> StyleTextAutoSize(Wire.UiPropertyKey key) =>
                StyleValue<UiTextAutoSize>(
                    key,
                    Wire.UiPropertyValue.TextAutoSizePropertyValue,
                    value =>
                    {
                        Wire.TextAutoSizePropertyValue item =
                            value.ValueAsTextAutoSizePropertyValue();
                        return item.Kind switch
                        {
                            Wire.TextAutoSizeKind.None => new UiTextAutoSize.None(),
                            Wire.TextAutoSizeKind.BestFit => new UiTextAutoSize.BestFit(
                                item.MinSize,
                                item.MaxSize
                            ),
                            _ => throw new InvalidDataException("Invalid text-auto-size kind."),
                        };
                    }
                );

            private Prop<UiStyleValue<float>> StyleFloat(Wire.UiPropertyKey key) =>
                StyleValue(
                    key,
                    Wire.UiPropertyValue.FloatPropertyValue,
                    value => value.ValueAsFloatPropertyValue().Value
                );

            private Prop<UiStyleValue<int>> StyleInt(Wire.UiPropertyKey key) =>
                StyleValue(
                    key,
                    Wire.UiPropertyValue.IntPropertyValue,
                    value => value.ValueAsIntPropertyValue().Value
                );

            private Prop<UiStyleValue<Color>> StyleColor(Wire.UiPropertyKey key) =>
                StyleValue(
                    key,
                    Wire.UiPropertyValue.ColorPropertyValue,
                    value =>
                        Rgba(
                            value.ValueAsColorPropertyValue().Value ?? throw Missing("style color")
                        )
                );

            private Prop<UiStyleValue<UiLength>> StyleLength(Wire.UiPropertyKey key) =>
                StyleValue(
                    key,
                    Wire.UiPropertyValue.LengthPropertyValue,
                    value => UiLength(value.ValueAsLengthPropertyValue(), allowAuto: false)
                );

            private Prop<UiStyleValue<UiLengthOrAuto>> StyleLengthOrAuto(Wire.UiPropertyKey key) =>
                StyleValue(
                    key,
                    Wire.UiPropertyValue.LengthPropertyValue,
                    value => UiLengthOrAuto(value.ValueAsLengthPropertyValue())
                );

            private static UiLength UiLength(Wire.LengthPropertyValue value, bool allowAuto) =>
                value.Kind switch
                {
                    Wire.LengthKind.Pixels => new UiLength.Px(value.Pixels),
                    Wire.LengthKind.Percent => new UiLength.Percent(value.Percentage),
                    Wire.LengthKind.Calc => new UiLength.Calc(value.Pixels, value.Percentage),
                    _ => throw new InvalidDataException("Invalid UI length kind."),
                };

            private static UiLengthOrAuto UiLengthOrAuto(Wire.LengthPropertyValue value) =>
                value.Kind switch
                {
                    Wire.LengthKind.Pixels => new UiLengthOrAuto.Px(value.Pixels),
                    Wire.LengthKind.Percent => new UiLengthOrAuto.Percent(value.Percentage),
                    Wire.LengthKind.Auto => new UiLengthOrAuto.Auto(),
                    _ => throw new InvalidDataException("Invalid UI length-or-auto kind."),
                };

            private Prop<UiStyleValue<UiAspectRatio>> StyleAspectRatio(Wire.UiPropertyKey key) =>
                StyleValue<UiAspectRatio>(
                    key,
                    Wire.UiPropertyValue.AspectRatioPropertyValue,
                    value =>
                    {
                        Wire.AspectRatioPropertyValue item =
                            value.ValueAsAspectRatioPropertyValue();
                        return item.Kind switch
                        {
                            Wire.AspectRatioKind.Auto => new UiAspectRatio.Auto(),
                            Wire.AspectRatioKind.Ratio => new UiAspectRatio.Ratio(
                                item.Width,
                                item.Height
                            ),
                            _ => throw new InvalidDataException("Invalid aspect-ratio kind."),
                        };
                    }
                );

            private Prop<UiStyleValue<UiBackgroundPosition>> StyleBackgroundPosition(
                Wire.UiPropertyKey key
            ) =>
                StyleValue(
                    key,
                    Wire.UiPropertyValue.BackgroundPositionPropertyValue,
                    value =>
                    {
                        Wire.BackgroundPositionPropertyValue item =
                            value.ValueAsBackgroundPositionPropertyValue();
                        return new UiBackgroundPosition(
                            (UiBackgroundPositionKeyword)item.Keyword,
                            UiLength(item.Offset ?? throw Missing("background offset"), false)
                        );
                    }
                );

            private Prop<UiStyleValue<UiBackgroundRepeat>> StyleBackgroundRepeat(
                Wire.UiPropertyKey key
            ) =>
                StyleValue(
                    key,
                    Wire.UiPropertyValue.BackgroundRepeatPropertyValue,
                    value =>
                    {
                        Wire.BackgroundRepeatPropertyValue item =
                            value.ValueAsBackgroundRepeatPropertyValue();
                        return new UiBackgroundRepeat(
                            (UiBackgroundRepeatMode)item.X,
                            (UiBackgroundRepeatMode)item.Y
                        );
                    }
                );

            private Prop<UiStyleValue<UiBackgroundSize>> StyleBackgroundSize(
                Wire.UiPropertyKey key
            ) =>
                StyleValue<UiBackgroundSize>(
                    key,
                    Wire.UiPropertyValue.BackgroundSizePropertyValue,
                    value =>
                    {
                        Wire.BackgroundSizePropertyValue item =
                            value.ValueAsBackgroundSizePropertyValue();
                        return item.Kind switch
                        {
                            Wire.BackgroundSizeKind.Auto => new UiBackgroundSize.Auto(),
                            Wire.BackgroundSizeKind.Cover => new UiBackgroundSize.Cover(),
                            Wire.BackgroundSizeKind.Contain => new UiBackgroundSize.Contain(),
                            Wire.BackgroundSizeKind.Axes when item.X.HasValue && item.Y.HasValue =>
                                new UiBackgroundSize.Axes(
                                    UiLengthOrAuto(item.X.Value),
                                    UiLengthOrAuto(item.Y.Value)
                                ),
                            _ => throw new InvalidDataException("Invalid background-size kind."),
                        };
                    }
                );
        }
    }
}

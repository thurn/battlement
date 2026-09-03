#nullable enable

namespace Battlement
{
    /// <summary>Runtime value shape accepted by an animation property.</summary>
    public enum MotionValueKind
    {
        Scalar,
        Length,
        Color,
        Vector2,
        Vector3,
        Angle,
        TransformList,
        FilterList,
        ShadowList,
        Gradient,
        ClipInset,
        ClipPolygon,
        Discrete,
    }

    /// <summary>Canonical interpolation behavior for one property.</summary>
    public enum MotionInterpolationCategory
    {
        Numeric,
        Length,
        Color,
        Structured,
        Discrete,
    }

    /// <summary>Additive composition available to a property.</summary>
    public enum MotionAdditiveRule
    {
        None,
        Sum,
        Multiply,
        Transform,
    }

    /// <summary>Reference box used to resolve percentage channels.</summary>
    public enum MotionPercentageReference
    {
        None,
        ContainingWidth,
        ContainingHeight,
        SelfWidth,
        SelfHeight,
    }

    /// <summary>Exhaustive animation-property catalog mirrored from Rust.</summary>
    public enum MotionProperty
    {
        AlignContent,
        AlignItems,
        AlignSelf,
        AspectRatio,
        BackgroundColor,
        BackgroundImage,
        BackgroundPositionX,
        BackgroundPositionY,
        BackgroundRepeat,
        BackgroundSize,
        BorderBottomColor,
        BorderBottomLeftRadius,
        BorderBottomRightRadius,
        BorderBottomWidth,
        BorderLeftColor,
        BorderLeftWidth,
        BorderRightColor,
        BorderRightWidth,
        BorderTopColor,
        BorderTopLeftRadius,
        BorderTopRightRadius,
        BorderTopWidth,
        Bottom,
        Color,
        Cursor,
        Display,
        Filter,
        FlexBasis,
        FlexDirection,
        FlexGrow,
        FlexShrink,
        FlexWrap,
        FontSize,
        Height,
        JustifyContent,
        Left,
        LetterSpacing,
        MarginBottom,
        MarginLeft,
        MarginRight,
        MarginTop,
        MaxHeight,
        MaxWidth,
        MinHeight,
        MinWidth,
        Opacity,
        Overflow,
        PaddingBottom,
        PaddingLeft,
        PaddingRight,
        PaddingTop,
        Position,
        Right,
        Rotate,
        Scale,
        TextOverflow,
        TextShadow,
        Top,
        TransformOrigin,
        Translate,
        UnityBackgroundImageTintColor,
        UnityEditorTextRenderingMode,
        UnityFontDefinition,
        UnityFontStyleAndWeight,
        UnityMaterial,
        UnityOverflowClipBox,
        UnityParagraphSpacing,
        UnitySliceBottom,
        UnitySliceLeft,
        UnitySliceRight,
        UnitySliceScale,
        UnitySliceTop,
        UnitySliceType,
        UnityTextAlign,
        UnityTextAutoSize,
        UnityTextGenerator,
        UnityTextOutlineColor,
        UnityTextOutlineWidth,
        UnityTextOverflowPosition,
        Visibility,
        WhiteSpace,
        Width,
        WordSpacing,
        X,
        Y,
        Z,
        RotateX,
        RotateY,
        ScaleX,
        ScaleY,
        SkewX,
        SkewY,
        TransformList,
        PaintFilter,
        BackgroundGradient,
        BoxShadow,
        ClipInset,
        ClipPolygon,
        Mask,
        Layout,
    }

    /// <summary>Complete generated metadata for one animation property.</summary>
    public sealed record MotionPropertyMetadata(
        string WireName,
        MotionValueKind ValueKind,
        string CanonicalUnit,
        string InitialValue,
        MotionInterpolationCategory Interpolation,
        MotionPercentageReference PercentageReference,
        MotionAdditiveRule Additive,
        string UnityWriter
    );
}

#nullable enable

using System.Collections.Generic;

namespace Battlement
{
    /// <summary>
    /// Describes a logical UI document authored in Rust and rendered by a Unity
    /// <c>UIDocument</c>.
    /// </summary>
    /// <param name="DocumentId">Identity of the GameObject hosting the native document.</param>
    /// <param name="RootId">Identity assigned to the native root visual element.</param>
    /// <param name="Name">Optional root name used by queries and USS ID selectors.</param>
    /// <param name="Enabled">Whether the root and its descendants can interact.</param>
    /// <param name="PickingMode">Whether pointer hit testing can select the root.</param>
    /// <param name="LanguageDirection">Text direction inherited by root descendants.</param>
    /// <param name="Focusable">Whether the root can receive focus.</param>
    /// <param name="TabIndex">Root ordering in the keyboard focus ring.</param>
    /// <param name="DelegatesFocus">Whether root focus transfers to a descendant.</param>
    /// <param name="Classes">USS classes applied to the root in the supplied order.</param>
    /// <param name="Style">Inline overrides applied directly to the root.</param>
    /// <param name="Events">Subscribed UI event kinds forwarded to Rust.</param>
    /// <param name="Children">Logical children added to the root in display order.</param>
    public sealed record UiDocument(
        ObjectId DocumentId,
        ObjectId RootId,
        Prop<string> Name = default,
        [property: Newtonsoft.Json.JsonProperty(
            NullValueHandling = Newtonsoft.Json.NullValueHandling.Include
        )]
            Prop<bool> Enabled = default,
        Prop<UiPickingMode> PickingMode = default,
        Prop<UiLanguageDirection> LanguageDirection = default,
        Prop<bool> Focusable = default,
        Prop<int> TabIndex = default,
        Prop<bool> DelegatesFocus = default,
        Prop<IReadOnlyList<string>> Classes = default,
        UiStyle? Style = null,
        Prop<IReadOnlyList<UiEventKind>> Events = default,
        IReadOnlyList<UiNode>? Children = null,
        Prop<IReadOnlyList<UiEventSubscription>> EventSubscriptions = default
    );

    /// <summary>One identified node in a logical UI hierarchy.</summary>
    public sealed record UiNode(
        ObjectId ObjectId,
        UiElement Element,
        IReadOnlyList<UiNode>? Children = null
    );

    /// <summary>
    /// Configures how a private runtime UI Toolkit panel is rendered, scaled,
    /// cleared, and atlased.
    /// </summary>
    /// <param name="RenderMode">Whether the panel renders over a display or in world space.</param>
    /// <param name="ScaleMode">How authored UI lengths become rendered pixels.</param>
    /// <param name="ReferenceSpritePixelsPerUnit">
    /// Number of sprite pixels corresponding to one UI unit.
    /// </param>
    /// <param name="Scale">Multiplier used by constant-pixel-size scaling.</param>
    /// <param name="ReferenceDpi">Design density used by physical-size scaling.</param>
    /// <param name="FallbackDpi">
    /// Density used when the target display reports no usable DPI.
    /// </param>
    /// <param name="ReferenceResolution">
    /// Design resolution used by scale-with-screen-size mode.
    /// </param>
    /// <param name="ScreenMatchMode">
    /// How target width and height contribute to screen-size scaling.
    /// </param>
    /// <param name="MatchFactor">
    /// Interpolation from width-based scaling at zero to height-based scaling at one.
    /// </param>
    /// <param name="TargetDisplay">Zero-based display for a screen-space overlay.</param>
    /// <param name="TargetTexture">
    /// Optional prepared render texture receiving panel output. Pointer mapping is explicit.
    /// </param>
    /// <param name="ClearDepthStencil">
    /// Whether depth and stencil buffers are cleared before rendering.
    /// </param>
    /// <param name="ClearColor">Whether the color buffer is cleared before rendering.</param>
    /// <param name="ColorClearValue">Color written when color clearing is enabled.</param>
    /// <param name="DynamicAtlas">Texture-atlas allocation limits and eligibility filters.</param>
    public sealed record PanelSettingsValue(
        PanelRenderMode RenderMode = PanelRenderMode.ScreenSpaceOverlay,
        PanelScaleMode ScaleMode = PanelScaleMode.ConstantPhysicalSize,
        float ReferenceSpritePixelsPerUnit = 100,
        float Scale = 1,
        float ReferenceDpi = 96,
        float FallbackDpi = 96,
        ScreenSize? ReferenceResolution = null,
        PanelScreenMatchMode ScreenMatchMode = PanelScreenMatchMode.MatchWidthOrHeight,
        float MatchFactor = 0,
        uint TargetDisplay = 0,
        RenderTextureAddress? TargetTexture = null,
        bool ClearDepthStencil = true,
        bool ClearColor = false,
        Color? ColorClearValue = null,
        DynamicAtlasSettingsValue? DynamicAtlas = null
    );

    /// <summary>Process-wide input settings for Battlement world-space documents.</summary>
    public sealed record PanelInputConfigurationValue(
        InteractionLayerMask InteractionLayers,
        InteractionDistance? MaximumInteractionDistance = null,
        PanelInputRedirection InputRedirection = PanelInputRedirection.AutoSwitch
    )
    {
        /// <summary>Creates Unity-compatible world-space input defaults.</summary>
        public PanelInputConfigurationValue()
            : this(new InteractionLayerMask(0xffff_fffb)) { }
    }

    /// <summary>Transparent Unity physics-layer mask.</summary>
    public readonly struct InteractionLayerMask : System.IEquatable<InteractionLayerMask>
    {
        /// <summary>Creates a mask from its exact Unity bit representation.</summary>
        public InteractionLayerMask(uint value) => Value = value;

        /// <summary>Gets the exact Unity bit representation.</summary>
        public uint Value { get; }

        public bool Equals(InteractionLayerMask other) => Value == other.Value;

        public override bool Equals(object? obj) =>
            obj is InteractionLayerMask other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();

        public static bool operator ==(InteractionLayerMask left, InteractionLayerMask right) =>
            left.Equals(right);

        public static bool operator !=(InteractionLayerMask left, InteractionLayerMask right) =>
            !left.Equals(right);
    }

    /// <summary>Maximum inclusive world-space UI picking distance.</summary>
    public abstract record InteractionDistance
    {
        private InteractionDistance() { }

        /// <summary>Maps to Unity positive infinity without non-finite JSON.</summary>
        public sealed record Unbounded : InteractionDistance;

        /// <summary>Uses a finite nonnegative inclusive distance.</summary>
        public sealed record Inclusive(float Value) : InteractionDistance;
    }

    /// <summary>
    /// Controls allocation and texture eligibility for the panel's dynamic atlas.
    /// Eligible textures can be batched to reduce rendering state changes.
    /// </summary>
    /// <param name="MinAtlasSize">Minimum width and height, in pixels, of a new atlas.</param>
    /// <param name="MaxAtlasSize">Maximum width and height, in pixels, of the atlas.</param>
    /// <param name="MaxSubTextureSize">
    /// Maximum texture width or height, in pixels, eligible for insertion.
    /// </param>
    /// <param name="Filters">Filters that exclude unsuitable textures from the atlas.</param>
    public sealed record DynamicAtlasSettingsValue(
        uint MinAtlasSize,
        uint MaxAtlasSize,
        uint MaxSubTextureSize,
        IReadOnlyList<DynamicAtlasFilter> Filters
    )
    {
        /// <summary>Creates the protocol default atlas configuration.</summary>
        public DynamicAtlasSettingsValue()
            : this(
                64,
                4096,
                64,
                new[]
                {
                    DynamicAtlasFilter.Readability,
                    DynamicAtlasFilter.Size,
                    DynamicAtlasFilter.Format,
                    DynamicAtlasFilter.ColorSpace,
                    DynamicAtlasFilter.FilterMode,
                }
            ) { }
    }

    /// <summary>An exclusive prepared source displayed by a UI Toolkit image.</summary>
    public abstract record ImageSource
    {
        private ImageSource() { }

        /// <summary>A raster Texture2D source.</summary>
        public sealed record Texture(TextureAddress Address) : ImageSource;

        /// <summary>A sprite source retaining imported sprite geometry.</summary>
        public sealed record Sprite(SpriteAddress Address) : ImageSource;

        /// <summary>A resolution-independent UI Toolkit vector image.</summary>
        public sealed record VectorImage(VectorImageAddress Address) : ImageSource;

        /// <summary>A live render-target texture.</summary>
        public sealed record RenderTexture(RenderTextureAddress Address) : ImageSource;
    }

    /// <summary>An exclusive prepared source displayed by a control icon.</summary>
    public abstract record IconSource
    {
        private IconSource() { }

        public sealed record Texture(TextureAddress Address) : IconSource;

        public sealed record Sprite(SpriteAddress Address) : IconSource;

        public sealed record VectorImage(VectorImageAddress Address) : IconSource;

        public sealed record RenderTexture(RenderTextureAddress Address) : IconSource;
    }

    /// <summary>A prepared graphical asset painted as an element background.</summary>
    public abstract record BackgroundSource
    {
        private BackgroundSource() { }

        /// <summary>A prepared raster texture.</summary>
        public sealed record Texture(TextureAddress Address) : BackgroundSource;

        /// <summary>A prepared sprite retaining imported geometry and border metadata.</summary>
        public sealed record Sprite(SpriteAddress Address) : BackgroundSource;

        /// <summary>A prepared resolution-independent UI Toolkit vector image.</summary>
        public sealed record VectorImage(VectorImageAddress Address) : BackgroundSource;

        /// <summary>A prepared live render-target texture.</summary>
        public sealed record RenderTexture(RenderTextureAddress Address) : BackgroundSource;
    }

    /// <summary>Controls how an image source fits its content rectangle.</summary>
    public enum ImageScaleMode
    {
        /// <summary>Preserves aspect ratio and fits the complete source.</summary>
        ScaleToFit,

        /// <summary>Preserves aspect ratio, fills the rectangle, and crops overflow.</summary>
        ScaleAndCrop,

        /// <summary>Stretches each axis independently to fill the rectangle.</summary>
        StretchToFill,
    }

    /// <summary>One concrete inline style value or an explicit reset keyword.</summary>
    public sealed record UiStyleValue<T>(T Value, UiInlineKeyword? Keyword = null)
    {
        /// <summary>Wraps one concrete property value.</summary>
        public static implicit operator UiStyleValue<T>(T value) => new(value);
    }

    /// <summary>A UI Toolkit pixel or percentage length.</summary>
    public abstract record UiLength
    {
        /// <summary>A device-independent pixel length.</summary>
        public sealed record Px(float Value) : UiLength;

        /// <summary>A parent-relative percentage length.</summary>
        public sealed record Percent(float Value) : UiLength;
    }

    /// <summary>A UI Toolkit length with an automatic layout case.</summary>
    public abstract record UiLengthOrAuto
    {
        /// <summary>A device-independent pixel length.</summary>
        public sealed record Px(float Value) : UiLengthOrAuto;

        /// <summary>A parent-relative percentage length.</summary>
        public sealed record Percent(float Value) : UiLengthOrAuto;

        /// <summary>Lets the layout engine derive the value.</summary>
        public sealed record Auto : UiLengthOrAuto;
    }

    /// <summary>A preferred width-to-height relationship.</summary>
    public abstract record UiAspectRatio
    {
        /// <summary>Leaves the preferred ratio automatic.</summary>
        public sealed record Auto : UiAspectRatio;

        /// <summary>Uses the finite positive quotient of width and height.</summary>
        public sealed record Ratio(float Width, float Height) : UiAspectRatio;
    }

    /// <summary>A background image anchor and offset on one axis.</summary>
    public sealed record UiBackgroundPosition(UiBackgroundPositionKeyword Keyword, UiLength Offset);

    /// <summary>Anchor used to position a background image.</summary>
    public enum UiBackgroundPositionKeyword
    {
        Center,
        Top,
        Bottom,
        Left,
        Right,
    }

    /// <summary>Background image repetition on both axes.</summary>
    public sealed record UiBackgroundRepeat(UiBackgroundRepeatMode X, UiBackgroundRepeatMode Y);

    /// <summary>How a background image repeats on one axis.</summary>
    public enum UiBackgroundRepeatMode
    {
        NoRepeat,
        Repeat,
        Round,
        Space,
    }

    /// <summary>How a background image is sized inside its element.</summary>
    public abstract record UiBackgroundSize
    {
        public sealed record Auto : UiBackgroundSize;

        public sealed record Cover : UiBackgroundSize;

        public sealed record Contain : UiBackgroundSize;

        public sealed record Axes(UiLengthOrAuto X, UiLengthOrAuto Y) : UiBackgroundSize;
    }

    /// <summary>Pixel coordinates inside a cursor texture.</summary>
    public sealed record UiCursorHotspot(float X, float Y);

    /// <summary>The pointer cursor shown while an element is hovered.</summary>
    public abstract record UiCursor
    {
        public sealed record Default : UiCursor;

        public sealed record Texture(TextureAddress Address, UiCursorHotspot Hotspot) : UiCursor;
    }

    /// <summary>A rotation in degrees around a nonzero three-dimensional axis.</summary>
    public sealed record UiRotate(float X, float Y, float Z, float Degrees);

    /// <summary>Horizontal and vertical paint-time size multipliers.</summary>
    public sealed record UiScale(float X, float Y);

    /// <summary>A paint-time offset with pixel or self-relative x and y axes.</summary>
    public sealed record UiTranslate(UiLength X, UiLength Y, float Z);

    /// <summary>Pivot used by scale and rotation transforms.</summary>
    public sealed record UiTransformOrigin(UiLength X, UiLength Y, float Z);

    public enum UiFontStyle
    {
        Normal,
        Bold,
        Italic,
        BoldAndItalic,
    }

    public enum UiTextAnchor
    {
        UpperLeft,
        UpperCenter,
        UpperRight,
        MiddleLeft,
        MiddleCenter,
        MiddleRight,
        LowerLeft,
        LowerCenter,
        LowerRight,
    }

    public abstract record UiTextAutoSize
    {
        public sealed record None : UiTextAutoSize;

        public sealed record BestFit(float MinSize, float MaxSize) : UiTextAutoSize;
    }

    public enum UiTextOverflow
    {
        Clip,
        Ellipsis,
    }

    public enum UiTextOverflowPosition
    {
        Start,
        Middle,
        End,
    }

    public enum UiWhiteSpace
    {
        Normal,
        NoWrap,
        Pre,
        PreWrap,
    }

    public enum UiTextGenerator
    {
        Standard,
        Advanced,
    }

    public enum UiEditorTextRenderingMode
    {
        Sdf,
        Bitmap,
    }

    public sealed record UiTextShadow(float X, float Y, float BlurRadius, Color Color);

    /// <summary>One standard UI Toolkit post-processing filter.</summary>
    public abstract record UiFilterFunction
    {
        private UiFilterFunction() { }

        public sealed record Tint(Color Value) : UiFilterFunction;

        public sealed record Opacity(float Value) : UiFilterFunction;

        public sealed record Invert(float Value) : UiFilterFunction;

        public sealed record Grayscale(float Value) : UiFilterFunction;

        public sealed record Sepia(float Value) : UiFilterFunction;

        public sealed record Blur(float Value) : UiFilterFunction;

        public sealed record Contrast(float Value) : UiFilterFunction;

        public sealed record HueRotate(float Value) : UiFilterFunction;
    }

    /// <summary>UI Toolkit easing curve used by a transition.</summary>
    public enum UiEasingFunction
    {
        Ease,
        EaseIn,
        EaseOut,
        EaseInOut,
        Linear,
        EaseInSine,
        EaseOutSine,
        EaseInOutSine,
        EaseInCubic,
        EaseOutCubic,
        EaseInOutCubic,
        EaseInCirc,
        EaseOutCirc,
        EaseInOutCirc,
        EaseInElastic,
        EaseOutElastic,
        EaseInOutElastic,
        EaseInBack,
        EaseOutBack,
        EaseInOutBack,
        EaseInBounce,
        EaseOutBounce,
        EaseInOutBounce,
    }

    /// <summary>Closed set of Battlement inline properties accepted by transitions.</summary>
    public enum UiTransitionProperty
    {
        All,
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
        TransitionDelay,
        TransitionDuration,
        TransitionProperty,
        TransitionTimingFunction,
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
    }

    /// <summary>
    /// Inline style overrides applied directly to a UI element. Unset properties
    /// preserve the current inline value; reset layout properties remove the
    /// authored declaration so USS or Unity's native initial style applies.
    /// </summary>
    public sealed record UiStyle(
        Prop<UiStyleValue<UiAlign>> AlignContent = default,
        Prop<UiStyleValue<UiAlign>> AlignItems = default,
        Prop<UiStyleValue<UiAlign>> AlignSelf = default,
        Prop<UiStyleValue<UiAspectRatio>> AspectRatio = default,
        UiStyleValue<Color>? BackgroundColor = null,
        UiStyleValue<BackgroundSource>? BackgroundImage = null,
        UiStyleValue<UiBackgroundPosition>? BackgroundPositionX = null,
        UiStyleValue<UiBackgroundPosition>? BackgroundPositionY = null,
        UiStyleValue<UiBackgroundRepeat>? BackgroundRepeat = null,
        UiStyleValue<UiBackgroundSize>? BackgroundSize = null,
        UiStyleValue<Color>? BorderBottomColor = null,
        UiStyleValue<UiLength>? BorderBottomLeftRadius = null,
        UiStyleValue<UiLength>? BorderBottomRightRadius = null,
        Prop<UiStyleValue<float>> BorderBottomWidth = default,
        UiStyleValue<Color>? BorderLeftColor = null,
        Prop<UiStyleValue<float>> BorderLeftWidth = default,
        UiStyleValue<Color>? BorderRightColor = null,
        Prop<UiStyleValue<float>> BorderRightWidth = default,
        UiStyleValue<Color>? BorderTopColor = null,
        UiStyleValue<UiLength>? BorderTopLeftRadius = null,
        UiStyleValue<UiLength>? BorderTopRightRadius = null,
        Prop<UiStyleValue<float>> BorderTopWidth = default,
        Prop<UiStyleValue<UiLengthOrAuto>> Bottom = default,
        UiStyleValue<Color>? Color = null,
        UiStyleValue<UiCursor>? Cursor = null,
        Prop<UiStyleValue<UiDisplay>> Display = default,
        UiStyleValue<IReadOnlyList<UiFilterFunction>>? Filter = null,
        Prop<UiStyleValue<UiLengthOrAuto>> FlexBasis = default,
        Prop<UiStyleValue<UiFlexDirection>> FlexDirection = default,
        Prop<UiStyleValue<float>> FlexGrow = default,
        Prop<UiStyleValue<float>> FlexShrink = default,
        Prop<UiStyleValue<UiFlexWrap>> FlexWrap = default,
        UiStyleValue<UiLength>? FontSize = null,
        Prop<UiStyleValue<UiLengthOrAuto>> Height = default,
        Prop<UiStyleValue<UiJustify>> JustifyContent = default,
        UiStyleValue<UiLength>? LetterSpacing = null,
        Prop<UiStyleValue<UiLengthOrAuto>> Left = default,
        Prop<UiStyleValue<UiLengthOrAuto>> MarginBottom = default,
        Prop<UiStyleValue<UiLengthOrAuto>> MarginLeft = default,
        Prop<UiStyleValue<UiLengthOrAuto>> MarginRight = default,
        Prop<UiStyleValue<UiLengthOrAuto>> MarginTop = default,
        Prop<UiStyleValue<UiLengthOrAuto>> MaxHeight = default,
        Prop<UiStyleValue<UiLengthOrAuto>> MaxWidth = default,
        Prop<UiStyleValue<UiLengthOrAuto>> MinHeight = default,
        Prop<UiStyleValue<UiLengthOrAuto>> MinWidth = default,
        UiStyleValue<float>? Opacity = null,
        Prop<UiStyleValue<UiOverflow>> Overflow = default,
        Prop<UiStyleValue<UiLength>> PaddingBottom = default,
        Prop<UiStyleValue<UiLength>> PaddingLeft = default,
        Prop<UiStyleValue<UiLength>> PaddingRight = default,
        Prop<UiStyleValue<UiLength>> PaddingTop = default,
        Prop<UiStyleValue<UiPosition>> Position = default,
        Prop<UiStyleValue<UiLengthOrAuto>> Right = default,
        UiStyleValue<UiRotate>? Rotate = null,
        UiStyleValue<UiScale>? Scale = null,
        UiStyleValue<UiTextOverflow>? TextOverflow = null,
        UiStyleValue<UiTextShadow>? TextShadow = null,
        Prop<UiStyleValue<UiLengthOrAuto>> Top = default,
        UiStyleValue<UiTransformOrigin>? TransformOrigin = null,
        UiStyleValue<IReadOnlyList<float>>? TransitionDelay = null,
        UiStyleValue<IReadOnlyList<float>>? TransitionDuration = null,
        UiStyleValue<IReadOnlyList<UiTransitionProperty>>? TransitionProperty = null,
        UiStyleValue<IReadOnlyList<UiEasingFunction>>? TransitionTimingFunction = null,
        UiStyleValue<UiTranslate>? Translate = null,
        UiStyleValue<Color>? UnityBackgroundImageTintColor = null,
        UiStyleValue<UiEditorTextRenderingMode>? UnityEditorTextRenderingMode = null,
        UiStyleValue<UiFontAddress>? UnityFontDefinition = null,
        UiStyleValue<UiFontStyle>? UnityFontStyleAndWeight = null,
        UiStyleValue<MaterialAddress>? UnityMaterial = null,
        UiStyleValue<UiOverflowClipBox>? UnityOverflowClipBox = null,
        UiStyleValue<UiLength>? UnityParagraphSpacing = null,
        UiStyleValue<int>? UnitySliceBottom = null,
        UiStyleValue<int>? UnitySliceLeft = null,
        UiStyleValue<int>? UnitySliceRight = null,
        UiStyleValue<float>? UnitySliceScale = null,
        UiStyleValue<int>? UnitySliceTop = null,
        UiStyleValue<UiSliceType>? UnitySliceType = null,
        UiStyleValue<UiTextAnchor>? UnityTextAlign = null,
        UiStyleValue<UiTextAutoSize>? UnityTextAutoSize = null,
        UiStyleValue<UiTextGenerator>? UnityTextGenerator = null,
        UiStyleValue<Color>? UnityTextOutlineColor = null,
        UiStyleValue<float>? UnityTextOutlineWidth = null,
        UiStyleValue<UiTextOverflowPosition>? UnityTextOverflowPosition = null,
        UiStyleValue<UiVisibility>? Visibility = null,
        UiStyleValue<UiWhiteSpace>? WhiteSpace = null,
        Prop<UiStyleValue<UiLengthOrAuto>> Width = default,
        UiStyleValue<UiLength>? WordSpacing = null
    )
    {
        /// <summary>Creates a concrete resettable inline-style assignment.</summary>
        public static Prop<UiStyleValue<T>> Set<T>(T value) =>
            Prop<UiStyleValue<T>>.Set(new UiStyleValue<T>(value));

        /// <summary>Creates a reset that removes the authored inline-style value.</summary>
        public static Prop<UiStyleValue<T>> Reset<T>() => Prop<UiStyleValue<T>>.Reset();
    }

    /// <summary>Panel rendering mode.</summary>
    public enum PanelRenderMode
    {
        /// <summary>Composites the panel over the selected display.</summary>
        ScreenSpaceOverlay,

        /// <summary>Renders the panel in the scene from the document transform.</summary>
        WorldSpace,
    }

    /// <summary>Panel scaling strategy.</summary>
    public enum PanelScaleMode
    {
        /// <summary>Uses authored pixel lengths with a uniform scale multiplier.</summary>
        ConstantPixelSize,

        /// <summary>Converts physical measurements using the display DPI.</summary>
        ConstantPhysicalSize,

        /// <summary>Scales relative to a configured design resolution.</summary>
        ScaleWithScreenSize,
    }

    /// <summary>Reference-resolution matching strategy.</summary>
    public enum PanelScreenMatchMode
    {
        /// <summary>Interpolates between width-based and height-based scale factors.</summary>
        MatchWidthOrHeight,

        /// <summary>Uses the smaller scale factor so the reference area fits.</summary>
        Shrink,

        /// <summary>Uses the larger scale factor so the reference area fills the target.</summary>
        Expand,
    }

    /// <summary>World-space panel input redirection policy.</summary>
    public enum PanelInputRedirection
    {
        /// <summary>Lets Unity select redirection based on current input state.</summary>
        AutoSwitch,

        /// <summary>Never redirects ordinary panel input.</summary>
        Never,

        /// <summary>Always redirects ordinary panel input.</summary>
        Always,
    }

    /// <summary>Dynamic atlas exclusion filter.</summary>
    public enum DynamicAtlasFilter
    {
        /// <summary>Excludes textures whose pixel data must remain CPU-readable.</summary>
        Readability,

        /// <summary>Excludes textures outside the configured atlas size limits.</summary>
        Size,

        /// <summary>Excludes texture formats the dynamic atlas cannot represent.</summary>
        Format,

        /// <summary>Excludes textures with a color space incompatible with the atlas.</summary>
        ColorSpace,

        /// <summary>Excludes textures whose sampling filter is incompatible.</summary>
        FilterMode,
    }

    /// <summary>UI document position mode.</summary>
    public enum DocumentPosition
    {
        /// <summary>Positions the document through ordinary layout flow.</summary>
        Relative,

        /// <summary>Positions the document independently of ordinary layout flow.</summary>
        Absolute,
    }

    /// <summary>World-space sizing strategy.</summary>
    public enum WorldSpaceSizeMode
    {
        /// <summary>Uses explicitly authored world-space dimensions.</summary>
        Fixed,

        /// <summary>Derives world-space dimensions from the document content.</summary>
        Dynamic,
    }

    /// <summary>Reference used for world-space pivot calculations.</summary>
    public enum PivotReferenceSize
    {
        /// <summary>Calculates the pivot relative to the document bounding box.</summary>
        BoundingBox,

        /// <summary>Calculates the pivot relative to the resolved layout size.</summary>
        Layout,
    }

    /// <summary>World-space document pivot.</summary>
    public enum DocumentPivot
    {
        /// <summary>Aligns the document's top-left corner with its transform.</summary>
        TopLeft,

        /// <summary>Aligns the midpoint of the top edge with its transform.</summary>
        TopCenter,

        /// <summary>Aligns the document's top-right corner with its transform.</summary>
        TopRight,

        /// <summary>Aligns the midpoint of the left edge with its transform.</summary>
        MiddleLeft,

        /// <summary>Aligns the document center with its transform.</summary>
        Center,

        /// <summary>Aligns the midpoint of the right edge with its transform.</summary>
        MiddleRight,

        /// <summary>Aligns the document's bottom-left corner with its transform.</summary>
        BottomLeft,

        /// <summary>Aligns the midpoint of the bottom edge with its transform.</summary>
        BottomCenter,

        /// <summary>Aligns the document's bottom-right corner with its transform.</summary>
        BottomRight,
    }

    /// <summary>Main-axis direction used by a flex container to arrange its children.</summary>
    public enum UiFlexDirection
    {
        /// <summary>Places children vertically from top to bottom.</summary>
        Column,

        /// <summary>Places children vertically from bottom to top.</summary>
        ColumnReverse,

        /// <summary>Places children horizontally from left to right.</summary>
        Row,

        /// <summary>Places children horizontally from right to left.</summary>
        RowReverse,
    }

    /// <summary>Explicit USS keyword for clearing an inline declaration.</summary>
    public enum UiInlineKeyword
    {
        /// <summary>Restores the property's Unity initial value.</summary>
        Initial,
    }

    /// <summary>Cross-axis alignment for a flex container or item.</summary>
    public enum UiAlign
    {
        /// <summary>Uses the container's alignment behavior.</summary>
        Auto,

        /// <summary>Packs content at the cross-axis start.</summary>
        FlexStart,

        /// <summary>Centers content on the cross axis.</summary>
        Center,

        /// <summary>Packs content at the cross-axis end.</summary>
        FlexEnd,

        /// <summary>Stretches auto-sized content across the cross axis.</summary>
        Stretch,
    }

    /// <summary>Multi-line placement behavior for a flex container.</summary>
    public enum UiFlexWrap
    {
        /// <summary>Keeps all children on one line.</summary>
        NoWrap,

        /// <summary>Moves overflowing children to additional lines.</summary>
        Wrap,

        /// <summary>Wraps in the reverse cross-axis direction.</summary>
        WrapReverse,
    }

    /// <summary>Main-axis distribution of flex children.</summary>
    public enum UiJustify
    {
        /// <summary>Packs children at the main-axis start.</summary>
        FlexStart,

        /// <summary>Centers children on the main axis.</summary>
        Center,

        /// <summary>Packs children at the main-axis end.</summary>
        FlexEnd,

        /// <summary>Places free space between children.</summary>
        SpaceBetween,

        /// <summary>Places free space around children.</summary>
        SpaceAround,

        /// <summary>Uses equal space between children and edges.</summary>
        SpaceEvenly,
    }

    /// <summary>Participation in normal flex flow.</summary>
    public enum UiPosition
    {
        /// <summary>Remains in flex flow and offsets from the normal position.</summary>
        Relative,

        /// <summary>Leaves flex flow and positions against the parent.</summary>
        Absolute,
    }

    /// <summary>Whether an element participates in layout and rendering.</summary>
    public enum UiDisplay
    {
        /// <summary>Keeps the element in UI Toolkit flex layout and renders it.</summary>
        Flex,

        /// <summary>Removes the element subtree from layout and rendering.</summary>
        None,
    }

    /// <summary>Whether an element draws while retaining its layout space.</summary>
    public enum UiVisibility
    {
        /// <summary>Draws the element normally.</summary>
        Visible,

        /// <summary>Suppresses drawing while preserving layout space.</summary>
        Hidden,
    }

    /// <summary>Whether descendant painting is clipped to an element's bounds.</summary>
    public enum UiOverflow
    {
        /// <summary>Allows descendants to paint outside the element.</summary>
        Visible,

        /// <summary>Clips descendants at the selected overflow clip box.</summary>
        Hidden,
    }

    /// <summary>Box edge used for hidden-overflow clipping.</summary>
    public enum UiOverflowClipBox
    {
        /// <summary>Clips at the outer edge of the padding box.</summary>
        PaddingBox,

        /// <summary>Clips at the content box inside padding.</summary>
        ContentBox,
    }

    /// <summary>Painting mode for nine-sliced background regions.</summary>
    public enum UiSliceType
    {
        /// <summary>Stretches the center and edge regions.</summary>
        Sliced,

        /// <summary>Repeats the center and edge regions.</summary>
        Tiled,
    }

    /// <summary>Pointer hit-testing behavior for a UI element.</summary>
    public enum UiPickingMode
    {
        /// <summary>Tests the element's layout rectangle for pointer input.</summary>
        Position,

        /// <summary>Excludes the element itself from pointer hit testing.</summary>
        Ignore,
    }

    /// <summary>Text directionality inherited through a UI hierarchy.</summary>
    public enum UiLanguageDirection
    {
        /// <summary>Uses the direction of the nearest explicit ancestor.</summary>
        Inherit,

        /// <summary>Uses left-to-right text direction.</summary>
        Ltr,

        /// <summary>Uses right-to-left text direction.</summary>
        Rtl,
    }

    /// <summary>A create-time rendering optimization hint for a UI element.</summary>
    public enum UiUsageHint
    {
        /// <summary>Optimizes frequent position or transform changes.</summary>
        DynamicTransform,

        /// <summary>Optimizes a transform-changing container with dynamic descendants.</summary>
        GroupTransform,

        /// <summary>Optimizes a container with nested descendant masks.</summary>
        MaskContainer,

        /// <summary>Optimizes frequent rendered-color changes.</summary>
        DynamicColor,

        /// <summary>Optimizes post-processing effects.</summary>
        DynamicPostProcessing,

        /// <summary>Optimizes elements covering a large pixel area.</summary>
        LargePixelCoverage,
    }
}

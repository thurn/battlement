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
        string? Name = null,
        bool? Enabled = null,
        UiPickingMode? PickingMode = null,
        UiLanguageDirection? LanguageDirection = null,
        bool? Focusable = null,
        int? TabIndex = null,
        bool? DelegatesFocus = null,
        IReadOnlyList<string>? Classes = null,
        UiStyle? Style = null,
        IReadOnlyList<UiEventKind>? Events = null,
        IReadOnlyList<UiNode>? Children = null
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
        bool ClearDepthStencil = true,
        bool ClearColor = false,
        Color? ColorClearValue = null,
        DynamicAtlasSettingsValue? DynamicAtlas = null
    );

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

    /// <summary>Represents sparse visual properties for a concrete UI Toolkit element.</summary>
    public abstract record UiElement
    {
        /// <summary>The name of this visual element.</summary>
        public string? Name { get; init; }

        /// <summary>Whether this visual element is enabled locally.</summary>
        public bool? Enabled { get; init; }

        /// <summary>Whether pointer hit testing can select this element.</summary>
        public UiPickingMode? PickingMode { get; init; }

        /// <summary>Text direction inherited by this element's descendants.</summary>
        public UiLanguageDirection? LanguageDirection { get; init; }

        /// <summary>Whether this element can receive focus.</summary>
        public bool? Focusable { get; init; }

        /// <summary>Ordering of this element in the keyboard focus ring.</summary>
        public int? TabIndex { get; init; }

        /// <summary>Whether focus requested here transfers to a descendant.</summary>
        public bool? DelegatesFocus { get; init; }

        /// <summary>The USS classes of this visual element.</summary>
        public IReadOnlyList<string>? Classes { get; init; }

        /// <summary>Create-time rendering optimization hints for this element.</summary>
        public IReadOnlyList<UiUsageHint>? UsageHints { get; init; }

        /// <summary>The style values on this visual element.</summary>
        public UiStyle? Style { get; init; }

        /// <summary>UI events forwarded to Rust.</summary>
        public IReadOnlyList<UiEventKind>? Events { get; init; }

        /// <summary>The base class for objects in the UI Toolkit visual tree.</summary>
        public sealed record VisualElement : UiElement;

        /// <summary>A Unity UI Toolkit Box.</summary>
        public sealed record Box : UiElement;

        /// <summary>A text element that displays text.</summary>
        public sealed record Label : UiElement
        {
            /// <summary>The text to be displayed.</summary>
            public string? Text { get; init; }
        }

        /// <summary>A clickable button with a text label element.</summary>
        public sealed record Button : UiElement
        {
            /// <summary>The text to be displayed.</summary>
            public string? Text { get; init; }
        }

        /// <summary>A leaf UI Toolkit image with one exclusive prepared source.</summary>
        public sealed record Image : UiElement
        {
            /// <summary>The prepared raster, sprite, vector, or render-texture source.</summary>
            public ImageSource? Source { get; init; }

            /// <summary>
            /// Upper-left-origin pixel rectangle sampled from a non-sprite source.
            /// </summary>
            public Rect? SourceRect { get; init; }

            /// <summary>Linear color multiplied with the sampled source pixels.</summary>
            public Color? TintColor { get; init; }

            /// <summary>How the source fits and crops inside the content rectangle.</summary>
            public ImageScaleMode? ScaleMode { get; init; }

            /// <summary>Lower-left-origin normalized base texture coordinates.</summary>
            public Rect? Uv { get; init; }
        }
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

    /// <summary>
    /// Inline style overrides applied directly to a UI element. Null properties
    /// preserve the current inline value; an Initial keyword clears it explicitly.
    /// </summary>
    public sealed record UiStyle(
        UiStyleValue<UiAlign>? AlignContent = null,
        UiStyleValue<UiAlign>? AlignItems = null,
        UiStyleValue<UiAlign>? AlignSelf = null,
        UiStyleValue<UiAspectRatio>? AspectRatio = null,
        Color? BackgroundColor = null,
        UiStyleValue<UiLengthOrAuto>? Bottom = null,
        Color? Color = null,
        UiStyleValue<UiLengthOrAuto>? FlexBasis = null,
        UiStyleValue<UiFlexDirection>? FlexDirection = null,
        UiStyleValue<float>? FlexGrow = null,
        UiStyleValue<float>? FlexShrink = null,
        UiStyleValue<UiFlexWrap>? FlexWrap = null,
        float? FontSize = null,
        UiStyleValue<UiLengthOrAuto>? Height = null,
        UiStyleValue<UiJustify>? JustifyContent = null,
        UiStyleValue<UiLengthOrAuto>? Left = null,
        UiStyleValue<UiLengthOrAuto>? MarginBottom = null,
        UiStyleValue<UiLengthOrAuto>? MarginLeft = null,
        UiStyleValue<UiLengthOrAuto>? MarginRight = null,
        UiStyleValue<UiLengthOrAuto>? MarginTop = null,
        UiStyleValue<UiLengthOrAuto>? MaxHeight = null,
        UiStyleValue<UiLengthOrAuto>? MaxWidth = null,
        UiStyleValue<UiLengthOrAuto>? MinHeight = null,
        UiStyleValue<UiLengthOrAuto>? MinWidth = null,
        UiStyleValue<UiLength>? PaddingBottom = null,
        UiStyleValue<UiLength>? PaddingLeft = null,
        UiStyleValue<UiLength>? PaddingRight = null,
        UiStyleValue<UiLength>? PaddingTop = null,
        UiStyleValue<UiPosition>? Position = null,
        UiStyleValue<UiLengthOrAuto>? Right = null,
        UiStyleValue<UiLengthOrAuto>? Top = null,
        UiStyleValue<UiLengthOrAuto>? Width = null
    );

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

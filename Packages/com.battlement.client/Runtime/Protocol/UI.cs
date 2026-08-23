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
    /// <param name="Classes">USS classes applied to the root in the supplied order.</param>
    /// <param name="Style">Inline overrides applied directly to the root.</param>
    /// <param name="Children">Logical children added to the root in display order.</param>
    public sealed record UiDocument(
        ObjectId DocumentId,
        ObjectId RootId,
        string? Name = null,
        bool Enabled = true,
        IReadOnlyList<string>? Classes = null,
        UiStyle? Style = null,
        IReadOnlyList<UiElement>? Children = null
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

    /// <summary>
    /// Represents a concrete UI Toolkit element in the recursive document hierarchy.
    /// The nested record type is the stable JSON discriminator used to select the
    /// corresponding Unity element class.
    /// </summary>
    public abstract record UiElement
    {
        private UiElement() { }

        /// <summary>
        /// A general-purpose layout, styling, and hierarchy node with no
        /// control-specific behavior or built-in box presentation.
        /// </summary>
        /// <param name="ObjectId">Stable identity used across snapshots and events.</param>
        /// <param name="Name">Optional name used by queries and USS ID selectors.</param>
        /// <param name="Enabled">Whether the element and its descendants can interact.</param>
        /// <param name="Classes">USS classes applied in the supplied order.</param>
        /// <param name="Style">Inline style overrides applied to the element.</param>
        /// <param name="Children">Logical children added in display order.</param>
        public sealed record VisualElement(
            ObjectId ObjectId,
            string? Name = null,
            bool Enabled = true,
            IReadOnlyList<string>? Classes = null,
            UiStyle? Style = null,
            IReadOnlyList<UiElement>? Children = null
        ) : UiElement;

        /// <summary>
        /// A container with ordinary flex-layout behavior and Unity's standard box
        /// USS class, allowing the active theme to supply box presentation.
        /// </summary>
        /// <param name="ObjectId">Stable identity used across snapshots and events.</param>
        /// <param name="Name">Optional name used by queries and USS ID selectors.</param>
        /// <param name="Enabled">Whether the box and its descendants can interact.</param>
        /// <param name="Classes">USS classes applied in the supplied order.</param>
        /// <param name="Style">Inline style overrides applied to the box.</param>
        /// <param name="Children">Logical children added in display order.</param>
        public sealed record Box(
            ObjectId ObjectId,
            string? Name = null,
            bool Enabled = true,
            IReadOnlyList<string>? Classes = null,
            UiStyle? Style = null,
            IReadOnlyList<UiElement>? Children = null
        ) : UiElement;

        /// <summary>
        /// A display-only text element whose natural layout size is derived from
        /// its text and font styling.
        /// </summary>
        /// <param name="ObjectId">Stable identity used across snapshots and events.</param>
        /// <param name="Name">Optional name used by queries and USS ID selectors.</param>
        /// <param name="Enabled">Whether the label can participate in interaction.</param>
        /// <param name="Classes">USS classes applied in the supplied order.</param>
        /// <param name="Style">Inline style overrides, including text presentation.</param>
        /// <param name="Children">Logical children added in display order.</param>
        /// <param name="Text">Non-editable text displayed by the label.</param>
        public sealed record Label(
            ObjectId ObjectId,
            string? Name = null,
            bool Enabled = true,
            IReadOnlyList<string>? Classes = null,
            UiStyle? Style = null,
            IReadOnlyList<UiElement>? Children = null,
            string? Text = null
        ) : UiElement;
    }

    /// <summary>
    /// Inline style overrides applied directly to a UI element. Null properties are
    /// omitted so USS, inheritance, or Unity defaults can determine resolved values.
    /// Length values are expressed in pixels.
    /// </summary>
    /// <param name="BackgroundColor">
    /// Color painted behind the content and padding area.
    /// </param>
    /// <param name="Color">
    /// Foreground color inherited by descendant text unless overridden.
    /// </param>
    /// <param name="Width">Width of the element's layout box in pixels.</param>
    /// <param name="Height">Height of the element's layout box in pixels.</param>
    /// <param name="FlexGrow">
    /// Proportion of remaining main-axis space assigned relative to growing siblings.
    /// </param>
    /// <param name="FlexDirection">Main axis used to arrange child elements.</param>
    /// <param name="Padding">
    /// Space in pixels on every side between the border and content.
    /// </param>
    /// <param name="Margin">
    /// Space in pixels on every side outside the border.
    /// </param>
    /// <param name="FontSize">Text size in pixels inherited by descendants.</param>
    public sealed record UiStyle(
        Color? BackgroundColor = null,
        Color? Color = null,
        float? Width = null,
        float? Height = null,
        float? FlexGrow = null,
        UiFlexDirection? FlexDirection = null,
        float? Padding = null,
        float? Margin = null,
        float? FontSize = null
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

        /// <summary>Places children horizontally from left to right.</summary>
        Row,
    }
}

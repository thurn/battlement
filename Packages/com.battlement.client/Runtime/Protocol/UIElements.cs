#nullable enable

using System.Collections.Generic;

namespace Battlement
{
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
            public bool? EnableRichText { get; init; }
            public bool? EmojiFallbackSupport { get; init; }
            public bool? ParseEscapeSequences { get; init; }
            public bool? DisplayTooltipWhenElided { get; init; }
            public bool? Selectable { get; init; }
            public bool? DoubleClickSelectsWord { get; init; }
            public bool? TripleClickSelectsLine { get; init; }
            public bool? SelectAllOnFocus { get; init; }
            public bool? SelectAllOnMouseUp { get; init; }
        }

        /// <summary>A leaf base text element with rich-text and selection preferences.</summary>
        public sealed record TextElement : UiElement
        {
            public string? Text { get; init; }
            public bool? EnableRichText { get; init; }
            public bool? EmojiFallbackSupport { get; init; }
            public bool? ParseEscapeSequences { get; init; }
            public bool? DisplayTooltipWhenElided { get; init; }
            public bool? Selectable { get; init; }
            public bool? DoubleClickSelectsWord { get; init; }
            public bool? TripleClickSelectsLine { get; init; }
            public bool? SelectAllOnFocus { get; init; }
            public bool? SelectAllOnMouseUp { get; init; }
        }

        /// <summary>A clickable button with a text label element.</summary>
        public sealed record Button : UiElement
        {
            /// <summary>The text to be displayed.</summary>
            public string? Text { get; init; }
            public bool? EnableRichText { get; init; }
            public bool? EmojiFallbackSupport { get; init; }
            public bool? ParseEscapeSequences { get; init; }
            public bool? DisplayTooltipWhenElided { get; init; }
            public bool? Selectable { get; init; }
            public bool? DoubleClickSelectsWord { get; init; }
            public bool? TripleClickSelectsLine { get; init; }
            public bool? SelectAllOnFocus { get; init; }
            public bool? SelectAllOnMouseUp { get; init; }

            /// <summary>The prepared asset displayed by the native icon slot.</summary>
            public IconSource? Icon { get; init; }
        }

        /// <summary>A button that repeatedly activates while held.</summary>
        public sealed record RepeatButton : UiElement
        {
            public string? Text { get; init; }
            public uint? DelayMs { get; init; }
            public uint? IntervalMs { get; init; }
            public bool? EnableRichText { get; init; }
            public bool? EmojiFallbackSupport { get; init; }
            public bool? ParseEscapeSequences { get; init; }
            public bool? DisplayTooltipWhenElided { get; init; }
            public bool? Selectable { get; init; }
            public bool? DoubleClickSelectsWord { get; init; }
            public bool? TripleClickSelectsLine { get; init; }
            public bool? SelectAllOnFocus { get; init; }
            public bool? SelectAllOnMouseUp { get; init; }
        }

        /// <summary>A container that groups related controls under an optional title.</summary>
        public sealed record GroupBox : UiElement
        {
            /// <summary>The text displayed by the conditional native title label.</summary>
            public string? Text { get; init; }
        }

        /// <summary>A popup-styled text container with a dedicated content container.</summary>
        public sealed record PopupWindow : UiElement
        {
            public string? Text { get; init; }
            public bool? EnableRichText { get; init; }
            public bool? EmojiFallbackSupport { get; init; }
            public bool? ParseEscapeSequences { get; init; }
            public bool? DisplayTooltipWhenElided { get; init; }
            public bool? Selectable { get; init; }
            public bool? DoubleClickSelectsWord { get; init; }
            public bool? TripleClickSelectsLine { get; init; }
            public bool? SelectAllOnFocus { get; init; }
            public bool? SelectAllOnMouseUp { get; init; }
        }

        /// <summary>A viewport that scrolls arbitrary child content on one or both axes.</summary>
        public sealed record ScrollView : UiElement
        {
            public UiScrollViewMode? Mode { get; init; }
            public UiNestedInteraction? NestedInteraction { get; init; }
            public UiScrollerVisibility? HorizontalScrollerVisibility { get; init; }
            public UiScrollerVisibility? VerticalScrollerVisibility { get; init; }
            public Vector? ScrollOffset { get; init; }
            public float? HorizontalPageSize { get; init; }
            public float? VerticalPageSize { get; init; }
            public float? MouseWheelScrollSize { get; init; }
            public UiTouchScrollBehavior? TouchScrollBehavior { get; init; }
            public float? ScrollDecelerationRate { get; init; }
            public float? Elasticity { get; init; }
            public uint? ElasticAnimationInterval { get; init; }
        }

        /// <summary>A controlled scrollbar that proposes values within an authored range.</summary>
        public sealed record Scroller : UiElement
        {
            public float? LowValue { get; init; }
            public float? HighValue { get; init; }
            public UiSliderDirection? Direction { get; init; }
            public float? Value { get; init; }
        }

        /// <summary>One labeled page placed directly beneath a TabView.</summary>
        public sealed record Tab : UiElement
        {
            public string? Text { get; init; }
            public IconSource? Icon { get; init; }
            public bool? Closeable { get; init; }
        }

        /// <summary>A controlled selection and reorder container for Tab children.</summary>
        public sealed record TabView : UiElement
        {
            public uint? SelectedTabIndex { get; init; }
            public bool? Reorderable { get; init; }
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

    public enum UiScrollViewMode
    {
        Vertical,
        Horizontal,
        VerticalAndHorizontal,
    }

    public enum UiNestedInteraction
    {
        Default,
        StopScrolling,
        ForwardScrolling,
    }

    public enum UiScrollerVisibility
    {
        Auto,
        AlwaysVisible,
        Hidden,
    }

    public enum UiTouchScrollBehavior
    {
        Unrestricted,
        Elastic,
        Clamped,
    }

    public enum UiSliderDirection
    {
        Horizontal,
        Vertical,
    }
}

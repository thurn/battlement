#nullable enable

using System;
using System.Collections.Generic;
using Newtonsoft.Json;

namespace Battlement
{
    /// <summary>Distinguishes omitted, assigned, and resettable property values.</summary>
    public enum PropState
    {
        /// <summary>The property is absent and leaves live state unchanged.</summary>
        Unset,

        /// <summary>The property assigns a concrete value.</summary>
        Set,

        /// <summary>The property restores its documented native default.</summary>
        Reset,
    }

    /// <summary>A sparse property that can be omitted, assigned, or reset.</summary>
    public readonly struct Prop<T> : IEquatable<Prop<T>>
    {
        private readonly T value;

        private Prop(PropState state, T value)
        {
            State = state;
            this.value = value;
        }

        /// <summary>The requested property operation.</summary>
        public PropState State { get; }

        /// <summary>The assigned value.</summary>
        /// <exception cref="InvalidOperationException">The property is not set.</exception>
        public T Value =>
            State == PropState.Set
                ? value
                : throw new InvalidOperationException("An unset or reset property has no value.");

        /// <summary>Whether this property is omitted.</summary>
        public bool IsUnset => State == PropState.Unset;

        /// <summary>Whether this property assigns a value.</summary>
        public bool IsSet => State == PropState.Set;

        /// <summary>Whether this property restores its native default.</summary>
        public bool IsReset => State == PropState.Reset;

        /// <summary>Creates a concrete property assignment.</summary>
        public static Prop<T> Set(T value) =>
            value is null
                ? throw new ArgumentNullException(nameof(value))
                : new Prop<T>(PropState.Set, value);

        /// <summary>Creates a request to restore the documented native default.</summary>
        public static Prop<T> Reset() => new(PropState.Reset, default!);

        /// <summary>Converts an ordinary value into a concrete property assignment.</summary>
        public static implicit operator Prop<T>(T value) => Set(value);

        /// <inheritdoc />
        public bool Equals(Prop<T> other) =>
            State == other.State
            && (State != PropState.Set || EqualityComparer<T>.Default.Equals(value, other.value));

        /// <inheritdoc />
        public override bool Equals(object? obj) => obj is Prop<T> other && Equals(other);

        /// <inheritdoc />
        public override int GetHashCode() =>
            HashCode.Combine(State, State == PropState.Set ? value : default);

        /// <summary>Compares two property operations and their assigned values.</summary>
        public static bool operator ==(Prop<T> left, Prop<T> right) => left.Equals(right);

        /// <summary>Compares two property operations and their assigned values.</summary>
        public static bool operator !=(Prop<T> left, Prop<T> right) => !left.Equals(right);
    }

    /// <summary>Represents sparse visual properties for a concrete UI Toolkit element.</summary>
    public abstract record UiElement
    {
        /// <summary>The name of this visual element.</summary>
        public string? Name { get; init; }

        /// <summary>Whether this visual element is enabled locally.</summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<bool> Enabled { get; init; }

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

        /// <summary>UI event subscriptions with explicit route phases.</summary>
        public IReadOnlyList<UiEventSubscription>? EventSubscriptions { get; init; }

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

        /// <summary>A controlled text editor with a native local draft.</summary>
        public sealed record TextField : UiElement
        {
            public new string? Label { get; init; }
            public string? Value { get; init; }
            public bool? Multiline { get; init; }
            public UiScrollerVisibility? VerticalScrollerVisibility { get; init; }
            public bool? Password { get; init; }
            public bool? ReadOnly { get; init; }
            public string? Placeholder { get; init; }
            public bool? HidePlaceholderOnFocus { get; init; }
            public uint? CursorIndex { get; init; }
            public uint? SelectIndex { get; init; }
            public bool? SelectAllOnFocus { get; init; }
            public bool? SelectAllOnMouseUp { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled Boolean switch.</summary>
        public sealed record Toggle : UiElement
        {
            public new string? Label { get; init; }
            public string? Text { get; init; }
            public bool? Value { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled standalone Boolean radio option.</summary>
        public sealed record RadioButton : UiElement
        {
            public new string? Label { get; init; }
            public string? Text { get; init; }
            public bool? Value { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled exclusive choice rendered as native radio options.</summary>
        public sealed record RadioButtonGroup : UiElement
        {
            public new string? Label { get; init; }
            public IReadOnlyList<string>? Choices { get; init; }
            public uint? SelectedIndex { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled selection group containing ordinary Button children.</summary>
        public sealed record ToggleButtonGroup : UiElement
        {
            public new string? Label { get; init; }
            public bool? MultipleSelection { get; init; }
            public bool? AllowEmptySelection { get; init; }
            public IReadOnlyList<uint>? SelectedIndices { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled single-choice popup selector.</summary>
        public sealed record DropdownField : UiElement
        {
            public new string? Label { get; init; }
            public bool? ShowMixedValue { get; init; }
            public IReadOnlyList<string>? Choices { get; init; }
            public DropdownChoice? Selection { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
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

            /// <summary>The prepared asset displayed by the native icon slot.</summary>
            public IconSource? Icon { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
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
        }

        /// <summary>A container that groups related controls under an optional title.</summary>
        public sealed record GroupBox : UiElement
        {
            /// <summary>The text displayed by the conditional native title label.</summary>
            public string? Text { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
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
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
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
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled scrollbar that proposes values within an authored range.</summary>
        public sealed record Scroller : UiElement
        {
            public float? LowValue { get; init; }
            public float? HighValue { get; init; }
            public UiSliderDirection? Direction { get; init; }
            public float? Value { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled floating-point range slider.</summary>
        public sealed record Slider : UiElement
        {
            public new string? Label { get; init; }
            public float? LowValue { get; init; }
            public float? HighValue { get; init; }
            public float? Value { get; init; }
            public bool? Fill { get; init; }
            public float? PageSize { get; init; }
            public bool? ShowInputField { get; init; }
            public UiSliderDirection? Direction { get; init; }
            public bool? Inverted { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled integer range slider.</summary>
        public sealed record SliderInt : UiElement
        {
            public new string? Label { get; init; }
            public int? LowValue { get; init; }
            public int? HighValue { get; init; }
            public int? Value { get; init; }
            public bool? Fill { get; init; }
            public float? PageSize { get; init; }
            public bool? ShowInputField { get; init; }
            public UiSliderDirection? Direction { get; init; }
            public bool? Inverted { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled dual-thumb floating-point range selector.</summary>
        public sealed record MinMaxSlider : UiElement
        {
            public new string? Label { get; init; }
            public float? MinValue { get; init; }
            public float? MaxValue { get; init; }
            public LowerLimit? LowLimit { get; init; }
            public UpperLimit? HighLimit { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>An output-only progress indicator.</summary>
        public sealed record ProgressBar : UiElement
        {
            public float? LowValue { get; init; }
            public float? HighValue { get; init; }
            public float? Value { get; init; }
            public string? Title { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>One labeled page placed directly beneath a TabView.</summary>
        public sealed record Tab : UiElement
        {
            public string? Text { get; init; }
            public IconSource? Icon { get; init; }
            public bool? Closeable { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled selection and reorder container for Tab children.</summary>
        public sealed record TabView : UiElement
        {
            public uint? SelectedTabIndex { get; init; }
            public bool? Reorderable { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
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

    /// <summary>One sparse inline-style update for a Unity-created control part.</summary>
    public sealed record UiPartStyle(UiPart Part, UiStyle Style)
    {
        public uint? Index { get; init; }
    }

    /// <summary>Closed wire keys for Unity-created parts owned by simple controls.</summary>
    public enum UiPart
    {
        ButtonIcon,
        GroupBoxTitle,
        PopupWindowContentContainer,
        ToggleLabel,
        ToggleInput,
        ToggleCheckmark,
        ToggleText,
        RadioButtonLabel,
        RadioButtonInput,
        RadioButtonCheckmarkBackground,
        RadioButtonCheckmark,
        RadioButtonText,
        DropdownFieldLabel,
        DropdownFieldInput,
        DropdownFieldText,
        DropdownFieldArrow,
        ProgressBarContainer,
        ProgressBarBackground,
        ProgressBarProgress,
        ProgressBarTitleContainer,
        ProgressBarTitle,
        ScrollViewContentAndVerticalScrollContainer,
        ScrollViewViewport,
        ScrollViewContentContainer,
        ScrollViewHorizontalScroller,
        ScrollViewHorizontalSlider,
        ScrollViewHorizontalLowButton,
        ScrollViewHorizontalHighButton,
        ScrollViewHorizontalTrack,
        ScrollViewHorizontalDragger,
        ScrollViewHorizontalDraggerBorder,
        ScrollViewVerticalScroller,
        ScrollViewVerticalSlider,
        ScrollViewVerticalLowButton,
        ScrollViewVerticalHighButton,
        ScrollViewVerticalTrack,
        ScrollViewVerticalDragger,
        ScrollViewVerticalDraggerBorder,
        ScrollerSlider,
        ScrollerLowButton,
        ScrollerHighButton,
        ScrollerTrack,
        ScrollerDragger,
        ScrollerDraggerBorder,
        TabHeader,
        TabLabel,
        TabIcon,
        TabUnderline,
        TabCloseButton,
        TabDragHandle,
        TabDragHandleLeadingBar,
        TabDragHandleTrailingBar,
        TabContentContainer,
        TabViewContentViewport,
        TabViewHeaderContainer,
        TabViewContentContainer,
        TabViewPreviousButton,
        TabViewNextButton,
        TextFieldLabel,
        TextFieldInput,
        TextFieldTextElement,
        TextFieldMultilineScrollView,
        TextFieldVerticalScroller,
        TextFieldVerticalSlider,
        TextFieldVerticalLowButton,
        TextFieldVerticalHighButton,
        TextFieldVerticalTrack,
        TextFieldVerticalDragger,
        TextFieldVerticalDraggerBorder,
        RadioButtonGroupLabel,
        RadioButtonGroupInput,
        RadioButtonGroupChoicesContainer,
        RadioButtonGroupContentContainer,
        RadioButtonGroupAllOptions,
        RadioButtonGroupOption,
        RadioButtonGroupOptionCheckmarkBackground,
        RadioButtonGroupOptionCheckmark,
        RadioButtonGroupOptionText,
        ToggleButtonGroupLabel,
        ToggleButtonGroupInput,
        SliderLabel,
        SliderInput,
        SliderTrack,
        SliderDragger,
        SliderDraggerBorder,
        SliderFill,
        SliderTextInput,
        SliderIntLabel,
        SliderIntInput,
        SliderIntTrack,
        SliderIntDragger,
        SliderIntDraggerBorder,
        SliderIntFill,
        SliderIntTextInput,
        MinMaxSliderLabel,
        MinMaxSliderInput,
        MinMaxSliderTrack,
        MinMaxSliderMinimumThumb,
        MinMaxSliderMaximumThumb,
        MinMaxSliderRangeDragger,
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

    /// <summary>An inclusive lower limit or Unity's native unbounded minimum.</summary>
    public abstract record LowerLimit
    {
        private LowerLimit() { }

        public sealed record Unbounded : LowerLimit;

        public sealed record Inclusive(float Value) : LowerLimit;
    }

    /// <summary>An inclusive upper limit or Unity's native unbounded maximum.</summary>
    public abstract record UpperLimit
    {
        private UpperLimit() { }

        public sealed record Unbounded : UpperLimit;

        public sealed record Inclusive(float Value) : UpperLimit;
    }
}

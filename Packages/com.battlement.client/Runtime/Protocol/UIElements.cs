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
    public abstract partial record UiElement
    {
        /// <summary>The name operation; reset restores the constructor's empty name.</summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<string> Name { get; init; }

        /// <summary>Whether this visual element is enabled locally.</summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<bool> Enabled { get; init; }

        /// <summary>
        /// Pointer picking; reset restores <see cref="UiPickingMode.Position" />.
        /// </summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<UiPickingMode> PickingMode { get; init; }

        /// <summary>
        /// Text direction; reset restores <see cref="UiLanguageDirection.Inherit" />.
        /// </summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<UiLanguageDirection> LanguageDirection { get; init; }

        /// <summary>
        /// Focusability; reset restores the concrete native constructor value.
        /// </summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<bool> Focusable { get; init; }

        /// <summary>
        /// Focus-ring order; reset restores the concrete native constructor value.
        /// </summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<int> TabIndex { get; init; }

        /// <summary>
        /// Focus delegation; reset restores the concrete native constructor value.
        /// </summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<bool> DelegatesFocus { get; init; }

        /// <summary>Whether this host requests focus once when it is mounted.</summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<bool> AutoFocus { get; init; }

        /// <summary>Whether this logical subtree is excluded from user interaction.</summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<bool> Inert { get; init; }

        /// <summary>Authored USS classes; reset removes them but retains native classes.</summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<IReadOnlyList<string>> Classes { get; init; }

        /// <summary>Create-time rendering optimization hints for this element.</summary>
        public IReadOnlyList<UiUsageHint>? UsageHints { get; init; }

        /// <summary>The style values on this visual element.</summary>
        public UiStyle? Style { get; init; }

        /// <summary>Static decorative paint rendered in this element's border box.</summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<PaintStyle> Paint { get; init; }

        /// <summary>
        /// UI events forwarded to Rust; reset removes every shorthand subscription.
        /// </summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<IReadOnlyList<UiEventKind>> Events { get; init; }

        /// <summary>Routed subscriptions; reset removes every explicit subscription.</summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<IReadOnlyList<UiEventSubscription>> EventSubscriptions { get; init; }

        /// <summary>Validated animation state installed beside this native host.</summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<MotionDescriptor> Motion { get; init; }

        /// <summary>Grid placement metadata for a placement child.</summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<GridItem> GridItem { get; init; }

        /// <summary>Stack placement metadata for a placement child.</summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<StackItem> StackItem { get; init; }

        /// <summary>Sticky positioning metadata resolved by a physical scroll view.</summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<Sticky> Sticky { get; init; }

        /// <summary>Overlay placement metadata for a top-level portal attachment.</summary>
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public Prop<OverlayPlacement> OverlayPlacement { get; init; }

        /// <summary>The base class for objects in the UI Toolkit visual tree.</summary>
        public sealed record VisualElement : UiElement;

        /// <summary>A Unity UI Toolkit Box.</summary>
        public sealed record Box : UiElement;

        /// <summary>A text element that displays text.</summary>
        public sealed record Label : UiElement
        {
            /// <summary>The text to display; reset restores the constructor's empty text.</summary>
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<string> Text { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> EnableRichText { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> EmojiFallbackSupport { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> ParseEscapeSequences { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> DisplayTooltipWhenElided { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> Selectable { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> DoubleClickSelectsWord { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> TripleClickSelectsLine { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> SelectAllOnFocus { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> SelectAllOnMouseUp { get; init; }
        }

        /// <summary>A leaf base text element with rich-text and selection preferences.</summary>
        public sealed record TextElement : UiElement
        {
            /// <summary>The text to display; reset restores the constructor's empty text.</summary>
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<string> Text { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> EnableRichText { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> EmojiFallbackSupport { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> ParseEscapeSequences { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> DisplayTooltipWhenElided { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> Selectable { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> DoubleClickSelectsWord { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> TripleClickSelectsLine { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> SelectAllOnFocus { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> SelectAllOnMouseUp { get; init; }
        }

        /// <summary>A controlled text editor with a native local draft.</summary>
        public sealed record TextField : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public new Prop<string> Label { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<string> Value { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> Multiline { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UiScrollerVisibility> VerticalScrollerVisibility { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> Password { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> ReadOnly { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<string> Placeholder { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> HidePlaceholderOnFocus { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<uint> CursorIndex { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<uint> SelectIndex { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> SelectAllOnFocus { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> SelectAllOnMouseUp { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled Boolean switch.</summary>
        public sealed record Toggle : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public new Prop<string> Label { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<string> Text { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> Value { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled standalone Boolean radio option.</summary>
        public sealed record RadioButton : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public new Prop<string> Label { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<string> Text { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> Value { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled exclusive choice rendered as native radio options.</summary>
        public sealed record RadioButtonGroup : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public new Prop<string> Label { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<IReadOnlyList<string>> Choices { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<uint> SelectedIndex { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled selection group containing ordinary Button children.</summary>
        public sealed record ToggleButtonGroup : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public new Prop<string> Label { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> MultipleSelection { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> AllowEmptySelection { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<IReadOnlyList<uint>> SelectedIndices { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled single-choice popup selector.</summary>
        public sealed record DropdownField : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public new Prop<string> Label { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> ShowMixedValue { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<IReadOnlyList<string>> Choices { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<DropdownChoice> Selection { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A clickable button with a text label element.</summary>
        public sealed record Button : UiElement
        {
            /// <summary>The text to display; reset restores the constructor's empty text.</summary>
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<string> Text { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> EnableRichText { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> EmojiFallbackSupport { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> ParseEscapeSequences { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> DisplayTooltipWhenElided { get; init; }

            /// <summary>The prepared asset displayed by the native icon slot.</summary>
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<IconSource> Icon { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A button that repeatedly activates while held.</summary>
        public sealed record RepeatButton : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<string> Text { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<uint> DelayMs { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<uint> IntervalMs { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> EnableRichText { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> EmojiFallbackSupport { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> ParseEscapeSequences { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> DisplayTooltipWhenElided { get; init; }
        }

        /// <summary>A container that groups related controls under an optional title.</summary>
        public sealed record GroupBox : UiElement
        {
            /// <summary>The text displayed by the conditional native title label.</summary>
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<string> Text { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A popup-styled text container with a dedicated content container.</summary>
        public sealed record PopupWindow : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<string> Text { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> EnableRichText { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> EmojiFallbackSupport { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> ParseEscapeSequences { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> DisplayTooltipWhenElided { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> Selectable { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> DoubleClickSelectsWord { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> TripleClickSelectsLine { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> SelectAllOnFocus { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> SelectAllOnMouseUp { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A viewport that scrolls arbitrary child content on one or both axes.</summary>
        public sealed record ScrollView : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UiScrollViewMode> Mode { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UiNestedInteraction> NestedInteraction { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UiScrollerVisibility> HorizontalScrollerVisibility { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UiScrollerVisibility> VerticalScrollerVisibility { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<Vector> ScrollOffset { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> HorizontalPageSize { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> VerticalPageSize { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> MouseWheelScrollSize { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UiTouchScrollBehavior> TouchScrollBehavior { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> ScrollDecelerationRate { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> Elasticity { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<uint> ElasticAnimationInterval { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled scrollbar that proposes values within an authored range.</summary>
        public sealed record Scroller : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> LowValue { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> HighValue { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UiSliderDirection> Direction { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> Value { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled floating-point range slider.</summary>
        public sealed record Slider : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public new Prop<string> Label { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> LowValue { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> HighValue { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> Value { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> Fill { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> PageSize { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> ShowInputField { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UiSliderDirection> Direction { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> Inverted { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled integer range slider.</summary>
        public sealed record SliderInt : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public new Prop<string> Label { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<int> LowValue { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<int> HighValue { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<int> Value { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> Fill { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> PageSize { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> ShowInputField { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UiSliderDirection> Direction { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> Inverted { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled dual-thumb floating-point range selector.</summary>
        public sealed record MinMaxSlider : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public new Prop<string> Label { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> MinValue { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> MaxValue { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<LowerLimit> LowLimit { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UpperLimit> HighLimit { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>An output-only progress indicator.</summary>
        public sealed record ProgressBar : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> LowValue { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> HighValue { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> Value { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<string> Title { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>One labeled page placed directly beneath a TabView.</summary>
        public sealed record Tab : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<string> Text { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<IconSource> Icon { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> Closeable { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A controlled selection and reorder container for Tab children.</summary>
        public sealed record TabView : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<uint> SelectedTabIndex { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<bool> Reorderable { get; init; }
            public IReadOnlyList<UiPartStyle>? Parts { get; init; }
        }

        /// <summary>A leaf UI Toolkit image with one exclusive prepared source.</summary>
        public sealed record Image : UiElement
        {
            /// <summary>The prepared raster, sprite, vector, or render-texture source.</summary>
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<ImageSource> Source { get; init; }

            /// <summary>
            /// Upper-left-origin pixel rectangle sampled from a non-sprite source.
            /// </summary>
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<Rect> SourceRect { get; init; }

            /// <summary>Linear color multiplied with the sampled source pixels.</summary>
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<Color> TintColor { get; init; }

            /// <summary>How the source fits and crops inside the content rectangle.</summary>
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<ImageScaleMode> ScaleMode { get; init; }

            /// <summary>Lower-left-origin normalized base texture coordinates.</summary>
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<Rect> Uv { get; init; }
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

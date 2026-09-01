#nullable enable

using System.Collections.Generic;
using Newtonsoft.Json;

namespace Battlement
{
    /// <summary>One explicit or implicit Grid track size.</summary>
    public abstract record GridTrack
    {
        private GridTrack() { }

        /// <summary>A fixed nonnegative pixel track.</summary>
        public sealed record Px(float Value) : GridTrack;

        /// <summary>A positive share of remaining space.</summary>
        public sealed record Fraction(float Value) : GridTrack;

        /// <summary>A track sized from preferred outer item sizes.</summary>
        public sealed record Auto : GridTrack;
    }

    /// <summary>Major-axis scan direction used by Grid auto-placement.</summary>
    public enum GridAutoFlow
    {
        Row,
        Column,
    }

    /// <summary>Placement and alignment of one Grid child.</summary>
    public sealed record GridItem(
        uint? Row,
        uint? Column,
        uint RowSpan,
        uint ColumnSpan,
        UiAlign AlignSelf,
        UiAlign JustifySelf
    );

    /// <summary>Placement and presentation order of one Stack child.</summary>
    public sealed record StackItem(
        int Order,
        UiAlign AlignSelf,
        UiAlign JustifySelf,
        float? Top,
        float? Right,
        float? Bottom,
        float? Left,
        bool ContributesToSize
    );

    /// <summary>Sticky positioning metadata for one normal-flow child.</summary>
    public sealed record Sticky(float? Top, float? Right, float? Bottom, float? Left, int Order);

    /// <summary>Overlay presentation tier.</summary>
    public enum OverlayLayer
    {
        Popover,
        Modal,
    }

    /// <summary>Physical side of an anchor used for popover placement.</summary>
    public enum PlacementSide
    {
        Top,
        Right,
        Bottom,
        Left,
    }

    /// <summary>Cross-axis alignment used for popover placement.</summary>
    public enum PlacementAlign
    {
        Start,
        Center,
        End,
    }

    /// <summary>Complete anchored-popover placement policy.</summary>
    public sealed record PopoverPlacement(
        PlacementSide Side,
        PlacementAlign Align,
        float MainOffset,
        float CrossOffset,
        float CollisionPadding,
        bool Flip,
        bool Shift
    );

    /// <summary>Placement metadata for one top-level overlay portal attachment.</summary>
    public abstract record OverlayPlacement
    {
        private OverlayPlacement() { }

        /// <summary>An unanchored host-filling layer.</summary>
        public sealed record Layer(OverlayLayer Value) : OverlayPlacement;

        /// <summary>A popover anchored to one public host identity.</summary>
        public sealed record Popover(ObjectId Anchor, PopoverPlacement Placement)
            : OverlayPlacement;

        /// <summary>A modal layer with optional focus targets.</summary>
        public sealed record Modal(ObjectId? InitialFocus, ObjectId? RestoreFocus)
            : OverlayPlacement;
    }

    public abstract partial record UiElement
    {
        /// <summary>A flex container with independent gaps.</summary>
        public sealed record Flex : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UiFlexDirection> Direction { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UiFlexWrap> Wrap { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UiAlign> AlignItems { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UiJustify> JustifyContent { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> RowGap { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> ColumnGap { get; init; }
        }

        /// <summary>A deterministic track-based Grid container.</summary>
        public sealed record Grid : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<IReadOnlyList<GridTrack>> Columns { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<IReadOnlyList<GridTrack>> Rows { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<GridTrack> AutoColumns { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<GridTrack> AutoRows { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<GridAutoFlow> AutoFlow { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> RowGap { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<float> ColumnGap { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UiAlign> AlignItems { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UiAlign> JustifyItems { get; init; }
        }

        /// <summary>An isolated overlapping Stack container.</summary>
        public sealed record Stack : UiElement
        {
            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UiAlign> AlignItems { get; init; }

            [JsonProperty(NullValueHandling = NullValueHandling.Include)]
            public Prop<UiAlign> JustifyItems { get; init; }
        }
    }
}

#nullable enable

using System.Collections.Generic;

namespace Battlement
{
    /// <summary>A panel-space position measured in pixels from the upper-left.</summary>
    public sealed record PanelPoint(double X, double Y);

    /// <summary>A two-dimensional displacement in upper-left-origin panel pixels.</summary>
    public sealed record Vector(float X, float Y);

    /// <summary>A value proposed or committed by a controlled UI component.</summary>
    public abstract record UiValue
    {
        private UiValue() { }

        public sealed record F32(float Value) : UiValue;
    }

    /// <summary>Physical modifier keys held for a UI event.</summary>
    public enum KeyModifier
    {
        Alt,
        Control,
        Command,
        Shift,
    }

    /// <summary>UI event kinds that Rust-authored elements can request.</summary>
    public enum UiEventKind
    {
        Click,
        NavigationSubmit,
        TransitionStart,
        TransitionEnd,
        TransitionCancel,
        ValueChanging,
        ValueCommitted,
        ScrollSettled,
        ScrollChanged,
        TabSelectionRequested,
        TabCloseRequested,
        TabReorderRequested,
    }

    /// <summary>One subscribed UI event emitted by a logical target.</summary>
    public sealed record UiEvent(ObjectId TargetId, UiEventBody Body);

    /// <summary>Exact union of supported UI event payloads.</summary>
    public abstract record UiEventBody
    {
        private UiEventBody() { }

        public sealed record Click(ClickEvent Value) : UiEventBody;

        public sealed record NavigationSubmit : UiEventBody;

        public sealed record TransitionStart(TransitionEvent Value) : UiEventBody;

        public sealed record TransitionEnd(TransitionEvent Value) : UiEventBody;

        public sealed record TransitionCancel(TransitionEvent Value) : UiEventBody;

        public sealed record ValueChanging(ValueChangingEvent Value) : UiEventBody;

        public sealed record ValueCommitted(ValueCommitEvent Value) : UiEventBody;

        public sealed record ScrollSettled(ScrollEvent Value) : UiEventBody;

        public sealed record ScrollChanged(ScrollEvent Value) : UiEventBody;

        public sealed record TabSelectionRequested(TabSelectionEvent Value) : UiEventBody;

        public sealed record TabCloseRequested(TabCloseEvent Value) : UiEventBody;

        public sealed record TabReorderRequested(TabReorderEvent Value) : UiEventBody;
    }

    public sealed record ValueChangingEvent(UiValue Proposed);

    public sealed record ValueCommitEvent(UiValue Previous, UiValue Proposed);

    public sealed record ScrollEvent(Vector Offset);

    public sealed record TabSelectionEvent(
        uint PreviousIndex,
        uint ProposedIndex,
        ObjectId ProposedTabId
    );

    public sealed record TabCloseEvent(ObjectId TabId, uint Index);

    public sealed record TabReorderEvent(ObjectId TabId, uint PreviousIndex, uint ProposedIndex);

    /// <summary>
    /// Supported properties and elapsed interpolation time from a transition event.
    /// </summary>
    public sealed record TransitionEvent(
        IReadOnlyList<UiTransitionProperty> Properties,
        float ElapsedMs
    );

    /// <summary>How a clickable element was activated.</summary>
    public abstract record ClickEvent
    {
        private ClickEvent() { }

        public sealed record Pointer(
            PanelPoint Position,
            uint ClickCount,
            int PointerId = 0,
            PointerButton Button = PointerButton.Left,
            IReadOnlyList<KeyModifier>? Modifiers = null
        ) : ClickEvent;

        public sealed record NavigationSubmit : ClickEvent;

        public sealed record Repeat : ClickEvent;
    }
}

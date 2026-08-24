#nullable enable

using System.Collections.Generic;

namespace Battlement
{
    /// <summary>A panel-space position measured in pixels from the upper-left.</summary>
    public sealed record PanelPoint(double X, double Y);

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
    }

    /// <summary>One subscribed UI event emitted by a logical target.</summary>
    public sealed record UiEvent(ObjectId TargetId, UiEventBody Body);

    /// <summary>Exact union of supported UI event payloads.</summary>
    public abstract record UiEventBody
    {
        private UiEventBody() { }

        public sealed record Click(ClickEvent Value) : UiEventBody;
    }

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

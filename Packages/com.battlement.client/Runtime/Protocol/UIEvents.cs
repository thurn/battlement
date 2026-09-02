#nullable enable

using System.Collections.Generic;
using System.ComponentModel;
using Newtonsoft.Json;

namespace Battlement
{
    /// <summary>A panel-space position measured in pixels from the upper-left.</summary>
    public sealed record PanelPoint(double X, double Y);

    /// <summary>A two-dimensional displacement in upper-left-origin panel pixels.</summary>
    public sealed record Vector(float X, float Y);

    /// <summary>A coherent optional dropdown index and display value.</summary>
    public sealed record DropdownChoice(uint? Index = null, string? Value = null)
    {
        public static DropdownChoice Selected(uint index, string value) => new(index, value);

        public static DropdownChoice None() => new();
    }

    /// <summary>An ordered finite floating-point range.</summary>
    public sealed record FloatRange(float Min, float Max);

    /// <summary>A value proposed or committed by a controlled UI component.</summary>
    public abstract record UiValue
    {
        private UiValue() { }

        public sealed record Bool(bool Value) : UiValue;

        public sealed record Index(uint? Value) : UiValue;

        public sealed record Indices(IReadOnlyList<uint> Value) : UiValue;

        public sealed record Choice(DropdownChoice Value) : UiValue;

        public sealed record F32(float Value) : UiValue;

        public sealed record I32(int Value) : UiValue;

        public sealed record F32Range(FloatRange Value) : UiValue;

        public sealed record String(string Value) : UiValue;
    }

    /// <summary>Physical modifier keys held for a UI event.</summary>
    public enum KeyModifier
    {
        Alt,
        Control,
        Command,
        Shift,
        CapsLock,
        Numeric,
        FunctionKey,
    }

    /// <summary>UI event kinds that Rust-authored elements can request.</summary>
    public enum UiEventKind
    {
        PointerDown,
        PointerMove,
        PointerUp,
        PointerCancel,

        /// <summary>
        /// Logical activation. On a Button this covers pointer click and keyboard or gamepad
        /// submit, allowing one handler for every activation method.
        /// </summary>
        Click,
        PointerEnter,
        PointerLeave,
        PointerOver,
        PointerOut,
        Wheel,
        PointerCapture,
        PointerCaptureOut,
        KeyDown,
        KeyUp,
        NavigationMove,
        NavigationCancel,
        FocusIn,
        Focus,
        FocusOut,
        Blur,
        GeometryChanged,
        AttachToPanel,
        DetachFromPanel,
        TransitionStart,
        TransitionEnd,
        TransitionCancel,
        ValueChanging,
        ValueCommitted,
        Input,
        SelectionChanged,
        LinkEnter,
        LinkLeave,
        LinkDown,
        LinkUp,
        ScrollSettled,
        ScrollChanged,
        TabSelectionRequested,
        TabCloseRequested,
        TabReorderRequested,
    }

    /// <summary>Logical event-routing phase.</summary>
    public enum UiEventPhase
    {
        Trickle,
        Target,
        Bubble,
    }

    /// <summary>One native event kind and logical route phase.</summary>
    public sealed record UiEventSubscription(
        UiEventKind Kind,
        UiEventPhase Phase = UiEventPhase.Target
    );

    /// <summary>Immediate decision returned before the native callback resumes.</summary>
    public enum UiEventDisposition : uint
    {
        Continue = 0,
        PreventDefault = 1,
    }

    /// <summary>One synchronous UI event submission.</summary>
    public sealed record UiEventAction(
        [property: JsonProperty("action_id")] ActionId Id,
        SessionId SessionId,
        UiEvent Event
    );

    /// <summary>One subscribed UI event emitted by a logical target.</summary>
    public sealed record UiEvent(
        ObjectId TargetId,
        bool Cancelable,
        bool DefaultPrevented,
        UiEventBody Body
    )
    {
        public UiEvent(ObjectId targetId, UiEventBody body)
            : this(targetId, false, false, body) { }
    }

    /// <summary>Exact union of supported UI event payloads.</summary>
    public abstract record UiEventBody
    {
        private UiEventBody() { }

        public sealed record PointerDown(UiPointerButtonEvent Value) : UiEventBody;

        public sealed record PointerMove(UiPointerMoveEvent Value) : UiEventBody;

        public sealed record PointerUp(UiPointerButtonEvent Value) : UiEventBody;

        public sealed record PointerCancel(UiPointerCancelEvent Value) : UiEventBody;

        public sealed record Click(ClickEvent Value) : UiEventBody;

        public sealed record PointerEnter(UiPointerBoundaryEvent Value) : UiEventBody;

        public sealed record PointerLeave(UiPointerBoundaryEvent Value) : UiEventBody;

        public sealed record PointerOver(UiPointerCrossingEvent Value) : UiEventBody;

        public sealed record PointerOut(UiPointerCrossingEvent Value) : UiEventBody;

        public sealed record Wheel(UiWheelEvent Value) : UiEventBody;

        public sealed record PointerCapture(UiPointerCaptureEvent Value) : UiEventBody;

        public sealed record PointerCaptureOut(UiPointerCaptureEvent Value) : UiEventBody;

        public sealed record KeyDown(UiKeyEvent Value) : UiEventBody;

        public sealed record KeyUp(UiKeyEvent Value) : UiEventBody;

        public sealed record NavigationMove(UiNavigationMoveEvent Value) : UiEventBody;

        public sealed record NavigationCancel(UiNavigationEvent Value) : UiEventBody;

        public sealed record FocusIn(UiFocusEvent Value) : UiEventBody;

        public sealed record Focus(UiFocusEvent Value) : UiEventBody;

        public sealed record FocusOut(UiFocusEvent Value) : UiEventBody;

        public sealed record Blur(UiFocusEvent Value) : UiEventBody;

        public sealed record GeometryChanged(GeometryEvent Value) : UiEventBody;

        public sealed record AttachToPanel(LifecycleEvent Value) : UiEventBody;

        public sealed record DetachFromPanel(LifecycleEvent Value) : UiEventBody;

        public sealed record TransitionStart(TransitionEvent Value) : UiEventBody;

        public sealed record TransitionEnd(TransitionEvent Value) : UiEventBody;

        public sealed record TransitionCancel(TransitionEvent Value) : UiEventBody;

        public sealed record ValueChanging(ValueChangingEvent Value) : UiEventBody;

        public sealed record ValueCommitted(ValueCommitEvent Value) : UiEventBody;

        public sealed record Input(TextInputEvent Value) : UiEventBody;

        public sealed record SelectionChanged(SelectionEvent Value) : UiEventBody;

        public sealed record LinkEnter(LinkEvent Value) : UiEventBody;

        public sealed record LinkLeave(LinkEvent Value) : UiEventBody;

        public sealed record LinkDown(LinkEvent Value) : UiEventBody;

        public sealed record LinkUp(LinkEvent Value) : UiEventBody;

        public sealed record ScrollSettled(ScrollEvent Value) : UiEventBody;

        public sealed record ScrollChanged(ScrollEvent Value) : UiEventBody;

        public sealed record TabSelectionRequested(TabSelectionEvent Value) : UiEventBody;

        public sealed record TabCloseRequested(TabCloseEvent Value) : UiEventBody;

        public sealed record TabReorderRequested(TabReorderEvent Value) : UiEventBody;
    }

    /// <summary>Native pointer-device category.</summary>
    public enum UiPointerType
    {
        Mouse,
        Touch,
        Pen,
        Unknown,
    }

    /// <summary>UI pointer button preserving nonstandard native indices.</summary>
    public abstract record UiPointerButton
    {
        private UiPointerButton() { }

        public sealed record Left : UiPointerButton;

        public sealed record Middle : UiPointerButton;

        public sealed record Right : UiPointerButton;

        public sealed record Other(int Value) : UiPointerButton;
    }

    public sealed record UiPointerButtonEvent(
        PanelPoint Position,
        Vector Delta,
        int PointerId = 0,
        UiPointerButton? Button = null,
        uint Buttons = 0,
        float Pressure = 0,
        [property: DefaultValue(1u)] uint ClickCount = 1,
        IReadOnlyList<KeyModifier>? Modifiers = null,
        UiPointerType PointerType = UiPointerType.Mouse
    );

    public sealed record UiPointerMoveEvent(
        PanelPoint Position,
        Vector Delta,
        int PointerId = 0,
        UiPointerButton? ChangedButton = null,
        uint Buttons = 0,
        float Pressure = 0,
        uint ClickCount = 0,
        IReadOnlyList<KeyModifier>? Modifiers = null,
        UiPointerType PointerType = UiPointerType.Mouse
    );

    public sealed record UiPointerCancelEvent(
        PanelPoint Position,
        Vector Delta,
        int PointerId = 0,
        uint Buttons = 0,
        float Pressure = 0,
        IReadOnlyList<KeyModifier>? Modifiers = null,
        UiPointerType PointerType = UiPointerType.Mouse
    );

    public sealed record UiPointerBoundaryEvent(
        PanelPoint Position,
        int PointerId = 0,
        UiPointerType PointerType = UiPointerType.Mouse
    );

    /// <summary>Pointer crossing data with the opposite Rust-owned picked target.</summary>
    public sealed record UiPointerCrossingEvent(
        PanelPoint Position,
        int PointerId = 0,
        UiPointerType PointerType = UiPointerType.Mouse,
        ObjectId? RelatedTargetId = null
    );

    public sealed record UiVector3(float X, float Y, float Z);

    public sealed record UiWheelEvent(
        PanelPoint Position,
        UiVector3 Delta,
        IReadOnlyList<KeyModifier>? Modifiers = null
    );

    public sealed record UiPointerCaptureEvent(int PointerId = 0);

    /// <summary>Focus relation mapped to a Rust-owned logical element.</summary>
    public sealed record UiFocusEvent(
        ObjectId? RelatedTargetId = null,
        UiFocusDirection? Direction = null
    );

    /// <summary>Physical key metadata from focused UI.</summary>
    public sealed record UiKeyEvent(
        PhysicalKey? PhysicalKey,
        string Text,
        IReadOnlyList<KeyModifier>? Modifiers = null
    );

    /// <summary>Public UI navigation directions.</summary>
    public enum UiNavigationDirection
    {
        None,
        Left,
        Up,
        Right,
        Down,
        Next,
        Previous,
    }

    /// <summary>Directional UI navigation metadata.</summary>
    public sealed record UiNavigationMoveEvent(UiNavigationDirection Direction, Vector Move);

    /// <summary>Empty navigation-cancel payload.</summary>
    public sealed record UiNavigationEvent;

    /// <summary>Public focus-change direction.</summary>
    public abstract record UiFocusDirection
    {
        private UiFocusDirection() { }

        public sealed record None : UiFocusDirection;

        public sealed record Unspecified : UiFocusDirection;

        public sealed record Left : UiFocusDirection;

        public sealed record Right : UiFocusDirection;

        public sealed record Other(int Value) : UiFocusDirection;
    }

    public sealed record ValueChangingEvent(UiValue Proposed);

    public sealed record ValueCommitEvent(UiValue Previous, UiValue Proposed);

    public sealed record TextInputEvent(string Value);

    public sealed record SelectionEvent(uint CursorIndex, uint SelectionIndex);

    public sealed record GeometryEvent(Rect Previous, Rect Current);

    public sealed record LifecycleEvent;

    public sealed record LinkEvent(
        string LinkId,
        string LinkText,
        PanelPoint Position,
        int PointerId = 0,
        UiPointerButton? Button = null
    );

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
            [property: DefaultValue(1u)] uint ClickCount,
            int PointerId = 0,
            UiPointerButton? Button = null,
            IReadOnlyList<KeyModifier>? Modifiers = null
        ) : ClickEvent;

        /// <summary>Keyboard or gamepad submit converted into a Button activation.</summary>
        public sealed record NavigationSubmit : ClickEvent;

        public sealed record Repeat : ClickEvent;
    }
}

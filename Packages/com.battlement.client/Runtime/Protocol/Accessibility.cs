#nullable enable

using System.Collections.Generic;
using Newtonsoft.Json;

namespace Battlement
{
    /// <summary>Roles retained by Reactant accessibility.</summary>
    public enum SemanticRole
    {
        Button,
        Checkbox,
        Switch,
        Radio,
        RadioGroup,
        Slider,
        Progress,
        Disclosure,
        ScrollArea,
        Tab,
        TabList,
        TabPanel,
        Dialog,
        Heading,
        Image,
        StaticText,
        Group,
        ListBox,
        Option,
        Table,
        Row,
        ColumnHeader,
        RowHeader,
        Cell,
        Link,
        Navigation,
        Region,
    }

    /// <summary>The current location represented by a button or link.</summary>
    public enum CurrentPage
    {
        Page,
    }

    /// <summary>Kind of popup controlled by a semantic button.</summary>
    public enum PopupKind
    {
        ListBox,
    }

    /// <summary>Canonical checked state.</summary>
    public enum CheckedState
    {
        False,
        True,
        Mixed,
    }

    /// <summary>Logical accessibility scrolling direction.</summary>
    public enum AccessibilityScrollDirection
    {
        Forward,
        Backward,
    }

    /// <summary>Axis owned by an accessible scroll region.</summary>
    public enum AccessibilityScrollAxis
    {
        Horizontal,
        Vertical,
    }

    /// <summary>Canonical semantic state.</summary>
    public sealed record SemanticState(
        bool Disabled = false,
        CheckedState? Checked = null,
        bool? Selected = null,
        bool? Expanded = null,
        bool Busy = false,
        CurrentPage? Current = null,
        PopupKind? Popup = null
    );

    /// <summary>Resolved finite range value.</summary>
    public sealed record AccessibilityRangeValue(
        double Current,
        double Minimum,
        double Maximum,
        string? Text = null
    );

    /// <summary>Direct actions declared on one semantic node.</summary>
    public sealed record AccessibilityActionSet(
        bool Activate = false,
        bool Increment = false,
        bool Decrement = false,
        bool Dismiss = false,
        IReadOnlyList<AccessibilityScrollDirection>? Scroll = null
    );

    /// <summary>One resolved host-backed semantic node.</summary>
    public sealed record AccessibilityNodeSnapshot(
        ObjectId ObjectId,
        ObjectId? ParentId,
        IReadOnlyList<ObjectId> Children,
        SemanticRole Role,
        [property: JsonProperty(Required = Required.AllowNull)] string? Label,
        [property: JsonProperty(Required = Required.AllowNull)] string? Hint,
        SemanticState State,
        [property: JsonProperty(Required = Required.AllowNull)] AccessibilityRangeValue? Value,
        AccessibilityActionSet Actions,
        byte? HeadingLevel = null,
        AccessibilityScrollAxis? ScrollAxis = null
    );

    /// <summary>One complete canonical semantic tree.</summary>
    public sealed record AccessibilitySnapshot(
        ulong CommitSequence,
        IReadOnlyList<ObjectId> Roots,
        IReadOnlyList<AccessibilityNodeSnapshot> Nodes
    );

    /// <summary>Atomic semantic replacement and one-shot announcement queue.</summary>
    public sealed record AccessibilityUpdatePayload(
        [property: JsonProperty(Required = Required.AllowNull)] AccessibilitySnapshot? Snapshot,
        IReadOnlyList<string> Announcements
    );

    /// <summary>Normalized direct accessibility callback.</summary>
    public abstract record AccessibilityAction
    {
        private AccessibilityAction() { }

        public sealed record Activate : AccessibilityAction;

        public sealed record Increment : AccessibilityAction;

        public sealed record Decrement : AccessibilityAction;

        public sealed record Dismiss : AccessibilityAction;

        public sealed record Scroll(AccessibilityScrollDirection Value) : AccessibilityAction;
    }

    /// <summary>One callback emitted by a live accessibility backend.</summary>
    public sealed record AccessibilityEvent(
        ulong BackendGeneration,
        ObjectId Target,
        AccessibilityAction Action
    );

    public abstract partial record CommandBody
    {
        /// <summary>Atomically replaces semantics and submits announcements.</summary>
        public sealed record AccessibilityUpdate(AccessibilityUpdatePayload Value) : CommandBody;
    }
}

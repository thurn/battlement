#nullable enable

using System.Collections.Generic;
using Newtonsoft.Json;

namespace Battlement
{
    /// <summary>Property and typed keyframe sequence for one track.</summary>
    public sealed record MotionPropertyTrack(
        MotionProperty Property,
        IReadOnlyList<MotionValue> Values,
        TransitionDefinition Transition,
        IReadOnlyList<double>? Times = null
    );

    /// <summary>One property assignment outside a sampled timeline.</summary>
    public sealed record MotionPropertyValue(MotionProperty Property, MotionValue Value);

    /// <summary>One flattened target layer.</summary>
    public sealed record MotionTargetDescriptor(
        IReadOnlyList<MotionPropertyTrack> Tracks,
        IReadOnlyList<MotionPropertyValue> TransitionEnd
    );

    /// <summary>Descriptor layer priority after variant resolution.</summary>
    public enum MotionLayer
    {
        Animate,
        InView,
        Focus,
        FocusVisible,
        Hover,
        Tap,
        Drag,
        Exit,
    }

    /// <summary>Axis ownership for pan and drag recognition.</summary>
    public enum MotionGestureAxis
    {
        X,
        Y,
        Both,
    }

    /// <summary>Pointer or navigation device which owns a gesture.</summary>
    public enum MotionPointerDevice
    {
        Mouse,
        Pen,
        Touch,
        Keyboard,
        Gamepad,
    }

    /// <summary>Panel-space point or vector carried by a gesture event.</summary>
    public sealed record MotionGestureVector(float X, float Y);

    /// <summary>Fixed panel-space drag bounds.</summary>
    public sealed record MotionDragBounds(float MinX, float MaxX, float MinY, float MaxY);

    /// <summary>Source used to resolve drag bounds locally.</summary>
    public abstract record MotionDragConstraint
    {
        public sealed record Bounds(MotionDragBounds Value) : MotionDragConstraint;

        public sealed record Element(ObjectId Value) : MotionDragConstraint;
    }

    /// <summary>Per-edge elasticity applied beyond drag bounds.</summary>
    public sealed record MotionDragElastic(float Left, float Right, float Top, float Bottom);

    /// <summary>Native release-inertia and boundary-spring settings.</summary>
    public sealed record MotionDragTransition(
        float VelocityRetention,
        float RestSpeed,
        float BounceStiffness,
        float BounceDamping
    );

    /// <summary>Native drag behavior attached to one host.</summary>
    public sealed record MotionDragDescriptor(
        MotionGestureAxis Axis,
        [property: JsonProperty(Required = Required.AllowNull)] MotionDragConstraint? Constraints,
        MotionDragElastic Elastic,
        bool Momentum,
        bool DirectionLock,
        bool Listener,
        MotionGestureAxis? SnapToOrigin,
        ObjectId? ControlId,
        bool Propagation,
        MotionDragTransition Transition,
        ObjectId? XValue,
        ObjectId? YValue
    );

    /// <summary>Explicit callback subscriptions for gesture boundaries and samples.</summary>
    public sealed record MotionGestureSubscriptions(
        bool Hover,
        bool Tap,
        bool Focus,
        bool Pan,
        bool PanUpdate,
        bool Drag,
        bool DragUpdate,
        bool MomentumComplete,
        bool ConstraintsMeasured,
        bool Scroll,
        bool InView,
        bool FocusVisible = false
    );

    /// <summary>Unity-local gesture recognition and presentation configuration.</summary>
    public sealed record MotionGestureDescriptor(
        float PanThreshold,
        float DirectionLockThreshold,
        float PointerTapSlop,
        float TouchTapSlop,
        bool Pan,
        [property: Newtonsoft.Json.JsonProperty(Required = Newtonsoft.Json.Required.AllowNull)]
            MotionDragDescriptor? Drag,
        bool InView,
        bool Scroll,
        ObjectId? ScrollXValue,
        ObjectId? ScrollYValue,
        ObjectId? InViewValue,
        MotionGestureSubscriptions Subscriptions
    );

    /// <summary>Explicit lifecycle subscription set for one slot.</summary>
    public sealed record MotionCallbackSubscriptions(
        bool Start,
        bool Update,
        bool Repeat,
        bool Complete,
        bool Stop,
        bool Cancel
    );

    /// <summary>One independently identified layer slot.</summary>
    public sealed record MotionSlotDescriptor(
        ulong Slot,
        uint Generation,
        MotionLayer Layer,
        MotionTargetDescriptor Target,
        MotionCallbackSubscriptions Callbacks
    );

    /// <summary>Clock selected for one descriptor.</summary>
    public abstract record MotionClockSource
    {
        public sealed record Unscaled : MotionClockSource;

        public sealed record Scaled : MotionClockSource;

        public sealed record Controlled(ObjectId Value) : MotionClockSource;

        public sealed record Audio(ObjectId Value) : MotionClockSource;
    }

    /// <summary>Resolved reduced-motion policy.</summary>
    public enum ReducedMotionPolicy
    {
        User,
        Always,
        Never,
    }

    /// <summary>Projection axes applied after one native layout pass.</summary>
    public enum MotionLayoutMode
    {
        Position,
        Size,
        Both,
    }

    /// <summary>Stable typed identity for a layout group or shared handoff.</summary>
    public sealed record MotionLayoutIdentity(string ValueType, ulong ValueHash);

    /// <summary>Native layout-projection configuration for one host.</summary>
    public sealed record MotionLayoutDescriptor(
        MotionLayoutMode Mode,
        MotionLayoutIdentity Group,
        [property: Newtonsoft.Json.JsonProperty(Required = Newtonsoft.Json.Required.AllowNull)]
            MotionLayoutIdentity? LayoutId,
        bool Scroll,
        bool Root,
        bool PopLayout,
        TransitionDefinition Transition
    );

    /// <summary>Complete validated animation state installed beside one host.</summary>
    public sealed record MotionDescriptor(
        ObjectId DescriptorId,
        ObjectId HostId,
        uint Generation,
        bool InitialDisabled,
        IReadOnlyList<MotionSlotDescriptor> Slots,
        MotionClockSource Clock,
        ReducedMotionPolicy ReducedMotion,
        MotionTargetDescriptor? Initial = null,
        IReadOnlyList<MotionPseudoStyle>? PseudoStyles = null,
        StyleTransitionDescriptor? StyleTransition = null,
        IReadOnlyList<CssAnimationDescriptor>? Animations = null,
        IReadOnlyList<MotionDecorationDescriptor>? Decorations = null,
        MotionVariantResolution? Variants = null,
        IReadOnlyList<MotionValueDescriptor>? Values = null,
        IReadOnlyList<MotionValueBinding>? ValueBindings = null,
        IReadOnlyList<MotionValueSubscription>? ValueSubscriptions = null,
        ObjectId? ControlId = null,
        ObjectId? ScopeId = null,
        bool ScopeRoot = false,
        string? MotionName = null,
        IReadOnlyList<MotionNamedTarget>? NamedTargets = null,
        MotionGestureDescriptor? Gestures = null,
        MotionLayoutDescriptor? Layout = null
    );

    /// <summary>Parent/child sequencing selected by a resolved variant.</summary>
    public enum VariantWhen
    {
        Together,
        BeforeChildren,
        AfterChildren,
    }

    /// <summary>Direction used to assign stagger positions.</summary>
    public enum StaggerDirection
    {
        Forward,
        Reverse,
    }

    /// <summary>Inspectable facts from Rust's logical variant resolution.</summary>
    public sealed record MotionVariantResolution(
        IReadOnlyList<string> Names,
        bool Inherited,
        ulong CustomSnapshot,
        uint ChildIndex,
        ulong DelayMicros,
        VariantWhen When,
        StaggerDirection StaggerDirection
    );

    /// <summary>A locally resolved UI pseudo-state.</summary>
    public enum MotionPseudoState
    {
        Hover,
        Focus,
        Active,
        Disabled,
    }

    /// <summary>Typed properties contributed by one pseudo-state.</summary>
    public sealed record MotionPseudoStyle(
        MotionPseudoState State,
        IReadOnlyList<MotionPropertyValue> Values
    );

    /// <summary>One property-specific CSS transition.</summary>
    public sealed record StylePropertyTransition(
        MotionProperty Property,
        TransitionDefinition Transition
    );

    /// <summary>CSS transition behavior for resolved static styles.</summary>
    public sealed record StyleTransitionDescriptor(
        IReadOnlyList<StylePropertyTransition> Properties,
        TransitionDefinition? All = null,
        bool AllowDiscrete = false
    );

    /// <summary>CSS animation playback direction.</summary>
    public enum AnimationDirection
    {
        Normal,
        Reverse,
        Alternate,
        AlternateReverse,
    }

    /// <summary>CSS animation fill behavior.</summary>
    public enum AnimationFill
    {
        None,
        Forwards,
        Backwards,
        Both,
    }

    /// <summary>CSS animation play state.</summary>
    public enum AnimationPlayState
    {
        Running,
        Paused,
    }

    /// <summary>CSS animation property composition.</summary>
    public enum AnimationComposition
    {
        Replace,
        Add,
        Accumulate,
    }

    /// <summary>One property-local CSS keyframe sequence.</summary>
    public sealed record CssPropertyTrack(
        MotionProperty Property,
        IReadOnlyList<MotionValue> Values,
        IReadOnlyList<double> Times,
        TransitionDefinition Transition
    );

    /// <summary>One reusable CSS-style animation slot.</summary>
    public sealed record CssAnimationDescriptor(
        ulong Slot,
        uint Generation,
        ulong RestartKey,
        IReadOnlyList<CssPropertyTrack> Tracks,
        AnimationDirection Direction,
        AnimationFill Fill,
        AnimationPlayState PlayState,
        AnimationComposition Composition,
        string? DiagnosticName = null
    );

    /// <summary>Paint order for one decoration layer.</summary>
    public enum DecorationPlacement
    {
        Before,
        After,
    }

    /// <summary>Geometry policy for one decoration layer.</summary>
    public enum DecorationPosition
    {
        Fill,
        Border,
    }

    /// <summary>Clip policy for one decoration layer.</summary>
    public enum DecorationOverflow
    {
        Hidden,
        Visible,
    }

    /// <summary>One non-interactive paint layer associated with a host.</summary>
    public sealed record MotionDecorationDescriptor(
        ulong Key,
        DecorationPlacement Placement,
        DecorationPosition Position,
        DecorationOverflow Overflow,
        UiStyle Style,
        IReadOnlyList<CssAnimationDescriptor> Animations
    );

    /// <summary>Renderer declaration for one supported property and value shape.</summary>
    public sealed record MotionRendererCapability(
        MotionProperty Property,
        MotionValueKind ValueKind
    );

    /// <summary>Reliable lifecycle boundary kind.</summary>
    public abstract record MotionEventKind
    {
        public sealed record Activated : MotionEventKind;

        public sealed record Started : MotionEventKind;

        public sealed record Repeated(uint First, uint Last) : MotionEventKind;

        public sealed record Completed : MotionEventKind;

        public sealed record Stopped : MotionEventKind;

        public sealed record Cancelled : MotionEventKind;
    }

    /// <summary>One reliable slot lifecycle boundary.</summary>
    public sealed record MotionLifecycleEvent(
        ulong Sequence,
        ObjectId DescriptorId,
        ulong Slot,
        uint Generation,
        ulong ElapsedMicros,
        MotionEventKind Kind
    );

    /// <summary>Replaceable presentation sample.</summary>
    public sealed record MotionPresentationSample(
        ObjectId DescriptorId,
        ulong Slot,
        uint Generation,
        ulong ElapsedMicros,
        IReadOnlyList<MotionPropertyValue> Values
    );

    /// <summary>Reliable or replaceable native gesture event kind.</summary>
    public enum MotionGestureEventKind
    {
        HoverStart,
        HoverEnd,
        TapStart,
        Tap,
        TapCancel,
        FocusStart,
        FocusEnd,
        FocusVisibleStart,
        FocusVisibleEnd,
        PanSessionStart,
        PanStart,
        Pan,
        PanEnd,
        PanCancel,
        DragStart,
        DragDirectionLock,
        Drag,
        DragEnd,
        DragCancel,
        DragMomentumComplete,
        DragConstraintsMeasured,
        Scroll,
        InViewEnter,
        InViewLeave,
    }

    /// <summary>One gesture boundary or coalesced movement sample.</summary>
    public sealed record MotionGestureEvent(
        ObjectId DescriptorId,
        uint Generation,
        MotionGestureEventKind Kind,
        int PointerId,
        MotionPointerDevice Device,
        MotionGestureVector Point,
        MotionGestureVector Delta,
        MotionGestureVector Offset,
        MotionGestureVector Velocity,
        MotionGestureAxis? Axis,
        uint MomentumGeneration,
        bool Constrained
    );

    /// <summary>One external drag-controls start sampled from a pointer event.</summary>
    public sealed record MotionDragControlOperation(
        ObjectId ControlId,
        int PointerId,
        MotionPointerDevice Device,
        MotionGestureVector Point,
        bool SnapToCursor
    );

    /// <summary>Terminal result for one stable imperative playback identity.</summary>
    public enum MotionPlaybackOutcome
    {
        Completed,
        Stopped,
        Cancelled,
    }

    /// <summary>One generation-checked terminal event for an imperative playback.</summary>
    public sealed record MotionPlaybackEvent(
        ObjectId PlaybackId,
        uint Generation,
        MotionPlaybackOutcome Outcome
    );

    /// <summary>Ordered lifecycle boundaries and coalesced samples.</summary>
    public sealed record MotionEventBatch(
        ulong FirstSequence,
        ulong LastSequence,
        IReadOnlyList<MotionLifecycleEvent> Events,
        IReadOnlyList<MotionPresentationSample> Samples,
        IReadOnlyList<MotionValueSample>? ValueSamples = null,
        IReadOnlyList<MotionPlaybackEvent>? PlaybackEvents = null,
        IReadOnlyList<MotionGestureEvent>? GestureEvents = null
    );

    /// <summary>Compact timeline checkpoint retained for reconnect.</summary>
    public sealed record MotionTimelineCheckpoint(
        ObjectId DescriptorId,
        ulong Slot,
        uint Generation,
        ulong ElapsedMicros,
        uint Iteration,
        bool Paused,
        IReadOnlyList<MotionPropertyValue> Values
    );

    /// <summary>Direction selected for one playback generation.</summary>
    public enum MotionPlaybackDirection
    {
        Forward,
        Reverse,
        Alternate,
        AlternateReverse,
    }

    /// <summary>Generation-checked playback operation.</summary>
    public abstract record MotionPlaybackCommand
    {
        public sealed record Play : MotionPlaybackCommand;

        public sealed record Pause : MotionPlaybackCommand;

        public sealed record Replay : MotionPlaybackCommand;

        public sealed record Stop : MotionPlaybackCommand;

        public sealed record Cancel : MotionPlaybackCommand;

        public sealed record Complete : MotionPlaybackCommand;

        public sealed record Seek(ulong ElapsedMicros) : MotionPlaybackCommand;

        public sealed record SetSpeed(double Value) : MotionPlaybackCommand;

        public sealed record SetDirection(MotionPlaybackDirection Value) : MotionPlaybackCommand;
    }

    /// <summary>Addressed playback operation for one current slot generation.</summary>
    public sealed record MotionPlaybackOperation(
        ObjectId DescriptorId,
        ulong Slot,
        uint Generation,
        MotionPlaybackCommand Command
    );

    /// <summary>Deterministic controlled-clock mutation.</summary>
    public abstract record MotionControlledClockCommand
    {
        public sealed record Set(ulong ElapsedMicros) : MotionControlledClockCommand;

        public sealed record Advance(ulong DeltaMicros) : MotionControlledClockCommand;
    }

    /// <summary>Addressed mutation for a controlled motion clock.</summary>
    public sealed record MotionControlledClockOperation(
        ObjectId ClockId,
        MotionControlledClockCommand Command
    );
}

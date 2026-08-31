#nullable enable

using System.Collections.Generic;

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
        Hover,
        Tap,
        Drag,
        Exit,
    }

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

    /// <summary>Complete validated animation state installed beside one host.</summary>
    public sealed record MotionDescriptor(
        ObjectId DescriptorId,
        ObjectId HostId,
        uint Generation,
        IReadOnlyList<MotionPropertyValue> StaticBaseline,
        bool InitialDisabled,
        IReadOnlyList<MotionSlotDescriptor> Slots,
        MotionClockSource Clock,
        ReducedMotionPolicy ReducedMotion,
        MotionTargetDescriptor? Initial = null
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

    /// <summary>Ordered lifecycle boundaries and coalesced samples.</summary>
    public sealed record MotionEventBatch(
        ulong FirstSequence,
        ulong LastSequence,
        IReadOnlyList<MotionLifecycleEvent> Events,
        IReadOnlyList<MotionPresentationSample> Samples
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

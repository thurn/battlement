#nullable enable

using System.Collections.Generic;

namespace Battlement
{
    /// <summary>One closed operation evaluated by the native motion-value graph.</summary>
    public abstract record MotionExpressionOperation
    {
        public sealed record Add : MotionExpressionOperation;

        public sealed record Subtract : MotionExpressionOperation;

        public sealed record Multiply : MotionExpressionOperation;

        public sealed record Divide : MotionExpressionOperation;

        public sealed record Power(double Value) : MotionExpressionOperation;

        public sealed record SquareRoot : MotionExpressionOperation;

        public sealed record Absolute : MotionExpressionOperation;

        public sealed record Minimum : MotionExpressionOperation;

        public sealed record Maximum : MotionExpressionOperation;

        public sealed record Clamp(double Min, double Max) : MotionExpressionOperation;

        public sealed record Modulo(double Value) : MotionExpressionOperation;

        public sealed record Wrap(double Min, double Max) : MotionExpressionOperation;

        public sealed record ExponentialDecay(double Rate) : MotionExpressionOperation;

        public sealed record Mix : MotionExpressionOperation;
    }

    /// <summary>Native source or derived operation for one stable motion value.</summary>
    public abstract record MotionValueSource
    {
        public sealed record Mutable : MotionValueSource;

        public sealed record Time(MotionClockSource Value) : MotionValueSource;

        public sealed record Velocity(ObjectId Source) : MotionValueSource;

        public sealed record Range(
            ObjectId Source,
            IReadOnlyList<MotionValue> Input,
            IReadOnlyList<MotionValue> Output,
            bool Clamp
        ) : MotionValueSource;

        public sealed record Spring(ObjectId Source, SpringConfiguration Configuration)
            : MotionValueSource;

        public sealed record Expression(
            MotionExpressionOperation Operation,
            IReadOnlyList<ObjectId> Inputs
        ) : MotionValueSource;
    }

    /// <summary>One stable node in the Unity-local motion-value graph.</summary>
    public sealed record MotionValueDescriptor(
        ObjectId ValueId,
        MotionValue Initial,
        MotionValueSource Source
    );

    /// <summary>One host property driven directly by a graph value.</summary>
    public sealed record MotionValueBinding(MotionProperty Property, ObjectId ValueId);

    /// <summary>Explicit replaceable event requested for one value.</summary>
    public enum MotionValueEventKind
    {
        Change,
        Velocity,
        AnimationFrame,
    }

    /// <summary>One explicit Rust-side graph observation.</summary>
    public sealed record MotionValueSubscription(
        ObjectId SubscriptionId,
        ObjectId ValueId,
        MotionValueEventKind Event
    );

    /// <summary>Mutable-value operation issued outside render.</summary>
    public abstract record MotionValueCommand
    {
        public sealed record Set(MotionValue Value) : MotionValueCommand;

        public sealed record Jump(MotionValue Value) : MotionValueCommand;

        public sealed record Stop : MotionValueCommand;

        public sealed record Animate(
            ObjectId PlaybackId,
            uint Generation,
            MotionValue Target,
            TransitionDefinition Transition
        ) : MotionValueCommand;
    }

    /// <summary>Addressed mutable-value operation.</summary>
    public sealed record MotionValueOperation(ObjectId ValueId, MotionValueCommand Command);

    /// <summary>Generation-checked operation for one motion-value playback.</summary>
    public sealed record MotionValuePlaybackOperation(
        ObjectId PlaybackId,
        uint Generation,
        MotionPlaybackCommand Command
    );

    /// <summary>One coalesced explicit-subscription sample.</summary>
    public sealed record MotionValueSample(
        ObjectId SubscriptionId,
        ObjectId ValueId,
        ulong Frame,
        MotionValue Value,
        MotionValue Velocity,
        bool Discontinuity
    );

    /// <summary>Concrete or named target broadcast by animation controls.</summary>
    public abstract record MotionControlTarget
    {
        public sealed record Target(MotionTargetDescriptor Value) : MotionControlTarget;

        public sealed record Variant(string Value) : MotionControlTarget;
    }

    /// <summary>One named target retained for imperative variant starts.</summary>
    public sealed record MotionNamedTarget(string Name, MotionTargetDescriptor Target);

    /// <summary>Broadcast operation for one animation-controls identity.</summary>
    public abstract record MotionControlCommand
    {
        public sealed record Start(ObjectId PlaybackId, uint Generation, MotionControlTarget Target)
            : MotionControlCommand;

        public sealed record Set(MotionControlTarget Value) : MotionControlCommand;

        public sealed record Stop : MotionControlCommand;

        public sealed record Clear : MotionControlCommand;
    }

    /// <summary>Addressed animation-controls operation.</summary>
    public sealed record MotionControlOperation(ObjectId ControlId, MotionControlCommand Command);

    /// <summary>Closed selector resolved inside one animation scope.</summary>
    public abstract record MotionSelector
    {
        public sealed record Element(ObjectId Value) : MotionSelector;

        public sealed record Name(string Value) : MotionSelector;

        public sealed record ScopeRoot : MotionSelector;

        public sealed record Children : MotionSelector;

        public sealed record Descendants : MotionSelector;
    }

    /// <summary>One scheduled scoped animation step.</summary>
    public sealed record MotionSequenceStep(
        MotionSelector Selector,
        MotionTargetDescriptor Target,
        ulong StartMicros
    );

    /// <summary>Scoped animation operation.</summary>
    public abstract record MotionScopeCommand
    {
        public sealed record Start(
            ObjectId PlaybackId,
            uint Generation,
            IReadOnlyList<MotionSequenceStep> Steps
        ) : MotionScopeCommand;

        public sealed record Set(MotionSelector Selector, MotionTargetDescriptor Target)
            : MotionScopeCommand;

        public sealed record Stop(MotionSelector Value) : MotionScopeCommand;
    }

    /// <summary>Addressed animation-scope operation.</summary>
    public sealed record MotionScopeOperation(ObjectId ScopeId, MotionScopeCommand Command);
}

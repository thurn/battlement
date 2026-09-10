#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

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

    /// <summary>How a graph value participates in a host property.</summary>
    public enum MotionBindingComposition
    {
        Replace,
        Compose,
    }

    /// <summary>One host property driven directly by a graph value.</summary>
    public sealed record MotionValueBinding(
        MotionProperty Property,
        ObjectId ValueId,
        MotionBindingComposition Composition = MotionBindingComposition.Replace
    );

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

    internal static class MotionGraphDefinitionEquality
    {
        public static bool Same(MotionValueDescriptor a, MotionValueDescriptor b) =>
            a.ValueId == b.ValueId && Same(a.Initial, b.Initial) && Source(a.Source, b.Source);

        private static bool Source(MotionValueSource a, MotionValueSource b) =>
            (a, b) switch
            {
                (MotionValueSource.Range x, MotionValueSource.Range y) => x.Source == y.Source
                    && x.Clamp == y.Clamp
                    && Ranges(x, y),
                (MotionValueSource.Expression x, MotionValueSource.Expression y) => x.Operation
                    == y.Operation
                    && x.Inputs.SequenceEqual(y.Inputs),
                _ => a == b,
            };

        private static bool Ranges(MotionValueSource.Range a, MotionValueSource.Range b) =>
            Sequence(a.Input, b.Input, Same) && Sequence(a.Output, b.Output, Same);

        public static bool Same(MotionValue a, MotionValue b) =>
            (a, b) switch
            {
                (MotionValue.Vector2 x, MotionValue.Vector2 y) => x.Value.SequenceEqual(y.Value),
                (MotionValue.Vector3 x, MotionValue.Vector3 y) => x.Value.SequenceEqual(y.Value),
                (MotionValue.TransformList x, MotionValue.TransformList y) => Sequence(
                    x.Value,
                    y.Value,
                    Transform
                ),
                (MotionValue.FilterList x, MotionValue.FilterList y) => x.Value.SequenceEqual(
                    y.Value
                ),
                (MotionValue.ShadowList x, MotionValue.ShadowList y) => x.Value.SequenceEqual(
                    y.Value
                ),
                (MotionValue.Gradient x, MotionValue.Gradient y) => Gradient(x.Value, y.Value),
                (MotionValue.ClipInset x, MotionValue.ClipInset y) => x.Value.SequenceEqual(
                    y.Value
                ),
                (MotionValue.ClipPolygon x, MotionValue.ClipPolygon y) => Sequence(
                    x.Value,
                    y.Value,
                    (left, right) => left.SequenceEqual(right)
                ),
                (MotionValue.Discrete x, MotionValue.Discrete y) =>
                    Newtonsoft.Json.Linq.JToken.DeepEquals(x.Value, y.Value),
                _ => a == b,
            };

        private static bool Transform(TransformOperation a, TransformOperation b) =>
            (a, b) switch
            {
                (TransformOperation.Translate x, TransformOperation.Translate y) =>
                    x.Value.SequenceEqual(y.Value),
                (TransformOperation.Rotate x, TransformOperation.Rotate y) => x.Value.SequenceEqual(
                    y.Value
                ),
                (TransformOperation.Skew x, TransformOperation.Skew y) => x.Value.SequenceEqual(
                    y.Value
                ),
                (TransformOperation.Scale x, TransformOperation.Scale y) => x.Value.SequenceEqual(
                    y.Value
                ),
                _ => false,
            };

        private static bool Gradient(Gradient a, Gradient b) =>
            (a, b) switch
            {
                (Battlement.Gradient.Linear x, Battlement.Gradient.Linear y) => x.Angle == y.Angle
                    && x.Stops.SequenceEqual(y.Stops),
                (Battlement.Gradient.Radial x, Battlement.Gradient.Radial y) =>
                    x.Center.SequenceEqual(y.Center)
                        && x.Radius.SequenceEqual(y.Radius)
                        && x.Stops.SequenceEqual(y.Stops),
                _ => false,
            };

        private static bool Sequence<T>(
            IReadOnlyList<T> a,
            IReadOnlyList<T> b,
            Func<T, T, bool> same
        )
        {
            if (a.Count != b.Count)
                return false;
            for (int index = 0; index < a.Count; index++)
                if (!same(a[index], b[index]))
                    return false;
            return true;
        }
    }
}

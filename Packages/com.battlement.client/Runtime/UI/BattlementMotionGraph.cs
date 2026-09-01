#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal readonly struct MotionClockSample
    {
        public MotionClockSample(ulong elapsedMicros, bool discontinuity) =>
            (ElapsedMicros, Discontinuity) = (elapsedMicros, discontinuity);

        public ulong ElapsedMicros { get; }

        public bool Discontinuity { get; }
    }

    internal sealed class BattlementMotionGraph
    {
        private readonly Dictionary<Guid, Registration> registrations = new();
        private readonly Dictionary<Guid, NodeState> nodes = new();
        private readonly Dictionary<Guid, ValuePlayback> playbacks = new();
        private readonly List<NodeState> order = new();
        private readonly List<MotionValueSample> samples = new();
        private readonly List<MotionPlaybackEvent> playbackEvents = new();
        private readonly Dictionary<MotionClockSource, MotionClockSample> clockSamples = new();
        private readonly Func<MotionClockSource, MotionClockSample> clock;
        private readonly Func<MotionDescriptor, bool> reducedMotion;
        private ulong frame;

        public BattlementMotionGraph(
            Func<MotionClockSource, MotionClockSample> clock,
            Func<MotionDescriptor, bool> reducedMotion
        ) => (this.clock, this.reducedMotion) = (clock, reducedMotion);

        public int NodeCount => nodes.Count;

        public int LastEvaluationCount { get; private set; }

        public static void ValidateDescriptor(MotionDescriptor descriptor)
        {
            IReadOnlyList<MotionValueDescriptor> values =
                descriptor.Values ?? Array.Empty<MotionValueDescriptor>();
            var ids = new HashSet<Guid>();
            foreach (MotionValueDescriptor value in values)
            {
                if (!ids.Add(value.ValueId.Value))
                    throw Invalid("Motion-value graph repeats a value identity.");
                ValidateSource(value.Source);
            }
            foreach (MotionValueDescriptor value in values)
            foreach (Guid dependency in Dependencies(value.Source))
            {
                if (!ids.Contains(dependency))
                    throw Invalid("Motion-value graph references an unavailable input.");
            }
            var byId = values.ToDictionary(value => value.ValueId.Value);
            var visiting = new HashSet<Guid>();
            var visited = new HashSet<Guid>();
            foreach (Guid id in ids)
                Visit(id, byId, visiting, visited);

            var properties = new HashSet<MotionProperty>();
            foreach (
                MotionValueBinding binding in descriptor.ValueBindings
                    ?? Array.Empty<MotionValueBinding>()
            )
            {
                if (!ids.Contains(binding.ValueId.Value))
                    throw Invalid("Motion-value binding references an unavailable value.");
                if (!properties.Add(binding.Property))
                    throw Invalid("Motion-value bindings repeat a host property.");
                if (!BattlementMotionPropertyWriter.Supports(binding.Property))
                    throw Invalid(
                        $"Motion property {binding.Property} has no renderer capability."
                    );
            }
            var subscriptions = new HashSet<Guid>();
            foreach (
                MotionValueSubscription subscription in descriptor.ValueSubscriptions
                    ?? Array.Empty<MotionValueSubscription>()
            )
            {
                if (!ids.Contains(subscription.ValueId.Value))
                    throw Invalid("Motion-value subscription references an unavailable value.");
                if (!subscriptions.Add(subscription.SubscriptionId.Value))
                    throw Invalid("Motion-value subscriptions repeat an identity.");
            }
        }

        public void Replace(MotionDescriptor descriptor, VisualElement target)
        {
            registrations[descriptor.DescriptorId.Value] = new Registration(descriptor, target);
            Rebuild();
        }

        public void Remove(ObjectId descriptorId)
        {
            if (registrations.Remove(descriptorId.Value))
                Rebuild();
        }

        public void Clear()
        {
            registrations.Clear();
            nodes.Clear();
            playbacks.Clear();
            order.Clear();
            samples.Clear();
            playbackEvents.Clear();
            frame = 0;
            LastEvaluationCount = 0;
        }

        public void Sample()
        {
            frame++;
            LastEvaluationCount = 0;
            clockSamples.Clear();
            foreach ((Guid id, ValuePlayback playback) in playbacks.ToArray())
            {
                playback.Sample(clock(new MotionClockSource.Unscaled()).ElapsedMicros);
                if (playback.Outcome is MotionPlaybackOutcome outcome)
                {
                    playbackEvents.Add(
                        new MotionPlaybackEvent(new ObjectId(id), playback.Generation, outcome)
                    );
                    playbacks.Remove(id);
                }
            }
            foreach (NodeState node in order)
            {
                if (!node.ShouldEvaluate(nodes))
                    continue;
                MotionClockSample now = ClockFor(node.Descriptor.Source);
                node.Evaluate(nodes, now);
                LastEvaluationCount++;
            }
            ApplyBindings();
            CaptureSubscriptions();
        }

        public void Apply(MotionValueOperation operation)
        {
            if (!nodes.TryGetValue(operation.ValueId.Value, out NodeState node))
                throw Invalid("The motion value does not exist.");
            if (node.Descriptor.Source is not MotionValueSource.Mutable)
                throw Invalid("Only mutable motion values accept imperative operations.");
            switch (operation.Command)
            {
                case MotionValueCommand.Set set:
                    node.Set(set.Value, discontinuity: false);
                    break;
                case MotionValueCommand.Jump jump:
                    node.Set(jump.Value, discontinuity: true);
                    break;
                case MotionValueCommand.Stop:
                    node.Stop();
                    foreach ((Guid id, ValuePlayback playback) in playbacks.ToArray())
                    {
                        if (playback.Node != node)
                            continue;
                        playback.Stop();
                        QueuePlaybackEvent(id, playback);
                        playbacks.Remove(id);
                    }
                    break;
                case MotionValueCommand.Animate animate:
                    foreach ((Guid id, ValuePlayback playback) in playbacks.ToArray())
                    {
                        if (playback.Node != node)
                            continue;
                        playback.Cancel();
                        QueuePlaybackEvent(id, playback);
                        playbacks.Remove(id);
                    }
                    playbacks.Add(
                        animate.PlaybackId.Value,
                        new ValuePlayback(
                            node,
                            animate,
                            clock(new MotionClockSource.Unscaled()).ElapsedMicros
                        )
                    );
                    break;
                default:
                    throw Invalid("Unknown motion-value command.");
            }
        }

        public void SetLocal(ObjectId valueId, MotionValue value)
        {
            if (!nodes.TryGetValue(valueId.Value, out NodeState node))
                throw Invalid("The gesture motion value does not exist.");
            if (node.Descriptor.Source is not MotionValueSource.Mutable)
                throw Invalid("Gesture motion values must be mutable.");
            node.Set(value, discontinuity: false);
        }

        public void Apply(MotionValuePlaybackOperation operation)
        {
            if (!playbacks.TryGetValue(operation.PlaybackId.Value, out ValuePlayback playback))
                return;
            if (playback.Generation != operation.Generation)
                throw Invalid("The motion-value playback generation is stale.");
            playback.Apply(
                operation.Command,
                clock(new MotionClockSource.Unscaled()).ElapsedMicros
            );
            if (playback.Terminal)
            {
                QueuePlaybackEvent(operation.PlaybackId.Value, playback);
                playbacks.Remove(operation.PlaybackId.Value);
            }
        }

        public IReadOnlyList<MotionValueSample> DrainSamples()
        {
            MotionValueSample[] drained = samples.ToArray();
            samples.Clear();
            return drained;
        }

        public IReadOnlyList<MotionPlaybackEvent> DrainPlaybackEvents()
        {
            MotionPlaybackEvent[] drained = playbackEvents.ToArray();
            playbackEvents.Clear();
            return drained;
        }

        private void QueuePlaybackEvent(Guid id, ValuePlayback playback)
        {
            if (playback.Outcome is not MotionPlaybackOutcome outcome)
                return;
            playbackEvents.Add(
                new MotionPlaybackEvent(new ObjectId(id), playback.Generation, outcome)
            );
        }

        private void Rebuild()
        {
            var retained = new Dictionary<Guid, NodeState>(nodes);
            nodes.Clear();
            order.Clear();
            foreach (Registration registration in registrations.Values)
            foreach (
                MotionValueDescriptor descriptor in registration.Descriptor.Values
                    ?? Array.Empty<MotionValueDescriptor>()
            )
            {
                if (nodes.TryGetValue(descriptor.ValueId.Value, out NodeState shared))
                {
                    if (!shared.Matches(descriptor))
                        throw Invalid(
                            "A shared motion-value identity has incompatible definitions."
                        );
                }
                else
                {
                    NodeState node =
                        retained.TryGetValue(descriptor.ValueId.Value, out NodeState old)
                        && old.Compatible(descriptor)
                            ? old
                            : new NodeState(descriptor);
                    nodes.Add(descriptor.ValueId.Value, node);
                }
            }
            var visited = new HashSet<Guid>();
            foreach (Guid id in nodes.Keys)
                Append(id, visited);
            foreach (
                Guid id in playbacks
                    .Where(pair => !nodes.ContainsValue(pair.Value.Node))
                    .Select(pair => pair.Key)
                    .ToArray()
            )
                playbacks.Remove(id);
        }

        private void Append(Guid id, HashSet<Guid> visited)
        {
            if (!visited.Add(id))
                return;
            foreach (Guid dependency in Dependencies(nodes[id].Descriptor.Source))
                Append(dependency, visited);
            order.Add(nodes[id]);
        }

        private void ApplyBindings()
        {
            foreach (Registration registration in registrations.Values)
            foreach (
                MotionValueBinding binding in registration.Descriptor.ValueBindings
                    ?? Array.Empty<MotionValueBinding>()
            )
            {
                MotionValue value =
                    reducedMotion(registration.Descriptor)
                    && BattlementMotionPropertyWriter.IsSpatial(binding.Property)
                        ? ReducedValue(registration.Descriptor, binding.Property)
                        : Adapt(binding.Property, nodes[binding.ValueId.Value].Value);
                BattlementMotionPropertyWriter.Write(registration.Target, binding.Property, value);
            }
        }

        private static MotionValue ReducedValue(
            MotionDescriptor descriptor,
            MotionProperty property
        )
        {
            foreach (MotionPropertyValue baseline in descriptor.StaticBaseline)
                if (baseline.Property == property)
                    return baseline.Value;
            return property switch
            {
                MotionProperty.X or MotionProperty.Y or MotionProperty.Z => new MotionValue.Length(
                    new MotionLength(0, 0)
                ),
                MotionProperty.Translate => new MotionValue.Vector2(new double[] { 0, 0 }),
                MotionProperty.Scale => new MotionValue.Vector2(new double[] { 1, 1 }),
                MotionProperty.ScaleX or MotionProperty.ScaleY => new MotionValue.Scalar(1),
                MotionProperty.Rotate
                or MotionProperty.RotateX
                or MotionProperty.RotateY
                or MotionProperty.SkewX
                or MotionProperty.SkewY => new MotionValue.Angle(0),
                MotionProperty.TransformList => new MotionValue.TransformList(
                    Array.Empty<MotionTransform>()
                ),
                _ => throw Invalid("Reduced motion received a non-spatial binding."),
            };
        }

        private void CaptureSubscriptions()
        {
            foreach (Registration registration in registrations.Values)
            foreach (
                MotionValueSubscription subscription in registration.Descriptor.ValueSubscriptions
                    ?? Array.Empty<MotionValueSubscription>()
            )
            {
                NodeState node = nodes[subscription.ValueId.Value];
                bool requested = subscription.Event switch
                {
                    MotionValueEventKind.AnimationFrame => true,
                    MotionValueEventKind.Change or MotionValueEventKind.Velocity => node.Changed
                        || node.Discontinuity,
                    _ => throw Invalid("Unknown motion-value event kind."),
                };
                if (!requested)
                    continue;
                int existing = samples.FindIndex(value =>
                    value.SubscriptionId == subscription.SubscriptionId
                );
                var sample = new MotionValueSample(
                    subscription.SubscriptionId,
                    subscription.ValueId,
                    frame,
                    node.Value,
                    node.Velocity,
                    node.Discontinuity
                );
                if (existing < 0)
                    samples.Add(sample);
                else
                    samples[existing] = sample;
            }
            foreach (NodeState node in nodes.Values)
            {
                node.Discontinuity = false;
                node.Changed = false;
            }
        }

        private MotionClockSample ClockFor(MotionValueSource source) =>
            source is MotionValueSource.Time time
                ? SampleClock(time.Value)
                : SampleClock(new MotionClockSource.Unscaled());

        private MotionClockSample SampleClock(MotionClockSource source)
        {
            if (!clockSamples.TryGetValue(source, out MotionClockSample sample))
            {
                sample = clock(source);
                clockSamples.Add(source, sample);
            }
            return sample;
        }

        private static MotionValue Adapt(MotionProperty property, MotionValue value)
        {
            if (value is not MotionValue.Scalar scalar)
                return value;
            return property switch
            {
                MotionProperty.X
                or MotionProperty.Y
                or MotionProperty.Z
                or MotionProperty.Width
                or MotionProperty.Height
                or MotionProperty.MinWidth
                or MotionProperty.MinHeight
                or MotionProperty.MaxWidth
                or MotionProperty.MaxHeight => new MotionValue.Length(
                    new MotionLength(scalar.Value, 0)
                ),
                MotionProperty.Scale => new MotionValue.Vector2(
                    new[] { scalar.Value, scalar.Value }
                ),
                _ => value,
            };
        }

        private static void ValidateSource(MotionValueSource source)
        {
            switch (source)
            {
                case MotionValueSource.Range range
                    when range.Input.Count < 2 || range.Input.Count != range.Output.Count:
                    throw Invalid(
                        "Motion-value ranges require aligned ranges of at least two values."
                    );
                case MotionValueSource.Expression expression
                    when expression.Inputs.Count != Arity(expression.Operation):
                    throw Invalid("Motion expression has the wrong input arity.");
                default:
                    break;
            }
        }

        private static int Arity(MotionExpressionOperation operation) =>
            operation switch
            {
                MotionExpressionOperation.Power
                or MotionExpressionOperation.SquareRoot
                or MotionExpressionOperation.Absolute
                or MotionExpressionOperation.Clamp
                or MotionExpressionOperation.Modulo
                or MotionExpressionOperation.Wrap
                or MotionExpressionOperation.ExponentialDecay => 1,
                MotionExpressionOperation.Mix => 3,
                _ => 2,
            };

        private static IEnumerable<Guid> Dependencies(MotionValueSource source) =>
            source switch
            {
                MotionValueSource.Velocity value => new[] { value.Source.Value },
                MotionValueSource.Range value => new[] { value.Source.Value },
                MotionValueSource.Spring value => new[] { value.Source.Value },
                MotionValueSource.Expression value => value.Inputs.Select(input => input.Value),
                _ => Array.Empty<Guid>(),
            };

        private static void Visit(
            Guid id,
            IReadOnlyDictionary<Guid, MotionValueDescriptor> values,
            HashSet<Guid> visiting,
            HashSet<Guid> visited
        )
        {
            if (visited.Contains(id))
                return;
            if (!visiting.Add(id))
                throw Invalid("Motion-value graph contains a cycle.");
            foreach (Guid dependency in Dependencies(values[id].Source))
                Visit(dependency, values, visiting, visited);
            visiting.Remove(id);
            visited.Add(id);
        }

        private static BattlementUiException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private sealed record Registration(MotionDescriptor Descriptor, VisualElement Target);

        private sealed class NodeState
        {
            private MotionValue previous;
            private ulong previousMicros;
            private MotionValue? springTarget;
            private MotionValue? springOrigin;
            private ulong springAnchor;

            public NodeState(MotionValueDescriptor descriptor)
            {
                Descriptor = descriptor;
                Value = descriptor.Initial;
                previous = descriptor.Initial;
                Velocity = Zero(descriptor.Initial);
            }

            public MotionValueDescriptor Descriptor { get; }

            public MotionValue Value { get; private set; }

            public MotionValue Velocity { get; private set; }

            public bool Discontinuity { get; set; }

            public bool Changed { get; set; } = true;

            public bool Compatible(MotionValueDescriptor descriptor) =>
                Descriptor.Source.GetType() == descriptor.Source.GetType();

            public bool Matches(MotionValueDescriptor descriptor) => Descriptor == descriptor;

            public bool ShouldEvaluate(IReadOnlyDictionary<Guid, NodeState> graph) =>
                Descriptor.Source switch
                {
                    MotionValueSource.Mutable => Changed || Discontinuity,
                    MotionValueSource.Time => true,
                    MotionValueSource.Spring value => graph[value.Source.Value].Changed
                        || graph[value.Source.Value].Discontinuity
                        || springTarget is not null && !Equals(Value, springTarget),
                    _ => Dependencies(Descriptor.Source)
                        .Any(id => graph[id].Changed || graph[id].Discontinuity),
                };

            public bool Evaluate(IReadOnlyDictionary<Guid, NodeState> graph, MotionClockSample now)
            {
                MotionValue next = Descriptor.Source switch
                {
                    MotionValueSource.Mutable => Value,
                    MotionValueSource.Time => new MotionValue.Scalar(
                        now.ElapsedMicros / 1_000_000d
                    ),
                    MotionValueSource.Velocity value => graph[value.Source.Value].Velocity,
                    MotionValueSource.Range value => Range(value, graph[value.Source.Value].Value),
                    MotionValueSource.Spring value => Spring(
                        value,
                        graph[value.Source.Value].Value,
                        now.ElapsedMicros
                    ),
                    MotionValueSource.Expression value => Expression(value, graph),
                    _ => throw Invalid("Unknown motion-value source."),
                };
                bool changed = !Equals(next, Value) || now.Discontinuity;
                previous = Value;
                Value = next;
                Velocity = now.Discontinuity
                    ? Zero(next)
                    : Difference(next, previous, now.ElapsedMicros - previousMicros);
                previousMicros = now.ElapsedMicros;
                Discontinuity |= now.Discontinuity;
                Changed |= changed;
                return changed;
            }

            public void Set(MotionValue value, bool discontinuity)
            {
                previous = Value;
                Value = value;
                Velocity = Zero(value);
                Discontinuity |= discontinuity;
                Changed = true;
                springTarget = null;
            }

            public void Stop()
            {
                Velocity = Zero(Value);
                Changed = true;
                springTarget = null;
            }

            private MotionValue Spring(
                MotionValueSource.Spring spring,
                MotionValue target,
                ulong now
            )
            {
                if (!Equals(target, springTarget))
                {
                    springTarget = target;
                    springOrigin = Value;
                    springAnchor = now;
                }
                if (
                    springOrigin is MotionValue.Scalar origin
                    && target is MotionValue.Scalar scalar
                )
                {
                    var transition = new TransitionDefinition(
                        new TransitionGenerator.Spring(spring.Configuration),
                        0,
                        new MotionRepeat.None(),
                        0,
                        MotionRepeatType.Loop
                    );
                    MotionScalarSample sample = BattlementMotionScalarSampler.Sample(
                        origin.Value,
                        scalar.Value,
                        Velocity is MotionValue.Scalar speed ? speed.Value : 0,
                        transition,
                        now - springAnchor
                    );
                    return new MotionValue.Scalar(sample.Value);
                }
                double progress = Math.Clamp((now - springAnchor) / 300_000d, 0, 1);
                return BattlementMotionValueSampler.Mix(springOrigin!, target, progress);
            }

            private static MotionValue Range(MotionValueSource.Range range, MotionValue value)
            {
                if (value is not MotionValue.Scalar scalar)
                    throw Invalid("Motion range input must be scalar.");
                int segment = range.Input.Count - 2;
                for (int index = 0; index < range.Input.Count - 1; index++)
                {
                    if (scalar.Value <= Scalar(range.Input[index + 1]))
                    {
                        segment = index;
                        break;
                    }
                }
                double start = Scalar(range.Input[segment]);
                double end = Scalar(range.Input[segment + 1]);
                double progress = (scalar.Value - start) / (end - start);
                if (range.Clamp)
                    progress = Math.Clamp(progress, 0, 1);
                return BattlementMotionValueSampler.Mix(
                    range.Output[segment],
                    range.Output[segment + 1],
                    progress
                );
            }

            private static MotionValue Expression(
                MotionValueSource.Expression expression,
                IReadOnlyDictionary<Guid, NodeState> graph
            )
            {
                MotionValue Input(int index) => graph[expression.Inputs[index].Value].Value;
                double ScalarInput(int index) => Scalar(Input(index));
                double result = expression.Operation switch
                {
                    MotionExpressionOperation.Add => ScalarInput(0) + ScalarInput(1),
                    MotionExpressionOperation.Subtract => ScalarInput(0) - ScalarInput(1),
                    MotionExpressionOperation.Multiply => ScalarInput(0) * ScalarInput(1),
                    MotionExpressionOperation.Divide => ScalarInput(0) / ScalarInput(1),
                    MotionExpressionOperation.Power value => Math.Pow(ScalarInput(0), value.Value),
                    MotionExpressionOperation.SquareRoot => Math.Sqrt(ScalarInput(0)),
                    MotionExpressionOperation.Absolute => Math.Abs(ScalarInput(0)),
                    MotionExpressionOperation.Minimum => Math.Min(ScalarInput(0), ScalarInput(1)),
                    MotionExpressionOperation.Maximum => Math.Max(ScalarInput(0), ScalarInput(1)),
                    MotionExpressionOperation.Clamp value => Math.Clamp(
                        ScalarInput(0),
                        value.Min,
                        value.Max
                    ),
                    MotionExpressionOperation.Modulo value => Euclidean(
                        ScalarInput(0),
                        value.Value
                    ),
                    MotionExpressionOperation.Wrap value => value.Min
                        + Euclidean(ScalarInput(0) - value.Min, value.Max - value.Min),
                    MotionExpressionOperation.ExponentialDecay value => Math.Exp(
                        -value.Rate * ScalarInput(0)
                    ),
                    MotionExpressionOperation.Mix => double.NaN,
                    _ => throw Invalid("Unknown motion expression operation."),
                };
                return expression.Operation is MotionExpressionOperation.Mix
                    ? BattlementMotionValueSampler.Mix(Input(0), Input(1), ScalarInput(2))
                    : new MotionValue.Scalar(result);
            }

            private static double Scalar(MotionValue value) =>
                value is MotionValue.Scalar scalar
                    ? scalar.Value
                    : throw Invalid("Motion expression expected a scalar value.");

            private static double Euclidean(double value, double modulus) =>
                ((value % modulus) + modulus) % modulus;

            private static MotionValue Difference(MotionValue value, MotionValue prior, ulong delta)
            {
                if (delta == 0)
                    return Zero(value);
                double seconds = delta / 1_000_000d;
                return (value, prior) switch
                {
                    (MotionValue.Scalar current, MotionValue.Scalar previousValue) =>
                        new MotionValue.Scalar((current.Value - previousValue.Value) / seconds),
                    _ => Zero(value),
                };
            }

            private static MotionValue Zero(MotionValue value) =>
                value switch
                {
                    MotionValue.Scalar => new MotionValue.Scalar(0),
                    MotionValue.Length => new MotionValue.Length(new MotionLength(0, 0)),
                    MotionValue.Color => new MotionValue.Color(new MotionColor(0, 0, 0, 0)),
                    MotionValue.Vector2 => new MotionValue.Vector2(new double[] { 0, 0 }),
                    MotionValue.Vector3 => new MotionValue.Vector3(new double[] { 0, 0, 0 }),
                    MotionValue.Angle => new MotionValue.Angle(0),
                    _ => new MotionValue.Scalar(0),
                };
        }

        private sealed class ValuePlayback
        {
            private readonly MotionValue origin;
            private readonly MotionValue target;
            private readonly TransitionDefinition transition;
            private ulong anchor;
            private ulong held;
            private bool paused;
            private double speed = 1;

            public ValuePlayback(NodeState node, MotionValueCommand.Animate command, ulong now)
            {
                Node = node;
                Generation = command.Generation;
                origin = node.Value;
                target = command.Target;
                transition = command.Transition;
                anchor = now;
            }

            public NodeState Node { get; }

            public uint Generation { get; }

            public bool Terminal => Outcome is not null;

            public MotionPlaybackOutcome? Outcome { get; private set; }

            public void Sample(ulong now)
            {
                if (Terminal)
                    return;
                ulong elapsed = paused ? held : held + checked((ulong)((now - anchor) * speed));
                if (origin is MotionValue.Scalar left && target is MotionValue.Scalar right)
                {
                    MotionScalarSample sample = BattlementMotionScalarSampler.Sample(
                        left.Value,
                        right.Value,
                        0,
                        transition,
                        elapsed
                    );
                    Node.Set(new MotionValue.Scalar(sample.Value), discontinuity: false);
                    if (sample.Done)
                        Outcome = MotionPlaybackOutcome.Completed;
                }
                else
                {
                    MotionScalarSample sample = BattlementMotionScalarSampler.Sample(
                        0,
                        1,
                        0,
                        transition,
                        elapsed
                    );
                    Node.Set(BattlementMotionValueSampler.Mix(origin, target, sample.Value), false);
                    if (sample.Done)
                        Outcome = MotionPlaybackOutcome.Completed;
                }
            }

            public void Apply(MotionPlaybackCommand command, ulong now)
            {
                switch (command)
                {
                    case MotionPlaybackCommand.Play when paused:
                        anchor = now;
                        paused = false;
                        break;
                    case MotionPlaybackCommand.Pause when !paused:
                        held += checked((ulong)((now - anchor) * speed));
                        paused = true;
                        break;
                    case MotionPlaybackCommand.Stop:
                        Outcome = MotionPlaybackOutcome.Stopped;
                        Node.Stop();
                        break;
                    case MotionPlaybackCommand.Cancel:
                        Outcome = MotionPlaybackOutcome.Cancelled;
                        Node.Set(origin, true);
                        break;
                    case MotionPlaybackCommand.Complete:
                        Outcome = MotionPlaybackOutcome.Completed;
                        Node.Set(target, true);
                        break;
                    case MotionPlaybackCommand.Seek seek:
                        held = seek.ElapsedMicros;
                        paused = true;
                        Sample(now);
                        break;
                    case MotionPlaybackCommand.SetSpeed value:
                        if (!paused)
                        {
                            held += checked((ulong)((now - anchor) * speed));
                            anchor = now;
                        }
                        speed = value.Value;
                        if (speed == 0)
                            paused = true;
                        break;
                    case MotionPlaybackCommand.SetDirection:
                    case MotionPlaybackCommand.Replay:
                        held = 0;
                        anchor = now;
                        paused = false;
                        break;
                    default:
                        break;
                }
            }

            public void Stop() => Outcome = MotionPlaybackOutcome.Stopped;

            public void Cancel() => Outcome = MotionPlaybackOutcome.Cancelled;
        }
    }
}

#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementMotionWorld : IDisposable
    {
        private readonly Dictionary<Guid, DescriptorState> descriptors = new();
        private readonly Dictionary<Guid, Guid> descriptorByHost = new();
        private readonly Dictionary<Guid, BattlementGestureState> gestures = new();
        private readonly BattlementSharedLayoutRegistry sharedLayouts = new();
        private readonly Dictionary<Guid, ulong> controlledClocks = new();
        private readonly BattlementImperativePlaybacks imperativePlaybacks = new();
        private readonly Dictionary<Guid, BattlementMotionSequence> activeSequences = new();
        private readonly Dictionary<Guid, ActiveControl> activeControls = new();
        private readonly HashSet<Guid> installingControls = new();
        private readonly BattlementMotionReconnectState reconnect = new();
        private readonly List<MotionLifecycleEvent> events = new();
        private readonly List<(DescriptorState Descriptor, SlotState Slot)> pendingSamples = new();
        private readonly List<MotionGestureEvent> gestureEvents = new();
        private readonly List<MotionGestureEvent> pendingGestureSamples = new();
        private readonly List<MotionSequenceLabelEvent> labelEvents = new();
        private readonly List<MotionEffectOccurrence> effectOccurrences = new();
        private readonly Func<double> unscaledTime;
        private readonly Func<double> scaledTime;
        private readonly Func<ObjectId, MotionClockSample>? audioTime;
        private readonly Func<ObjectId, VisualElement?> resolveElement;
        private readonly Func<VisualElement, BattlementUiProjectionSpace> uiProjectionSpace;
        private readonly Func<TimeSpan> gestureTime;
        private readonly Func<bool> reducedMotion;
        private readonly System.Action? presentationChanged;
        private readonly BattlementMotionGraph graph;
        private readonly BattlementMotionPerformance performance = new();
        private readonly bool enablePlayerLoop;
        private readonly IBattlementUiAssetLookup? assets;
        private IBattlementMotionEffects? effects;
        private ulong sequence;
        private bool disposed;

        public BattlementMotionWorld(
            Func<double>? unscaledTime = null,
            Func<double>? scaledTime = null,
            bool registerPlayerLoop = true,
            IBattlementUiAssetLookup? assetLookup = null,
            Func<ObjectId, MotionClockSample>? audioTime = null,
            Func<ObjectId, VisualElement?>? resolveElement = null,
            Func<VisualElement, BattlementUiProjectionSpace>? uiProjectionSpace = null,
            Func<TimeSpan>? gestureTime = null,
            Func<bool>? reducedMotion = null,
            System.Action? presentationChanged = null
        )
        {
            this.unscaledTime = unscaledTime ?? (() => Time.unscaledTimeAsDouble);
            this.scaledTime = scaledTime ?? (() => Time.timeAsDouble);
            this.audioTime = audioTime;
            this.resolveElement = resolveElement ?? (_ => null);
            this.uiProjectionSpace = uiProjectionSpace ?? DefaultUiProjectionSpace;
            this.gestureTime =
                gestureTime ?? (() => TimeSpan.FromSeconds(Time.realtimeSinceStartupAsDouble));
            this.reducedMotion = reducedMotion ?? BattlementReducedMotion.Read;
            this.presentationChanged = presentationChanged;
            enablePlayerLoop = registerPlayerLoop;
            assets = assetLookup;
            graph = new BattlementMotionGraph(ClockSample, IsReduced);
        }

        public int DescriptorCount => descriptors.Count;

        public int GraphNodeCount => graph.NodeCount;

        public int LastGraphEvaluationCount => graph.LastEvaluationCount;

        public BattlementMotionPerformanceSnapshot Performance => performance.Snapshot;

        public IReadOnlyList<MotionEffectOccurrence> EffectOccurrences => effectOccurrences;

        internal int ActiveFiniteTimelineCount =>
            descriptors.Values.Sum(value => value.ActiveFiniteTimelineCount)
            + graph.ActiveFiniteTimelineCount;

        internal int ActiveInfiniteTimelineCount =>
            descriptors.Values.Sum(value => value.ActiveInfiniteTimelineCount)
            + graph.ActiveInfiniteTimelineCount;

        internal int ActiveHeldTimelineCount =>
            descriptors.Values.Sum(value => value.ActiveHeldTimelineCount);

        internal string ActiveTimelineDiagnostic =>
            string.Join(
                ";",
                descriptors
                    .Values.SelectMany(value => value.ActiveTimelineDiagnostics())
                    .Concat(graph.ActiveTimelineDiagnostics())
                    .Take(8)
            );

        internal int CompleteReadySlots() =>
            descriptors.Values.Sum(descriptor => descriptor.CompleteSlots(this));

        public void RecordPerformanceTraffic(int payloadBytes) =>
            performance.RecordTraffic(payloadBytes);

        internal void BindEffects(IBattlementMotionEffects value)
        {
            if (effects is not null)
                throw new InvalidOperationException("Motion effects are already bound.");
            effects = value;
        }

        internal void SetPseudoState(ObjectId descriptorId, MotionPseudoState state, bool value) =>
            descriptors[descriptorId.Value].SetPseudoState(state, value);

        public bool IsPlayerLoopRegistered { get; private set; }

        public BattlementPreparedMotionAdmission? Prepare(
            VisualElement target,
            ObjectId hostId,
            Prop<MotionDescriptor> motion,
            Prop<PaintStyle> paint = default
        )
        {
            ThrowIfDisposed();
            MotionDescriptor? nextMotion = motion.IsSet ? motion.Value : null;
            if (motion.IsUnset && descriptorByHost.TryGetValue(hostId.Value, out Guid currentId))
                nextMotion = descriptors[currentId].Descriptor;
            BattlementPaintAdmission.Validate(target, paint, nextMotion);
            if (motion.IsSet)
                BattlementMotionPropertyWriter.Configure(target, assets);
            return Prepare(BattlementUiMotionTarget.For(target), hostId, motion);
        }

        public BattlementPreparedMotionAdmission? Prepare(
            IBattlementMotionTarget target,
            ObjectId hostId,
            Prop<MotionDescriptor> motion
        )
        {
            ThrowIfDisposed();
            if (motion.IsUnset)
                return null;
            if (motion.IsReset)
                return new BattlementPreparedMotionAdmission(this, hostId.Value, null);
            MotionDescriptor descriptor = motion.Value;
            BattlementMotionValidator.Validate(descriptor, hostId);
            BattlementMotionDescriptorValidator.ValidateCapabilities(descriptor, target);
            BattlementMotionGraph.ValidateDescriptor(descriptor, target.Supports);
            graph.ValidateReplacement(descriptor);
            DescriptorState? previous = descriptors.TryGetValue(
                descriptor.DescriptorId.Value,
                out DescriptorState value
            )
                ? value
                : null;
            if (previous is not null && descriptor.Generation <= previous.Descriptor.Generation)
            {
                bool sameReconnectGeneration =
                    reconnect.Active && descriptor.Generation == previous.Descriptor.Generation;
                if (!sameReconnectGeneration)
                    throw Invalid("A motion descriptor update must advance its generation.");
            }
            if (
                descriptorByHost.TryGetValue(hostId.Value, out Guid existingId)
                && existingId != descriptor.DescriptorId.Value
            )
                throw Invalid("A host cannot own two motion descriptors.");

            bool retainedImperatives = false;
            if (previous is not null)
            {
                HashSet<ulong> declared = descriptor.Slots.Select(slot => slot.Slot).ToHashSet();
                MotionSlotDescriptor[] retained = previous
                    .Descriptor.Slots.Where(slot =>
                        slot.Slot >= ulong.MaxValue - 2048 && !declared.Contains(slot.Slot)
                    )
                    .ToArray();
                if (retained.Length != 0)
                {
                    descriptor = descriptor with
                    {
                        Slots = descriptor.Slots.Concat(retained).ToArray(),
                    };
                    retainedImperatives = true;
                }
            }

            IBattlementLayoutProjectionTarget? layoutTarget = descriptor.Layout is null
                ? null
                : LayoutTarget(target);
            var prepared = new DescriptorState(
                descriptor,
                target,
                ClockMicros(descriptor.Clock),
                previous,
                layoutTarget,
                layoutTarget is null
                    ? null
                    : sharedLayouts.Origin(descriptor, layoutTarget, previous, descriptors.Values),
                reconnect.Active,
                retainedImperatives
            );
            ValidateActiveControl(prepared);
            return new BattlementPreparedMotionAdmission(this, hostId.Value, prepared);
        }

        public void Install(VisualElement target, ObjectId hostId, Prop<MotionDescriptor> motion) =>
            Prepare(target, hostId, motion)?.Commit();

        public void SynchronizeStaticStyles(ObjectId hostId)
        {
            if (!descriptorByHost.TryGetValue(hostId.Value, out Guid descriptorId))
                return;
            if (descriptors.TryGetValue(descriptorId, out DescriptorState descriptor))
                descriptor.SynchronizeStaticStyles();
        }

        public void CommitPaint(ObjectId hostId)
        {
            if (!descriptorByHost.TryGetValue(hostId.Value, out Guid descriptorId))
                return;
            if (descriptors.TryGetValue(descriptorId, out DescriptorState descriptor))
                descriptor.CommitPaint();
        }

        public void RemoveHost(ObjectId hostId)
        {
            if (!descriptorByHost.Remove(hostId.Value, out Guid descriptorId))
                return;
            if (descriptors.Remove(descriptorId, out DescriptorState descriptor))
            {
                sharedLayouts.Remember(descriptor);
                if (gestures.Remove(descriptorId, out BattlementGestureState gesture))
                    gesture.Dispose();
                descriptor.Dispose();
                descriptor.Properties.Release();
                graph.Remove(descriptor.Descriptor.DescriptorId);
            }
        }

        public void Clear()
        {
            foreach (BattlementGestureState gesture in gestures.Values)
                gesture.Dispose();
            gestures.Clear();
            foreach (DescriptorState descriptor in descriptors.Values)
            {
                descriptor.Dispose();
                descriptor.Properties.Release();
            }
            descriptors.Clear();
            descriptorByHost.Clear();
            controlledClocks.Clear();
            imperativePlaybacks.Clear();
            DisposePreparedEffects(activeSequences.Values.SelectMany(value => value.Entries));
            activeSequences.Clear();
            activeControls.Clear();
            installingControls.Clear();
            reconnect.Clear();
            graph.Clear();
            events.Clear();
            pendingSamples.Clear();
            pendingGestureSamples.Clear();
            labelEvents.Clear();
            effectOccurrences.Clear();
            effects?.Reset();
            sharedLayouts.Clear();
            performance.Reset();
            if (IsPlayerLoopRegistered)
            {
                BattlementMotionPlayerLoop.Unregister(this);
                IsPlayerLoopRegistered = false;
            }
        }

        public void BeginReconnect()
        {
            ThrowIfDisposed();
            reconnect.Begin(descriptors.Keys);
        }

        public void EndReconnect()
        {
            foreach (Guid descriptorId in reconnect.Complete())
            {
                if (!descriptors.Remove(descriptorId, out DescriptorState descriptor))
                    continue;
                descriptorByHost.Remove(descriptor.Descriptor.HostId.Value);
                if (gestures.Remove(descriptorId, out BattlementGestureState gesture))
                    gesture.Dispose();
                descriptor.Dispose();
                descriptor.Properties.Release();
                graph.Remove(descriptor.Descriptor.DescriptorId);
            }
        }

        public void AbortReconnect() => Clear();

        public void AdvanceControlledClock(ObjectId clockId, ulong deltaMicros)
        {
            controlledClocks.TryGetValue(clockId.Value, out ulong current);
            controlledClocks[clockId.Value] = checked(current + deltaMicros);
        }

        public void SetControlledClock(ObjectId clockId, ulong elapsedMicros) =>
            controlledClocks[clockId.Value] = elapsedMicros;

        public void Apply(MotionValueOperation operation) => graph.Apply(operation);

        public IBattlementCommandOperation? ApplyValue(
            ObjectId valueId,
            MotionValueOperationKind kind,
            MotionValue? value,
            ObjectId playbackId,
            uint generation,
            TransitionDefinition? transition,
            bool blocking = false
        ) => graph.ApplyValue(valueId, kind, value, playbackId, generation, transition, blocking);

        public void Apply(MotionValuePlaybackOperation operation)
        {
            graph.Apply(operation);
            if (
                !imperativePlaybacks.TryGet(
                    operation.PlaybackId.Value,
                    out ImperativePlayback playback
                )
            )
                return;
            if (playback.Generation != operation.Generation)
                throw Invalid("The imperative playback generation is stale.");
            ApplySequencePlayback(operation.PlaybackId, operation.Generation, operation.Command);
            foreach (MotionPlaybackAddress address in playback.Addresses.ToArray())
                Apply(address, operation.Command);
            if (
                operation.Command
                is MotionPlaybackCommand.Stop
                    or MotionPlaybackCommand.Cancel
                    or MotionPlaybackCommand.Complete
            )
                FinishImperative(
                    operation.PlaybackId.Value,
                    operation.Command switch
                    {
                        MotionPlaybackCommand.Stop => MotionPlaybackOutcome.Stopped,
                        MotionPlaybackCommand.Cancel => MotionPlaybackOutcome.Cancelled,
                        _ => MotionPlaybackOutcome.Completed,
                    }
                );
        }

        public void ApplyValuePlayback(
            ObjectId playbackId,
            uint generation,
            MotionPlaybackOperationKind kind,
            ulong micros,
            double number,
            MotionPlaybackDirection direction
        )
        {
            graph.ApplyValuePlayback(playbackId, generation, kind, micros, number);
            if (!imperativePlaybacks.TryGet(playbackId.Value, out ImperativePlayback playback))
                return;
            if (playback.Generation != generation)
                throw Invalid("The imperative playback generation is stale.");
            ApplySequencePlayback(playbackId, generation, kind, number);
            foreach (MotionPlaybackAddress address in playback.Addresses.ToArray())
                ApplyPlayback(
                    address.DescriptorId,
                    address.Slot,
                    address.Generation,
                    kind,
                    micros,
                    number,
                    direction
                );
            MotionPlaybackOutcome? outcome = kind switch
            {
                MotionPlaybackOperationKind.Stop => MotionPlaybackOutcome.Stopped,
                MotionPlaybackOperationKind.Cancel => MotionPlaybackOutcome.Cancelled,
                MotionPlaybackOperationKind.Complete => MotionPlaybackOutcome.Completed,
                _ => null,
            };
            if (outcome is MotionPlaybackOutcome terminal)
                FinishImperative(playbackId.Value, terminal);
        }

        private void ApplySequencePlayback(
            ObjectId playbackId,
            uint generation,
            MotionPlaybackCommand command
        )
        {
            if (!activeSequences.TryGetValue(playbackId.Value, out var sequence))
                return;
            if (sequence.Generation != generation)
                throw Invalid("The Motion sequence generation is stale.");
            ulong now = ClockMicros(sequence.Clock);
            switch (command)
            {
                case MotionPlaybackCommand.Play:
                    sequence.Play(now);
                    break;
                case MotionPlaybackCommand.Pause:
                    sequence.Pause(now);
                    break;
                case MotionPlaybackCommand.SetSpeed speed:
                    if (!double.IsFinite(speed.Value) || speed.Value < 0)
                        throw Invalid("Motion playback speed must be finite and nonnegative.");
                    sequence.SetSpeed(now, speed.Value);
                    break;
                case MotionPlaybackCommand.Stop:
                case MotionPlaybackCommand.Cancel:
                    DisposePreparedEffects(sequence.Entries);
                    activeSequences.Remove(playbackId.Value);
                    break;
                case MotionPlaybackCommand.Complete:
                    sequence.Finish(now);
                    DisposePreparedEffects(sequence.Entries);
                    activeSequences.Remove(playbackId.Value);
                    break;
                default:
                    break;
            }
        }

        private void ApplySequencePlayback(
            ObjectId playbackId,
            uint generation,
            MotionPlaybackOperationKind kind,
            double number
        )
        {
            MotionPlaybackCommand? command = kind switch
            {
                MotionPlaybackOperationKind.Play => new MotionPlaybackCommand.Play(),
                MotionPlaybackOperationKind.Pause => new MotionPlaybackCommand.Pause(),
                MotionPlaybackOperationKind.Stop => new MotionPlaybackCommand.Stop(),
                MotionPlaybackOperationKind.Cancel => new MotionPlaybackCommand.Cancel(),
                MotionPlaybackOperationKind.Complete => new MotionPlaybackCommand.Complete(),
                MotionPlaybackOperationKind.SetSpeed => new MotionPlaybackCommand.SetSpeed(number),
                _ => null,
            };
            if (command is not null)
                ApplySequencePlayback(playbackId, generation, command);
        }

        public void Apply(MotionControlOperation operation)
        {
            switch (operation.Command)
            {
                case MotionControlCommand.Start start:
                    ApplyControl(
                        operation.ControlId,
                        MotionControlOperationKind.Start,
                        start.PlaybackId,
                        start.Generation,
                        start.Target
                    );
                    break;
                case MotionControlCommand.Set set:
                    ApplyControl(
                        operation.ControlId,
                        MotionControlOperationKind.Set,
                        default,
                        0,
                        set.Value
                    );
                    break;
                case MotionControlCommand.Stop:
                    ApplyControl(
                        operation.ControlId,
                        MotionControlOperationKind.Stop,
                        default,
                        0,
                        null
                    );
                    break;
                case MotionControlCommand.Clear:
                    ApplyControl(
                        operation.ControlId,
                        MotionControlOperationKind.Clear,
                        default,
                        0,
                        null
                    );
                    break;
                default:
                    throw Invalid("Unknown animation-controls operation.");
            }
        }

        public IBattlementCommandOperation? ApplyControl(
            ObjectId controlId,
            MotionControlOperationKind kind,
            ObjectId playbackId,
            uint generation,
            MotionControlTarget? target,
            bool blocking = false
        )
        {
            if (
                (kind is MotionControlOperationKind.Start or MotionControlOperationKind.Set)
                && target is null
            )
                throw Invalid("A motion control target is absent.");
            if (
                blocking
                && kind == MotionControlOperationKind.Start
                && ControlBindings(controlId).Length == 0
            )
                throw Invalid("Blocking Motion controls require a mounted target.");
            if (target is not null)
                foreach (DescriptorState binding in ControlBindings(controlId))
                    ValidateImperative(
                        binding,
                        BattlementMotionControlUtilities.Resolve(binding, target),
                        blocking
                    );
            switch (kind)
            {
                case MotionControlOperationKind.Start:
                    RemoveActiveControl(controlId.Value, false, MotionPlaybackOutcome.Cancelled);
                    var addresses = new List<MotionPlaybackAddress>();
                    var active = new ActiveControl(
                        playbackId,
                        generation,
                        target!,
                        addresses,
                        blocking
                    );
                    activeControls[controlId.Value] = active;
                    imperativePlaybacks.Register(playbackId, generation, addresses);
                    installingControls.Add(controlId.Value);
                    try
                    {
                        foreach (DescriptorState binding in ControlBindings(controlId))
                            addresses.Add(InstallActiveControl(binding, active));
                    }
                    finally
                    {
                        installingControls.Remove(controlId.Value);
                    }
                    break;
                case MotionControlOperationKind.Set:
                    RemoveActiveControl(controlId.Value, true, MotionPlaybackOutcome.Cancelled);
                    foreach (DescriptorState binding in ControlBindings(controlId))
                        BattlementMotionControlUtilities.ApplyImmediately(
                            binding,
                            BattlementMotionControlUtilities.Resolve(binding, target!)
                        );
                    break;
                case MotionControlOperationKind.Stop:
                    RemoveActiveControl(controlId.Value, false, MotionPlaybackOutcome.Stopped);
                    foreach (DescriptorState binding in ControlBindings(controlId))
                        StopImperative(binding);
                    break;
                case MotionControlOperationKind.Clear:
                    RemoveActiveControl(controlId.Value, false, MotionPlaybackOutcome.Cancelled);
                    foreach (DescriptorState binding in ControlBindings(controlId))
                        ClearImperative(binding);
                    break;
                default:
                    throw Invalid("Unknown animation-controls operation.");
            }
            return kind == MotionControlOperationKind.Start ? RunningOperation(playbackId) : null;
        }

        public void Apply(MotionScopeOperation operation)
        {
            DescriptorState? root = descriptors.Values.FirstOrDefault(value =>
                value.Descriptor.ScopeRoot && value.Descriptor.ScopeId == operation.ScopeId
            );
            if (root is null)
                return;
            switch (operation.Command)
            {
                case MotionScopeCommand.Start start:
                    StartSequence(root, start.PlaybackId, start.Generation, start.Entries, false);
                    break;
                case MotionScopeCommand.Set set:
                    foreach (
                        DescriptorState target in BattlementMotionControlUtilities.Select(
                            descriptors.Values,
                            root,
                            set.Selector
                        )
                    )
                        BattlementMotionControlUtilities.ApplyImmediately(target, set.Target);
                    break;
                case MotionScopeCommand.Stop stop:
                    foreach (
                        DescriptorState target in BattlementMotionControlUtilities.Select(
                            descriptors.Values,
                            root,
                            stop.Value
                        )
                    )
                        StopImperative(target);
                    break;
                default:
                    throw Invalid("Unknown animation-scope operation.");
            }
        }

        public IBattlementCommandOperation? ApplyScope(
            IBattlementMotionScopeView operation,
            bool blocking = false
        )
        {
            DescriptorState? root = descriptors.Values.FirstOrDefault(value =>
                value.Descriptor.ScopeRoot && value.Descriptor.ScopeId == operation.ScopeId
            );
            if (root is null)
                return null;
            switch (operation.Kind)
            {
                case MotionScopeOperationKind.Start:
                    var entries = new MotionSequenceEntry[operation.EntryCount];
                    for (int index = 0; index < entries.Length; index++)
                        entries[index] = operation.ReadEntry(index);
                    StartSequence(
                        root,
                        operation.PlaybackId,
                        operation.Generation,
                        entries,
                        blocking
                    );
                    return RunningOperation(operation.PlaybackId);
                case MotionScopeOperationKind.Set:
                    MotionSelector setSelector = operation.ReadSelector();
                    MotionTargetDescriptor setTarget = operation.ReadTarget();
                    foreach (
                        DescriptorState selectedTarget in BattlementMotionControlUtilities.Select(
                            descriptors.Values,
                            root,
                            setSelector
                        )
                    )
                        BattlementMotionControlUtilities.ApplyImmediately(
                            selectedTarget,
                            setTarget
                        );
                    break;
                case MotionScopeOperationKind.Stop:
                    foreach (
                        DescriptorState selectedTarget in BattlementMotionControlUtilities.Select(
                            descriptors.Values,
                            root,
                            operation.ReadSelector()
                        )
                    )
                        StopImperative(selectedTarget);
                    break;
                default:
                    throw Invalid("Unknown animation-scope operation.");
            }
            return null;
        }

        public void Apply(MotionDragControlOperation operation)
        {
            BattlementGestureState[] bindings = gestures
                .Values.Where(value => value.ControlId == operation.ControlId)
                .ToArray();
            if (bindings.Length > 1)
                throw Invalid("External drag controls are bound to more than one host.");
            if (bindings.Length == 1)
                bindings[0].StartExternal(operation);
        }

        public void ApplyDragControl(
            ObjectId controlId,
            int pointerId,
            MotionPointerDevice device,
            float x,
            float y,
            bool snapToCursor
        )
        {
            BattlementGestureState[] bindings = gestures
                .Values.Where(value => value.ControlId == controlId)
                .ToArray();
            if (bindings.Length > 1)
                throw Invalid("External drag controls are bound to more than one host.");
            if (bindings.Length == 1)
                bindings[0].StartExternal(pointerId, device, x, y, snapToCursor);
        }

        public void Apply(MotionPlaybackOperation operation)
        {
            switch (operation.Command)
            {
                case MotionPlaybackCommand.Play:
                    Play(operation.DescriptorId, operation.Slot, operation.Generation);
                    break;
                case MotionPlaybackCommand.Pause:
                    Pause(operation.DescriptorId, operation.Slot, operation.Generation);
                    break;
                case MotionPlaybackCommand.Replay:
                    Replay(operation.DescriptorId, operation.Slot, operation.Generation);
                    break;
                case MotionPlaybackCommand.Stop:
                    Stop(operation.DescriptorId, operation.Slot, operation.Generation);
                    break;
                case MotionPlaybackCommand.Cancel:
                    Cancel(operation.DescriptorId, operation.Slot, operation.Generation);
                    break;
                case MotionPlaybackCommand.Complete:
                    Complete(operation.DescriptorId, operation.Slot, operation.Generation);
                    break;
                case MotionPlaybackCommand.Seek seek:
                    Seek(
                        operation.DescriptorId,
                        operation.Slot,
                        operation.Generation,
                        seek.ElapsedMicros
                    );
                    break;
                case MotionPlaybackCommand.SetSpeed speed:
                    SetSpeed(
                        operation.DescriptorId,
                        operation.Slot,
                        operation.Generation,
                        speed.Value
                    );
                    break;
                case MotionPlaybackCommand.SetDirection direction:
                    SetDirection(
                        operation.DescriptorId,
                        operation.Slot,
                        operation.Generation,
                        direction.Value
                    );
                    break;
                default:
                    throw Invalid("Unknown motion playback operation.");
            }
        }

        public void ApplyPlayback(
            ObjectId descriptorId,
            ulong slot,
            uint generation,
            MotionPlaybackOperationKind kind,
            ulong micros,
            double number,
            MotionPlaybackDirection direction
        )
        {
            switch (kind)
            {
                case MotionPlaybackOperationKind.Play:
                    Play(descriptorId, slot, generation);
                    break;
                case MotionPlaybackOperationKind.Pause:
                    Pause(descriptorId, slot, generation);
                    break;
                case MotionPlaybackOperationKind.Replay:
                    Replay(descriptorId, slot, generation);
                    break;
                case MotionPlaybackOperationKind.Stop:
                    Stop(descriptorId, slot, generation);
                    break;
                case MotionPlaybackOperationKind.Cancel:
                    Cancel(descriptorId, slot, generation);
                    break;
                case MotionPlaybackOperationKind.Complete:
                    Complete(descriptorId, slot, generation);
                    break;
                case MotionPlaybackOperationKind.Seek:
                    Seek(descriptorId, slot, generation, micros);
                    break;
                case MotionPlaybackOperationKind.SetSpeed:
                    SetSpeed(descriptorId, slot, generation, number);
                    break;
                case MotionPlaybackOperationKind.SetDirection:
                    SetDirection(descriptorId, slot, generation, direction);
                    break;
                default:
                    throw Invalid("Unknown motion playback operation.");
            }
        }

        public void Apply(MotionControlledClockOperation operation)
        {
            switch (operation.Command)
            {
                case MotionControlledClockCommand.Set set:
                    SetControlledClock(operation.ClockId, set.ElapsedMicros);
                    break;
                case MotionControlledClockCommand.Advance advance:
                    AdvanceControlledClock(operation.ClockId, advance.DeltaMicros);
                    break;
                default:
                    throw Invalid("Unknown controlled-clock operation.");
            }
        }

        public void Play(ObjectId descriptorId, ulong slot, uint generation)
        {
            SlotState state = RequireSlot(descriptorId, slot, generation);
            if (state.Terminal)
                return;
            if (state.Paused)
            {
                state.AnchorMicros = ClockMicros(state.Clock);
                state.Paused = false;
            }
        }

        public void Pause(ObjectId descriptorId, ulong slot, uint generation)
        {
            SlotState state = RequireSlot(descriptorId, slot, generation);
            if (state.Terminal)
                return;
            if (!state.Paused)
            {
                state.HeldMicros = state.Elapsed(ClockMicros(state.Clock));
                state.Paused = true;
            }
        }

        public void Replay(ObjectId descriptorId, ulong slot, uint generation)
        {
            SlotState state = RequireSlot(descriptorId, slot, generation);
            if (state.Terminal)
                return;
            state.Reset(ClockMicros(state.Clock));
        }

        public void Seek(ObjectId descriptorId, ulong slot, uint generation, ulong elapsedMicros)
        {
            SlotState state = RequireSlot(descriptorId, slot, generation);
            if (state.Terminal)
                return;
            state.HeldMicros = elapsedMicros;
            state.Paused = true;
            state.SeekPending = true;
        }

        public void SetSpeed(ObjectId descriptorId, ulong slot, uint generation, double speed)
        {
            if (!double.IsFinite(speed) || speed < 0)
                throw Invalid("Motion playback speed must be finite and nonnegative.");
            SlotState state = RequireSlot(descriptorId, slot, generation);
            if (state.Terminal)
                return;
            ulong now = ClockMicros(state.Clock);
            ulong elapsed = state.Elapsed(now);
            state.Speed = speed;
            state.HeldMicros = elapsed;
            state.AnchorMicros = now;
            if (speed == 0)
                state.Paused = true;
        }

        public void SetDirection(
            ObjectId descriptorId,
            ulong slot,
            uint generation,
            MotionPlaybackDirection direction
        )
        {
            SlotState state = RequireSlot(descriptorId, slot, generation);
            if (state.Terminal)
                return;
            ulong now = ClockMicros(state.Clock);
            state.HeldMicros = state.Elapsed(now);
            state.AnchorMicros = now;
            state.Direction = direction;
        }

        public void Stop(ObjectId descriptorId, ulong slot, uint generation)
        {
            (DescriptorState descriptor, SlotState state) = RequireAddress(
                descriptorId,
                slot,
                generation
            );
            ulong elapsed = state.Elapsed(ClockMicros(state.Clock));
            descriptor.Stop(state, this, elapsed);
        }

        public void Cancel(ObjectId descriptorId, ulong slot, uint generation)
        {
            (DescriptorState descriptor, SlotState state) = RequireAddress(
                descriptorId,
                slot,
                generation
            );
            ulong elapsed = state.Elapsed(ClockMicros(state.Clock));
            descriptor.Cancel(state, this, elapsed);
        }

        public void Complete(ObjectId descriptorId, ulong slot, uint generation)
        {
            (DescriptorState descriptor, SlotState state) = RequireAddress(
                descriptorId,
                slot,
                generation
            );
            ulong elapsed = state.Elapsed(ClockMicros(state.Clock));
            descriptor.Complete(state, this, elapsed);
        }

        public IReadOnlyList<MotionLifecycleEvent> DrainEvents()
        {
            MotionLifecycleEvent[] drained = events.ToArray();
            events.Clear();
            return drained;
        }

        public IReadOnlyList<MotionPresentationSample> DrainSamples()
        {
            var samples = new MotionPresentationSample[pendingSamples.Count];
            for (int index = 0; index < samples.Length; index++)
            {
                (DescriptorState descriptor, SlotState slot) = pendingSamples[index];
                samples[index] = new MotionPresentationSample(
                    descriptor.Descriptor.DescriptorId,
                    slot.Definition.Slot,
                    slot.Definition.Generation,
                    slot.LastElapsedMicros,
                    slot.CaptureValues(descriptor.Properties)
                );
            }
            pendingSamples.Clear();
            return samples;
        }

        public MotionEventBatch? DrainEventBatch()
        {
            List<MotionLifecycleEvent> boundaries = new(events);
            IReadOnlyList<MotionPresentationSample> samples = DrainSamples();
            IReadOnlyList<MotionValueSample> valueSamples = graph.DrainSamples();
            var nativeGestures = new List<MotionGestureEvent>(gestureEvents);
            nativeGestures.AddRange(pendingGestureSamples);
            var terminalPlaybacks = new List<MotionPlaybackEvent>(
                imperativePlaybacks.DrainEvents()
            );
            terminalPlaybacks.AddRange(graph.DrainPlaybackEvents());
            events.Clear();
            gestureEvents.Clear();
            pendingGestureSamples.Clear();
            if (
                boundaries.Count == 0
                && samples.Count == 0
                && valueSamples.Count == 0
                && terminalPlaybacks.Count == 0
                && nativeGestures.Count == 0
                && labelEvents.Count == 0
            )
                return null;
            ulong first = boundaries.Count == 0 ? sequence : boundaries[0].Sequence;
            ulong last = boundaries.Count == 0 ? sequence : boundaries[^1].Sequence;
            var drained = new MotionEventBatch(
                first,
                last,
                boundaries,
                samples,
                valueSamples,
                terminalPlaybacks,
                nativeGestures,
                labelEvents.ToArray()
            );
            labelEvents.Clear();
            return drained;
        }

        public void PreLayout()
        {
            performance.BeginFrame(Time.realtimeSinceStartupAsDouble);
            graph.Sample();
            Sample(layout: true);
            foreach (DescriptorState descriptor in descriptors.Values)
                descriptor.CompleteSlots(this);
        }

        public void PostLayout()
        {
            foreach (DescriptorState descriptor in descriptors.Values)
                descriptor.CaptureLayoutTarget();
            ProgressSequences();
            Sample(layout: false);
            foreach (DescriptorState descriptor in descriptors.Values)
                descriptor.SampleLayout(
                    ClockMicros(descriptor.Descriptor.Clock),
                    IsReduced(descriptor.Descriptor)
                );
            presentationChanged?.Invoke();
            ProgressSequences();
            effects?.Advance();
            CompleteImperativePlaybacks();
            foreach (BattlementGestureState gesture in gestures.Values)
                gesture.Sample();
            performance.EndFrame(
                Time.realtimeSinceStartupAsDouble,
                descriptors.Values,
                graph.LastEvaluationCount
            );
        }

        public void Dispose()
        {
            if (disposed)
                return;
            Clear();
            effects?.Dispose();
            effects = null;
            if (IsPlayerLoopRegistered)
                BattlementMotionPlayerLoop.Unregister(this);
            disposed = true;
        }

        internal System.Action? PrepareStyle(ObjectId hostId, UiStyle? style)
        {
            if (style is null || !descriptorByHost.TryGetValue(hostId.Value, out Guid descriptorId))
                return null;
            return descriptors.TryGetValue(descriptorId, out DescriptorState descriptor)
                ? descriptor.PrepareStyle(style)
                : null;
        }

        internal void SetNativeGesture(ObjectId hostId, MotionLayer layer, bool active)
        {
            if (
                descriptorByHost.TryGetValue(hostId.Value, out Guid id)
                && descriptors.TryGetValue(id, out DescriptorState descriptor)
            )
                descriptor.SetGestureLayer(layer, active, ClockMicros(descriptor.Descriptor.Clock));
        }

        public void SetFocusVisible(ObjectId hostId, bool value)
        {
            if (!descriptorByHost.TryGetValue(hostId.Value, out Guid descriptorId))
                return;
            if (!descriptors.TryGetValue(descriptorId, out DescriptorState descriptor))
                return;
            descriptor.SetGestureLayer(
                MotionLayer.FocusVisible,
                value,
                ClockMicros(descriptor.Descriptor.Clock)
            );
            if (gestures.TryGetValue(descriptorId, out BattlementGestureState state))
                state.SetFocusVisible(value);
        }

        internal void Commit(Guid hostId, DescriptorState? prepared)
        {
            if (prepared is null)
            {
                RemoveHost(new ObjectId(hostId));
                return;
            }
            if (
                descriptors.TryGetValue(
                    prepared.Descriptor.DescriptorId.Value,
                    out DescriptorState previous
                )
            )
            {
                if (!reconnect.Active)
                    previous.CancelActiveSlots(
                        this,
                        ClockMicros(previous.Descriptor.Clock),
                        prepared
                    );
                if (
                    gestures.Remove(
                        prepared.Descriptor.DescriptorId.Value,
                        out BattlementGestureState gesture
                    )
                )
                    gesture.Dispose();
                previous.Dispose();
            }
            descriptors[prepared.Descriptor.DescriptorId.Value] = prepared;
            reconnect.Restored(prepared.Descriptor.DescriptorId.Value);
            descriptorByHost[hostId] = prepared.Descriptor.DescriptorId.Value;
            graph.Replace(prepared.Descriptor, prepared.Properties);
            EnsurePlayerLoop();
            prepared.SynchronizeStaticStyles();
            if (reconnect.Active)
                prepared.ApplyReconnectPresentation();
            else
                prepared.ApplyInitialPresentation(IsReduced(prepared.Descriptor));
            if (!reconnect.Active)
                prepared.EmitActivated(this);
            if (prepared.Descriptor.Gestures is not null && prepared.Element is not null)
            {
                Guid descriptorId = prepared.Descriptor.DescriptorId.Value;
                gestures[descriptorId] = new BattlementGestureState(
                    prepared.Descriptor,
                    prepared.Target,
                    resolveElement,
                    gestureTime,
                    () => IsReduced(prepared.Descriptor),
                    (layer, value) =>
                        prepared.SetGestureLayer(
                            layer,
                            value,
                            ClockMicros(prepared.Descriptor.Clock)
                        ),
                    (valueId, value) => graph.SetLocal(valueId, value),
                    EmitGesture
                );
            }
            AttachActiveControl(prepared);
        }

        internal IBattlementCommandOperation? DescriptorOperation(
            ObjectId hostId,
            bool includeTimelines
        )
        {
            DescriptorState? initial = CurrentDescriptor(hostId);
            if (initial is null)
                return null;
            bool layout = initial.LayoutProjection is not null;
            var finite = new HashSet<ulong>(
                includeTimelines ? initial.PendingFiniteSlotIds() : Array.Empty<ulong>()
            );
            var infinite = new HashSet<ulong>(
                includeTimelines && !layout && finite.Count == 0
                    ? initial.PendingInfiniteSlotIds()
                    : Array.Empty<ulong>()
            );
            if (!layout && finite.Count == 0 && infinite.Count == 0)
                return null;
            return new RunningDescriptorMotion(
                () =>
                {
                    DescriptorState? current = CurrentDescriptor(hostId);
                    return current is null
                        ? (true, false)
                        : (
                            (!layout || current.IsLayoutProjectionComplete)
                                && !current.HasPendingFiniteSlot(finite)
                                && !current.HasPendingInfiniteSlot(infinite),
                            (layout && current.IsLayoutProjectionInfinite)
                                || current.HasPendingInfiniteSlot(infinite)
                        );
                },
                () =>
                {
                    DescriptorState? current = CurrentDescriptor(hostId);
                    current?.CancelSlots(
                        finite.Count == 0 ? infinite : finite,
                        this,
                        current is null ? 0 : ClockMicros(current.Descriptor.Clock)
                    );
                    if (layout)
                        current?.LayoutProjection?.Release();
                }
            );
        }

        private DescriptorState? CurrentDescriptor(ObjectId hostId)
        {
            if (!descriptorByHost.TryGetValue(hostId.Value, out Guid descriptorId))
                return null;
            if (!descriptors.TryGetValue(descriptorId, out DescriptorState descriptor))
                return null;
            return descriptor;
        }

        private void EmitGesture(MotionGestureEvent value, bool replaceable)
        {
            if (!replaceable)
            {
                gestureEvents.Add(value);
                return;
            }
            int index = pendingGestureSamples.FindIndex(existing =>
                existing.DescriptorId == value.DescriptorId && existing.Kind == value.Kind
            );
            if (index < 0)
                pendingGestureSamples.Add(value);
            else
                pendingGestureSamples[index] = value;
        }

        private DescriptorState[] ControlBindings(ObjectId controlId) =>
            descriptors.Values.Where(value => value.Descriptor.ControlId == controlId).ToArray();

        private void ValidateActiveControl(DescriptorState descriptor)
        {
            if (
                descriptor.Descriptor.ControlId is ObjectId controlId
                && activeControls.TryGetValue(controlId.Value, out ActiveControl active)
            )
                ValidateImperative(
                    descriptor,
                    BattlementMotionControlUtilities.Resolve(descriptor, active.Target),
                    active.Blocking
                );
        }

        private void AttachActiveControl(DescriptorState descriptor)
        {
            if (
                descriptor.Descriptor.ControlId is not ObjectId controlId
                || installingControls.Contains(controlId.Value)
                || !activeControls.TryGetValue(controlId.Value, out ActiveControl active)
            )
                return;
            ValidateActiveControl(descriptor);
            installingControls.Add(controlId.Value);
            try
            {
                active.Addresses.Add(InstallActiveControl(descriptor, active));
            }
            finally
            {
                installingControls.Remove(controlId.Value);
            }
        }

        private MotionPlaybackAddress InstallActiveControl(
            DescriptorState descriptor,
            ActiveControl active
        ) =>
            InstallImperative(
                descriptor,
                BattlementMotionControlUtilities.Resolve(descriptor, active.Target),
                active.Generation,
                0
            );

        private void RemoveActiveControl(
            Guid controlId,
            bool clearSlots,
            MotionPlaybackOutcome outcome
        )
        {
            if (!activeControls.Remove(controlId, out ActiveControl active))
                return;
            FinishImperative(active.PlaybackId.Value, outcome);
            if (!clearSlots)
                return;
            foreach (DescriptorState binding in descriptors.Values.ToArray())
                if (binding.Descriptor.ControlId?.Value == controlId)
                    ClearImperative(binding);
        }

        private void CompleteImperativePlaybacks()
        {
            IReadOnlyList<Guid> completed = imperativePlaybacks.Complete(
                descriptors,
                activeSequences.Keys
            );
            for (int index = 0; index < completed.Count; index++)
                ForgetActiveControl(completed[index]);
        }

        private void FinishImperative(Guid id, MotionPlaybackOutcome outcome)
        {
            if (activeSequences.Remove(id, out BattlementMotionSequence sequence))
                DisposePreparedEffects(sequence.Entries);
            if (!imperativePlaybacks.Finish(id, outcome))
                return;
            ForgetActiveControl(id);
        }

        private void ForgetActiveControl(Guid id)
        {
            foreach ((Guid controlId, ActiveControl control) in activeControls.ToArray())
                if (control.PlaybackId.Value == id)
                    activeControls.Remove(controlId);
        }

        internal void EnsurePlayerLoop()
        {
            if (!enablePlayerLoop || IsPlayerLoopRegistered)
                return;
            BattlementMotionPlayerLoop.Register(this);
            IsPlayerLoopRegistered = true;
        }

        private void Sample(bool layout)
        {
            ThrowIfDisposed();
            List<(DescriptorState Descriptor, Exception Failure)>? failures = null;
            foreach (DescriptorState descriptor in descriptors.Values)
            {
                try
                {
                    descriptor.Sample(
                        ClockMicros(descriptor.Descriptor.Clock),
                        layout,
                        IsReduced(descriptor.Descriptor),
                        this
                    );
                }
                catch (Exception failure)
                {
                    if (
                        !imperativePlaybacks.OwnsDescriptor(
                            descriptor.Descriptor.DescriptorId.Value
                        )
                    )
                        throw;
                    pendingSamples.RemoveAll(sample =>
                        ReferenceEquals(sample.Descriptor, descriptor)
                    );
                    (failures ??= new()).Add((descriptor, failure));
                }
            }
            if (failures is null)
                return;
            foreach ((DescriptorState descriptor, Exception failure) in failures)
            {
                foreach (
                    Guid playback in imperativePlaybacks.FailDescriptor(
                        descriptor.Descriptor.DescriptorId.Value,
                        failure,
                        CancelAddress
                    )
                )
                    ForgetActiveControl(playback);
                RemoveHost(descriptor.Descriptor.HostId);
            }
        }

        private void CancelAddress(MotionPlaybackAddress address)
        {
            if (
                descriptors.TryGetValue(address.DescriptorId.Value, out DescriptorState descriptor)
                && descriptor.FindSlot(address.Slot)?.Definition.Generation == address.Generation
            )
                Cancel(address.DescriptorId, address.Slot, address.Generation);
        }

        internal IBattlementCommandOperation? RunningOperation(ObjectId playbackId) =>
            imperativePlaybacks.Operation(
                playbackId,
                descriptors,
                CompleteImperativePlaybacks,
                playback =>
                {
                    foreach (MotionPlaybackAddress address in playback.Addresses)
                        if (
                            descriptors.TryGetValue(
                                address.DescriptorId.Value,
                                out DescriptorState descriptor
                            )
                            && descriptor.FindSlot(address.Slot)?.Definition.Generation
                                == address.Generation
                        )
                            Cancel(address.DescriptorId, address.Slot, address.Generation);
                    FinishImperative(playbackId.Value, MotionPlaybackOutcome.Cancelled);
                }
            );

        private static void ValidateImperative(
            DescriptorState descriptor,
            MotionTargetDescriptor target,
            bool blocking
        )
        {
            var proposed = descriptor.Descriptor with
            {
                Slots = new[]
                {
                    new MotionSlotDescriptor(
                        1,
                        1,
                        MotionLayer.Animate,
                        target,
                        new MotionCallbackSubscriptions(false, false, false, false, false, false)
                    ),
                },
            };
            BattlementMotionValidator.Validate(proposed, proposed.HostId);
            BattlementMotionDescriptorValidator.ValidateCapabilities(
                proposed,
                descriptor.Properties
            );
            if (
                blocking
                && target.Tracks.Any(track => track.Transition.Repeat is MotionRepeat.Forever)
            )
                throw Invalid("An infinite Motion playback must be nonblocking.");
        }

        private void StartSequence(
            DescriptorState root,
            ObjectId playbackId,
            uint generation,
            IReadOnlyList<MotionSequenceEntry> definitions,
            bool blocking
        )
        {
            ValidateSequenceGraph(definitions);
            var entries = new List<MotionSequenceEntryState>(definitions.Count);
            try
            {
                for (int index = 0; index < definitions.Count; index++)
                {
                    MotionSequenceEntry definition = definitions[index];
                    if (definition is MotionSequenceEntry.Sound or MotionSequenceEntry.Particle)
                    {
                        IBattlementMotionEffects service =
                            effects ?? throw Invalid("Motion effects are unavailable.");
                        IBattlementPreparedMotionEffect prepared = service.Prepare(definition);
                        try
                        {
                            UnityEngine.Vector3? capturedPosition = null;
                            if (definition is MotionSequenceEntry.Particle particle)
                            {
                                UnityEngine.Vector3 position = service.Resolve(
                                    particle.Occurrence.Position
                                );
                                if (
                                    particle.Occurrence.Position.Resolution
                                    == MotionReferenceResolution.CaptureAtStart
                                )
                                    capturedPosition = position;
                            }
                            entries.Add(
                                new MotionSequenceEntryState(
                                    definition,
                                    Array.Empty<Guid>(),
                                    new Dictionary<Guid, IReadOnlyList<MotionPropertyValue>>(),
                                    prepared,
                                    capturedPosition
                                )
                            );
                        }
                        catch
                        {
                            prepared.Dispose();
                            throw;
                        }
                        continue;
                    }
                    if (definition is not MotionSequenceEntry.Animate animation)
                    {
                        entries.Add(
                            new MotionSequenceEntryState(
                                definition,
                                Array.Empty<Guid>(),
                                new Dictionary<Guid, IReadOnlyList<MotionPropertyValue>>()
                            )
                        );
                        continue;
                    }
                    DescriptorState[] selected = BattlementMotionControlUtilities
                        .Select(descriptors.Values, root, animation.Selector)
                        .ToArray();
                    if (selected.Length == 0)
                        throw Invalid("A Motion sequence target does not exist.");
                    var captured = new Dictionary<Guid, IReadOnlyList<MotionPropertyValue>>();
                    foreach (DescriptorState target in selected)
                    {
                        MotionTargetDescriptor resolved = ResolveSequenceTarget(
                            target,
                            animation,
                            capture: true,
                            captured
                        );
                        ValidateImperative(target, resolved, blocking);
                    }
                    entries.Add(
                        new MotionSequenceEntryState(
                            definition,
                            selected.Select(value => value.Descriptor.DescriptorId.Value).ToArray(),
                            captured
                        )
                    );
                }
                ValidateSequenceConflicts(definitions, entries);
            }
            catch
            {
                DisposePreparedEffects(entries);
                throw;
            }
            var addresses = new List<MotionPlaybackAddress>();
            imperativePlaybacks.Register(playbackId, generation, addresses);
            var sequence = new BattlementMotionSequence(
                playbackId,
                generation,
                root.Descriptor.Clock,
                ClockMicros(root.Descriptor.Clock),
                entries
            );
            activeSequences[playbackId.Value] = sequence;
            ProgressSequence(sequence);
        }

        private MotionTargetDescriptor ResolveSequenceTarget(
            DescriptorState target,
            MotionSequenceEntry.Animate animation,
            bool capture,
            IDictionary<Guid, IReadOnlyList<MotionPropertyValue>> captured
        )
        {
            if (animation.Position is not MotionPositionReference position)
                return animation.Target;
            IReadOnlyList<MotionPropertyValue> values;
            if (
                position.Resolution == MotionReferenceResolution.CaptureAtStart
                && captured.TryGetValue(target.Descriptor.DescriptorId.Value, out var retained)
            )
                values = retained;
            else
            {
                if (
                    !descriptorByHost.TryGetValue(position.ObjectId.Value, out Guid referenceId)
                    || !descriptors.TryGetValue(referenceId, out DescriptorState reference)
                )
                    throw Invalid("A Motion sequence position reference does not exist.");
                try
                {
                    values = target.Properties.ResolvePosition(
                        reference.Properties,
                        position.Anchor
                    );
                }
                catch (Exception failure)
                {
                    throw Invalid(failure.Message);
                }
                if (capture && position.Resolution == MotionReferenceResolution.CaptureAtStart)
                    captured[target.Descriptor.DescriptorId.Value] = values;
            }
            MotionPropertyTrack[] tracks = animation
                .Target.Tracks.Concat(
                    values.Select(value => new MotionPropertyTrack(
                        value.Property,
                        new[] { value.Value },
                        animation.PositionTransition
                    ))
                )
                .ToArray();
            return animation.Target with { Tracks = tracks };
        }

        private void ProgressSequences()
        {
            foreach (BattlementMotionSequence sequence in activeSequences.Values.ToArray())
                ProgressSequence(sequence);
        }

        private void ProgressSequence(BattlementMotionSequence sequence)
        {
            ulong now = ClockMicros(sequence.Clock);
            MotionPlaybackOutcome? interrupted = SequenceTerminalOutcome(sequence);
            if (interrupted is MotionPlaybackOutcome outcome)
            {
                activeSequences.Remove(sequence.PlaybackId.Value);
                DisposePreparedEffects(sequence.Entries);
                FinishImperative(sequence.PlaybackId.Value, outcome);
                ClearSequenceImperatives(sequence);
                return;
            }
            foreach (MotionSequenceEntryState entry in sequence.Entries)
            {
                if (
                    entry.StartedAt is null
                    || entry.CompletedAt is not null
                    || entry.Definition is not MotionSequenceEntry.Animate animation
                    || animation.Position?.Resolution != MotionReferenceResolution.Follow
                )
                    continue;
                foreach (MotionPlaybackAddress address in entry.Addresses)
                    if (
                        descriptors.TryGetValue(address.DescriptorId.Value, out var descriptor)
                        && descriptor.FindSlot(address.Slot) is SlotState slot
                        && slot.Definition.Generation == address.Generation
                    )
                        slot.RetargetPosition(
                            ResolveSequencePosition(descriptor, animation.Position),
                            ClockMicros(slot.Clock)
                        );
            }
            bool finished = sequence.Progress(
                now,
                (index, entry) => StartSequenceEntry(sequence, index, entry),
                SequenceEntryTerminal,
                label =>
                    labelEvents.Add(
                        new MotionSequenceLabelEvent(
                            sequence.PlaybackId,
                            sequence.Generation,
                            label
                        )
                    )
            );
            if (finished)
            {
                activeSequences.Remove(sequence.PlaybackId.Value);
                DisposePreparedEffects(sequence.Entries);
                FinishImperative(sequence.PlaybackId.Value, MotionPlaybackOutcome.Completed);
                ClearSequenceImperatives(sequence);
            }
        }

        private void ClearSequenceImperatives(BattlementMotionSequence sequence)
        {
            MotionPlaybackAddress[] addresses = sequence
                .Entries.SelectMany(entry => entry.Addresses)
                .Distinct()
                .ToArray();
            foreach (
                IGrouping<Guid, MotionPlaybackAddress> group in addresses.GroupBy(address =>
                    address.DescriptorId.Value
                )
            )
            {
                if (!descriptors.TryGetValue(group.Key, out DescriptorState descriptor))
                    continue;
                HashSet<(ulong Slot, uint Generation)> remove = group
                    .Select(address => (address.Slot, address.Generation))
                    .ToHashSet();
                HashSet<MotionProperty> released = descriptor
                    .Descriptor.Slots.Where(slot => remove.Contains((slot.Slot, slot.Generation)))
                    .SelectMany(slot =>
                        slot.Target.Tracks.Select(track => track.Property)
                            .Concat(slot.Target.TransitionEnd.Select(value => value.Property))
                    )
                    .ToHashSet();
                ulong now = ClockMicros(descriptor.Descriptor.Clock);
                foreach (
                    MotionSlotDescriptor slot in descriptor.Descriptor.Slots.Where(slot =>
                        slot.Slot < ulong.MaxValue - 2048
                        && slot.Target.Tracks.Any(track => released.Contains(track.Property))
                    )
                )
                    descriptor.FindSlot(slot.Slot)?.RetargetFromPresentation(now);
                MotionSlotDescriptor[] slots = descriptor
                    .Descriptor.Slots.Where(slot => !remove.Contains((slot.Slot, slot.Generation)))
                    .ToArray();
                if (slots.Length == descriptor.Descriptor.Slots.Count)
                    continue;
                MotionDescriptor updated = descriptor.Descriptor with { Slots = slots };
                Commit(
                    updated.HostId.Value,
                    new DescriptorState(
                        updated,
                        descriptor.Properties,
                        ClockMicros(updated.Clock),
                        descriptor,
                        retainUnchangedSlots: true
                    )
                );
            }
        }

        private bool StartSequenceEntry(
            BattlementMotionSequence sequence,
            int index,
            MotionSequenceEntryState entry
        )
        {
            if (entry.Definition is MotionSequenceEntry.Sound or MotionSequenceEntry.Particle)
            {
                IBattlementPreparedMotionEffect prepared =
                    entry.PreparedEffect ?? throw Invalid("A Motion effect was not prepared.");
                (effects ?? throw Invalid("Motion effects are unavailable.")).Start(
                    sequence.PlaybackId,
                    index,
                    entry.Definition,
                    prepared,
                    entry.CapturedEffectPosition
                );
                if (effectOccurrences.Count == 256)
                    effectOccurrences.RemoveAt(0);
                effectOccurrences.Add(
                    new MotionEffectOccurrence(
                        sequence.PlaybackId,
                        checked((uint)index),
                        entry.Definition is MotionSequenceEntry.Sound
                            ? MotionEffectOccurrenceKind.Sound
                            : MotionEffectOccurrenceKind.Particle,
                        entry.Definition switch
                        {
                            MotionSequenceEntry.Sound value => value.Occurrence.Address,
                            MotionSequenceEntry.Particle value => value.Occurrence.Address,
                            _ => throw Invalid("Unknown Motion effect occurrence."),
                        }
                    )
                );
                return true;
            }
            var animation = (MotionSequenceEntry.Animate)entry.Definition;
            foreach (Guid targetId in entry.Targets)
            {
                if (!descriptors.TryGetValue(targetId, out DescriptorState descriptor))
                    throw Invalid("A Motion sequence target was removed before it started.");
                var captured =
                    (IDictionary<Guid, IReadOnlyList<MotionPropertyValue>>)entry.CapturedPositions;
                MotionTargetDescriptor target = ResolveSequenceTarget(
                    descriptor,
                    animation,
                    capture: false,
                    captured
                );
                MotionPlaybackAddress address = InstallSequenceImperative(
                    descriptor,
                    target,
                    sequence.Generation,
                    (ulong)index,
                    out var remaps
                );
                RemapSequenceAddresses(sequence, remaps);
                entry.Addresses.Add(address);
                if (imperativePlaybacks.TryGet(sequence.PlaybackId.Value, out var playback))
                {
                    playback.Addresses.RemoveAll(existing => !AddressExists(existing));
                    if (!playback.Addresses.Contains(address))
                        playback.Addresses.Add(address);
                }
            }
            return entry.Addresses.Count == 0;
        }

        private void RemapSequenceAddresses(
            BattlementMotionSequence sequence,
            IReadOnlyList<(MotionPlaybackAddress Old, MotionPlaybackAddress New)> remaps
        )
        {
            foreach ((MotionPlaybackAddress old, MotionPlaybackAddress replacement) in remaps)
            foreach (MotionSequenceEntryState entry in sequence.Entries)
                for (int index = 0; index < entry.Addresses.Count; index++)
                    if (entry.Addresses[index] == old)
                        entry.Addresses[index] = replacement;
            if (!imperativePlaybacks.TryGet(sequence.PlaybackId.Value, out var playback))
                return;
            foreach ((MotionPlaybackAddress old, MotionPlaybackAddress replacement) in remaps)
                for (int index = 0; index < playback.Addresses.Count; index++)
                    if (playback.Addresses[index] == old)
                        playback.Addresses[index] = replacement;
        }

        private IReadOnlyList<MotionPropertyValue> ResolveSequencePosition(
            DescriptorState target,
            MotionPositionReference position
        )
        {
            if (
                !descriptorByHost.TryGetValue(position.ObjectId.Value, out Guid referenceId)
                || !descriptors.TryGetValue(referenceId, out DescriptorState reference)
            )
                throw Invalid("A Motion sequence position reference does not exist.");
            return target.Properties.ResolvePosition(reference.Properties, position.Anchor);
        }

        private bool SequenceEntryTerminal(MotionSequenceEntryState entry)
        {
            if (entry.Addresses.Count == 0)
                return false;
            foreach (MotionPlaybackAddress address in entry.Addresses)
            {
                if (!descriptors.TryGetValue(address.DescriptorId.Value, out var descriptor))
                    throw Invalid("A Motion sequence target was removed while running.");
                if (
                    descriptor.FindSlot(address.Slot) is not SlotState slot
                    || slot.Definition.Generation != address.Generation
                )
                    continue;
                if (!slot.Terminal)
                    return false;
                if (slot.Outcome is not MotionPlaybackOutcome.Completed)
                    return false;
            }
            return true;
        }

        private bool AddressExists(MotionPlaybackAddress address) =>
            descriptors.TryGetValue(address.DescriptorId.Value, out var descriptor)
            && descriptor.FindSlot(address.Slot)?.Definition.Generation == address.Generation;

        private static void DisposePreparedEffects(IEnumerable<MotionSequenceEntryState> entries)
        {
            foreach (MotionSequenceEntryState entry in entries)
                entry.PreparedEffect?.Dispose();
        }

        private MotionPlaybackOutcome? SequenceTerminalOutcome(BattlementMotionSequence sequence)
        {
            foreach (MotionSequenceEntryState entry in sequence.Entries)
            foreach (MotionPlaybackAddress address in entry.Addresses)
                if (
                    descriptors.TryGetValue(address.DescriptorId.Value, out var descriptor)
                    && descriptor.FindSlot(address.Slot) is SlotState slot
                    && slot.Definition.Generation == address.Generation
                    && slot.Outcome
                        is MotionPlaybackOutcome.Stopped
                            or MotionPlaybackOutcome.Cancelled
                            or MotionPlaybackOutcome.Failed
                )
                    return slot.Outcome;
            return null;
        }

        private static void ValidateSequenceGraph(IReadOnlyList<MotionSequenceEntry> entries)
        {
            var labels = new Dictionary<string, int>();
            for (int index = 0; index < entries.Count; index++)
                if (entries[index] is MotionSequenceEntry.Label label)
                {
                    if (string.IsNullOrWhiteSpace(label.Name) || !labels.TryAdd(label.Name, index))
                        throw Invalid("Motion sequence labels must be nonempty and unique.");
                }
            var states = new byte[entries.Count];
            for (int index = 0; index < entries.Count; index++)
                Visit(index);

            void Visit(int index)
            {
                if (states[index] == 2)
                    return;
                if (states[index] == 1)
                    throw Invalid("Motion sequence dependencies must be acyclic.");
                states[index] = 1;
                MotionSequenceSchedule schedule = EntrySchedule(entries[index]);
                int? dependency = schedule switch
                {
                    MotionSequenceSchedule.RelativeStart relative => Checked(relative.Entry),
                    MotionSequenceSchedule.AfterCompletion completion => Checked(completion.Entry),
                    MotionSequenceSchedule.Label named => labels.TryGetValue(
                        named.Name,
                        out int label
                    )
                        ? label
                        : throw Invalid("Motion sequence references a missing label."),
                    _ => null,
                };
                if (
                    schedule is MotionSequenceSchedule.AfterCompletion
                    && dependency is int completed
                    && entries[completed] is MotionSequenceEntry.Animate animation
                    && (
                        animation.Target.Tracks.Any(track =>
                            track.Transition.Repeat is MotionRepeat.Forever
                        )
                        || (
                            animation.Position is not null
                            && animation.PositionTransition.Repeat is MotionRepeat.Forever
                        )
                    )
                )
                    throw Invalid("A Motion sequence cannot wait for an infinite entry.");
                if (dependency is int value)
                    Visit(value);
                states[index] = 2;
            }

            int Checked(uint value) =>
                value < entries.Count
                    ? (int)value
                    : throw Invalid("Motion sequence references a missing entry.");
        }

        private static MotionSequenceSchedule EntrySchedule(MotionSequenceEntry entry) =>
            entry switch
            {
                MotionSequenceEntry.Animate value => value.Schedule,
                MotionSequenceEntry.Label value => value.Schedule,
                MotionSequenceEntry.Sound value => value.Schedule,
                MotionSequenceEntry.Particle value => value.Schedule,
                _ => throw Invalid("Unknown Motion sequence entry."),
            };

        private static void ValidateSequenceConflicts(
            IReadOnlyList<MotionSequenceEntry> definitions,
            IReadOnlyList<MotionSequenceEntryState> states
        )
        {
            for (int later = 0; later < definitions.Count; later++)
            {
                if (definitions[later] is not MotionSequenceEntry.Animate next)
                    continue;
                for (int earlier = 0; earlier < later; earlier++)
                {
                    if (definitions[earlier] is not MotionSequenceEntry.Animate prior)
                        continue;
                    if (!states[later].Targets.Intersect(states[earlier].Targets).Any())
                        continue;
                    bool overlaps = SequenceClaims(next).Intersect(SequenceClaims(prior)).Any();
                    if (
                        overlaps
                        && next.Conflict != MotionSequenceConflict.Replace
                        && !DependsOnCompletion(definitions, later, earlier, new HashSet<int>())
                    )
                        throw Invalid(
                            "Overlapping Motion sequence property writes require replacement."
                        );
                }
            }
        }

        private static IEnumerable<(MotionProperty, MotionPropertyTarget)> SequenceClaims(
            MotionSequenceEntry.Animate animation
        )
        {
            foreach (MotionPropertyTrack track in animation.Target.Tracks)
                yield return (track.Property, track.Target);
            foreach (MotionPropertyValue value in animation.Target.TransitionEnd)
                yield return (value.Property, new MotionPropertyTarget.Host());
            if (animation.Position is null)
                yield break;
            foreach (
                MotionProperty property in new[]
                {
                    MotionProperty.X,
                    MotionProperty.Y,
                    MotionProperty.Z,
                    MotionProperty.LocalPositionX,
                    MotionProperty.LocalPositionY,
                    MotionProperty.LocalPositionZ,
                }
            )
                yield return (property, new MotionPropertyTarget.Host());
        }

        private static bool DependsOnCompletion(
            IReadOnlyList<MotionSequenceEntry> entries,
            int current,
            int expected,
            HashSet<int> visited
        )
        {
            if (!visited.Add(current))
                return false;
            MotionSequenceSchedule schedule = EntrySchedule(entries[current]);
            if (schedule is MotionSequenceSchedule.AfterCompletion completion)
                return completion.Entry == expected
                    || DependsOnCompletion(entries, (int)completion.Entry, expected, visited);
            if (schedule is MotionSequenceSchedule.RelativeStart relative)
                return DependsOnCompletion(entries, (int)relative.Entry, expected, visited);
            if (schedule is MotionSequenceSchedule.Label label)
            {
                int index = -1;
                for (int candidate = 0; candidate < entries.Count; candidate++)
                    if (
                        entries[candidate] is MotionSequenceEntry.Label value
                        && value.Name == label.Name
                    )
                    {
                        index = candidate;
                        break;
                    }
                return index >= 0 && DependsOnCompletion(entries, index, expected, visited);
            }
            return false;
        }

        private bool IsReduced(MotionDescriptor descriptor) =>
            descriptor.ReducedMotion switch
            {
                ReducedMotionPolicy.Always => true,
                ReducedMotionPolicy.Never => false,
                ReducedMotionPolicy.User => reducedMotion(),
                _ => throw Invalid("Unknown reduced-motion policy."),
            };

        private ulong ClockMicros(MotionClockSource source) => ClockSample(source).ElapsedMicros;

        private MotionClockSample ClockSample(MotionClockSource source) =>
            BattlementMotionClockSampler.Sample(
                source,
                unscaledTime,
                scaledTime,
                controlledClocks,
                audioTime
            );

        private SlotState RequireSlot(ObjectId descriptorId, ulong slot, uint generation)
        {
            return RequireAddress(descriptorId, slot, generation).Slot;
        }

        private (DescriptorState Descriptor, SlotState Slot) RequireAddress(
            ObjectId descriptorId,
            ulong slot,
            uint generation
        )
        {
            if (!descriptors.TryGetValue(descriptorId.Value, out DescriptorState descriptor))
                throw Invalid("The motion descriptor does not exist.");
            SlotState? state = descriptor.FindSlot(slot);
            if (state is null || state.Definition.Generation != generation)
                throw Invalid("The motion slot generation is stale.");
            return (descriptor, state);
        }

        internal void Emit(
            DescriptorState descriptor,
            SlotState slot,
            MotionEventKind kind,
            ulong at
        )
        {
            events.Add(
                new MotionLifecycleEvent(
                    ++sequence,
                    descriptor.Descriptor.DescriptorId,
                    slot.Definition.Slot,
                    slot.Definition.Generation,
                    at,
                    kind
                )
            );
        }

        internal void MarkUpdate(DescriptorState descriptor, SlotState slot)
        {
            foreach ((DescriptorState existingDescriptor, SlotState existingSlot) in pendingSamples)
            {
                if (
                    ReferenceEquals(existingDescriptor, descriptor)
                    && ReferenceEquals(existingSlot, slot)
                )
                    return;
            }
            pendingSamples.Add((descriptor, slot));
        }

        private MotionPlaybackAddress InstallImperative(
            DescriptorState descriptor,
            MotionTargetDescriptor target,
            uint generation,
            ulong offset
        ) => InstallImperatives(descriptor, new[] { (target, offset) }, generation)[0];

        private MotionPlaybackAddress InstallSequenceImperative(
            DescriptorState descriptor,
            MotionTargetDescriptor target,
            uint generation,
            ulong offset,
            out IReadOnlyList<(MotionPlaybackAddress Old, MotionPlaybackAddress New)> remaps
        ) => InstallImperatives(descriptor, new[] { (target, offset) }, generation, out remaps)[0];

        private IReadOnlyList<MotionPlaybackAddress> InstallImperatives(
            DescriptorState descriptor,
            IReadOnlyList<(MotionTargetDescriptor Target, ulong Offset)> targets,
            uint generation
        ) => InstallImperatives(descriptor, targets, generation, out _);

        private IReadOnlyList<MotionPlaybackAddress> InstallImperatives(
            DescriptorState descriptor,
            IReadOnlyList<(MotionTargetDescriptor Target, ulong Offset)> targets,
            uint generation,
            out IReadOnlyList<(MotionPlaybackAddress Old, MotionPlaybackAddress New)> remaps
        )
        {
            var replacements = new List<MotionSlotDescriptor>();
            var addresses = new List<MotionPlaybackAddress>();
            foreach ((MotionTargetDescriptor target, ulong offset) in targets)
            {
                ulong slot = ulong.MaxValue - 1024 - offset;
                SlotState? previousSlot = descriptor.FindSlot(slot);
                uint actualGeneration = previousSlot is null
                    ? generation
                    : Math.Max(generation, checked(previousSlot.Definition.Generation + 1));
                replacements.Add(
                    new MotionSlotDescriptor(
                        slot,
                        actualGeneration,
                        MotionLayer.Animate,
                        target,
                        new MotionCallbackSubscriptions(false, false, false, false, false, false)
                    )
                );
                addresses.Add(
                    new MotionPlaybackAddress(
                        descriptor.Descriptor.DescriptorId,
                        slot,
                        actualGeneration
                    )
                );
            }
            List<MotionSlotDescriptor> slots = BattlementMotionOwnership.RetainDisjoint(
                descriptor,
                replacements,
                out var retained
            );
            remaps = retained
                .Select(value =>
                {
                    MotionSlotDescriptor replacement = slots.First(slot => slot.Slot == value.Slot);
                    return (
                        new MotionPlaybackAddress(
                            descriptor.Descriptor.DescriptorId,
                            value.Previous.Definition.Slot,
                            value.Previous.Definition.Generation
                        ),
                        new MotionPlaybackAddress(
                            descriptor.Descriptor.DescriptorId,
                            replacement.Slot,
                            replacement.Generation
                        )
                    );
                })
                .ToArray();
            slots.AddRange(replacements);
            MotionDescriptor updated = descriptor.Descriptor with { Slots = slots };
            var prepared = new DescriptorState(
                updated,
                descriptor.Properties,
                ClockMicros(updated.Clock),
                descriptor,
                retainUnchangedSlots: true
            );
            foreach ((ulong slot, SlotState previous) in retained)
                prepared.RetainPlayback(slot, previous, ClockMicros(updated.Clock));
            Commit(updated.HostId.Value, prepared);
            return addresses;
        }

        private void StopImperative(DescriptorState descriptor)
        {
            foreach (MotionSlotDescriptor slot in descriptor.Descriptor.Slots)
                if (slot.Slot >= ulong.MaxValue - 2048)
                    Stop(descriptor.Descriptor.DescriptorId, slot.Slot, slot.Generation);
        }

        private void ClearImperative(DescriptorState descriptor)
        {
            MotionSlotDescriptor[] slots = descriptor
                .Descriptor.Slots.Where(value => value.Slot < ulong.MaxValue - 2048)
                .ToArray();
            if (slots.Length == descriptor.Descriptor.Slots.Count)
                return;
            MotionDescriptor updated = descriptor.Descriptor with { Slots = slots };
            Commit(
                updated.HostId.Value,
                new DescriptorState(
                    updated,
                    descriptor.Properties,
                    ClockMicros(updated.Clock),
                    descriptor,
                    retainUnchangedSlots: true
                )
            );
        }

        private void Apply(MotionPlaybackAddress address, MotionPlaybackCommand command) =>
            Apply(
                new MotionPlaybackOperation(
                    address.DescriptorId,
                    address.Slot,
                    address.Generation,
                    command
                )
            );

        private void ThrowIfDisposed()
        {
            if (disposed)
                throw new ObjectDisposedException(nameof(BattlementMotionWorld));
        }

        private static BattlementUiException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private IBattlementLayoutProjectionTarget LayoutTarget(IBattlementMotionTarget target) =>
            target switch
            {
                BattlementUiMotionTarget ui => new BattlementUiLayoutProjectionTarget(
                    ui.Element,
                    uiProjectionSpace
                ),
                IBattlementLayoutProjectionTarget projection => projection,
                _ => throw Invalid("Layout Motion requires a projection-capable host."),
            };

        private static BattlementUiProjectionSpace DefaultUiProjectionSpace(
            VisualElement element
        ) => new(new DisplayId(0), element.panel is null ? 1 : element.panel.scaledPixelsPerPoint);

        private sealed record ActiveControl(
            ObjectId PlaybackId,
            uint Generation,
            MotionControlTarget Target,
            List<MotionPlaybackAddress> Addresses,
            bool Blocking
        );
    }
}

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
        private readonly Dictionary<Guid, ActiveControl> activeControls = new();
        private readonly HashSet<Guid> installingControls = new();
        private readonly BattlementMotionReconnectState reconnect = new();
        private readonly List<MotionLifecycleEvent> events = new();
        private readonly List<(DescriptorState Descriptor, SlotState Slot)> pendingSamples = new();
        private readonly List<MotionGestureEvent> gestureEvents = new();
        private readonly List<MotionGestureEvent> pendingGestureSamples = new();
        private readonly Func<double> unscaledTime;
        private readonly Func<double> scaledTime;
        private readonly Func<ObjectId, MotionClockSample>? audioTime;
        private readonly Func<ObjectId, VisualElement?> resolveElement;
        private readonly Func<TimeSpan> gestureTime;
        private readonly Func<bool> reducedMotion;
        private readonly System.Action? presentationChanged;
        private readonly BattlementMotionGraph graph;
        private readonly BattlementMotionPerformance performance = new();
        private readonly bool enablePlayerLoop;
        private readonly IBattlementUiAssetLookup? assets;
        private ulong sequence;
        private bool disposed;

        public BattlementMotionWorld(
            Func<double>? unscaledTime = null,
            Func<double>? scaledTime = null,
            bool registerPlayerLoop = true,
            IBattlementUiAssetLookup? assetLookup = null,
            Func<ObjectId, MotionClockSample>? audioTime = null,
            Func<ObjectId, VisualElement?>? resolveElement = null,
            Func<TimeSpan>? gestureTime = null,
            Func<bool>? reducedMotion = null,
            System.Action? presentationChanged = null
        )
        {
            this.unscaledTime = unscaledTime ?? (() => Time.unscaledTimeAsDouble);
            this.scaledTime = scaledTime ?? (() => Time.timeAsDouble);
            this.audioTime = audioTime;
            this.resolveElement = resolveElement ?? (_ => null);
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

            var prepared = new DescriptorState(
                descriptor,
                target,
                ClockMicros(descriptor.Clock),
                previous,
                target is BattlementUiMotionTarget ui
                    ? sharedLayouts.Origin(descriptor, ui.Element, previous, descriptors.Values)
                    : null,
                reconnect.Active
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
            activeControls.Clear();
            installingControls.Clear();
            reconnect.Clear();
            graph.Clear();
            events.Clear();
            pendingSamples.Clear();
            pendingGestureSamples.Clear();
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
                    var addresses = new List<MotionPlaybackAddress>();
                    var selected =
                        new Dictionary<
                            Guid,
                            (
                                DescriptorState Descriptor,
                                List<(MotionTargetDescriptor Target, ulong Offset)> Targets
                            )
                        >();
                    for (int index = 0; index < start.Steps.Count; index++)
                    {
                        MotionSequenceStep step = start.Steps[index];
                        foreach (
                            DescriptorState target in BattlementMotionControlUtilities.Select(
                                descriptors.Values,
                                root,
                                step.Selector
                            )
                        )
                        {
                            Guid id = target.Descriptor.DescriptorId.Value;
                            if (!selected.TryGetValue(id, out var group))
                            {
                                group = (target, new List<(MotionTargetDescriptor, ulong)>());
                                selected.Add(id, group);
                            }
                            group.Targets.Add(
                                (
                                    BattlementMotionControlUtilities.Delay(
                                        step.Target,
                                        step.StartMicros
                                    ),
                                    (ulong)index
                                )
                            );
                        }
                    }
                    foreach (var group in selected.Values)
                    foreach (var step in group.Targets)
                        ValidateImperative(group.Descriptor, step.Target, false);
                    foreach (var group in selected.Values)
                        addresses.AddRange(
                            InstallImperatives(group.Descriptor, group.Targets, start.Generation)
                        );
                    imperativePlaybacks.Register(start.PlaybackId, start.Generation, addresses);
                    if (addresses.Count == 0)
                        FinishImperative(start.PlaybackId.Value, MotionPlaybackOutcome.Completed);
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
                    var addresses = new List<MotionPlaybackAddress>();
                    var selected =
                        new Dictionary<
                            Guid,
                            (
                                DescriptorState Descriptor,
                                List<(MotionTargetDescriptor Target, ulong Offset)> Targets
                            )
                        >();
                    for (int index = 0; index < operation.StepCount; index++)
                    {
                        MotionSelector selector = operation.ReadStepSelector(index);
                        MotionTargetDescriptor descriptor = operation.ReadStepTarget(index);
                        ulong startMicros = operation.ReadStepStartMicros(index);
                        foreach (
                            DescriptorState target in BattlementMotionControlUtilities.Select(
                                descriptors.Values,
                                root,
                                selector
                            )
                        )
                        {
                            Guid id = target.Descriptor.DescriptorId.Value;
                            if (!selected.TryGetValue(id, out var group))
                            {
                                group = (target, new List<(MotionTargetDescriptor, ulong)>());
                                selected.Add(id, group);
                            }
                            group.Targets.Add(
                                (
                                    BattlementMotionControlUtilities.Delay(descriptor, startMicros),
                                    (ulong)index
                                )
                            );
                        }
                    }
                    foreach (var group in selected.Values)
                    foreach (var step in group.Targets)
                        ValidateImperative(group.Descriptor, step.Target, blocking);
                    foreach (var group in selected.Values)
                        addresses.AddRange(
                            InstallImperatives(
                                group.Descriptor,
                                group.Targets,
                                operation.Generation
                            )
                        );
                    imperativePlaybacks.Register(
                        operation.PlaybackId,
                        operation.Generation,
                        addresses
                    );
                    if (addresses.Count == 0)
                        FinishImperative(
                            operation.PlaybackId.Value,
                            MotionPlaybackOutcome.Completed
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
            )
                return null;
            ulong first = boundaries.Count == 0 ? sequence : boundaries[0].Sequence;
            ulong last = boundaries.Count == 0 ? sequence : boundaries[^1].Sequence;
            return new MotionEventBatch(
                first,
                last,
                boundaries,
                samples,
                valueSamples,
                terminalPlaybacks,
                nativeGestures
            );
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
            Sample(layout: false);
            foreach (DescriptorState descriptor in descriptors.Values)
                descriptor.SampleLayout(
                    ClockMicros(descriptor.Descriptor.Clock),
                    IsReduced(descriptor.Descriptor)
                );
            presentationChanged?.Invoke();
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
            IReadOnlyList<Guid> completed = imperativePlaybacks.Complete(descriptors);
            for (int index = 0; index < completed.Count; index++)
                ForgetActiveControl(completed[index]);
        }

        private void FinishImperative(Guid id, MotionPlaybackOutcome outcome)
        {
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

        private IReadOnlyList<MotionPlaybackAddress> InstallImperatives(
            DescriptorState descriptor,
            IReadOnlyList<(MotionTargetDescriptor Target, ulong Offset)> targets,
            uint generation
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

        private sealed record ActiveControl(
            ObjectId PlaybackId,
            uint Generation,
            MotionControlTarget Target,
            List<MotionPlaybackAddress> Addresses,
            bool Blocking
        );
    }
}

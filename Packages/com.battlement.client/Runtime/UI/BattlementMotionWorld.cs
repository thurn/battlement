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

        public void RecordPerformanceTraffic(int payloadBytes) =>
            performance.RecordTraffic(payloadBytes);

        internal void SetPseudoState(ObjectId descriptorId, MotionPseudoState state, bool value) =>
            descriptors[descriptorId.Value].SetPseudoState(state, value);

        public bool IsPlayerLoopRegistered { get; private set; }

        public BattlementPreparedMotionAdmission? Prepare(
            VisualElement target,
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
            BattlementMotionPropertyWriter.Configure(target, assets);
            BattlementMotionValidator.Validate(descriptor, hostId);
            BattlementMotionDescriptorValidator.ValidateCapabilities(descriptor);
            BattlementMotionGraph.ValidateDescriptor(descriptor);
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
                throw Invalid("A UI host cannot own two motion descriptors.");

            var prepared = new DescriptorState(
                descriptor,
                target,
                ClockMicros(descriptor.Clock),
                previous,
                sharedLayouts.Origin(descriptor, target, previous, descriptors.Values),
                reconnect.Active
            );
            return new BattlementPreparedMotionAdmission(this, hostId.Value, prepared);
        }

        public void Install(VisualElement target, ObjectId hostId, Prop<MotionDescriptor> motion) =>
            Prepare(target, hostId, motion)?.Commit();

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
                BattlementMotionPropertyWriter.Release(descriptor.Target);
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
                BattlementMotionPropertyWriter.Release(descriptor.Target);
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
                BattlementMotionPropertyWriter.Release(descriptor.Target);
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

        public void Apply(MotionControlOperation operation)
        {
            switch (operation.Command)
            {
                case MotionControlCommand.Start start:
                    RemoveActiveControl(
                        operation.ControlId.Value,
                        clearSlots: true,
                        MotionPlaybackOutcome.Cancelled
                    );
                    var addresses = new List<MotionPlaybackAddress>();
                    var active = new ActiveControl(start, addresses);
                    activeControls[operation.ControlId.Value] = active;
                    imperativePlaybacks.Register(start.PlaybackId, start.Generation, addresses);
                    installingControls.Add(operation.ControlId.Value);
                    try
                    {
                        foreach (DescriptorState binding in ControlBindings(operation.ControlId))
                            addresses.Add(InstallActiveControl(binding, active));
                    }
                    finally
                    {
                        installingControls.Remove(operation.ControlId.Value);
                    }
                    break;
                case MotionControlCommand.Set set:
                    RemoveActiveControl(
                        operation.ControlId.Value,
                        clearSlots: true,
                        MotionPlaybackOutcome.Cancelled
                    );
                    foreach (DescriptorState binding in ControlBindings(operation.ControlId))
                        BattlementMotionControlUtilities.ApplyImmediately(
                            binding,
                            BattlementMotionControlUtilities.Resolve(binding, set.Value)
                        );
                    break;
                case MotionControlCommand.Stop:
                    RemoveActiveControl(
                        operation.ControlId.Value,
                        clearSlots: false,
                        MotionPlaybackOutcome.Stopped
                    );
                    foreach (DescriptorState binding in ControlBindings(operation.ControlId))
                        StopImperative(binding);
                    break;
                case MotionControlCommand.Clear:
                    RemoveActiveControl(
                        operation.ControlId.Value,
                        clearSlots: false,
                        MotionPlaybackOutcome.Cancelled
                    );
                    foreach (DescriptorState binding in ControlBindings(operation.ControlId))
                        ClearImperative(binding);
                    break;
                default:
                    throw Invalid("Unknown animation-controls operation.");
            }
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
                    slot.CaptureValues(descriptor.Target)
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
            foreach (DescriptorState descriptor in descriptors.Values)
                descriptor.CompleteSlots(this);
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
                    previous.CancelActiveSlots(this, ClockMicros(previous.Descriptor.Clock));
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
            graph.Replace(prepared.Descriptor, prepared.Target);
            EnsurePlayerLoop();
            foreach (MotionPropertyValue value in prepared.Descriptor.StaticBaseline)
                BattlementMotionPropertyWriter.Write(prepared.Target, value.Property, value.Value);
            prepared.SynchronizeStaticStyles();
            if (reconnect.Active)
                prepared.ApplyReconnectPresentation();
            else
                prepared.ApplyInitialPresentation(IsReduced(prepared.Descriptor));
            if (!reconnect.Active)
                prepared.EmitActivated(this);
            if (prepared.Descriptor.Gestures is not null)
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

        private void AttachActiveControl(DescriptorState descriptor)
        {
            if (
                descriptor.Descriptor.ControlId is not ObjectId controlId
                || installingControls.Contains(controlId.Value)
                || !activeControls.TryGetValue(controlId.Value, out ActiveControl active)
            )
                return;
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
                BattlementMotionControlUtilities.Resolve(descriptor, active.Start.Target),
                active.Start.Generation,
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
            FinishImperative(active.Start.PlaybackId.Value, outcome);
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
                if (control.Start.PlaybackId.Value == id)
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
            foreach (DescriptorState descriptor in descriptors.Values)
                descriptor.Sample(
                    ClockMicros(descriptor.Descriptor.Clock),
                    layout,
                    IsReduced(descriptor.Descriptor),
                    this
                );
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
            var slots = descriptor.Descriptor.Slots.ToList();
            var addresses = new List<MotionPlaybackAddress>();
            foreach ((MotionTargetDescriptor target, ulong offset) in targets)
            {
                ulong slot = ulong.MaxValue - 1024 - offset;
                SlotState? previousSlot = descriptor.FindSlot(slot);
                uint actualGeneration = previousSlot is null
                    ? generation
                    : Math.Max(generation, checked(previousSlot.Definition.Generation + 1));
                slots.RemoveAll(value => value.Slot == slot);
                slots.Add(
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
            MotionDescriptor updated = descriptor.Descriptor with { Slots = slots };
            var prepared = new DescriptorState(
                updated,
                descriptor.Target,
                ClockMicros(updated.Clock),
                descriptor
            );
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
                    descriptor.Target,
                    ClockMicros(updated.Clock),
                    descriptor
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
            MotionControlCommand.Start Start,
            List<MotionPlaybackAddress> Addresses
        );
    }
}

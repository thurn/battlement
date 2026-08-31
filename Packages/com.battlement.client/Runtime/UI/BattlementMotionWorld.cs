#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementMotionWorld : IDisposable
    {
        private readonly Dictionary<Guid, DescriptorState> descriptors = new();
        private readonly Dictionary<Guid, Guid> descriptorByHost = new();
        private readonly Dictionary<Guid, ulong> controlledClocks = new();
        private readonly List<MotionLifecycleEvent> events = new();
        private readonly List<(DescriptorState Descriptor, SlotState Slot)> pendingSamples = new();
        private readonly Func<double> unscaledTime;
        private readonly Func<double> scaledTime;
        private readonly bool enablePlayerLoop;
        private readonly IBattlementUiAssetLookup? assets;
        private ulong sequence;
        private bool disposed;

        public BattlementMotionWorld(
            Func<double>? unscaledTime = null,
            Func<double>? scaledTime = null,
            bool registerPlayerLoop = true,
            IBattlementUiAssetLookup? assetLookup = null
        )
        {
            this.unscaledTime = unscaledTime ?? (() => Time.unscaledTimeAsDouble);
            this.scaledTime = scaledTime ?? (() => Time.timeAsDouble);
            enablePlayerLoop = registerPlayerLoop;
            assets = assetLookup;
        }

        public int DescriptorCount => descriptors.Count;

        internal void SetPseudoState(ObjectId descriptorId, MotionPseudoState state, bool value) =>
            descriptors[descriptorId.Value].SetPseudoState(state, value);

        public bool IsPlayerLoopRegistered { get; private set; }

        public PreparedAdmission? Prepare(
            VisualElement target,
            ObjectId hostId,
            Prop<MotionDescriptor> motion
        )
        {
            ThrowIfDisposed();
            if (motion.IsUnset)
                return null;
            if (motion.IsReset)
                return new PreparedAdmission(this, hostId.Value, null);

            MotionDescriptor descriptor = motion.Value;
            BattlementMotionPropertyWriter.Configure(target, assets);
            BattlementMotionValidator.Validate(descriptor, hostId);
            ValidateCapabilities(descriptor);
            DescriptorState? previous = descriptors.TryGetValue(
                descriptor.DescriptorId.Value,
                out DescriptorState value
            )
                ? value
                : null;
            if (previous is not null && descriptor.Generation <= previous.Descriptor.Generation)
                throw Invalid("A motion descriptor update must advance its generation.");
            if (
                descriptorByHost.TryGetValue(hostId.Value, out Guid existingId)
                && existingId != descriptor.DescriptorId.Value
            )
                throw Invalid("A UI host cannot own two motion descriptors.");

            var prepared = new DescriptorState(
                descriptor,
                target,
                ClockMicros(descriptor.Clock),
                previous
            );
            return new PreparedAdmission(this, hostId.Value, prepared);
        }

        public void Install(VisualElement target, ObjectId hostId, Prop<MotionDescriptor> motion) =>
            Prepare(target, hostId, motion)?.Commit();

        public void RemoveHost(ObjectId hostId)
        {
            if (!descriptorByHost.Remove(hostId.Value, out Guid descriptorId))
                return;
            if (descriptors.Remove(descriptorId, out DescriptorState descriptor))
            {
                descriptor.Dispose();
                BattlementMotionPropertyWriter.Release(descriptor.Target);
            }
        }

        public void Clear()
        {
            foreach (DescriptorState descriptor in descriptors.Values)
            {
                descriptor.Dispose();
                BattlementMotionPropertyWriter.Release(descriptor.Target);
            }
            descriptors.Clear();
            descriptorByHost.Clear();
            controlledClocks.Clear();
            events.Clear();
            pendingSamples.Clear();
            if (IsPlayerLoopRegistered)
            {
                BattlementMotionPlayerLoop.Unregister(this);
                IsPlayerLoopRegistered = false;
            }
        }

        public void AdvanceControlledClock(ObjectId clockId, ulong deltaMicros)
        {
            controlledClocks.TryGetValue(clockId.Value, out ulong current);
            controlledClocks[clockId.Value] = checked(current + deltaMicros);
        }

        public void SetControlledClock(ObjectId clockId, ulong elapsedMicros) =>
            controlledClocks[clockId.Value] = elapsedMicros;

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

        public void PreLayout() => Sample(layout: true);

        public void PostLayout()
        {
            Sample(layout: false);
            foreach (DescriptorState descriptor in descriptors.Values)
                descriptor.CompleteSlots(this);
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

        private void Commit(Guid hostId, DescriptorState? prepared)
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
                previous.CancelActiveSlots(this, ClockMicros(previous.Descriptor.Clock));
                previous.Dispose();
            }
            descriptors[prepared.Descriptor.DescriptorId.Value] = prepared;
            descriptorByHost[hostId] = prepared.Descriptor.DescriptorId.Value;
            EnsurePlayerLoop();
            foreach (MotionPropertyValue value in prepared.Descriptor.StaticBaseline)
                BattlementMotionPropertyWriter.Write(prepared.Target, value.Property, value.Value);
            prepared.SynchronizeStaticStyles();
            prepared.ApplyInitialPresentation();
            prepared.EmitActivated(this);
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
                descriptor.Sample(ClockMicros(descriptor.Descriptor.Clock), layout, this);
        }

        private ulong ClockMicros(MotionClockSource source) =>
            source switch
            {
                MotionClockSource.Unscaled => SecondsToMicros(unscaledTime()),
                MotionClockSource.Scaled => SecondsToMicros(scaledTime()),
                MotionClockSource.Controlled value => controlledClocks.TryGetValue(
                    value.Value.Value,
                    out ulong elapsed
                )
                    ? elapsed
                    : 0,
                MotionClockSource.Audio => throw Invalid(
                    "Audio motion clocks require the Task 08 audio playhead bridge."
                ),
                _ => throw Invalid("Unknown motion clock source."),
            };

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

        private static void ValidateCapabilities(MotionDescriptor descriptor)
        {
            ValidateTarget(descriptor.Initial);
            foreach (MotionSlotDescriptor slot in descriptor.Slots)
                ValidateTarget(slot.Target);
            foreach (MotionPropertyValue value in descriptor.StaticBaseline)
                RequireWriter(value.Property);
            foreach (
                MotionPseudoStyle style in descriptor.PseudoStyles
                    ?? Array.Empty<MotionPseudoStyle>()
            )
            foreach (MotionPropertyValue value in style.Values)
                RequireWriter(value.Property);
            foreach (
                CssAnimationDescriptor animation in descriptor.Animations
                    ?? Array.Empty<CssAnimationDescriptor>()
            )
            foreach (CssPropertyTrack track in animation.Tracks)
                RequireWriter(track.Property);
            foreach (
                MotionDecorationDescriptor decoration in descriptor.Decorations
                    ?? Array.Empty<MotionDecorationDescriptor>()
            )
            foreach (CssAnimationDescriptor animation in decoration.Animations)
            foreach (CssPropertyTrack track in animation.Tracks)
                RequireWriter(track.Property);
        }

        private static void ValidateTarget(MotionTargetDescriptor? target)
        {
            if (target is null)
                return;
            foreach (MotionPropertyTrack track in target.Tracks)
                RequireWriter(track.Property);
            foreach (MotionPropertyValue value in target.TransitionEnd)
                RequireWriter(value.Property);
        }

        private static void RequireWriter(MotionProperty property)
        {
            if (!BattlementMotionPropertyWriter.Supports(property))
                throw Invalid($"Motion property {property} has no renderer capability.");
        }

        private static ulong SecondsToMicros(double seconds)
        {
            if (!double.IsFinite(seconds) || seconds < 0)
                throw Invalid("A motion clock returned invalid time.");
            return checked((ulong)Math.Round(seconds * 1_000_000d));
        }

        private void ThrowIfDisposed()
        {
            if (disposed)
                throw new ObjectDisposedException(nameof(BattlementMotionWorld));
        }

        private static BattlementUiException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        internal sealed class PreparedAdmission
        {
            private readonly BattlementMotionWorld world;
            private readonly Guid hostId;
            private readonly DescriptorState? prepared;
            private bool committed;

            public PreparedAdmission(
                BattlementMotionWorld world,
                Guid hostId,
                DescriptorState? prepared
            ) => (this.world, this.hostId, this.prepared) = (world, hostId, prepared);

            public void Commit()
            {
                if (committed)
                    throw new InvalidOperationException("Motion admission was already committed.");
                world.Commit(hostId, prepared);
                committed = true;
            }
        }
    }
}

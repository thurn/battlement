#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Battlement.UI
{
    internal sealed class SlotState
    {
        private readonly TrackState[] tracks;
        private TrackState[] completionTracks = Array.Empty<TrackState>();
        private IEnumerable<TrackState> TimingTracks => tracks.Concat(completionTracks);
        private readonly IBattlementMotionTarget target;
        private readonly Dictionary<MotionProperty, MotionValue> presentation = new();
        private uint emittedIteration;
        private bool emittedStart;

        public SlotState(
            MotionSlotDescriptor definition,
            MotionClockSource clock,
            IBattlementMotionTarget target,
            ulong anchorMicros,
            IReadOnlyDictionary<MotionProperty, MotionValue>? initial,
            DescriptorState? previous
        )
        {
            Definition = definition;
            this.target = target;
            Clock = clock;
            AnchorMicros = anchorMicros;
            Speed = 1;
            Direction = MotionPlaybackDirection.Forward;
            Active = definition.Layer is MotionLayer.Animate or MotionLayer.Exit;
            tracks = new TrackState[definition.Target.Tracks.Count];
            for (int index = 0; index < tracks.Length; index++)
            {
                MotionPropertyTrack track = definition.Target.Tracks[index];
                MotionValue presentation = target.Read(track.Property);
                MotionValue origin =
                    initial is not null
                    && initial.TryGetValue(track.Property, out MotionValue value)
                        ? value
                    : track.Values.Count > 1 ? track.Values[0]
                    : presentation;
                tracks[index] = new TrackState(
                    track,
                    origin,
                    previous?.IncomingVelocity(track.Property) ?? 0
                );
            }
        }

        public MotionSlotDescriptor Definition { get; }

        public MotionClockSource Clock { get; }

        public ulong AnchorMicros { get; set; }

        public ulong HeldMicros { get; set; }

        public double Speed { get; set; }

        public bool Paused { get; set; }

        public bool SeekPending { get; set; }

        public bool Active { get; private set; }

        internal bool IsFiniteActive =>
            Active
            && !Terminal
            && !Paused
            && Clock is not MotionClockSource.Controlled
            && (TimingTracks.Any(track => !track.Done && !track.IsInfinite) || SeekPending);

        internal bool IsInfiniteActive =>
            Active
            && !Terminal
            && !Paused
            && TimingTracks.Any(track => !track.Done && track.IsInfinite);

        internal bool IsHeldActive =>
            Active
            && !Terminal
            && !Paused
            && Clock is MotionClockSource.Controlled
            && TimingTracks.Any(track => !track.Done && !track.IsInfinite);

        internal string ReadinessDiagnostic() =>
            $"layer={Definition.Layer},clock={Clock.GetType().Name},"
            + $"elapsed-ms={LastElapsedMicros / 1000},incomplete-tracks="
            + $"{tracks.Count(track => !track.Done && !track.IsInfinite)},"
            + $"seek-pending={SeekPending}";

        public int TrackCount => tracks.Length;

        public int LayoutTrackCount
        {
            get
            {
                int count = 0;
                foreach (TrackState track in tracks)
                    if (target.IsLayout(track.Definition.Property))
                        count++;
                return count;
            }
        }

        public MotionPlaybackDirection Direction { get; set; }

        public bool Terminal { get; private set; }

        public bool Cancelled { get; private set; }

        public MotionPlaybackOutcome? Outcome { get; private set; }

        public ulong LastElapsedMicros { get; private set; }

        public bool AllTracksDone
        {
            get
            {
                foreach (TrackState track in TimingTracks)
                    if (!track.Done)
                        return false;
                return true;
            }
        }

        public ulong Elapsed(ulong clockMicros)
        {
            if (Paused)
                return HeldMicros;
            double advanced = (clockMicros - AnchorMicros) * Speed;
            return checked(HeldMicros + (ulong)Math.Round(advanced));
        }

        public bool InDelay(ulong clockMicros)
        {
            ulong elapsed = Elapsed(clockMicros);
            foreach (TrackState track in tracks)
                if (track.Definition.Transition.DelayMicros <= 0)
                    return false;
                else if (elapsed >= (ulong)track.Definition.Transition.DelayMicros)
                    return false;
            return tracks.Length != 0;
        }

        public void AdoptPlayback(SlotState previous, ulong clockMicros, bool paused)
        {
            if (Definition.Target.TransitionEnd.Count != 0)
                completionTracks = previous
                    .completionTracks.Concat(
                        previous.tracks.Where(track => FindTrack(track.Definition.Property) is null)
                    )
                    .Select(track =>
                    {
                        var copy = new TrackState(track.Definition, track.Origin, 0);
                        copy.Adopt(track);
                        return copy;
                    })
                    .ToArray();
            HeldMicros = previous.Elapsed(clockMicros);
            AnchorMicros = clockMicros;
            Speed = previous.Speed;
            Direction = previous.Direction;
            Active = previous.Active;
            LastElapsedMicros = previous.LastElapsedMicros;
            Paused = paused;
            SeekPending = previous.SeekPending;
            Terminal = previous.Terminal;
            Cancelled = previous.Cancelled;
            Outcome = previous.Outcome;
            emittedStart = previous.emittedStart;
            emittedIteration = previous.emittedIteration;
            foreach (TrackState track in tracks)
            {
                TrackState? oldTrack = previous.FindTrack(track.Definition.Property);
                if (oldTrack is not null)
                    track.Adopt(oldTrack);
            }
            foreach ((MotionProperty property, MotionValue value) in previous.presentation)
                if (FindTrack(property) is not null)
                    presentation[property] = value;
        }

        public void Reset(ulong clockMicros)
        {
            AnchorMicros = clockMicros;
            HeldMicros = 0;
            Paused = false;
            SeekPending = false;
            Terminal = false;
            Cancelled = false;
            Outcome = null;
            emittedStart = false;
            emittedIteration = 0;
            foreach (TrackState track in tracks)
                track.Reset();
        }

        public void SetActive(bool value, IBattlementMotionTarget target, ulong clockMicros)
        {
            if (Active == value)
                return;
            Active = value;
            if (!value)
                return;
            AnchorMicros = clockMicros;
            HeldMicros = 0;
            Paused = false;
            SeekPending = false;
            Terminal = false;
            Cancelled = false;
            Outcome = null;
            emittedStart = false;
            emittedIteration = 0;
            foreach (TrackState track in tracks)
                track.Retarget(target);
        }

        public TrackState? FindTrack(MotionProperty property)
        {
            foreach (TrackState state in tracks)
                if (state.Definition.Property == property)
                    return state;
            return null;
        }

        public void RetargetPosition(IReadOnlyList<MotionPropertyValue> values, ulong clockMicros)
        {
            ulong elapsed = Elapsed(clockMicros);
            foreach (MotionPropertyValue value in values)
                FindTrack(value.Property)?.RetargetDestination(target, value.Value, elapsed);
        }

        public void ApplyOrigin(IBattlementMotionTarget target)
        {
            foreach (TrackState track in tracks)
                track.ApplyOrigin(target);
        }

        public void ApplyInitialOrigin(IBattlementMotionTarget target, bool reducedMotion)
        {
            foreach (TrackState track in tracks)
                if (reducedMotion && target.IsSpatial(track.Definition.Property))
                    track.ApplyTerminal(target);
                else
                    track.ApplyOrigin(target);
        }

        public void ApplyOrigin(IBattlementMotionTarget target, bool layout)
        {
            foreach (TrackState track in tracks)
            {
                if (target.IsLayout(track.Definition.Property) == layout)
                    track.ApplyOrigin(target);
            }
        }

        public void ApplyTerminal(IBattlementMotionTarget target)
        {
            foreach (TrackState track in tracks)
            {
                track.ApplyTerminal(target);
                presentation[track.Definition.Property] = target.Read(track.Definition.Property);
            }
        }

        public void Sample(
            IBattlementMotionTarget target,
            ulong clockMicros,
            bool layout,
            bool reducedMotion
        )
        {
            if (Cancelled)
                return;
            if (Terminal && !Paused)
            {
                foreach ((MotionProperty property, MotionValue value) in presentation)
                    if (target.IsLayout(property) == layout)
                        target.Write(property, value);
                foreach (MotionPropertyValue value in Definition.Target.TransitionEnd)
                    if (target.IsLayout(value.Property) == layout)
                        target.Write(value.Property, value.Value);
                return;
            }
            LastElapsedMicros = Elapsed(clockMicros);
            foreach (TrackState track in completionTracks)
                if (target.IsLayout(track.Definition.Property) == layout)
                    track.Sample(
                        target,
                        LastElapsedMicros,
                        Direction,
                        reducedMotion && target.IsSpatial(track.Definition.Property),
                        write: false
                    );
            foreach (TrackState track in tracks)
            {
                if (target.IsLayout(track.Definition.Property) != layout)
                    continue;
                track.Sample(
                    target,
                    LastElapsedMicros,
                    Direction,
                    reducedMotion && target.IsSpatial(track.Definition.Property)
                );
                presentation[track.Definition.Property] = target.Read(track.Definition.Property);
            }
        }

        public IReadOnlyDictionary<MotionProperty, MotionValue> CaptureValues(
            IBattlementMotionTarget target,
            bool layout
        )
        {
            var values = new Dictionary<MotionProperty, MotionValue>();
            foreach (TrackState track in tracks)
                if (target.IsLayout(track.Definition.Property) == layout)
                    values[track.Definition.Property] = target.Read(track.Definition.Property);
            return values;
        }

        public void RestoreValues(
            IBattlementMotionTarget target,
            IReadOnlyDictionary<MotionProperty, MotionValue> values
        )
        {
            foreach ((MotionProperty property, MotionValue value) in values)
                target.Write(property, value);
        }

        public void Compose(
            IBattlementMotionTarget target,
            IReadOnlyDictionary<MotionProperty, MotionValue> lower,
            AnimationComposition composition,
            bool layout
        )
        {
            foreach (TrackState track in tracks)
            {
                MotionProperty property = track.Definition.Property;
                if (target.IsLayout(property) != layout)
                    continue;
                if (!lower.TryGetValue(property, out MotionValue under))
                    continue;
                MotionValue sample = target.Read(property);
                target.Write(
                    property,
                    BattlementMotionComposition.Compose(
                        property,
                        under,
                        sample,
                        track.Origin,
                        track.End,
                        track.Iteration,
                        composition
                    )
                );
            }
        }

        public void EmitCrossedBoundaries(DescriptorState descriptor, BattlementMotionWorld world)
        {
            if (SeekPending)
                return;
            long earliestDelay = long.MaxValue;
            uint iteration = 0;
            foreach (TrackState track in tracks)
            {
                if (track.Suppressed)
                    continue;
                earliestDelay = Math.Min(earliestDelay, track.Definition.Transition.DelayMicros);
                iteration = Math.Max(iteration, track.Iteration);
            }
            ulong startBoundary = earliestDelay <= 0 ? 0 : checked((ulong)earliestDelay);
            if (!emittedStart && LastElapsedMicros >= startBoundary)
            {
                emittedStart = true;
                if (Definition.Callbacks.Start)
                    world.Emit(descriptor, this, new MotionEventKind.Started(), startBoundary);
            }
            if (iteration <= emittedIteration)
                return;
            if (Definition.Callbacks.Repeat)
            {
                world.Emit(
                    descriptor,
                    this,
                    new MotionEventKind.Repeated(emittedIteration + 1, iteration),
                    LastElapsedMicros
                );
            }
            emittedIteration = iteration;
        }

        public void ConsumeSeek()
        {
            SeekPending = false;
            emittedStart = true;
            foreach (TrackState track in tracks)
                emittedIteration = Math.Max(emittedIteration, track.Iteration);
        }

        public IReadOnlyList<MotionPropertyValue> CaptureValues(IBattlementMotionTarget target)
        {
            var values = new MotionPropertyValue[tracks.Length];
            for (int index = 0; index < tracks.Length; index++)
            {
                MotionProperty property = tracks[index].Definition.Property;
                values[index] = new MotionPropertyValue(property, target.Read(property));
            }
            return values;
        }

        public void MarkStopped(ulong elapsedMicros)
        {
            Outcome = MotionPlaybackOutcome.Stopped;
            HeldMicros = elapsedMicros;
            Paused = true;
            Terminal = true;
            foreach (TrackState track in tracks)
                track.Freeze();
        }

        public void MarkCancelled(ulong elapsedMicros)
        {
            Outcome = MotionPlaybackOutcome.Cancelled;
            HeldMicros = elapsedMicros;
            Paused = true;
            Terminal = true;
            Cancelled = true;
            foreach (TrackState track in tracks)
                track.Freeze();
        }

        public void MarkCompleted()
        {
            Outcome = MotionPlaybackOutcome.Completed;
            Paused = false;
            Terminal = true;
            foreach (TrackState track in tracks)
                track.Freeze();
        }
    }
}

#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Battlement.UI
{
    internal sealed class BattlementMotionSequence
    {
        private ulong anchorMicros;
        private ulong heldMicros;

        public BattlementMotionSequence(
            ObjectId playbackId,
            uint generation,
            MotionClockSource clock,
            ulong now,
            IReadOnlyList<MotionSequenceEntryState> entries
        )
        {
            PlaybackId = playbackId;
            Generation = generation;
            Clock = clock;
            anchorMicros = now;
            Entries = entries;
            Labels = entries
                .Select((entry, index) => (entry.Definition, index))
                .Where(value => value.Definition is MotionSequenceEntry.Label)
                .ToDictionary(
                    value => ((MotionSequenceEntry.Label)value.Definition).Name,
                    value => value.index
                );
        }

        public ObjectId PlaybackId { get; }
        public uint Generation { get; }
        public MotionClockSource Clock { get; }
        public IReadOnlyList<MotionSequenceEntryState> Entries { get; }
        public IReadOnlyDictionary<string, int> Labels { get; }
        public double Speed { get; private set; } = 1;
        public bool Paused { get; private set; }

        public bool IsInfinite => Entries.Any(entry => entry.Infinite);

        public bool Complete => Entries.All(entry => entry.CompletedAt is not null);

        public ulong Elapsed(ulong now)
        {
            if (Paused)
                return heldMicros;
            return checked(heldMicros + (ulong)Math.Round((now - anchorMicros) * Speed));
        }

        public void Play(ulong now)
        {
            if (!Paused)
                return;
            anchorMicros = now;
            Paused = false;
        }

        public void Pause(ulong now)
        {
            if (Paused)
                return;
            heldMicros = Elapsed(now);
            Paused = true;
        }

        public void SetSpeed(ulong now, double value)
        {
            heldMicros = Elapsed(now);
            anchorMicros = now;
            Speed = value;
            Paused = value == 0;
        }

        public void Finish(ulong now)
        {
            ulong elapsed = Elapsed(now);
            foreach (MotionSequenceEntryState entry in Entries)
            {
                entry.StartedAt ??= elapsed;
                entry.CompletedAt ??= elapsed;
            }
        }

        public bool Progress(
            ulong now,
            Func<int, MotionSequenceEntryState, bool> start,
            Func<MotionSequenceEntryState, bool> terminal,
            Action<string> emitLabel
        )
        {
            if (Paused)
                return Complete;
            ulong elapsed = Elapsed(now);
            bool changed;
            do
            {
                changed = false;
                for (int index = 0; index < Entries.Count; index++)
                {
                    MotionSequenceEntryState entry = Entries[index];
                    if (entry.StartedAt is null)
                    {
                        ulong? eligible = EligibleAt(index);
                        if (eligible is null || eligible > elapsed)
                            continue;
                        entry.StartedAt = elapsed;
                        if (entry.Definition is MotionSequenceEntry.Label label)
                        {
                            emitLabel(label.Name);
                            entry.CompletedAt = elapsed;
                            changed = true;
                        }
                        else if (start(index, entry))
                        {
                            entry.CompletedAt = elapsed;
                            changed = true;
                        }
                    }
                    if (entry.StartedAt is not null && entry.CompletedAt is null && terminal(entry))
                    {
                        entry.CompletedAt = elapsed;
                        changed = true;
                    }
                }
            } while (changed);
            return Complete;
        }

        public ulong? EligibleAt(int index)
        {
            MotionSequenceSchedule schedule = Entries[index].Schedule;
            return schedule switch
            {
                MotionSequenceSchedule.Absolute value => value.StartMicros,
                MotionSequenceSchedule.RelativeStart value => Offset(
                    Entry(value.Entry).StartedAt,
                    value.OffsetMicros
                ),
                MotionSequenceSchedule.AfterCompletion value => Offset(
                    Entry(value.Entry).CompletedAt,
                    value.OffsetMicros
                ),
                MotionSequenceSchedule.Label value => Offset(
                    Entries[Labels[value.Name]].StartedAt,
                    value.OffsetMicros
                ),
                _ => throw Invalid("Unknown Motion sequence schedule."),
            };
        }

        private MotionSequenceEntryState Entry(uint index) =>
            index < Entries.Count
                ? Entries[(int)index]
                : throw Invalid("Motion sequence references a missing entry.");

        private static ulong? Offset(ulong? value, long offset) =>
            value is ulong time ? time.SaturatingAdd(offset) : null;

        private static BattlementUiException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }

    internal sealed class MotionSequenceEntryState
    {
        public MotionSequenceEntryState(
            MotionSequenceEntry definition,
            IReadOnlyList<Guid> targets,
            IReadOnlyDictionary<Guid, IReadOnlyList<MotionPropertyValue>> captured
        ) => (Definition, Targets, CapturedPositions) = (definition, targets, captured);

        public MotionSequenceEntry Definition { get; }
        public IReadOnlyList<Guid> Targets { get; }
        public IReadOnlyDictionary<
            Guid,
            IReadOnlyList<MotionPropertyValue>
        > CapturedPositions { get; }
        public List<MotionPlaybackAddress> Addresses { get; } = new();
        public ulong? StartedAt { get; set; }
        public ulong? CompletedAt { get; set; }

        public MotionSequenceSchedule Schedule =>
            Definition switch
            {
                MotionSequenceEntry.Animate value => value.Schedule,
                MotionSequenceEntry.Label value => value.Schedule,
                _ => throw new InvalidOperationException("Unknown Motion sequence entry."),
            };

        public bool Infinite =>
            Definition is MotionSequenceEntry.Animate animation
            && (
                animation.Target.Tracks.Any(track =>
                    track.Transition.Repeat is MotionRepeat.Forever
                )
                || (
                    animation.Position is not null
                    && animation.PositionTransition.Repeat is MotionRepeat.Forever
                )
            );
    }

    internal static class MotionSequenceTime
    {
        public static ulong SaturatingAdd(this ulong value, long offset) =>
            offset >= 0
                ? value > ulong.MaxValue - (ulong)offset
                    ? ulong.MaxValue
                    : value + (ulong)offset
                : value < (ulong)(-(offset + 1)) + 1
                    ? 0
                    : value - ((ulong)(-(offset + 1)) + 1);
    }
}

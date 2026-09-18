#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;
using UnityEngine;

namespace Battlement
{
    internal sealed class BattlementMotionEffects : IBattlementMotionEffects
    {
        private readonly BattlementWorld world;
        private readonly BattlementPreparedAssets assets;
        private readonly BattlementAudioSources audio;
        private readonly BattlementParticleEffects particles;
        private readonly DittoMotionClock clock;
        private readonly List<RunningEffect> operations = new();

        public BattlementMotionEffects(
            BattlementWorld world,
            BattlementPreparedAssets assets,
            BattlementAudioSources audio,
            BattlementParticleEffects particles,
            DittoMotionClock clock
        ) =>
            (this.world, this.assets, this.audio, this.particles, this.clock) = (
                world,
                assets,
                audio,
                particles,
                clock
            );

        public IBattlementPreparedMotionEffect Prepare(MotionSequenceEntry entry)
        {
            PreparedAsset asset = entry switch
            {
                MotionSequenceEntry.Sound value => new PreparedAsset.AudioClip(
                    new AudioClipAddress(value.Occurrence.Address)
                ),
                MotionSequenceEntry.Particle value => new PreparedAsset.ParticleEffect(
                    new ParticleEffectAddress(value.Occurrence.Address)
                ),
                _ => throw new InvalidOperationException("Unknown Motion effect occurrence."),
            };
            IBattlementAssetLease lease = assets.Acquire(asset);
            try
            {
                Type expected =
                    entry is MotionSequenceEntry.Sound ? typeof(AudioClip) : typeof(GameObject);
                if (!expected.IsInstanceOfType(lease.Value))
                    throw new BattlementCommandException(
                        CoreErrorCode.AssetTypeMismatch,
                        $"Prepared effect asset '{Address(asset)}' has the wrong type."
                    );
                if (
                    entry is MotionSequenceEntry.Particle
                    && ((GameObject)lease.Value)
                        .GetComponentsInChildren<ParticleSystem>(true)
                        .Length == 0
                )
                    throw new BattlementCommandException(
                        CoreErrorCode.ComponentMissing,
                        $"Prepared particle effect '{Address(asset)}' has no ParticleSystem."
                    );
                return new PreparedEffect(lease);
            }
            catch
            {
                lease.Dispose();
                throw;
            }
        }

        public void Start(
            ObjectId playbackId,
            int entryIndex,
            MotionSequenceEntry entry,
            IBattlementPreparedMotionEffect prepared,
            UnityEngine.Vector3? capturedPosition
        )
        {
            if (prepared is not PreparedEffect retained)
                throw new InvalidOperationException("Unknown prepared Motion effect.");
            CommandId commandId = OccurrenceId(playbackId, entryIndex);
            IBattlementAssetLease lease = retained.Take();
            IBattlementCommandOperation? operation;
            try
            {
                operation = entry switch
                {
                    MotionSequenceEntry.Sound value => audio.Play(
                        commandId,
                        new BattlementDirectAudioPlay(
                            value.Occurrence.Address,
                            value.Occurrence.Volume,
                            value.Occurrence.Pitch,
                            value.Occurrence.Looping,
                            value.Occurrence.FadeInMilliseconds
                        ),
                        clock.Elapsed,
                        lease
                    ),
                    MotionSequenceEntry.Particle value => Spawn(
                        commandId,
                        value.Occurrence,
                        capturedPosition,
                        lease
                    ),
                    _ => throw new InvalidOperationException("Unknown Motion effect occurrence."),
                };
                lease = null!;
            }
            finally
            {
                lease?.Dispose();
            }
            if (operation is not null)
                operations.Add(new RunningEffect(playbackId, operation));
        }

        public void Advance()
        {
            for (int index = operations.Count - 1; index >= 0; index--)
                if (operations[index].Operation.IsComplete(clock.Elapsed))
                    operations.RemoveAt(index);
        }

        public void Pause(ObjectId playbackId)
        {
            foreach (RunningEffect effect in operations)
                if (
                    effect.PlaybackId == playbackId
                    && effect.Operation is IBattlementPausableCommandOperation pausable
                )
                    pausable.Pause(clock.Elapsed);
        }

        public void Resume(ObjectId playbackId)
        {
            foreach (RunningEffect effect in operations)
                if (
                    effect.PlaybackId == playbackId
                    && effect.Operation is IBattlementPausableCommandOperation pausable
                )
                    pausable.Resume(clock.Elapsed);
        }

        public void Cancel(ObjectId playbackId)
        {
            for (int index = operations.Count - 1; index >= 0; index--)
            {
                if (operations[index].PlaybackId != playbackId)
                    continue;
                operations[index].Operation.Cancel();
                operations.RemoveAt(index);
            }
        }

        public void Reset()
        {
            foreach (RunningEffect effect in operations)
                effect.Operation.Cancel();
            operations.Clear();
        }

        public void Dispose() => Reset();

        private IBattlementCommandOperation? Spawn(
            CommandId commandId,
            MotionParticleOccurrence occurrence,
            UnityEngine.Vector3? capturedPosition,
            IBattlementAssetLease lease
        )
        {
            UnityEngine.Vector3 position = capturedPosition ?? Resolve(occurrence.Position);
            return particles.Spawn(
                commandId,
                new BattlementDirectParticleSpawn(
                    occurrence.Address,
                    null,
                    position.x,
                    position.y,
                    position.z,
                    occurrence.LifetimeMilliseconds
                ),
                clock.Elapsed,
                lease
            );
        }

        public UnityEngine.Vector3 Resolve(MotionPositionReference reference)
        {
            GameObject target = world.RequireObject(reference.ObjectId);
            Transform point = reference.Anchor is string anchor
                ? BattlementWorldPointGeometry.FindAnchor(target, new AnchorName(anchor))
                : target.transform;
            return point.TransformPoint(
                new UnityEngine.Vector3(
                    (float)reference.Offset.X,
                    (float)reference.Offset.Y,
                    (float)reference.Offset.Z
                )
            );
        }

        private static string Address(PreparedAsset asset) =>
            asset switch
            {
                PreparedAsset.AudioClip value => value.Address.Value,
                PreparedAsset.ParticleEffect value => value.Address.Value,
                _ => throw new InvalidOperationException("Unknown Motion effect asset."),
            };

        private static CommandId OccurrenceId(ObjectId playbackId, int entryIndex)
        {
            byte[] bytes = playbackId.Value.ToByteArray();
            bytes[0] ^= 0xe6;
            byte[] index = BitConverter.GetBytes(entryIndex);
            for (int offset = 0; offset < index.Length; offset++)
                bytes[12 + offset] ^= index[offset];
            return new CommandId(new Guid(bytes));
        }

        private sealed class PreparedEffect : IBattlementPreparedMotionEffect
        {
            private IBattlementAssetLease? lease;

            public PreparedEffect(IBattlementAssetLease lease) => this.lease = lease;

            public IBattlementAssetLease Take()
            {
                IBattlementAssetLease retained =
                    lease ?? throw new ObjectDisposedException(nameof(PreparedEffect));
                lease = null;
                return retained;
            }

            public void Dispose()
            {
                lease?.Dispose();
                lease = null;
            }
        }

        private sealed record RunningEffect(
            ObjectId PlaybackId,
            IBattlementCommandOperation Operation
        );
    }
}

#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;

namespace Battlement
{
    internal sealed class BattlementAudioSources : IDisposable, IBattlementMotionAudio
    {
        private readonly BattlementWorld world;
        private readonly BattlementPreparedAssets preparedAssets;
        private readonly DittoMotionClock motionClock;
        private readonly Transform poolRoot;
        private readonly Dictionary<Guid, BattlementAudioInstance> live = new();
        private readonly Dictionary<Guid, TimeSpan> releasedPlayheads = new();
        private readonly HashSet<Guid> suppressed = new();
        private readonly Stack<BattlementAudioInstance> inactive = new();
        private Camera? inputCamera;
        private bool isDisposed;
        private AudioMix mix = AudioMix.FullVolume;

        public BattlementAudioSources(
            BattlementWorld world,
            BattlementPreparedAssets preparedAssets,
            Transform owner,
            DittoMotionClock motionClock
        )
        {
            this.world = world;
            this.preparedAssets = preparedAssets;
            this.motionClock = motionClock;
            var root = new GameObject("Battlement Audio Pool");
            root.transform.SetParent(owner, false);
            root.AddComponent<AudioListener>();
            poolRoot = root.transform;
            world.InputCameraChanged += Reassociate;
            Application.lowMemory += HandleLowMemory;
        }

        public IBattlementCommandOperation? Play(
            CommandId commandId,
            BattlementDirectAudioPlay command,
            TimeSpan now
        )
        {
            RequireBus(command.Bus);
            float volume = RequireVolume(command.Volume);
            float pitch = RequirePitch(command.Pitch);
            TimeSpan fadeIn = RequireDuration(
                TimeSpan.FromMilliseconds(command.FadeInMilliseconds),
                "Audio fade-in"
            );
            if (motionClock.IsInstant)
            {
                suppressed.Add(commandId.Value);
                return null;
            }
            var asset = new PreparedAsset.AudioClip(new AudioClipAddress(command.Address));
            IBattlementAssetLease lease = preparedAssets.Acquire(asset);
            return Play(commandId, command, now, lease, volume, pitch, fadeIn);
        }

        internal IBattlementCommandOperation? Play(
            CommandId commandId,
            BattlementDirectAudioPlay command,
            TimeSpan now,
            IBattlementAssetLease lease
        )
        {
            RequireBus(command.Bus);
            float volume = RequireVolume(command.Volume);
            float pitch = RequirePitch(command.Pitch);
            TimeSpan fadeIn = RequireDuration(
                TimeSpan.FromMilliseconds(command.FadeInMilliseconds),
                "Audio fade-in"
            );
            if (motionClock.IsInstant)
            {
                suppressed.Add(commandId.Value);
                lease.Dispose();
                return null;
            }
            return Play(commandId, command, now, lease, volume, pitch, fadeIn);
        }

        private IBattlementCommandOperation Play(
            CommandId commandId,
            BattlementDirectAudioPlay command,
            TimeSpan now,
            IBattlementAssetLease lease,
            float volume,
            float pitch,
            TimeSpan fadeIn
        )
        {
            BattlementAudioInstance? instance = null;
            try
            {
                if (lease.Value is not AudioClip clip)
                    throw new BattlementCommandException(
                        CoreErrorCode.AssetTypeMismatch,
                        $"Prepared audio clip '{command.Address}' is not an AudioClip."
                    );
                instance =
                    inactive.Count == 0
                        ? BattlementAudioInstance.Create(poolRoot, motionClock)
                        : inactive.Pop();
                instance.Acquire(
                    commandId.Value,
                    lease,
                    clip,
                    inputCamera,
                    volume,
                    pitch,
                    command.Loop,
                    fadeIn,
                    now,
                    command.Bus,
                    mix
                );
                lease = null!;
                live.Add(commandId.Value, instance);
                return new PlaybackOperation(this, instance);
            }
            catch
            {
                if (instance?.IsActive == true)
                    Release(instance);
                else if (instance is not null)
                    inactive.Push(instance);
                lease?.Dispose();
                throw;
            }
        }

        public IBattlementCommandOperation? Stop(BattlementDirectAudioStop command, TimeSpan now) =>
            Stop(
                command.AudioCommandId,
                TimeSpan.FromMilliseconds(command.FadeOutMilliseconds),
                now
            );

        private IBattlementCommandOperation? Stop(
            CommandId audioCommandId,
            TimeSpan requestedFadeOut,
            TimeSpan now
        )
        {
            TimeSpan fadeOut = RequireDuration(requestedFadeOut, "Audio fade-out");
            if (suppressed.Remove(audioCommandId.Value))
            {
                return null;
            }
            BattlementAudioInstance instance = Require(audioCommandId);
            instance.CancelFadeIn();
            if (fadeOut == TimeSpan.Zero)
            {
                Release(instance);
                return null;
            }

            return new FadeOutOperation(this, instance, now, fadeOut);
        }

        public IBattlementCommandOperation? Pause(CommandId audioCommandId)
        {
            Require(audioCommandId).Pause();
            return null;
        }

        public IBattlementCommandOperation? Resume(CommandId audioCommandId)
        {
            Require(audioCommandId).Resume();
            return null;
        }

        public IBattlementCommandOperation? Seek(
            CommandId audioCommandId,
            TimeSpan position,
            TimeSpan now
        )
        {
            Require(audioCommandId).Seek(position, now);
            return null;
        }

        public IBattlementCommandOperation? SetBuffering(CommandId audioCommandId, bool buffering)
        {
            Require(audioCommandId).SetBuffering(buffering);
            return null;
        }

        public IBattlementCommandOperation? Replace(
            CommandId audioCommandId,
            string address,
            TimeSpan now
        )
        {
            BattlementAudioInstance instance = Require(audioCommandId);
            var asset = new PreparedAsset.AudioClip(new AudioClipAddress(address));
            IBattlementAssetLease lease = preparedAssets.Acquire(asset);
            try
            {
                if (lease.Value is not AudioClip clip)
                {
                    throw new BattlementCommandException(
                        CoreErrorCode.AssetTypeMismatch,
                        $"Prepared audio clip '{address}' is not an AudioClip."
                    );
                }
                instance.Replace(lease, clip, now);
                lease = null!;
                return null;
            }
            finally
            {
                lease?.Dispose();
            }
        }

        public IBattlementCommandOperation? SetMix(AudioMix value)
        {
            RequireVolume(value.Master);
            RequireVolume(value.Music);
            RequireVolume(value.Effects);
            mix = value;
            foreach (BattlementAudioInstance instance in live.Values)
                instance.SetMix(value);
            return null;
        }

        private static void RequireBus(AudioBus bus)
        {
            if (bus != AudioBus.Music && bus != AudioBus.Effects)
                throw Invalid("Audio bus is unknown.");
        }

        public IBattlementCommandOperation? SetVolume(BattlementDirectAudioVolume command) =>
            SetVolume(command.AudioCommandId, command.Volume);

        private IBattlementCommandOperation? SetVolume(CommandId audioCommandId, double requested)
        {
            float volume = RequireVolume(requested);
            if (suppressed.Contains(audioCommandId.Value))
            {
                return null;
            }
            Require(audioCommandId).SetVolume(volume);
            return null;
        }

        public IBattlementCommandOperation? TweenVolume(
            BattlementDirectTweenAudioVolume command,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            float target = RequireVolume(command.Volume);
            if (suppressed.Contains(command.AudioCommandId.Value))
            {
                tweens.ValidateOnly(command.Tween);
                return null;
            }
            BattlementAudioInstance instance = Require(command.AudioCommandId);
            instance.CancelFadeIn();
            IBattlementCommandOperation? operation = tweens.Float(
                instance.Transform,
                instance.Volume,
                target,
                command.Tween,
                now,
                instance.TrySetVolume
            );
            return operation is null ? null : new ActiveAudioOperation(instance, operation);
        }

        public (TimeSpan Elapsed, bool Discontinuity) MotionTime(ObjectId playbackId)
        {
            if (live.TryGetValue(playbackId.Value, out BattlementAudioInstance instance))
            {
                return instance.MotionTime();
            }

            return releasedPlayheads.TryGetValue(playbackId.Value, out TimeSpan elapsed)
                ? (elapsed, false)
                : (TimeSpan.Zero, false);
        }

        public bool HasMotionPlayback(ObjectId playbackId) =>
            live.TryGetValue(playbackId.Value, out BattlementAudioInstance instance)
            && instance.IsActive;

        public float ReadMotionVolume(ObjectId playbackId) =>
            Require(new CommandId(playbackId.Value)).Volume;

        public void WriteMotionVolume(ObjectId playbackId, double requested) =>
            Require(new CommandId(playbackId.Value)).SetVolume(RequireVolume(requested));

        public void ClearInactive(bool clearSuppressed = false)
        {
            if (clearSuppressed)
            {
                suppressed.Clear();
            }
            while (inactive.Count > 0)
            {
                inactive.Pop().Destroy();
            }
        }

        public void Dispose()
        {
            if (isDisposed)
            {
                return;
            }

            Application.lowMemory -= HandleLowMemory;
            world.InputCameraChanged -= Reassociate;
            foreach (
                BattlementAudioInstance instance in new List<BattlementAudioInstance>(live.Values)
            )
            {
                instance.Destroy();
            }

            live.Clear();
            releasedPlayheads.Clear();
            suppressed.Clear();
            ClearInactive();
            if (poolRoot != null)
            {
                BattlementAudioInstance.DestroyUnityObject(poolRoot.gameObject);
            }

            isDisposed = true;
        }

        private void Reassociate(Camera? camera)
        {
            inputCamera = camera;
            foreach (BattlementAudioInstance instance in live.Values)
            {
                instance.Reassociate(camera);
            }
        }

        private void Release(BattlementAudioInstance instance)
        {
            if (!instance.IsActive)
            {
                return;
            }

            Guid commandId = instance.CommandId;
            releasedPlayheads[commandId] = instance.MotionTime().Elapsed;
            live.Remove(commandId);
            instance.Reset(poolRoot);
            inactive.Push(instance);
        }

        private BattlementAudioInstance Require(CommandId id)
        {
            if (
                live.TryGetValue(id.Value, out BattlementAudioInstance instance)
                && instance.IsActive
            )
            {
                return instance;
            }

            throw new BattlementCommandException(
                CoreErrorCode.UnknownCommand,
                $"Audio command UUID {id.Value} is not playing."
            );
        }

        private void HandleLowMemory()
        {
            ClearInactive();
            Resources.UnloadUnusedAssets();
        }

        private static float RequireVolume(double value)
        {
            float converted = (float)value;
            if (!double.IsFinite(value) || !float.IsFinite(converted))
            {
                throw Invalid("Audio volume must be finite and between 0 and 1.");
            }

            if (converted is < 0 or > 1)
            {
                throw Invalid("Audio volume must be finite and between 0 and 1.");
            }

            return converted;
        }

        private static float RequirePitch(double value)
        {
            float converted = (float)value;
            if (!double.IsFinite(value) || !float.IsFinite(converted))
            {
                throw Invalid("Audio pitch must be finite, greater than 0, and at most 3.");
            }

            if (converted is <= 0 or > 3)
            {
                throw Invalid("Audio pitch must be finite, greater than 0, and at most 3.");
            }

            return converted;
        }

        private static TimeSpan RequireDuration(TimeSpan value, string name) =>
            BattlementProtocolLimits.RequireDuration(value, name);

        private static BattlementCommandException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private sealed class PlaybackOperation
            : IBattlementHeldCommandOperation,
                IBattlementPausableCommandOperation
        {
            private readonly BattlementAudioSources owner;
            private readonly BattlementAudioInstance instance;
            private bool pausedByScope;
            private TimeSpan? pausedAt;

            public PlaybackOperation(
                BattlementAudioSources owner,
                BattlementAudioInstance instance
            ) => (this.owner, this.instance) = (owner, instance);

            public bool IsInfinite => instance.IsActive && instance.IsLooping;

            public bool IsHeld => instance.IsActive && instance.IsHeld;

            public bool IsComplete(TimeSpan now)
            {
                if (instance.UpdatePlayback(now))
                {
                    owner.Release(instance);
                    return true;
                }

                return false;
            }

            public void Cancel() => owner.Release(instance);

            public void Pause(TimeSpan now)
            {
                if (instance.IsPaused)
                    return;
                instance.Pause();
                pausedByScope = true;
                pausedAt = now;
            }

            public void Resume(TimeSpan now)
            {
                if (!pausedByScope)
                    return;
                pausedByScope = false;
                if (pausedAt is TimeSpan paused)
                    instance.ShiftTiming(now - paused);
                pausedAt = null;
                instance.Resume();
            }
        }

        private sealed class FadeOutOperation : IBattlementCommandOperation
        {
            private readonly BattlementAudioSources owner;
            private readonly BattlementAudioInstance instance;
            private readonly TimeSpan started;
            private readonly TimeSpan duration;
            private readonly float initialEnvelope;

            public FadeOutOperation(
                BattlementAudioSources owner,
                BattlementAudioInstance instance,
                TimeSpan started,
                TimeSpan duration
            ) =>
                (this.owner, this.instance, this.started, this.duration, initialEnvelope) = (
                    owner,
                    instance,
                    started,
                    duration,
                    instance.FadeEnvelope
                );

            public bool IsInfinite => false;

            public bool IsComplete(TimeSpan now)
            {
                if (!instance.IsActive)
                {
                    return true;
                }

                float progress = Mathf.Clamp01(
                    (float)((now - started).TotalMilliseconds / duration.TotalMilliseconds)
                );
                instance.SetFadeEnvelope(Mathf.Lerp(initialEnvelope, 0f, progress));
                if (progress < 1f)
                {
                    return false;
                }

                owner.Release(instance);
                return true;
            }

            public void Cancel() { }
        }

        private sealed class ActiveAudioOperation : IBattlementCommandOperation
        {
            private readonly BattlementAudioInstance instance;
            private readonly IBattlementCommandOperation inner;

            public ActiveAudioOperation(
                BattlementAudioInstance instance,
                IBattlementCommandOperation inner
            ) => (this.instance, this.inner) = (instance, inner);

            public bool IsInfinite => instance.IsActive && inner.IsInfinite;

            public bool IsComplete(TimeSpan now)
            {
                if (instance.IsActive)
                {
                    return inner.IsComplete(now);
                }

                inner.Cancel();
                return true;
            }

            public void Cancel() => inner.Cancel();
        }
    }
}

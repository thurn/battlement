#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Battlement
{
    internal sealed class BattlementAudioSources : IDisposable
    {
        private readonly BattlementWorld world;
        private readonly BattlementPreparedAssets preparedAssets;
        private readonly DittoMotionClock motionClock;
        private readonly Transform poolRoot;
        private readonly Dictionary<Guid, AudioInstance> live = new();
        private readonly HashSet<Guid> suppressed = new();
        private readonly Stack<AudioInstance> inactive = new();
        private Camera? inputCamera;
        private bool isDisposed;

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
            CommandBody.Audio.Play command,
            TimeSpan now
        )
        {
            float volume = RequireVolume(command.Volume);
            float pitch = RequirePitch(command.Pitch);
            TimeSpan fadeIn = RequireDuration(command.FadeIn, "Audio fade-in");
            if (motionClock.IsInstant)
            {
                suppressed.Add(commandId.Value);
                return null;
            }
            var asset = new PreparedAsset.AudioClip(command.Address);
            IBattlementAssetLease lease = preparedAssets.Acquire(asset);
            AudioInstance? instance = null;
            try
            {
                if (lease.Value is not AudioClip clip)
                {
                    throw new BattlementCommandException(
                        CoreErrorCode.AssetTypeMismatch,
                        $"Prepared audio clip '{command.Address.Value}' is not an AudioClip."
                    );
                }

                instance = inactive.Count == 0 ? AudioInstance.Create(poolRoot) : inactive.Pop();
                instance.Acquire(
                    commandId.Value,
                    lease,
                    clip,
                    inputCamera,
                    volume,
                    pitch,
                    command.Loop,
                    fadeIn,
                    now
                );
                lease = null!;
                live.Add(commandId.Value, instance);
                return new PlaybackOperation(this, instance);
            }
            catch
            {
                if (instance?.IsActive == true)
                {
                    Release(instance);
                }
                else if (instance is not null)
                {
                    inactive.Push(instance);
                }

                lease?.Dispose();
                throw;
            }
        }

        public IBattlementCommandOperation? Stop(CommandBody.Audio.Stop command, TimeSpan now)
        {
            TimeSpan fadeOut = RequireDuration(command.FadeOut, "Audio fade-out");
            if (suppressed.Remove(command.AudioCommandId.Value))
            {
                return null;
            }
            AudioInstance instance = Require(command.AudioCommandId);
            instance.CancelFadeIn();
            if (fadeOut == TimeSpan.Zero)
            {
                Release(instance);
                return null;
            }

            return new FadeOutOperation(this, instance, now, fadeOut);
        }

        public IBattlementCommandOperation? SetVolume(CommandBody.Audio.SetVolume command)
        {
            float volume = RequireVolume(command.Volume);
            if (suppressed.Contains(command.AudioCommandId.Value))
            {
                return null;
            }
            Require(command.AudioCommandId).SetVolume(volume);
            return null;
        }

        public IBattlementCommandOperation? TweenVolume(
            CommandBody.Audio.TweenVolume command,
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
            AudioInstance instance = Require(command.AudioCommandId);
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
            foreach (AudioInstance instance in new List<AudioInstance>(live.Values))
            {
                instance.Destroy();
            }

            live.Clear();
            suppressed.Clear();
            ClearInactive();
            if (poolRoot != null)
            {
                DestroyUnityObject(poolRoot.gameObject);
            }

            isDisposed = true;
        }

        private void Reassociate(Camera? camera)
        {
            inputCamera = camera;
            foreach (AudioInstance instance in live.Values)
            {
                instance.Reassociate(camera);
            }
        }

        private void Release(AudioInstance instance)
        {
            if (!instance.IsActive)
            {
                return;
            }

            live.Remove(instance.CommandId);
            instance.Reset(poolRoot);
            inactive.Push(instance);
        }

        private AudioInstance Require(CommandId id)
        {
            if (live.TryGetValue(id.Value, out AudioInstance instance) && instance.IsActive)
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

        private sealed class AudioInstance
        {
            private readonly AudioSource source;
            private IBattlementAssetLease? lease;
            private TimeSpan started;
            private TimeSpan fadeIn;
            private TimeSpan completion;
            private float requestedVolume;

            private AudioInstance(AudioSource source) => this.source = source;

            public Guid CommandId { get; private set; }

            public bool IsActive => CommandId != Guid.Empty;

            public bool IsLooping => source.loop;

            public Transform Transform => source.transform;

            public float Volume => source.volume;

            public static AudioInstance Create(Transform poolRoot)
            {
                var gameObject = new GameObject("Battlement Audio Source");
                gameObject.transform.SetParent(poolRoot, false);
                AudioSource source = gameObject.AddComponent<AudioSource>();
                source.playOnAwake = false;
                source.spatialBlend = 0f;
                source.dopplerLevel = 0f;
                gameObject.SetActive(false);
                return new AudioInstance(source);
            }

            public void Acquire(
                Guid commandId,
                IBattlementAssetLease assetLease,
                AudioClip clip,
                Camera? camera,
                float volume,
                float pitch,
                bool loop,
                TimeSpan fadeDuration,
                TimeSpan now
            )
            {
                CommandId = commandId;
                lease = assetLease;
                requestedVolume = volume;
                fadeIn = fadeDuration;
                started = now;
                completion = now + TimeSpan.FromSeconds(clip.length / pitch);
                source.clip = clip;
                source.pitch = pitch;
                source.loop = loop;
                source.volume = fadeDuration == TimeSpan.Zero ? volume : 0f;
                Reassociate(camera);
                source.gameObject.SetActive(true);
                source.Play();
            }

            public bool UpdatePlayback(TimeSpan now)
            {
                if (!IsActive)
                {
                    return true;
                }

                if (fadeIn > TimeSpan.Zero)
                {
                    double progress = (now - started).TotalMilliseconds / fadeIn.TotalMilliseconds;
                    source.volume = Mathf.Lerp(0f, requestedVolume, Mathf.Clamp01((float)progress));
                    if (progress >= 1)
                    {
                        fadeIn = TimeSpan.Zero;
                    }
                }

                return !source.loop
                    && (now >= completion || (Application.isPlaying && !source.isPlaying));
            }

            public void CancelFadeIn() => fadeIn = TimeSpan.Zero;

            public void SetVolume(float value)
            {
                CancelFadeIn();
                source.volume = value;
                requestedVolume = value;
            }

            public void TrySetVolume(float value)
            {
                if (IsActive)
                {
                    source.volume = value;
                    requestedVolume = value;
                }
            }

            public void Reassociate(Camera? camera)
            {
                source.transform.SetParent(camera == null ? null : camera.transform, false);
                source.transform.SetLocalPositionAndRotation(
                    UnityEngine.Vector3.zero,
                    UnityEngine.Quaternion.identity
                );
                source.transform.localScale = UnityEngine.Vector3.one;
            }

            public void Reset(Transform poolRoot)
            {
                source.Stop();
                source.clip = null;
                source.loop = false;
                source.pitch = 1f;
                source.volume = 1f;
                source.transform.SetParent(poolRoot, false);
                source.gameObject.SetActive(false);
                CommandId = Guid.Empty;
                lease?.Dispose();
                lease = null;
            }

            public void Destroy()
            {
                if (IsActive)
                {
                    source.Stop();
                    source.clip = null;
                    CommandId = Guid.Empty;
                    lease?.Dispose();
                    lease = null;
                }

                if (source != null)
                {
                    DestroyUnityObject(source.gameObject);
                }
            }
        }

        private sealed class PlaybackOperation : IBattlementCommandOperation
        {
            private readonly BattlementAudioSources owner;
            private readonly AudioInstance instance;

            public PlaybackOperation(BattlementAudioSources owner, AudioInstance instance) =>
                (this.owner, this.instance) = (owner, instance);

            public bool IsInfinite => instance.IsActive && instance.IsLooping;

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
        }

        private sealed class FadeOutOperation : IBattlementCommandOperation
        {
            private readonly BattlementAudioSources owner;
            private readonly AudioInstance instance;
            private readonly TimeSpan started;
            private readonly TimeSpan duration;
            private readonly float initialVolume;

            public FadeOutOperation(
                BattlementAudioSources owner,
                AudioInstance instance,
                TimeSpan started,
                TimeSpan duration
            ) =>
                (this.owner, this.instance, this.started, this.duration, initialVolume) = (
                    owner,
                    instance,
                    started,
                    duration,
                    instance.Volume
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
                instance.TrySetVolume(Mathf.Lerp(initialVolume, 0f, progress));
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
            private readonly AudioInstance instance;
            private readonly IBattlementCommandOperation inner;

            public ActiveAudioOperation(
                AudioInstance instance,
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

        private static void DestroyUnityObject(Object value)
        {
            if (Application.isPlaying)
            {
                Object.Destroy(value);
            }
            else
            {
                Object.DestroyImmediate(value);
            }
        }
    }
}

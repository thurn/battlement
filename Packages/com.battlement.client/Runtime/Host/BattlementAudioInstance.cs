#nullable enable

using System;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Battlement
{
    internal sealed class BattlementAudioInstance
    {
        private readonly AudioSource source;
        private IBattlementAssetLease? lease;
        private TimeSpan started;
        private TimeSpan fadeIn;
        private TimeSpan completion;
        private float requestedVolume;
        private float fadeEnvelope = 1f;
        private float mixGain = 1f;
        private AudioBus bus;
        private int previousTimeSamples;
        private bool discontinuity;
        private bool paused;
        private bool buffering;

        private BattlementAudioInstance(AudioSource source) => this.source = source;

        public Guid CommandId { get; private set; }

        public bool IsActive => CommandId != Guid.Empty;

        public bool IsLooping => source.loop;

        public bool IsPaused => paused;

        public Transform Transform => source.transform;

        public float Volume => requestedVolume;

        public float FadeEnvelope => fadeEnvelope;

        public static BattlementAudioInstance Create(Transform poolRoot)
        {
            var gameObject = new GameObject("Battlement Audio Source");
            gameObject.transform.SetParent(poolRoot, false);
            AudioSource source = gameObject.AddComponent<AudioSource>();
            source.playOnAwake = false;
            source.spatialBlend = 0f;
            source.dopplerLevel = 0f;
            gameObject.SetActive(false);
            return new BattlementAudioInstance(source);
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
            TimeSpan now,
            AudioBus bus,
            AudioMix mix
        )
        {
            CommandId = commandId;
            lease = assetLease;
            requestedVolume = volume;
            this.bus = bus;
            mixGain = mix.Gain(bus);
            fadeIn = fadeDuration;
            started = now;
            completion = now + TimeSpan.FromSeconds(clip.length / pitch);
            source.clip = clip;
            source.pitch = pitch;
            source.loop = loop;
            fadeEnvelope = fadeDuration == TimeSpan.Zero ? 1f : 0f;
            ApplyVolume();
            Reassociate(camera);
            source.gameObject.SetActive(true);
            source.Play();
            previousTimeSamples = 0;
            discontinuity = true;
            paused = false;
            buffering = false;
        }

        public (TimeSpan Elapsed, bool Discontinuity) MotionTime()
        {
            int sample = source.timeSamples;
            bool jumped = discontinuity || (source.loop && sample < previousTimeSamples);
            discontinuity = false;
            previousTimeSamples = sample;
            int frequency = source.clip == null ? 0 : source.clip.frequency;
            return frequency <= 0
                ? (TimeSpan.Zero, jumped)
                : (TimeSpan.FromSeconds((double)sample / frequency), jumped);
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
                fadeEnvelope = Mathf.Clamp01((float)progress);
                ApplyVolume();
                if (progress >= 1)
                {
                    fadeIn = TimeSpan.Zero;
                }
            }

            return !source.loop
                && !paused
                && !buffering
                && (now >= completion || (Application.isPlaying && !source.isPlaying));
        }

        public void CancelFadeIn()
        {
            fadeIn = TimeSpan.Zero;
            requestedVolume *= fadeEnvelope;
            fadeEnvelope = 1f;
            ApplyVolume();
        }

        public void SetMix(AudioMix mix)
        {
            mixGain = mix.Gain(bus);
            ApplyVolume();
        }

        public void SetFadeEnvelope(float value)
        {
            fadeEnvelope = value;
            ApplyVolume();
        }

        private void ApplyVolume() => source.volume = requestedVolume * fadeEnvelope * mixGain;

        public void Pause()
        {
            paused = true;
            source.Pause();
        }

        public void Resume()
        {
            paused = false;
            if (!buffering)
                source.UnPause();
        }

        public void ShiftTiming(TimeSpan offset)
        {
            started += offset;
            completion += offset;
        }

        public void SetBuffering(bool value)
        {
            buffering = value;
            if (value)
                source.Pause();
            else if (!paused)
                source.UnPause();
        }

        public void Seek(TimeSpan position, TimeSpan now)
        {
            if (position < TimeSpan.Zero)
                throw new BattlementCommandException(
                    CoreErrorCode.InvalidProperty,
                    "Audio seek position must be nonnegative."
                );
            AudioClip clip = source.clip;
            double seconds = position.TotalSeconds;
            if (source.loop && clip.length > 0)
                seconds %= clip.length;
            else
                seconds = Math.Min(seconds, clip.length);
            int sample = Math.Min(
                Math.Max(0, clip.samples - 1),
                checked((int)Math.Round(seconds * clip.frequency))
            );
            source.timeSamples = sample;
            previousTimeSamples = sample;
            completion = now + TimeSpan.FromSeconds((clip.length - seconds) / source.pitch);
            discontinuity = true;
        }

        public void Replace(IBattlementAssetLease assetLease, AudioClip clip, TimeSpan now)
        {
            source.Stop();
            lease?.Dispose();
            lease = assetLease;
            source.clip = clip;
            source.timeSamples = 0;
            completion = now + TimeSpan.FromSeconds(clip.length / source.pitch);
            previousTimeSamples = 0;
            discontinuity = true;
            source.Play();
            if (paused || buffering)
                source.Pause();
        }

        public void SetVolume(float value)
        {
            CancelFadeIn();
            requestedVolume = value;
            ApplyVolume();
        }

        public void TrySetVolume(float value)
        {
            if (IsActive)
            {
                requestedVolume = value;
                ApplyVolume();
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
            fadeEnvelope = 1f;
            mixGain = 1f;
            requestedVolume = 1f;
            bus = AudioBus.Effects;
            paused = false;
            buffering = false;
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

        internal static void DestroyUnityObject(Object value)
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

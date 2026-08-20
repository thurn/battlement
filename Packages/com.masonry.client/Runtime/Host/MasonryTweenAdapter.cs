#nullable enable

using System;
using UnityEngine;
using PrimeEase = PrimeTween.Ease;
using PrimeTweenHandle = PrimeTween.Tween;

namespace Masonry
{
    internal sealed class MasonryTweenAdapter
    {
        private const uint MaximumAdditionalTraversals = 10_000;
        private static readonly TimeSpan MaximumTime = TimeSpan.FromDays(1);

        private readonly bool useInstantAnimations;
        private readonly bool useDeterministicClock;

        public MasonryTweenAdapter(bool useInstantAnimations, bool useDeterministicClock) =>
            (this.useInstantAnimations, this.useDeterministicClock) = (
                useInstantAnimations,
                useDeterministicClock
            );

        public IMasonryCommandOperation? Vector(
            Transform target,
            UnityEngine.Vector3 start,
            UnityEngine.Vector3 end,
            Tween settings,
            TimeSpan now,
            Action<Transform, UnityEngine.Vector3> apply
        ) =>
            Start(
                target,
                settings,
                now,
                factor => apply(target, UnityEngine.Vector3.LerpUnclamped(start, end, factor))
            );

        public IMasonryCommandOperation? Rotation(
            Transform target,
            UnityEngine.Quaternion start,
            UnityEngine.Quaternion end,
            Tween settings,
            TimeSpan now,
            Action<Transform, UnityEngine.Quaternion> apply
        ) =>
            Start(
                target,
                settings,
                now,
                factor => apply(target, UnityEngine.Quaternion.SlerpUnclamped(start, end, factor))
            );

        public IMasonryCommandOperation? Float(
            Transform target,
            float start,
            float end,
            Tween settings,
            TimeSpan now,
            Action<float> apply
        ) => Start(target, settings, now, factor => apply(Mathf.LerpUnclamped(start, end, factor)));

        public IMasonryCommandOperation? Color(
            Transform target,
            UnityEngine.Color start,
            UnityEngine.Color end,
            Tween settings,
            TimeSpan now,
            Action<UnityEngine.Color> apply
        ) =>
            Start(
                target,
                settings,
                now,
                factor => apply(UnityEngine.Color.LerpUnclamped(start, end, factor))
            );

        public static Tween? For(CommandBody body) =>
            body switch
            {
                CommandBody.Transform.TweenLocalPosition value => value.Tween,
                CommandBody.Transform.TweenWorldPosition value => value.Tween,
                CommandBody.Transform.TweenLocalRotation value => value.Tween,
                CommandBody.Transform.TweenWorldRotation value => value.Tween,
                CommandBody.Transform.TweenLocalScale value => value.Tween,
                CommandBody.Camera.TweenFieldOfView value => value.Tween,
                CommandBody.Camera.TweenOrthographicSize value => value.Tween,
                CommandBody.Light.TweenColor value => value.Tween,
                CommandBody.Light.TweenIntensity value => value.Tween,
                CommandBody.Image.TweenTint value => value.Tween,
                CommandBody.Image.TweenOpacity value => value.Tween,
                CommandBody.Text.TweenSize value => value.Tween,
                CommandBody.Text.TweenColor value => value.Tween,
                CommandBody.Audio.TweenVolume value => value.Tween,
                _ => null,
            };

        public static bool IsForever(Tween? settings) => settings?.Repeat is TweenRepeat.Forever;

        private IMasonryCommandOperation? Start(
            Transform target,
            Tween settings,
            TimeSpan now,
            Action<float> apply
        )
        {
            TweenTiming timing = Validate(settings);
            if (useInstantAnimations)
            {
                apply(timing.FinalFactor);
                return null;
            }

            if (useDeterministicClock)
            {
                return new DeterministicOperation(target, timing, now, apply);
            }

            PrimeTweenHandle tween = PrimeTweenHandle.Custom(
                target,
                0f,
                1f,
                timing.DurationSeconds,
                (_, factor) => apply(factor),
                timing.Ease,
                timing.Cycles,
                timing.CycleMode,
                timing.DelaySeconds,
                useUnscaledTime: true
            );
            return tween.isAlive ? new PrimeTweenOperation(target, tween, timing.IsInfinite) : null;
        }

        private static TweenTiming Validate(Tween settings)
        {
            if (settings is null)
            {
                throw Invalid("Tween settings are required.");
            }

            RequireTime(settings.Duration, "Tween duration");
            RequireTime(settings.Delay, "Tween delay");
            if (!Enum.IsDefined(typeof(Easing), settings.Easing))
            {
                throw Invalid("Tween easing is unknown.");
            }

            int cycles;
            bool isInfinite;
            RepeatMode mode;
            switch (settings.Repeat)
            {
                case TweenRepeat.Once:
                    (cycles, isInfinite, mode) = (1, false, RepeatMode.Restart);
                    break;
                case TweenRepeat.Count count:
                    if (count.AdditionalTraversals > MaximumAdditionalTraversals)
                    {
                        throw new MasonryCommandException(
                            CoreErrorCode.LimitExceeded,
                            $"A tween may repeat at most {MaximumAdditionalTraversals} times."
                        );
                    }

                    RequireMode(count.Mode);
                    (cycles, isInfinite, mode) = (
                        checked((int)count.AdditionalTraversals + 1),
                        false,
                        count.Mode
                    );
                    break;
                case TweenRepeat.Forever forever:
                    RequireMode(forever.Mode);
                    (cycles, isInfinite, mode) = (-1, true, forever.Mode);
                    break;
                default:
                    throw Invalid("Tween repeat behavior is unknown.");
            }

            if (settings.Duration == TimeSpan.Zero && (isInfinite || cycles > 1))
            {
                throw Invalid("A zero-duration tween cannot repeat.");
            }

            return new TweenTiming(
                settings.Duration,
                settings.Delay,
                ToPrime(settings.Easing),
                cycles,
                mode == RepeatMode.Restart
                    ? PrimeTween.CycleMode.Restart
                    : PrimeTween.CycleMode.Yoyo,
                isInfinite
            );
        }

        private static void RequireTime(TimeSpan value, string name)
        {
            if (value < TimeSpan.Zero)
            {
                throw Invalid($"{name} cannot be negative.");
            }

            if (value > MaximumTime)
            {
                throw new MasonryCommandException(
                    CoreErrorCode.LimitExceeded,
                    $"{name} cannot exceed {MaximumTime.TotalMilliseconds} milliseconds."
                );
            }
        }

        private static void RequireMode(RepeatMode mode)
        {
            if (!Enum.IsDefined(typeof(RepeatMode), mode))
            {
                throw Invalid("Tween repeat mode is unknown.");
            }
        }

        private static PrimeEase ToPrime(Easing easing) =>
            easing switch
            {
                Easing.Linear => PrimeEase.Linear,
                Easing.InSine => PrimeEase.InSine,
                Easing.OutSine => PrimeEase.OutSine,
                Easing.InOutSine => PrimeEase.InOutSine,
                Easing.InQuad => PrimeEase.InQuad,
                Easing.OutQuad => PrimeEase.OutQuad,
                Easing.InOutQuad => PrimeEase.InOutQuad,
                Easing.InCubic => PrimeEase.InCubic,
                Easing.OutCubic => PrimeEase.OutCubic,
                Easing.InOutCubic => PrimeEase.InOutCubic,
                Easing.InQuart => PrimeEase.InQuart,
                Easing.OutQuart => PrimeEase.OutQuart,
                Easing.InOutQuart => PrimeEase.InOutQuart,
                Easing.InQuint => PrimeEase.InQuint,
                Easing.OutQuint => PrimeEase.OutQuint,
                Easing.InOutQuint => PrimeEase.InOutQuint,
                Easing.InExpo => PrimeEase.InExpo,
                Easing.OutExpo => PrimeEase.OutExpo,
                Easing.InOutExpo => PrimeEase.InOutExpo,
                Easing.InCirc => PrimeEase.InCirc,
                Easing.OutCirc => PrimeEase.OutCirc,
                Easing.InOutCirc => PrimeEase.InOutCirc,
                Easing.InBack => PrimeEase.InBack,
                Easing.OutBack => PrimeEase.OutBack,
                Easing.InOutBack => PrimeEase.InOutBack,
                Easing.InElastic => PrimeEase.InElastic,
                Easing.OutElastic => PrimeEase.OutElastic,
                Easing.InOutElastic => PrimeEase.InOutElastic,
                Easing.InBounce => PrimeEase.InBounce,
                Easing.OutBounce => PrimeEase.OutBounce,
                Easing.InOutBounce => PrimeEase.InOutBounce,
                _ => throw Invalid("Tween easing is unknown."),
            };

        private static MasonryCommandException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private sealed class PrimeTweenOperation : IMasonryCommandOperation
        {
            private readonly Transform target;
            private PrimeTweenHandle tween;

            public PrimeTweenOperation(Transform target, PrimeTweenHandle tween, bool isInfinite) =>
                (this.target, this.tween, IsInfinite) = (target, tween, isInfinite);

            public bool IsInfinite { get; }

            public bool IsComplete(TimeSpan now)
            {
                if (target != null)
                {
                    return !tween.isAlive;
                }

                tween.Stop();
                return true;
            }

            public void Cancel() => tween.Stop();
        }

        private sealed class DeterministicOperation : IMasonryCommandOperation
        {
            private readonly Transform target;
            private readonly TweenTiming timing;
            private readonly TimeSpan started;
            private readonly Action<float> apply;
            private bool isCanceled;

            public DeterministicOperation(
                Transform target,
                TweenTiming timing,
                TimeSpan started,
                Action<float> apply
            ) =>
                (this.target, this.timing, this.started, this.apply) = (
                    target,
                    timing,
                    started,
                    apply
                );

            public bool IsInfinite => timing.IsInfinite;

            public bool IsComplete(TimeSpan now)
            {
                if (isCanceled || target == null)
                {
                    return true;
                }

                TimeSpan elapsed = now - started;
                if (elapsed < timing.Delay)
                {
                    return false;
                }

                TimeSpan active = elapsed - timing.Delay;
                if (timing.Duration == TimeSpan.Zero)
                {
                    apply(1f);
                    return true;
                }

                if (!timing.IsInfinite && active >= timing.TotalDuration)
                {
                    apply(timing.FinalFactor);
                    return true;
                }

                long traversal = active.Ticks / timing.Duration.Ticks;
                float progress =
                    (float)(active.Ticks % timing.Duration.Ticks) / timing.Duration.Ticks;
                float eased = PrimeTween.Easing.Evaluate(progress, timing.Ease);
                apply(timing.IsPingPong && traversal % 2 == 1 ? 1f - eased : eased);
                return false;
            }

            public void Cancel() => isCanceled = true;
        }

        private sealed record TweenTiming(
            TimeSpan Duration,
            TimeSpan Delay,
            PrimeEase Ease,
            int Cycles,
            PrimeTween.CycleMode CycleMode,
            bool IsInfinite
        )
        {
            public float DurationSeconds => (float)Duration.TotalSeconds;

            public float DelaySeconds => (float)Delay.TotalSeconds;

            public bool IsPingPong => CycleMode == PrimeTween.CycleMode.Yoyo;

            public TimeSpan TotalDuration => TimeSpan.FromTicks(Duration.Ticks * Cycles);

            public float FinalFactor => IsPingPong && Cycles % 2 == 0 ? 0f : 1f;
        }
    }
}

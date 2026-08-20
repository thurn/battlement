#nullable enable

using System;
using System.Linq;
using UnityEngine;

namespace Masonry
{
    internal static class MasonryAnimatorCommands
    {
        public static IMasonryCommandOperation? Play(
            CommandBody.Animator.Play command,
            MasonryWorld world,
            TimeSpan now
        )
        {
            Animator animator = RequireAnimator(command.ObjectId, world);
            int layer = RequireState(animator, command.State, command.Layer);
            float startTime = RequireUnit(
                command.NormalizedStartTime,
                "Animator normalized start time"
            );
            TimeSpan completion = RequireWait(command.Wait);
            animator.Play(Animator.StringToHash(command.State), layer, startTime);
            animator.Update(0);
            return Wait(completion, now);
        }

        public static IMasonryCommandOperation? CrossFade(
            CommandBody.Animator.CrossFade command,
            MasonryWorld world,
            TimeSpan now
        )
        {
            Animator animator = RequireAnimator(command.ObjectId, world);
            int layer = RequireState(animator, command.State, command.Layer);
            float startTime = RequireUnit(
                command.NormalizedStartTime,
                "Animator normalized start time"
            );
            float duration = RequirePositiveSeconds(
                command.CrossFadeDuration,
                "Animator cross-fade duration"
            );
            TimeSpan completion = RequireWait(command.Wait);
            animator.CrossFade(
                Animator.StringToHash(command.State),
                RequireNormalizedTransitionDuration(animator, layer, duration),
                layer,
                startTime
            );
            animator.Update(0);
            return Wait(completion, now);
        }

        public static IMasonryCommandOperation? SetBool(
            CommandBody.Animator.SetBool command,
            MasonryWorld world
        )
        {
            Animator animator = RequireAnimator(command.ObjectId, world);
            animator.SetBool(
                RequireParameter(animator, command.Parameter, AnimatorControllerParameterType.Bool),
                command.Value
            );
            return null;
        }

        public static IMasonryCommandOperation? SetInt(
            CommandBody.Animator.SetInt command,
            MasonryWorld world
        )
        {
            Animator animator = RequireAnimator(command.ObjectId, world);
            animator.SetInteger(
                RequireParameter(animator, command.Parameter, AnimatorControllerParameterType.Int),
                command.Value
            );
            return null;
        }

        public static IMasonryCommandOperation? SetFloat(
            CommandBody.Animator.SetFloat command,
            MasonryWorld world
        )
        {
            Animator animator = RequireAnimator(command.ObjectId, world);
            animator.SetFloat(
                RequireParameter(
                    animator,
                    command.Parameter,
                    AnimatorControllerParameterType.Float
                ),
                RequireFinite(command.Value, $"Animator parameter '{command.Parameter}'")
            );
            return null;
        }

        public static IMasonryCommandOperation? SetTrigger(
            CommandBody.Animator.SetTrigger command,
            MasonryWorld world
        )
        {
            Animator animator = RequireAnimator(command.ObjectId, world);
            animator.SetTrigger(
                RequireParameter(
                    animator,
                    command.Parameter,
                    AnimatorControllerParameterType.Trigger
                )
            );
            return null;
        }

        public static IMasonryCommandOperation? SetSpeed(
            CommandBody.Animator.SetSpeed command,
            MasonryWorld world
        )
        {
            RequireAnimator(command.ObjectId, world).speed = RequireNonnegative(
                command.Speed,
                "Animator speed"
            );
            return null;
        }

        private static Animator RequireAnimator(ObjectId objectId, MasonryWorld world)
        {
            Animator[] animators = world.RequireObject(objectId).GetComponents<Animator>();
            if (animators.Length != 1)
            {
                throw new MasonryCommandException(
                    animators.Length == 0
                        ? CoreErrorCode.ComponentMissing
                        : CoreErrorCode.InvalidComponentCount,
                    "Animator command requires exactly one root Animator; "
                        + $"found {animators.Length}."
                );
            }

            return animators[0];
        }

        private static int RequireState(Animator animator, string state, uint layer)
        {
            if (layer >= animator.layerCount)
            {
                throw Invalid($"Animator layer {layer} does not exist.");
            }

            int convertedLayer = checked((int)layer);
            if (
                string.IsNullOrEmpty(state)
                || !animator.HasState(convertedLayer, Animator.StringToHash(state))
            )
            {
                throw Invalid($"Animator state '{state}' does not exist on layer {layer}.");
            }

            return convertedLayer;
        }

        private static int RequireParameter(
            Animator animator,
            string name,
            AnimatorControllerParameterType expectedType
        )
        {
            AnimatorControllerParameter? parameter = animator.parameters.FirstOrDefault(candidate =>
                candidate.name == name
            );
            if (parameter == null || parameter.type != expectedType)
            {
                throw Invalid($"Animator parameter '{name}' is missing or has the wrong type.");
            }

            return parameter.nameHash;
        }

        private static TimeSpan RequireWait(TimeSpan value) =>
            MasonryProtocolLimits.RequireDuration(value, "Animator wait duration");

        private static float RequirePositiveSeconds(TimeSpan value, string name)
        {
            return (float)
                MasonryProtocolLimits.RequireDuration(value, name, allowZero: false).TotalSeconds;
        }

        private static float RequireNormalizedTransitionDuration(
            Animator animator,
            int layer,
            float duration
        )
        {
            float sourceDuration = animator.GetCurrentAnimatorStateInfo(layer).length;
            if (!float.IsFinite(sourceDuration) || sourceDuration <= 0)
            {
                throw Invalid("Animator source state must have a positive finite duration.");
            }

            float normalized = duration / sourceDuration;
            return float.IsFinite(normalized)
                ? normalized
                : throw Invalid("Animator cross-fade duration is too large.");
        }

        private static float RequireUnit(double value, string name) =>
            double.IsFinite(value) && value is >= 0 and <= 1
                ? (float)value
                : throw Invalid($"{name} must be in the inclusive range [0, 1].");

        private static float RequireNonnegative(double value, string name)
        {
            float converted = RequireFinite(value, name);
            return converted >= 0 ? converted : throw Invalid($"{name} must be nonnegative.");
        }

        private static float RequireFinite(double value, string name)
        {
            float converted = (float)value;
            return double.IsFinite(value) && float.IsFinite(converted)
                ? converted
                : throw Invalid($"{name} must be finite.");
        }

        private static IMasonryCommandOperation? Wait(TimeSpan duration, TimeSpan now) =>
            duration == TimeSpan.Zero
                ? null
                : MasonryTimeCommands.Wait(new CommandBody.Time.Wait(duration), now);

        private static MasonryCommandException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}

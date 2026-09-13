#nullable enable

using System;
using System.Linq;
using UnityEngine;

namespace Battlement
{
    internal static class BattlementAnimatorCommands
    {
        public static IBattlementCommandOperation? Launch(
            BattlementDirectAnimatorCommand command,
            BattlementWorld world,
            TimeSpan now,
            bool skipWait
        )
        {
            Animator animator = RequireAnimator(command.ObjectId, world);
            switch (command.Kind)
            {
                case BattlementDirectAnimatorCommandKind.Play:
                case BattlementDirectAnimatorCommandKind.CrossFade:
                {
                    int layer = RequireState(animator, command.Value, command.Layer);
                    float start = RequireUnit(command.Number, "Animator normalized start time");
                    if (command.Kind == BattlementDirectAnimatorCommandKind.CrossFade)
                    {
                        float duration = RequirePositiveSeconds(
                            TimeSpan.FromMilliseconds(command.CrossFadeMilliseconds),
                            "Animator cross-fade duration"
                        );
                        animator.CrossFade(
                            Animator.StringToHash(command.Value),
                            RequireNormalizedTransitionDuration(animator, layer, duration),
                            layer,
                            start
                        );
                    }
                    else
                    {
                        animator.Play(Animator.StringToHash(command.Value), layer, start);
                    }
                    animator.Update(0);
                    return Wait(
                        RequireWait(TimeSpan.FromMilliseconds(command.WaitMilliseconds)),
                        now,
                        skipWait
                    );
                }
                case BattlementDirectAnimatorCommandKind.SetBool:
                    animator.SetBool(
                        RequireParameter(
                            animator,
                            command.Value,
                            AnimatorControllerParameterType.Bool
                        ),
                        command.Enabled
                    );
                    return null;
                case BattlementDirectAnimatorCommandKind.SetInt:
                    animator.SetInteger(
                        RequireParameter(
                            animator,
                            command.Value,
                            AnimatorControllerParameterType.Int
                        ),
                        checked((int)command.Integer)
                    );
                    return null;
                case BattlementDirectAnimatorCommandKind.SetFloat:
                    animator.SetFloat(
                        RequireParameter(
                            animator,
                            command.Value,
                            AnimatorControllerParameterType.Float
                        ),
                        RequireFinite(command.Number, $"Animator parameter '{command.Value}'")
                    );
                    return null;
                case BattlementDirectAnimatorCommandKind.SetTrigger:
                    animator.SetTrigger(
                        RequireParameter(
                            animator,
                            command.Value,
                            AnimatorControllerParameterType.Trigger
                        )
                    );
                    return null;
                case BattlementDirectAnimatorCommandKind.SetSpeed:
                    animator.speed = RequireNonnegative(command.Number, "Animator speed");
                    return null;
                default:
                    throw Invalid("An animator command kind is unknown.");
            }
        }

        private static Animator RequireAnimator(ObjectId objectId, BattlementWorld world)
        {
            Animator[] animators = world.RequireObject(objectId).GetComponents<Animator>();
            if (animators.Length != 1)
            {
                throw new BattlementCommandException(
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
            BattlementProtocolLimits.RequireDuration(value, "Animator wait duration");

        private static float RequirePositiveSeconds(TimeSpan value, string name)
        {
            return (float)
                BattlementProtocolLimits
                    .RequireDuration(value, name, allowZero: false)
                    .TotalSeconds;
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

        private static IBattlementCommandOperation? Wait(
            TimeSpan duration,
            TimeSpan now,
            bool skipWait
        ) =>
            duration == TimeSpan.Zero || skipWait
                ? null
                : BattlementTimeCommands.Wait(
                    new BattlementDirectWait(checked((ulong)duration.TotalMilliseconds)),
                    now
                );

        private static BattlementCommandException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}

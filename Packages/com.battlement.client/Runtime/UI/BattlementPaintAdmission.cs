#nullable enable

using System;
using System.Linq;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    /// <summary>Checks compositing ownership before a sparse update changes the host.</summary>
    internal static class BattlementPaintAdmission
    {
        public static void Validate(
            VisualElement target,
            Prop<PaintStyle> paint,
            MotionDescriptor? motion
        )
        {
            PaintBlendMode? blend = paint.IsSet ? paint.Value.BlendMode : null;
            if (paint.IsUnset && BattlementAdvancedPaint.TryGet(target, out var current))
                blend = current.StaticBlendMode;
            if (!OwnsMaterial(blend))
                return;
            if (target is not BattlementPaintHost)
                throw Invalid("Subtree blend modes require a decorative View host.");
            if (motion == null)
                return;
            ValidateTarget(motion.Initial, blend);
            foreach (MotionSlotDescriptor slot in motion.Slots)
                ValidateTarget(slot.Target, blend);
            foreach (
                MotionNamedTarget named in motion.NamedTargets ?? Array.Empty<MotionNamedTarget>()
            )
                ValidateTarget(named.Target, blend);
            foreach (
                MotionPseudoStyle style in motion.PseudoStyles ?? Array.Empty<MotionPseudoStyle>()
            )
                if (style.Values.Any(value => value.Property == MotionProperty.UnityMaterial))
                    ValidateMaterial(blend);
            foreach (
                CssAnimationDescriptor animation in motion.Animations
                    ?? Array.Empty<CssAnimationDescriptor>()
            )
                if (animation.Tracks.Any(track => track.Property == MotionProperty.UnityMaterial))
                    ValidateMaterial(blend);
            if (
                motion.ValueBindings?.Any(binding =>
                    binding.Property == MotionProperty.UnityMaterial
                ) == true
            )
                ValidateMaterial(blend);
        }

        public static void ValidateMaterial(PaintBlendMode? blend)
        {
            if (OwnsMaterial(blend))
                throw Invalid(
                    "Subtree blending owns the host material; animate material on a child instead."
                );
        }

        private static void ValidateTarget(MotionTargetDescriptor? target, PaintBlendMode? blend)
        {
            if (target == null)
                return;
            if (target.Tracks.Any(track => track.Property == MotionProperty.UnityMaterial))
                ValidateMaterial(blend);
            if (target.TransitionEnd.Any(value => value.Property == MotionProperty.UnityMaterial))
                ValidateMaterial(blend);
        }

        private static bool OwnsMaterial(PaintBlendMode? blend) =>
            blend is PaintBlendMode.Screen or PaintBlendMode.Additive;

        private static BattlementUiException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}

#nullable enable

using System;

namespace Battlement.UI
{
    internal static class BattlementMotionDescriptorValidator
    {
        public static void ValidateCapabilities(MotionDescriptor descriptor)
        {
            ValidateTarget(descriptor.Initial);
            foreach (MotionSlotDescriptor slot in descriptor.Slots)
                ValidateTarget(slot.Target);
            foreach (
                MotionPseudoStyle style in descriptor.PseudoStyles
                    ?? Array.Empty<MotionPseudoStyle>()
            )
            foreach (MotionPropertyValue value in style.Values)
                RequireWriter(value.Property);
            foreach (
                CssAnimationDescriptor animation in descriptor.Animations
                    ?? Array.Empty<CssAnimationDescriptor>()
            )
            foreach (CssPropertyTrack track in animation.Tracks)
                RequireWriter(track.Property);
            foreach (
                MotionDecorationDescriptor decoration in descriptor.Decorations
                    ?? Array.Empty<MotionDecorationDescriptor>()
            )
            foreach (CssAnimationDescriptor animation in decoration.Animations)
            foreach (CssPropertyTrack track in animation.Tracks)
                RequireWriter(track.Property);
        }

        private static void ValidateTarget(MotionTargetDescriptor? target)
        {
            if (target is null)
                return;
            foreach (MotionPropertyTrack track in target.Tracks)
                RequireWriter(track.Property);
            foreach (MotionPropertyValue value in target.TransitionEnd)
                RequireWriter(value.Property);
        }

        private static void RequireWriter(MotionProperty property)
        {
            if (!BattlementMotionPropertyWriter.Supports(property))
                throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    $"Motion property {property} has no renderer capability."
                );
        }
    }
}

#nullable enable

using System;

namespace Battlement.UI
{
    internal static class BattlementMotionDescriptorValidator
    {
        public static void ValidateCapabilities(
            MotionDescriptor descriptor,
            IBattlementMotionTarget? writer = null
        ) =>
            ValidateCapabilities(
                descriptor,
                writer is null ? BattlementMotionPropertyWriter.Supports : writer.Supports,
                writer is null || writer is BattlementUiMotionTarget
            );

        internal static void ValidateCapabilities(
            MotionDescriptor descriptor,
            Func<MotionProperty, bool> supports,
            bool ui
        )
        {
            if (!ui)
                ValidateNonUi(descriptor);
            ValidateTarget(descriptor.Initial, supports);
            foreach (MotionSlotDescriptor slot in descriptor.Slots)
                ValidateTarget(slot.Target, supports);
            foreach (
                MotionNamedTarget named in descriptor.NamedTargets
                    ?? Array.Empty<MotionNamedTarget>()
            )
                ValidateTarget(named.Target, supports);
            foreach (
                MotionPseudoStyle style in descriptor.PseudoStyles
                    ?? Array.Empty<MotionPseudoStyle>()
            )
            foreach (MotionPropertyValue value in style.Values)
                RequireWriter(value.Property, supports);
            foreach (
                CssAnimationDescriptor animation in descriptor.Animations
                    ?? Array.Empty<CssAnimationDescriptor>()
            )
            foreach (CssPropertyTrack track in animation.Tracks)
                RequireWriter(track.Property, supports);
            foreach (
                MotionDecorationDescriptor decoration in descriptor.Decorations
                    ?? Array.Empty<MotionDecorationDescriptor>()
            )
            foreach (CssAnimationDescriptor animation in decoration.Animations)
            foreach (CssPropertyTrack track in animation.Tracks)
                RequireWriter(track.Property, supports);
        }

        private static void ValidateNonUi(MotionDescriptor descriptor)
        {
            if (descriptor.Gestures is MotionGestureDescriptor gestures)
            {
                bool input = gestures.Pan || gestures.Drag is not null;
                bool viewport = gestures.InView || gestures.Scroll;
                bool values =
                    gestures.ScrollXValue is not null || gestures.ScrollYValue is not null;
                var noSubscriptions = new MotionGestureSubscriptions(
                    false,
                    false,
                    false,
                    false,
                    false,
                    false,
                    false,
                    false,
                    false,
                    false,
                    false
                );
                if (input || viewport || values)
                    throw new BattlementUiException(
                        CoreErrorCode.InvalidProperty,
                        "Pan, drag, and viewport gestures require a UI Motion host."
                    );
                if (gestures.InViewValue is not null || gestures.Subscriptions != noSubscriptions)
                    throw new BattlementUiException(
                        CoreErrorCode.InvalidProperty,
                        "Motion gesture callbacks and viewport values require a UI Motion host."
                    );
            }
            if (descriptor.Layout is MotionLayoutDescriptor layout && layout.Projection is null)
                throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    "World layout Motion requires an explicit projection."
                );
            if (descriptor.StyleTransition?.All is not null)
                throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    "Style transitions require a UI Motion host."
                );
            if (descriptor.StyleTransition?.Properties.Count > 0)
                throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    "Style transitions require a UI Motion host."
                );
            if (descriptor.Animations?.Count > 0 || descriptor.Decorations?.Count > 0)
                throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    "CSS animations and decorations require a UI Motion host."
                );
            if (descriptor.PseudoStyles?.Count > 0)
                throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    "Pseudo styles require a UI Motion host."
                );
        }

        private static void ValidateTarget(
            MotionTargetDescriptor? target,
            Func<MotionProperty, bool> supports
        )
        {
            if (target is null)
                return;
            foreach (MotionPropertyTrack track in target.Tracks)
                RequireWriter(track.Property, supports);
            foreach (MotionPropertyValue value in target.TransitionEnd)
                RequireWriter(value.Property, supports);
        }

        private static void RequireWriter(
            MotionProperty property,
            Func<MotionProperty, bool> supports
        )
        {
            if (!supports(property))
                throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    $"Motion property {property} has no renderer capability."
                );
        }
    }
}

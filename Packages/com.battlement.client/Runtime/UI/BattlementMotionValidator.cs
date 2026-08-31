#nullable enable

using System;
using System.Collections.Generic;

namespace Battlement.UI
{
    internal static class BattlementMotionValidator
    {
        public static void Validate(MotionDescriptor descriptor, ObjectId? expectedHost = null)
        {
            if (
                descriptor.DescriptorId.Value == Guid.Empty
                || descriptor.HostId.Value == Guid.Empty
            )
                throw Invalid("Motion descriptor and host identities must be nonzero.");
            if (expectedHost is ObjectId host && descriptor.HostId != host)
                throw Invalid("Motion descriptor host identity does not match its UI element.");
            if (descriptor.InitialDisabled && descriptor.Initial is not null)
                throw Invalid("A disabled initial target cannot carry tracks.");
            ValidatePropertyValues(descriptor.StaticBaseline, "static baseline");
            if (descriptor.Initial is not null)
                ValidateTarget(descriptor.Initial);

            var slots = new HashSet<ulong>();
            var motionProperties = new HashSet<MotionProperty>();
            foreach (MotionSlotDescriptor slot in descriptor.Slots)
            {
                if (!slots.Add(slot.Slot))
                    throw Invalid("A motion descriptor cannot repeat a slot identity.");
                ValidateTarget(slot.Target);
                foreach (MotionPropertyTrack track in slot.Target.Tracks)
                    motionProperties.Add(track.Property);
            }
            StyleTransitionDescriptor transition =
                descriptor.StyleTransition
                ?? new StyleTransitionDescriptor(
                    Array.Empty<StylePropertyTransition>(),
                    null,
                    false
                );
            var transitionProperties = new HashSet<MotionProperty>();
            foreach (StylePropertyTransition item in transition.Properties)
            {
                if (!transitionProperties.Add(item.Property))
                    throw Invalid("Style transitions cannot repeat a property.");
                ValidateStyleTransition(item.Transition);
                if (motionProperties.Contains(item.Property))
                    throw Invalid("Motion and CSS transition property ownership conflicts.");
            }
            if (transition.All is not null)
            {
                ValidateStyleTransition(transition.All);
                if (motionProperties.Count != 0)
                    throw Invalid("Style transition `all` conflicts with Motion properties.");
            }
            var pseudoStates = new HashSet<MotionPseudoState>();
            foreach (
                MotionPseudoStyle style in descriptor.PseudoStyles
                    ?? Array.Empty<MotionPseudoStyle>()
            )
            {
                if (!pseudoStates.Add(style.State))
                    throw Invalid("Pseudo styles cannot repeat a state.");
                ValidatePropertyValues(style.Values, "pseudo style");
            }
            var animationSlots = new HashSet<ulong>(slots);
            foreach (
                CssAnimationDescriptor animation in descriptor.Animations
                    ?? Array.Empty<CssAnimationDescriptor>()
            )
            {
                if (!animationSlots.Add(animation.Slot))
                    throw Invalid("CSS animations cannot repeat a slot identity.");
                ValidateCssTarget(animation.Tracks);
                foreach (CssPropertyTrack track in animation.Tracks)
                {
                    if (
                        motionProperties.Contains(track.Property)
                        || transitionProperties.Contains(track.Property)
                        || transition.All is not null
                    )
                        throw Invalid("Motion and CSS animation property ownership conflicts.");
                    ValidateComposition(animation, track);
                }
            }
            var decorationKeys = new HashSet<ulong>();
            foreach (
                MotionDecorationDescriptor decoration in descriptor.Decorations
                    ?? Array.Empty<MotionDecorationDescriptor>()
            )
            {
                if (!decorationKeys.Add(decoration.Key))
                    throw Invalid("Decorations cannot repeat a key.");
                var decorationSlots = new HashSet<ulong>();
                foreach (CssAnimationDescriptor animation in decoration.Animations)
                {
                    if (!decorationSlots.Add(animation.Slot))
                        throw Invalid("Decoration animations cannot repeat a slot identity.");
                    ValidateCssTarget(animation.Tracks);
                    foreach (CssPropertyTrack track in animation.Tracks)
                        ValidateComposition(animation, track);
                }
            }
        }

        private static void ValidateStyleTransition(TransitionDefinition transition)
        {
            if (
                transition.Generator
                is not TransitionGenerator.Immediate
                    and not TransitionGenerator.Tween
            )
                throw Invalid("Style transitions accept only tween or immediate timing.");
            ValidateTransition(MotionProperty.Opacity, transition, 1);
        }

        private static void ValidateCssTarget(IReadOnlyList<CssPropertyTrack> tracks)
        {
            var properties = new HashSet<MotionProperty>();
            foreach (CssPropertyTrack track in tracks)
            {
                if (!properties.Add(track.Property))
                    throw Invalid("A CSS animation cannot repeat a property.");
                if (track.Values.Count == 0 || track.Values.Count != track.Times.Count)
                    throw Invalid("CSS property values and times must be nonempty and aligned.");
                foreach (MotionValue value in track.Values)
                    ValidateValue(track.Property, value);
                ValidateCssTimes(track.Times);
                ValidateTransition(track.Property, track.Transition, track.Values.Count);
            }
        }

        private static void ValidateCssTimes(IReadOnlyList<double> times)
        {
            Finite(times);
            for (int index = 0; index < times.Count; index++)
            {
                if (times[index] < 0 || times[index] > 1)
                    throw Invalid("CSS keyframe times must be in 0..=1.");
                if (index != 0 && times[index] < times[index - 1])
                    throw Invalid("CSS keyframe times must be nondecreasing.");
            }
        }

        private static void ValidateComposition(
            CssAnimationDescriptor animation,
            CssPropertyTrack track
        )
        {
            if (animation.Composition == AnimationComposition.Replace)
                return;
            if (!SupportsComposition(track.Property))
                throw Invalid("A CSS animation property does not support additive composition.");
            if (track.Property != MotionProperty.TransformList)
                return;
            if (track.Values[0] is not MotionValue.TransformList first)
                throw Invalid("Additive transform tracks require transform-list values.");
            foreach (MotionValue value in track.Values)
            {
                if (value is not MotionValue.TransformList current)
                    throw Invalid("Additive transform tracks require transform-list values.");
                if (!CompatibleTransforms(first.Value, current.Value))
                    throw Invalid("Additive transform lists require compatible operations.");
            }
        }

        private static bool CompatibleTransforms(
            IReadOnlyList<MotionTransform> left,
            IReadOnlyList<MotionTransform> right
        )
        {
            if (left.Count != right.Count)
                return false;
            for (int index = 0; index < left.Count; index++)
                if (left[index].GetType() != right[index].GetType())
                    return false;
            return true;
        }

        private static bool SupportsComposition(MotionProperty property) =>
            property
                is MotionProperty.BackgroundPositionX
                    or MotionProperty.BackgroundPositionY
                    or MotionProperty.BorderBottomLeftRadius
                    or MotionProperty.BorderBottomRightRadius
                    or MotionProperty.BorderBottomWidth
                    or MotionProperty.BorderLeftWidth
                    or MotionProperty.BorderRightWidth
                    or MotionProperty.BorderTopLeftRadius
                    or MotionProperty.BorderTopRightRadius
                    or MotionProperty.BorderTopWidth
                    or MotionProperty.Bottom
                    or MotionProperty.FlexBasis
                    or MotionProperty.FlexGrow
                    or MotionProperty.FlexShrink
                    or MotionProperty.FontSize
                    or MotionProperty.Height
                    or MotionProperty.Left
                    or MotionProperty.LetterSpacing
                    or MotionProperty.MarginBottom
                    or MotionProperty.MarginLeft
                    or MotionProperty.MarginRight
                    or MotionProperty.MarginTop
                    or MotionProperty.MaxHeight
                    or MotionProperty.MaxWidth
                    or MotionProperty.MinHeight
                    or MotionProperty.MinWidth
                    or MotionProperty.Opacity
                    or MotionProperty.PaddingBottom
                    or MotionProperty.PaddingLeft
                    or MotionProperty.PaddingRight
                    or MotionProperty.PaddingTop
                    or MotionProperty.Right
                    or MotionProperty.Rotate
                    or MotionProperty.RotateX
                    or MotionProperty.RotateY
                    or MotionProperty.Scale
                    or MotionProperty.ScaleX
                    or MotionProperty.ScaleY
                    or MotionProperty.SkewX
                    or MotionProperty.SkewY
                    or MotionProperty.Top
                    or MotionProperty.TransformList
                    or MotionProperty.Translate
                    or MotionProperty.UnityParagraphSpacing
                    or MotionProperty.UnitySliceBottom
                    or MotionProperty.UnitySliceLeft
                    or MotionProperty.UnitySliceRight
                    or MotionProperty.UnitySliceScale
                    or MotionProperty.UnitySliceTop
                    or MotionProperty.UnityTextOutlineWidth
                    or MotionProperty.Width
                    or MotionProperty.WordSpacing
                    or MotionProperty.X
                    or MotionProperty.Y
                    or MotionProperty.Z;

        private static void ValidateTarget(MotionTargetDescriptor target)
        {
            var properties = new HashSet<MotionProperty>();
            foreach (MotionPropertyTrack track in target.Tracks)
            {
                if (!properties.Add(track.Property))
                    throw Invalid("A motion target cannot repeat a property.");
                if (track.Values.Count == 0)
                    throw Invalid("A motion property track cannot be empty.");
                foreach (MotionValue value in track.Values)
                    ValidateValue(track.Property, value);
                ValidateTimes(track.Times, track.Values.Count, requiredMatch: true);
                ValidateTransition(track.Property, track.Transition, track.Values.Count);
            }
            properties.Clear();
            foreach (MotionPropertyValue value in target.TransitionEnd)
            {
                if (!properties.Add(value.Property))
                    throw Invalid("transition_end cannot repeat a property.");
                ValidateValue(value.Property, value.Value);
            }
        }

        private static void ValidatePropertyValues(
            IReadOnlyList<MotionPropertyValue> values,
            string description
        )
        {
            var properties = new HashSet<MotionProperty>();
            foreach (MotionPropertyValue value in values)
            {
                if (!properties.Add(value.Property))
                    throw Invalid($"A motion {description} cannot repeat a property.");
                ValidateValue(value.Property, value.Value);
            }
        }

        private static void ValidateTransition(
            MotionProperty property,
            TransitionDefinition transition,
            int keyframeCount
        )
        {
            switch (transition.Generator)
            {
                case TransitionGenerator.Immediate:
                    break;
                case TransitionGenerator.Tween tween:
                    if (tween.DurationMicros == 0)
                        throw Invalid("A motion tween duration must be positive.");
                    ValidateTimes(tween.Times, keyframeCount, requiredMatch: false);
                    foreach (MotionEasing easing in tween.Easings)
                        ValidateEasing(easing);
                    break;
                case TransitionGenerator.Spring spring:
                    if (ExpectedKind(property) == MotionValueKind.Discrete)
                        throw Invalid("A discrete motion property cannot use a spring.");
                    ValidateSpring(spring.Value);
                    break;
                case TransitionGenerator.Inertia inertia:
                    if (ExpectedKind(property) != MotionValueKind.Scalar)
                        throw Invalid("Inertia requires a scalar motion property.");
                    Finite(
                        inertia.InitialVelocity,
                        inertia.Power,
                        inertia.RestDelta,
                        inertia.BounceStiffness,
                        inertia.BounceDamping
                    );
                    if (inertia.TimeConstantMicros == 0 || inertia.RestDelta < 0)
                        throw Invalid("Inertia timing and rest values are invalid.");
                    if (inertia.BounceStiffness <= 0 || inertia.BounceDamping < 0)
                        throw Invalid("Inertia bounce values are invalid.");
                    if (inertia.Minimum is double minimum && inertia.Maximum is double maximum)
                    {
                        Finite(minimum, maximum);
                        if (minimum > maximum)
                            throw Invalid("Inertia minimum cannot exceed maximum.");
                    }
                    ValidateTargetModifier(inertia.Target);
                    break;
                default:
                    throw Invalid("Unknown motion transition generator.");
            }
        }

        private static void ValidateSpring(SpringConfiguration spring)
        {
            switch (spring)
            {
                case SpringConfiguration.Physical value:
                    Finite(
                        value.Stiffness,
                        value.Damping,
                        value.Mass,
                        value.InitialVelocity,
                        value.RestSpeed,
                        value.RestDelta
                    );
                    if (value.Stiffness <= 0 || value.Damping < 0 || value.Mass <= 0)
                        throw Invalid("Physical spring values are invalid.");
                    if (value.RestSpeed < 0 || value.RestDelta < 0)
                        throw Invalid("Spring rest thresholds cannot be negative.");
                    break;
                case SpringConfiguration.Duration value:
                    Finite(value.Bounce, value.Mass);
                    if (value.DurationMicros == 0 || value.Mass <= 0)
                        throw Invalid("Duration spring values are invalid.");
                    break;
                case SpringConfiguration.VisualDuration value:
                    Finite(value.Bounce, value.Mass);
                    if (value.DurationMicros == 0 || value.Mass <= 0)
                        throw Invalid("Visual-duration spring values are invalid.");
                    break;
                default:
                    throw Invalid("Unknown spring configuration.");
            }
        }

        private static void ValidateEasing(MotionEasing easing)
        {
            if (easing is MotionEasing.CubicBezier cubicBezier)
            {
                if (cubicBezier.Value.Count != 4)
                    throw Invalid("Cubic Bézier easing requires four values.");
                Finite(cubicBezier.Value);
                if (cubicBezier.Value[0] < 0 || cubicBezier.Value[0] > 1)
                    throw Invalid("Cubic Bézier x coordinates must be in 0..=1.");
                if (cubicBezier.Value[2] < 0 || cubicBezier.Value[2] > 1)
                    throw Invalid("Cubic Bézier x coordinates must be in 0..=1.");
            }
            if (easing is MotionEasing.Steps steps && steps.Count == 0)
                throw Invalid("Stepped easing requires a positive count.");
        }

        private static void ValidateTimes(
            IReadOnlyList<double>? times,
            int count,
            bool requiredMatch
        )
        {
            if (times is null)
                return;
            Finite(times);
            if (requiredMatch && times.Count != count)
                throw Invalid("Property keyframe times must match its values.");
            if (!requiredMatch && times.Count != count)
                return;
            if (times.Count < 2 || times[0] != 0 || times[^1] != 1)
                throw Invalid("Keyframe times must begin at zero and end at one.");
            for (int index = 1; index < times.Count; index++)
                if (times[index] < times[index - 1])
                    throw Invalid("Keyframe times must be nondecreasing.");
        }

        private static void ValidateValue(MotionProperty property, MotionValue value)
        {
            if (ExpectedKind(property) != Kind(value))
                throw Invalid("A motion property received an incompatible value shape.");
            if (value is MotionValue.Scalar scalar)
                Finite(scalar.Value);
            else if (value is MotionValue.Length length)
                Finite(length.Value.Px, length.Value.Percent);
            else if (value is MotionValue.Color color)
                ValidateColor(color.Value);
            else if (value is MotionValue.Vector2 vector2)
                ValidateVector(vector2.Value, 2);
            else if (value is MotionValue.Vector3 vector3)
                ValidateVector(vector3.Value, 3);
            else if (value is MotionValue.Angle angle)
                Finite(angle.Value);
            else if (value is MotionValue.TransformList transforms)
                ValidateTransforms(transforms.Value);
            else if (value is MotionValue.FilterList filters)
                ValidateFilters(filters.Value);
            else if (value is MotionValue.ShadowList shadows)
                ValidateShadows(shadows.Value);
            else if (value is MotionValue.Gradient gradient)
                ValidateGradient(gradient.Value);
            else if (value is MotionValue.ClipInset inset)
                ValidateInset(inset.Value);
            else if (value is MotionValue.ClipPolygon polygon)
                ValidatePolygon(polygon.Value);
        }

        private static MotionValueKind ExpectedKind(MotionProperty property) =>
            property switch
            {
                MotionProperty.AspectRatio
                or MotionProperty.FlexGrow
                or MotionProperty.FlexShrink
                or MotionProperty.Opacity
                or MotionProperty.UnitySliceBottom
                or MotionProperty.UnitySliceLeft
                or MotionProperty.UnitySliceRight
                or MotionProperty.UnitySliceScale
                or MotionProperty.UnitySliceTop
                or MotionProperty.UnityTextOutlineWidth
                or MotionProperty.ScaleX
                or MotionProperty.ScaleY => MotionValueKind.Scalar,
                MotionProperty.BackgroundPositionX
                or MotionProperty.BackgroundPositionY
                or MotionProperty.BorderBottomLeftRadius
                or MotionProperty.BorderBottomRightRadius
                or MotionProperty.BorderBottomWidth
                or MotionProperty.BorderLeftWidth
                or MotionProperty.BorderRightWidth
                or MotionProperty.BorderTopLeftRadius
                or MotionProperty.BorderTopRightRadius
                or MotionProperty.BorderTopWidth
                or MotionProperty.Bottom
                or MotionProperty.FlexBasis
                or MotionProperty.FontSize
                or MotionProperty.Height
                or MotionProperty.Left
                or MotionProperty.LetterSpacing
                or MotionProperty.MarginBottom
                or MotionProperty.MarginLeft
                or MotionProperty.MarginRight
                or MotionProperty.MarginTop
                or MotionProperty.MaxHeight
                or MotionProperty.MaxWidth
                or MotionProperty.MinHeight
                or MotionProperty.MinWidth
                or MotionProperty.PaddingBottom
                or MotionProperty.PaddingLeft
                or MotionProperty.PaddingRight
                or MotionProperty.PaddingTop
                or MotionProperty.Right
                or MotionProperty.Top
                or MotionProperty.UnityParagraphSpacing
                or MotionProperty.Width
                or MotionProperty.WordSpacing
                or MotionProperty.X
                or MotionProperty.Y
                or MotionProperty.Z => MotionValueKind.Length,
                MotionProperty.BackgroundColor
                or MotionProperty.BorderBottomColor
                or MotionProperty.BorderLeftColor
                or MotionProperty.BorderRightColor
                or MotionProperty.BorderTopColor
                or MotionProperty.Color
                or MotionProperty.UnityBackgroundImageTintColor
                or MotionProperty.UnityTextOutlineColor => MotionValueKind.Color,
                MotionProperty.BackgroundSize or MotionProperty.Scale => MotionValueKind.Vector2,
                MotionProperty.TransformOrigin
                or MotionProperty.Translate
                or MotionProperty.Layout => MotionValueKind.Vector3,
                MotionProperty.Rotate
                or MotionProperty.RotateX
                or MotionProperty.RotateY
                or MotionProperty.SkewX
                or MotionProperty.SkewY => MotionValueKind.Angle,
                MotionProperty.TransformList => MotionValueKind.TransformList,
                MotionProperty.Filter => MotionValueKind.FilterList,
                MotionProperty.TextShadow or MotionProperty.BoxShadow => MotionValueKind.ShadowList,
                MotionProperty.BackgroundGradient => MotionValueKind.Gradient,
                MotionProperty.ClipInset => MotionValueKind.ClipInset,
                MotionProperty.ClipPolygon => MotionValueKind.ClipPolygon,
                _ => MotionValueKind.Discrete,
            };

        private static MotionValueKind Kind(MotionValue value) =>
            value switch
            {
                MotionValue.Scalar => MotionValueKind.Scalar,
                MotionValue.Length => MotionValueKind.Length,
                MotionValue.Color => MotionValueKind.Color,
                MotionValue.Vector2 => MotionValueKind.Vector2,
                MotionValue.Vector3 => MotionValueKind.Vector3,
                MotionValue.Angle => MotionValueKind.Angle,
                MotionValue.TransformList => MotionValueKind.TransformList,
                MotionValue.FilterList => MotionValueKind.FilterList,
                MotionValue.ShadowList => MotionValueKind.ShadowList,
                MotionValue.Gradient => MotionValueKind.Gradient,
                MotionValue.ClipInset => MotionValueKind.ClipInset,
                MotionValue.ClipPolygon => MotionValueKind.ClipPolygon,
                _ => MotionValueKind.Discrete,
            };

        private static void ValidateTransforms(IReadOnlyList<MotionTransform> values)
        {
            foreach (MotionTransform value in values)
                if (value is MotionTransform.Translate translate)
                    foreach (MotionLength length in translate.Value)
                        Finite(length.Px, length.Percent);
                else if (value is MotionTransform.Rotate rotate)
                    Finite(rotate.Value);
                else if (value is MotionTransform.Skew skew)
                    Finite(skew.Value);
                else if (value is MotionTransform.Scale scale)
                    Finite(scale.Value);
        }

        private static void ValidateFilters(IReadOnlyList<MotionFilter> values)
        {
            foreach (MotionFilter value in values)
                if (value is MotionFilter.Blur blur)
                    Finite(blur.Value);
                else if (value is MotionFilter.Brightness brightness)
                    Finite(brightness.Value);
                else if (value is MotionFilter.Saturate saturate)
                    Finite(saturate.Value);
                else if (value is MotionFilter.Contrast contrast)
                    Finite(contrast.Value);
                else if (value is MotionFilter.HueRotate hueRotate)
                    Finite(hueRotate.Value);
                else if (value is MotionFilter.Opacity opacity)
                    Finite(opacity.Value);
                else if (value is MotionFilter.DropShadow shadow)
                    ValidateShadow(shadow.Value);
        }

        private static void ValidateGradient(MotionGradient value)
        {
            IReadOnlyList<MotionGradientStop> stops = value switch
            {
                MotionGradient.Linear item => item.Stops,
                MotionGradient.Radial item => item.Stops,
                _ => Array.Empty<MotionGradientStop>(),
            };
            if (stops.Count < 2)
                throw Invalid("A motion gradient requires at least two stops.");
            foreach (MotionGradientStop stop in stops)
            {
                ValidateColor(stop.Color);
                Finite(stop.Position);
            }
        }

        private static void ValidateShadow(MotionShadow value)
        {
            Finite(value.X, value.Y, value.Blur, value.Spread);
            ValidateColor(value.Color);
        }

        private static void ValidateColor(MotionColor value) =>
            Finite(value.Red, value.Green, value.Blue, value.Alpha);

        private static void ValidateVector(IReadOnlyList<double> values, int count)
        {
            if (values.Count != count)
                throw Invalid($"A motion vector requires {count} values.");
            Finite(values);
        }

        private static void ValidateTargetModifier(InertiaTarget target)
        {
            if (target is InertiaTarget.NearestMultiple nearest)
            {
                Finite(nearest.Value);
                if (nearest.Value <= 0)
                    throw Invalid("An inertia multiple must be positive.");
            }
            else if (target is InertiaTarget.FloorMultiple floor)
            {
                Finite(floor.Value);
                if (floor.Value <= 0)
                    throw Invalid("An inertia multiple must be positive.");
            }
            else if (target is InertiaTarget.CeilingMultiple ceiling)
            {
                Finite(ceiling.Value);
                if (ceiling.Value <= 0)
                    throw Invalid("An inertia multiple must be positive.");
            }
            else if (target is InertiaTarget.Clamp clamp)
            {
                Finite(clamp.Min, clamp.Max);
                if (clamp.Min > clamp.Max)
                    throw Invalid("An inertia clamp is inverted.");
            }
        }

        private static void ValidateShadows(IReadOnlyList<MotionShadow> shadows)
        {
            foreach (MotionShadow shadow in shadows)
                ValidateShadow(shadow);
        }

        private static void ValidateInset(IReadOnlyList<MotionLength> value)
        {
            if (value.Count != 4)
                throw Invalid("A clip inset requires four lengths.");
            foreach (MotionLength length in value)
                Finite(length.Px, length.Percent);
        }

        private static void ValidatePolygon(IReadOnlyList<IReadOnlyList<MotionLength>> value)
        {
            if (value.Count < 3)
                throw Invalid("A clip polygon requires at least three vertices.");
            foreach (IReadOnlyList<MotionLength> vertex in value)
            {
                if (vertex.Count != 2)
                    throw Invalid("A clip polygon vertex requires two lengths.");
                foreach (MotionLength length in vertex)
                    Finite(length.Px, length.Percent);
            }
        }

        private static void Finite(params double?[] values)
        {
            foreach (double? value in values)
                if (value is double number && !double.IsFinite(number))
                    throw Invalid("Motion values must be finite.");
        }

        private static void Finite(IReadOnlyList<double> values)
        {
            foreach (double value in values)
                if (!double.IsFinite(value))
                    throw Invalid("Motion values must be finite.");
        }

        private static BattlementUiException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}

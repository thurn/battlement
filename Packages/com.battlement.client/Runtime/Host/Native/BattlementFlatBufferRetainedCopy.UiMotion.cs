#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    internal static partial class BattlementFlatBufferRetainedCopy
    {
        internal static MotionDescriptor ReadMotionDescriptor(Wire.MotionDescriptor value) =>
            UiProperties.MotionDescriptor(value);

        private sealed partial class UiProperties
        {
            private Prop<MotionDescriptor> MotionProperty(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.MotionDescriptorPropertyValue,
                    property =>
                        MotionDescriptor(
                            property.ValueAsMotionDescriptorPropertyValue().Value
                                ?? throw Missing("motion descriptor")
                        )
                );

            internal static MotionDescriptor MotionDescriptor(Wire.MotionDescriptor value)
            {
                var slots = new MotionSlotDescriptor[value.SlotsLength];
                for (int index = 0; index < slots.Length; index++)
                    slots[index] = MotionSlot(value.Slots(index) ?? throw Missing("motion slot"));
                var pseudoStyles = new MotionPseudoStyle[value.PseudoStylesLength];
                for (int index = 0; index < pseudoStyles.Length; index++)
                    pseudoStyles[index] = MotionPseudoStyle(
                        value.PseudoStyles(index) ?? throw Missing("motion pseudo style")
                    );
                var animations = new CssAnimationDescriptor[value.AnimationsLength];
                for (int index = 0; index < animations.Length; index++)
                    animations[index] = CssAnimation(
                        value.Animations(index) ?? throw Missing("CSS animation")
                    );
                var decorations = new MotionDecorationDescriptor[value.DecorationsLength];
                for (int index = 0; index < decorations.Length; index++)
                    decorations[index] = MotionDecoration(
                        value.Decorations(index) ?? throw Missing("motion decoration")
                    );
                var graphValues = new MotionValueDescriptor[value.ValuesLength];
                for (int index = 0; index < graphValues.Length; index++)
                    graphValues[index] = MotionGraphValue(
                        value.Values(index) ?? throw Missing("motion graph value")
                    );
                var bindings = new MotionValueBinding[value.ValueBindingsLength];
                for (int index = 0; index < bindings.Length; index++)
                {
                    Wire.MotionValueBinding item =
                        value.ValueBindings(index) ?? throw Missing("motion value binding");
                    bindings[index] = new MotionValueBinding(
                        (MotionProperty)(ushort)item.Property,
                        ObjectId(item.ValueId),
                        (MotionBindingComposition)(byte)item.Composition
                    );
                }
                var subscriptions = new MotionValueSubscription[value.ValueSubscriptionsLength];
                for (int index = 0; index < subscriptions.Length; index++)
                {
                    Wire.MotionValueSubscription item =
                        value.ValueSubscriptions(index)
                        ?? throw Missing("motion value subscription");
                    subscriptions[index] = new MotionValueSubscription(
                        ObjectId(item.SubscriptionId),
                        ObjectId(item.ValueId),
                        (MotionValueEventKind)(byte)item.Event
                    );
                }
                var namedTargets = new MotionNamedTarget[value.NamedTargetsLength];
                for (int index = 0; index < namedTargets.Length; index++)
                {
                    Wire.MotionNamedTarget item =
                        value.NamedTargets(index) ?? throw Missing("named motion target");
                    namedTargets[index] = new MotionNamedTarget(
                        Required(item.Name, "named motion target name"),
                        MotionTarget(item.Target ?? throw Missing("named motion target value"))
                    );
                }
                return new MotionDescriptor(
                    ObjectId(value.DescriptorId),
                    ObjectId(value.HostId),
                    value.Generation,
                    value.InitialDisabled,
                    slots,
                    MotionClock(value.Clock ?? throw Missing("motion clock")),
                    (ReducedMotionPolicy)(byte)value.ReducedMotion,
                    value.Initial.HasValue ? MotionTarget(value.Initial.Value) : null,
                    pseudoStyles,
                    StyleTransition(value.StyleTransition ?? throw Missing("style transition")),
                    animations,
                    decorations,
                    value.Variants.HasValue ? MotionVariants(value.Variants.Value) : null,
                    graphValues,
                    bindings,
                    subscriptions,
                    OptionalObjectId(value.ControlId),
                    OptionalObjectId(value.ScopeId),
                    value.ScopeRoot,
                    value.MotionName,
                    namedTargets,
                    value.Gestures.HasValue ? MotionGestures(value.Gestures.Value) : null,
                    value.Layout.HasValue ? MotionLayout(value.Layout.Value) : null
                );
            }

            private static MotionSlotDescriptor MotionSlot(Wire.MotionSlotDescriptor value)
            {
                Wire.MotionCallbackSubscriptions callbacks =
                    value.Callbacks ?? throw Missing("motion callbacks");
                return new MotionSlotDescriptor(
                    value.Slot,
                    value.Generation,
                    (MotionLayer)(byte)value.Layer,
                    MotionTarget(value.Target ?? throw Missing("motion slot target")),
                    new MotionCallbackSubscriptions(
                        callbacks.Start,
                        callbacks.Update,
                        callbacks.Repeat,
                        callbacks.Complete,
                        callbacks.Stop,
                        callbacks.Cancel
                    )
                );
            }

            private static MotionClockSource MotionClock(Wire.MotionClockSource value) =>
                value.Kind switch
                {
                    Wire.MotionClockKind.Unscaled => new MotionClockSource.Unscaled(),
                    Wire.MotionClockKind.Scaled => new MotionClockSource.Scaled(),
                    Wire.MotionClockKind.Controlled when value.ObjectId.HasValue =>
                        new MotionClockSource.Controlled(ObjectId(value.ObjectId)),
                    Wire.MotionClockKind.Audio when value.ObjectId.HasValue =>
                        new MotionClockSource.Audio(ObjectId(value.ObjectId)),
                    _ => throw new InvalidDataException(
                        "Motion clock kind and payload do not match."
                    ),
                };

            private static MotionPseudoStyle MotionPseudoStyle(Wire.MotionPseudoStyle value) =>
                new(
                    (MotionPseudoState)(byte)value.State,
                    MotionPropertyValues(value.ValuesLength, value.Values)
                );

            private static IReadOnlyList<MotionPropertyValue> MotionPropertyValues(
                int length,
                Func<int, Wire.MotionPropertyValue?> read
            )
            {
                var result = new MotionPropertyValue[length];
                for (int index = 0; index < length; index++)
                {
                    Wire.MotionPropertyValue item =
                        read(index) ?? throw Missing("motion property value");
                    result[index] = new MotionPropertyValue(
                        (MotionProperty)(ushort)item.Property,
                        BattlementFlatBufferRetainedCopy.MotionValue(item)
                    );
                }
                return result;
            }

            private static StyleTransitionDescriptor StyleTransition(
                Wire.StyleTransitionDescriptor value
            )
            {
                var properties = new StylePropertyTransition[value.PropertiesLength];
                for (int index = 0; index < properties.Length; index++)
                {
                    Wire.StylePropertyTransition item =
                        value.Properties(index) ?? throw Missing("style property transition");
                    properties[index] = new StylePropertyTransition(
                        (MotionProperty)(ushort)item.Property,
                        Transition(item.Transition ?? throw Missing("style transition value"))
                    );
                }
                return new StyleTransitionDescriptor(
                    properties,
                    value.All.HasValue ? Transition(value.All.Value) : null,
                    value.AllowDiscrete
                );
            }

            private static CssAnimationDescriptor CssAnimation(Wire.CssAnimationDescriptor value)
            {
                var tracks = new CssPropertyTrack[value.TracksLength];
                for (int index = 0; index < tracks.Length; index++)
                {
                    Wire.CssPropertyTrack track =
                        value.Tracks(index) ?? throw Missing("CSS animation track");
                    var values = new MotionValue[track.ValuesLength];
                    for (int item = 0; item < values.Length; item++)
                        values[item] = BattlementFlatBufferRetainedCopy.MotionValue(
                            track.Values(item) ?? throw Missing("CSS animation value")
                        );
                    var times = new double[track.TimesLength];
                    for (int item = 0; item < times.Length; item++)
                        times[item] = track.Times(item);
                    tracks[index] = new CssPropertyTrack(
                        (MotionProperty)(ushort)track.Property,
                        values,
                        times,
                        Transition(track.Transition ?? throw Missing("CSS animation transition"))
                    );
                }
                return new CssAnimationDescriptor(
                    value.Slot,
                    value.Generation,
                    value.RestartKey,
                    tracks,
                    (AnimationDirection)(byte)value.Direction,
                    (AnimationFill)(byte)value.Fill,
                    (AnimationPlayState)(byte)value.PlayState,
                    (AnimationComposition)(byte)value.Composition,
                    value.DiagnosticName
                );
            }

            private static MotionDecorationDescriptor MotionDecoration(
                Wire.MotionDecorationDescriptor value
            )
            {
                var properties = new Dictionary<Wire.UiPropertyKey, Wire.UiProperty>(
                    value.StyleLength
                );
                for (int index = 0; index < value.StyleLength; index++)
                {
                    Wire.UiProperty property =
                        value.Style(index) ?? throw Missing("motion decoration style");
                    if (!properties.TryAdd(property.Key, property))
                        throw new InvalidDataException(
                            "A motion decoration repeats a style property."
                        );
                }
                var reader = new UiProperties(default, properties);
                UiStyle style = reader.Style() ?? new UiStyle();
                if (reader.values.Count != 0)
                    throw new InvalidDataException(
                        "A motion decoration contains a non-style property."
                    );
                var animations = new CssAnimationDescriptor[value.AnimationsLength];
                for (int index = 0; index < animations.Length; index++)
                    animations[index] = CssAnimation(
                        value.Animations(index) ?? throw Missing("decoration animation")
                    );
                return new MotionDecorationDescriptor(
                    value.Key,
                    (DecorationPlacement)(byte)value.Placement,
                    (DecorationPosition)(byte)value.Position,
                    (DecorationOverflow)(byte)value.Overflow,
                    style,
                    animations
                );
            }

            private static MotionVariantResolution MotionVariants(
                Wire.MotionVariantResolution value
            )
            {
                var names = new string[value.NamesLength];
                for (int index = 0; index < names.Length; index++)
                    names[index] = Required(value.Names(index), "motion variant name");
                return new MotionVariantResolution(
                    names,
                    value.Inherited,
                    value.CustomSnapshot,
                    value.ChildIndex,
                    value.DelayMicros,
                    (VariantWhen)(byte)value.When,
                    (StaggerDirection)(byte)value.StaggerDirection
                );
            }

            private static MotionValueDescriptor MotionGraphValue(
                Wire.MotionValueDescriptor value
            ) =>
                new(
                    ObjectId(value.ValueId),
                    MotionValue(value),
                    MotionValueSource(value.Source ?? throw Missing("motion value source"))
                );

            private static MotionValue MotionValue(Wire.MotionValueDescriptor value) =>
                value.InitialType switch
                {
                    Wire.MotionValue.ScalarMotionValue => Scalar(
                        value.InitialAsScalarMotionValue()
                    ),
                    Wire.MotionValue.LengthMotionValue => Length(
                        value.InitialAsLengthMotionValue()
                    ),
                    Wire.MotionValue.ColorMotionValue => MotionColor(
                        value.InitialAsColorMotionValue()
                    ),
                    Wire.MotionValue.Vector2MotionValue => Vector2(
                        value.InitialAsVector2MotionValue()
                    ),
                    Wire.MotionValue.Vector3MotionValue => Vector3(
                        value.InitialAsVector3MotionValue()
                    ),
                    Wire.MotionValue.AngleMotionValue => Angle(value.InitialAsAngleMotionValue()),
                    Wire.MotionValue.TransformListMotionValue => TransformList(
                        value.InitialAsTransformListMotionValue()
                    ),
                    Wire.MotionValue.FilterListMotionValue => FilterList(
                        value.InitialAsFilterListMotionValue()
                    ),
                    Wire.MotionValue.ShadowListMotionValue => ShadowList(
                        value.InitialAsShadowListMotionValue()
                    ),
                    Wire.MotionValue.GradientMotionValue => Gradient(
                        value.InitialAsGradientMotionValue()
                    ),
                    Wire.MotionValue.ClipInsetMotionValue => ClipInset(
                        value.InitialAsClipInsetMotionValue()
                    ),
                    Wire.MotionValue.ClipPolygonMotionValue => ClipPolygon(
                        value.InitialAsClipPolygonMotionValue()
                    ),
                    Wire.MotionValue.DiscreteMotionValue => Discrete(
                        value.InitialAsDiscreteMotionValue()
                    ),
                    _ => throw new InvalidDataException("Unknown initial motion value kind."),
                };

            private static MotionValueSource MotionValueSource(Wire.MotionValueSource value) =>
                value.Kind switch
                {
                    Wire.MotionValueSourceKind.Mutable => new MotionValueSource.Mutable(),
                    Wire.MotionValueSourceKind.Time when value.Clock.HasValue =>
                        new MotionValueSource.Time(MotionClock(value.Clock.Value)),
                    Wire.MotionValueSourceKind.Velocity when value.Source.HasValue =>
                        new MotionValueSource.Velocity(ObjectId(value.Source)),
                    Wire.MotionValueSourceKind.Range when value.Source.HasValue =>
                        new MotionValueSource.Range(
                            ObjectId(value.Source),
                            MotionValues(value.InputLength, value.Input),
                            MotionValues(value.OutputLength, value.Output),
                            value.Clamp
                        ),
                    Wire.MotionValueSourceKind.Spring
                        when value.Source.HasValue && value.Spring.HasValue =>
                        new MotionValueSource.Spring(
                            ObjectId(value.Source),
                            Spring(value.Spring.Value)
                        ),
                    Wire.MotionValueSourceKind.Expression when value.Expression.HasValue =>
                        new MotionValueSource.Expression(
                            MotionExpression(value.Expression.Value),
                            ObjectIds(value.InputsLength, value.Inputs)
                        ),
                    _ => throw new InvalidDataException(
                        "Motion value source kind and payload do not match."
                    ),
                };

            private static IReadOnlyList<MotionValue> MotionValues(
                int length,
                Func<int, Wire.MotionValueEntry?> read
            )
            {
                var result = new MotionValue[length];
                for (int index = 0; index < length; index++)
                    result[index] = BattlementFlatBufferRetainedCopy.MotionValue(
                        read(index) ?? throw Missing("motion graph range value")
                    );
                return result;
            }

            private static IReadOnlyList<ObjectId> ObjectIds(int length, Func<int, Wire.Uuid?> read)
            {
                var result = new ObjectId[length];
                for (int index = 0; index < length; index++)
                    result[index] = ObjectId(read(index));
                return result;
            }

            private static SpringConfiguration Spring(Wire.MotionSpringConfiguration value) =>
                value.Generator switch
                {
                    Wire.TransitionGeneratorKind.SpringPhysical => new SpringConfiguration.Physical(
                        value.Stiffness,
                        value.Damping,
                        value.Mass,
                        value.InitialVelocity,
                        value.RestSpeed,
                        value.RestDelta
                    ),
                    Wire.TransitionGeneratorKind.SpringDuration => new SpringConfiguration.Duration(
                        value.DurationMicros,
                        value.Bounce,
                        value.Mass
                    ),
                    Wire.TransitionGeneratorKind.SpringVisualDuration =>
                        new SpringConfiguration.VisualDuration(
                            value.DurationMicros,
                            value.Bounce,
                            value.Mass
                        ),
                    _ => throw new InvalidDataException(
                        "A motion spring uses a non-spring generator."
                    ),
                };

            private static MotionExpressionOperation MotionExpression(
                Wire.MotionExpressionOperation value
            ) =>
                value.Kind switch
                {
                    Wire.MotionExpressionOperationKind.Add => new MotionExpressionOperation.Add(),
                    Wire.MotionExpressionOperationKind.Subtract =>
                        new MotionExpressionOperation.Subtract(),
                    Wire.MotionExpressionOperationKind.Multiply =>
                        new MotionExpressionOperation.Multiply(),
                    Wire.MotionExpressionOperationKind.Divide =>
                        new MotionExpressionOperation.Divide(),
                    Wire.MotionExpressionOperationKind.Power => new MotionExpressionOperation.Power(
                        value.Value
                    ),
                    Wire.MotionExpressionOperationKind.SquareRoot =>
                        new MotionExpressionOperation.SquareRoot(),
                    Wire.MotionExpressionOperationKind.Absolute =>
                        new MotionExpressionOperation.Absolute(),
                    Wire.MotionExpressionOperationKind.Minimum =>
                        new MotionExpressionOperation.Minimum(),
                    Wire.MotionExpressionOperationKind.Maximum =>
                        new MotionExpressionOperation.Maximum(),
                    Wire.MotionExpressionOperationKind.Clamp => new MotionExpressionOperation.Clamp(
                        value.Minimum,
                        value.Maximum
                    ),
                    Wire.MotionExpressionOperationKind.Modulo =>
                        new MotionExpressionOperation.Modulo(value.Value),
                    Wire.MotionExpressionOperationKind.Wrap => new MotionExpressionOperation.Wrap(
                        value.Minimum,
                        value.Maximum
                    ),
                    Wire.MotionExpressionOperationKind.ExponentialDecay =>
                        new MotionExpressionOperation.ExponentialDecay(value.Value),
                    Wire.MotionExpressionOperationKind.Mix => new MotionExpressionOperation.Mix(),
                    _ => throw new InvalidDataException("Unknown motion expression operation."),
                };

            private static MotionGestureDescriptor MotionGestures(
                Wire.MotionGestureDescriptor value
            )
            {
                Wire.MotionGestureSubscriptions subscriptions =
                    value.Subscriptions ?? throw Missing("motion gesture subscriptions");
                return new MotionGestureDescriptor(
                    value.PanThreshold,
                    value.DirectionLockThreshold,
                    value.PointerTapSlop,
                    value.TouchTapSlop,
                    value.Pan,
                    value.Drag.HasValue ? MotionDrag(value.Drag.Value) : null,
                    value.InView,
                    value.Scroll,
                    OptionalObjectId(value.ScrollXValue),
                    OptionalObjectId(value.ScrollYValue),
                    OptionalObjectId(value.InViewValue),
                    new MotionGestureSubscriptions(
                        subscriptions.Hover,
                        subscriptions.Tap,
                        subscriptions.Focus,
                        subscriptions.Pan,
                        subscriptions.PanUpdate,
                        subscriptions.Drag,
                        subscriptions.DragUpdate,
                        subscriptions.MomentumComplete,
                        subscriptions.ConstraintsMeasured,
                        subscriptions.Scroll,
                        subscriptions.InView,
                        subscriptions.FocusVisible
                    )
                );
            }

            private static MotionDragDescriptor MotionDrag(Wire.MotionDragDescriptor value)
            {
                Wire.MotionDragElastic elastic =
                    value.Elastic ?? throw Missing("motion drag elasticity");
                Wire.MotionDragTransition transition =
                    value.Transition ?? throw Missing("motion drag transition");
                MotionDragConstraint? constraint = value.Constraints.HasValue
                    ? MotionDragConstraint(value.Constraints.Value)
                    : null;
                return new MotionDragDescriptor(
                    (MotionGestureAxis)(byte)value.Axis,
                    constraint,
                    new MotionDragElastic(elastic.Left, elastic.Right, elastic.Top, elastic.Bottom),
                    value.Momentum,
                    value.DirectionLock,
                    value.Listener,
                    value.HasSnapToOrigin ? (MotionGestureAxis?)(byte)value.SnapToOrigin : null,
                    OptionalObjectId(value.ControlId),
                    value.Propagation,
                    new MotionDragTransition(
                        transition.VelocityRetention,
                        transition.RestSpeed,
                        transition.BounceStiffness,
                        transition.BounceDamping
                    ),
                    OptionalObjectId(value.XValue),
                    OptionalObjectId(value.YValue)
                );
            }

            private static MotionDragConstraint? MotionDragConstraint(
                Wire.MotionDragConstraint value
            ) =>
                value.Kind switch
                {
                    Wire.MotionDragConstraintKind.None => null,
                    Wire.MotionDragConstraintKind.Bounds when value.Bounds.HasValue =>
                        new MotionDragConstraint.Bounds(
                            new MotionDragBounds(
                                value.Bounds.Value.MinX,
                                value.Bounds.Value.MaxX,
                                value.Bounds.Value.MinY,
                                value.Bounds.Value.MaxY
                            )
                        ),
                    Wire.MotionDragConstraintKind.Element when value.ElementId.HasValue =>
                        new MotionDragConstraint.Element(ObjectId(value.ElementId)),
                    _ => throw new InvalidDataException(
                        "Motion drag constraint kind and payload do not match."
                    ),
                };

            private static MotionLayoutDescriptor MotionLayout(Wire.MotionLayoutDescriptor value) =>
                new(
                    (MotionLayoutMode)(byte)value.Mode,
                    MotionLayoutIdentity(value.Group ?? throw Missing("motion layout group")),
                    value.LayoutId.HasValue ? MotionLayoutIdentity(value.LayoutId.Value) : null,
                    value.Scroll,
                    value.Root,
                    value.PopLayout,
                    Transition(value.Transition ?? throw Missing("motion layout transition")),
                    value.Projection.HasValue ? MotionProjection(value.Projection.Value) : null
                );

            private static MotionProjectionDescriptor MotionProjection(
                Wire.MotionProjectionDescriptor value
            )
            {
                Wire.MotionProjectionPlane plane =
                    value.Plane ?? throw Missing("motion projection plane");
                Wire.Rectd rectangle =
                    value.WorldRect ?? throw Missing("motion projection rectangle");
                CameraTarget camera = value.CameraKind switch
                {
                    Wire.MotionProjectionCameraKind.Input => new CameraTarget.Input(),
                    Wire.MotionProjectionCameraKind.Object when value.CameraObjectId.HasValue =>
                        new CameraTarget.Object(ObjectId(value.CameraObjectId)),
                    _ => throw new InvalidDataException(
                        "Motion projection camera kind and payload do not match."
                    ),
                };
                return new MotionProjectionDescriptor(
                    camera,
                    new MotionProjectionPlane(
                        new Vector3(plane.Origin.X, plane.Origin.Y, plane.Origin.Z),
                        new Vector3(plane.XAxis.X, plane.XAxis.Y, plane.XAxis.Z),
                        new Vector3(plane.YAxis.X, plane.YAxis.Y, plane.YAxis.Z)
                    ),
                    new Rect(rectangle.X, rectangle.Y, rectangle.Width, rectangle.Height)
                );
            }

            private static MotionLayoutIdentity MotionLayoutIdentity(
                Wire.MotionLayoutIdentity value
            ) => new(Required(value.ValueType, "motion layout identity type"), value.ValueHash);
        }
    }
}

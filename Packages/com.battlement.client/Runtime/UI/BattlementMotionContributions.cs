#nullable enable

using System;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    /// <summary>Keeps graph values separate from the local animation's base.</summary>
    internal static class BattlementMotionContributions
    {
        private static readonly ConditionalWeakTable<
            VisualElement,
            Dictionary<MotionProperty, Layer>
        > Layers = new();

        public static MotionValue? Base(VisualElement target, MotionProperty property)
        {
            MotionProperty key = property is MotionProperty.ScaleX or MotionProperty.ScaleY
                ? MotionProperty.Scale
                : property;
            if (
                !Layers.TryGetValue(target, out var layers)
                || !layers.TryGetValue(key, out Layer layer)
            )
                return null;
            if (property is MotionProperty.ScaleX or MotionProperty.ScaleY)
            {
                var scale = (MotionValue.Vector2)layer.Base;
                return new MotionValue.Scalar(
                    scale.Value[property == MotionProperty.ScaleX ? 0 : 1]
                );
            }
            return layer.Base;
        }

        public static MotionValue UpdateBase(
            VisualElement target,
            MotionProperty property,
            MotionValue value
        )
        {
            if (
                !Layers.TryGetValue(target, out var layers)
                || !layers.TryGetValue(property, out Layer layer)
            )
                return value;
            layer.Base = value;
            return Compose(property, value, layer.Contribution);
        }

        public static void Set(VisualElement target, MotionProperty property, MotionValue value)
        {
            var layers = Layers.GetOrCreateValue(target);
            if (!layers.TryGetValue(property, out Layer layer))
            {
                layer = new Layer(
                    BattlementMotionPropertyWriter.ReadRendered(target, property),
                    value
                );
                layers.Add(property, layer);
            }
            layer.Contribution = value;
            BattlementMotionPropertyWriter.WriteRendered(
                target,
                property,
                Compose(property, layer.Base, value)
            );
        }

        public static void Remove(VisualElement target, MotionProperty property)
        {
            if (
                !Layers.TryGetValue(target, out var layers)
                || !layers.Remove(property, out Layer layer)
            )
                return;
            BattlementMotionPropertyWriter.WriteRendered(target, property, layer.Base);
        }

        private static MotionValue Compose(
            MotionProperty property,
            MotionValue lower,
            MotionValue upper
        ) =>
            (property, lower, upper) switch
            {
                (MotionProperty.Scale, MotionValue.Vector2 a, MotionValue.Vector2 b) =>
                    new MotionValue.Vector2(
                        new[] { a.Value[0] * b.Value[0], a.Value[1] * b.Value[1] }
                    ),
                _ => throw new InvalidOperationException(
                    "Graph composition requires compatible scale values."
                ),
            };

        private sealed class Layer
        {
            public MotionValue Base;
            public MotionValue Contribution;

            public Layer(MotionValue basis, MotionValue contribution)
            {
                Base = basis;
                Contribution = contribution;
            }
        }
    }
}

#nullable enable

using System;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementUiMotionTarget : IBattlementMotionTarget
    {
        private static readonly ConditionalWeakTable<
            VisualElement,
            BattlementUiMotionTarget
        > targets = new();

        private BattlementUiMotionTarget(VisualElement element) => Element = element;

        public static BattlementUiMotionTarget For(VisualElement element) =>
            targets.GetValue(element, value => new BattlementUiMotionTarget(value));

        public VisualElement Element { get; }

        public bool Supports(MotionProperty property) =>
            BattlementMotionPropertyWriter.Supports(property);

        public bool IsLayout(MotionProperty property) =>
            BattlementMotionPropertyWriter.IsLayout(property);

        public bool IsSpatial(MotionProperty property) =>
            BattlementMotionPropertyWriter.IsSpatial(property);

        public MotionValue Read(MotionProperty property) =>
            BattlementMotionPropertyWriter.Read(Element, property);

        public void Write(MotionProperty property, MotionValue value) =>
            BattlementMotionPropertyWriter.Write(Element, property, value);

        public void WriteScalar(MotionProperty property, double value) =>
            BattlementMotionPropertyWriter.WriteScalar(Element, property, value);

        public void WriteAdaptedScalar(MotionProperty property, double value) =>
            BattlementMotionPropertyWriter.WriteAdaptedScalar(Element, property, value);

        public void SetContribution(MotionProperty property, MotionValue value) =>
            BattlementMotionContributions.Set(Element, property, value);

        public void RemoveContribution(MotionProperty property) =>
            BattlementMotionContributions.Remove(Element, property);

        public bool Contains(IBattlementMotionTarget target) =>
            target is BattlementUiMotionTarget ui && Element.Contains(ui.Element);

        public bool IsParentOf(IBattlementMotionTarget target) =>
            target is BattlementUiMotionTarget ui && ReferenceEquals(ui.Element.parent, Element);

        public IReadOnlyList<MotionPropertyValue> ResolvePosition(
            IBattlementMotionTarget reference,
            string? anchor
        )
        {
            if (reference is not BattlementUiMotionTarget ui || anchor is not null)
                throw new InvalidOperationException(
                    "UI Motion positions require an unanchored UI reference."
                );
            if (Element.parent is null || ui.Element.panel != Element.panel)
                throw new InvalidOperationException(
                    "UI Motion positions require attached hosts in one panel."
                );
            Vector2 desired = Element.parent.WorldToLocal(ui.Element.worldBound.center);
            Vector2 origin = Element.layout.center;
            return new MotionPropertyValue[]
            {
                new(
                    MotionProperty.X,
                    new MotionValue.Length(new UiLength.Px(desired.x - origin.x))
                ),
                new(
                    MotionProperty.Y,
                    new MotionValue.Length(new UiLength.Px(desired.y - origin.y))
                ),
                new(MotionProperty.Z, new MotionValue.Length(new UiLength.Px(0))),
            };
        }

        public void Release() => BattlementMotionPropertyWriter.Release(Element);
    }
}

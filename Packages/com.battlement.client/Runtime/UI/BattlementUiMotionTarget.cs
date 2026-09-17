#nullable enable

using System.Runtime.CompilerServices;
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

        public void Release() => BattlementMotionPropertyWriter.Release(Element);
    }
}

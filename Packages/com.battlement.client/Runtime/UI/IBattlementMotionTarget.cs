#nullable enable

using System.Collections.Generic;

namespace Battlement.UI
{
    /// <summary>Reads and writes the presentation owned by a Motion host.</summary>
    internal interface IBattlementMotionTarget
    {
        bool Supports(MotionProperty property);
        bool IsLayout(MotionProperty property);
        bool IsSpatial(MotionProperty property);
        MotionValue Read(MotionProperty property);
        void Write(MotionProperty property, MotionValue value);
        void WriteScalar(MotionProperty property, double value);
        void WriteAdaptedScalar(MotionProperty property, double value);
        void SetContribution(MotionProperty property, MotionValue value);
        void RemoveContribution(MotionProperty property);
        bool Contains(IBattlementMotionTarget target);
        bool IsParentOf(IBattlementMotionTarget target);
        IReadOnlyList<MotionPropertyValue> ResolvePosition(
            IBattlementMotionTarget reference,
            string? anchor
        );
        void Release();
    }
}

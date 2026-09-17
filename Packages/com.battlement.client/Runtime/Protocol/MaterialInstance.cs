#nullable enable
using System.Collections.Generic;

namespace Battlement
{
    /// <summary>Continuous shader property types supported by renderer overrides.</summary>
    public enum MaterialParameterKind
    {
        Float,
        Color,
        Vector,
    }

    /// <summary>A required named property on a prepared material's shader.</summary>
    public sealed record MaterialParameterDeclaration(string Name, MaterialParameterKind Kind);

    /// <summary>A typed renderer-local value; unused scalar components are zero.</summary>
    public sealed record MaterialParameterValue(
        string Name,
        MaterialParameterKind Kind,
        double X,
        double Y = 0,
        double Z = 0,
        double W = 0
    );

    /// <summary>A prepared material and renderer-local values for one material slot.</summary>
    public sealed record MaterialInstance(
        MaterialAddress Address,
        uint Slot,
        IReadOnlyList<MaterialParameterValue> Parameters
    );
}

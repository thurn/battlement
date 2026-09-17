#nullable enable
using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.Rendering;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    internal static class BattlementMaterialParameters
    {
        internal static PreparedAsset.MaterialParameters Read(Wire.PreparedAsset value)
        {
            var parameters = new MaterialParameterDeclaration[value.ParametersLength];
            for (int i = 0; i < parameters.Length; i++)
            {
                Wire.MaterialParameterDeclaration p = value.Parameters(i)!.Value;
                parameters[i] = new MaterialParameterDeclaration(
                    p.Name,
                    (MaterialParameterKind)p.Kind
                );
            }
            return new PreparedAsset.MaterialParameters(
                new MaterialAddress(value.Address),
                parameters
            );
        }

        internal static MaterialInstance[] Read(int count, Func<int, Wire.MaterialInstance?> read)
        {
            var result = new MaterialInstance[count];
            for (int i = 0; i < count; i++)
            {
                Wire.MaterialInstance m = read(i)!.Value;
                var parameters = new MaterialParameterValue[m.ParametersLength];
                for (int p = 0; p < parameters.Length; p++)
                {
                    Wire.MaterialParameterValue v = m.Parameters(p)!.Value;
                    parameters[p] = new MaterialParameterValue(
                        v.Name,
                        (MaterialParameterKind)v.Kind,
                        v.X,
                        v.Y,
                        v.Z,
                        v.W
                    );
                }
                result[i] = new MaterialInstance(
                    new MaterialAddress(m.Address),
                    m.Slot,
                    parameters
                );
            }
            return result;
        }

        internal static void Validate(Material material, string name, MaterialParameterKind kind)
        {
            int index = material.shader.FindPropertyIndex(name);
            if (index < 0)
                throw Invalid($"Material '{material.name}' is missing shader property '{name}'.");
            ShaderPropertyType actual = material.shader.GetPropertyType(index);
            bool valid = kind switch
            {
                MaterialParameterKind.Float => actual
                    is ShaderPropertyType.Float
                        or ShaderPropertyType.Range,
                MaterialParameterKind.Color => actual == ShaderPropertyType.Color,
                MaterialParameterKind.Vector => actual == ShaderPropertyType.Vector,
                _ => false,
            };
            if (!valid)
                throw Invalid($"Material property '{name}' is {actual}, not {kind}.");
        }

        internal static void Validate(
            Material material,
            IReadOnlyList<MaterialParameterDeclaration> parameters
        )
        {
            var names = new HashSet<string>(StringComparer.Ordinal);
            foreach (MaterialParameterDeclaration p in parameters)
            {
                if (string.IsNullOrEmpty(p.Name) || !names.Add(p.Name))
                    throw Invalid("Invalid material parameter declaration.");
                Validate(material, p.Name, p.Kind);
            }
        }

        internal static BattlementAssetException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}

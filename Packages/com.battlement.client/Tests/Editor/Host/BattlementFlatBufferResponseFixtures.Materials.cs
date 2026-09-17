#nullable enable
using System.Collections.Generic;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    internal static partial class BattlementFlatBufferResponseFixtures
    {
        private static VectorOffset WriteInstances(
            FlatBufferBuilder builder,
            IReadOnlyList<MaterialInstance> values
        )
        {
            var entries = new int[values.Count];
            for (int i = 0; i < entries.Length; i++)
            {
                MaterialInstance m = values[i];
                StringOffset address = builder.CreateString(m.Address.Value);
                var parameters = new int[m.Parameters.Count];
                for (int j = 0; j < parameters.Length; j++)
                {
                    MaterialParameterValue p = m.Parameters[j];
                    parameters[j] = Wire
                        .MaterialParameterValue.CreateMaterialParameterValue(
                            builder,
                            builder.CreateString(p.Name),
                            (Wire.MaterialParameterKind)p.Kind,
                            p.X,
                            p.Y,
                            p.Z,
                            p.W
                        )
                        .Value;
                }
                VectorOffset vector = OffsetVector(builder, parameters);
                entries[i] = Wire
                    .MaterialInstance.CreateMaterialInstance(builder, address, m.Slot, vector)
                    .Value;
            }
            return OffsetVector(builder, entries);
        }

        private static Payload SetInstances(
            FlatBufferBuilder builder,
            CommandBody.Renderer.SetInstances value
        )
        {
            VectorOffset instances = WriteInstances(builder, value.Instances);
            Wire.RendererInstancesPayload.StartRendererInstancesPayload(builder);
            Wire.RendererInstancesPayload.AddInstances(builder, instances);
            Wire.RendererInstancesPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new Payload(
                Wire.CoreCommandKind.RendererSetInstances,
                Wire.CoreCommandPayload.RendererInstancesPayload,
                Wire.RendererInstancesPayload.EndRendererInstancesPayload(builder).Value
            );
        }
    }
}

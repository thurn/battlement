#nullable enable

using System;
using System.IO;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    internal static class BattlementFlatBufferWriter
    {
        internal static Offset<Wire.Uuid> WriteUuid(FlatBufferBuilder builder, Guid value)
        {
            if (value == Guid.Empty)
                throw new InvalidDataException("Protocol UUIDs must be nonzero.");
            Span<byte> mixed = stackalloc byte[16];
            if (!value.TryWriteBytes(mixed))
                throw new InvalidOperationException("Could not write a UUID.");
            ReadOnlySpan<byte> order = stackalloc byte[16]
            {
                3,
                2,
                1,
                0,
                5,
                4,
                7,
                6,
                8,
                9,
                10,
                11,
                12,
                13,
                14,
                15,
            };
            builder.Prep(1, 16);
            for (int index = 15; index >= 0; index--)
                builder.PutByte(mixed[order[index]]);
            return new Offset<Wire.Uuid>(builder.Offset);
        }
    }
}

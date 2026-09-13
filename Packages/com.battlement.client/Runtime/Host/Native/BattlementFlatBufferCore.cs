#nullable enable

using System;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    /// <summary>Narrow helpers shared by generated build-composed response readers.</summary>
    public static class BattlementFlatBufferCore
    {
        /// <summary>Validates one already structurally verified core command.</summary>
        public static void ValidateCommand(Wire.CoreCommand value) =>
            BattlementFlatBufferResponse.ValidateCommand(value);

        public static void ValidateSnapshot(Wire.Snapshot value, Guid expectedSession) =>
            BattlementFlatBufferResponse.ValidateSnapshot(value, expectedSession);

        /// <summary>Reads one canonical nonzero UUID.</summary>
        public static Guid ReadUuid(Wire.Uuid? value, string field) =>
            BattlementFlatBufferResponse.ReadUuid(value, field);
    }
}

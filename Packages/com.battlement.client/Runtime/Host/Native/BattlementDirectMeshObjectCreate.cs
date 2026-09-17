#nullable enable

namespace Battlement
{
    internal readonly struct BattlementDirectMeshObjectCreate
    {
        internal BattlementDirectMeshObjectCreate(
            BattlementDirectObjectPlacement placement,
            string address,
            BattlementDirectMaterialAssignment[] materials
        ) => (Placement, Address, Materials) = (placement, address, materials);

        internal BattlementDirectObjectPlacement Placement { get; }
        internal string Address { get; }
        internal BattlementDirectMaterialAssignment[] Materials { get; }
    }
}

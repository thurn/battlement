#nullable enable

using System;

namespace Battlement.UI
{
    /// <summary>
    /// Acquires prepared assets for live UI Toolkit properties without loading them.
    /// </summary>
    public interface IBattlementUiAssetLookup
    {
        /// <summary>
        /// Acquires an owned usage lease for an exact prepared declaration.
        /// </summary>
        IBattlementUiAssetLease Acquire(PreparedAsset asset);
    }

    /// <summary>Keeps a prepared Unity asset alive while a UI property references it.</summary>
    public interface IBattlementUiAssetLease : IDisposable
    {
        /// <summary>Gets the exact prepared declaration retained by this lease.</summary>
        PreparedAsset Asset { get; }

        /// <summary>Gets the prepared Unity object after type-checked loading completed.</summary>
        object Value { get; }
    }
}

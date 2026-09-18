#nullable enable

using System;

namespace Battlement
{
    /// <summary>Reports unfinished work whose clock is paused or explicitly controlled.</summary>
    public interface IBattlementHeldCommandOperation : IBattlementCommandOperation
    {
        bool IsHeld { get; }
    }

    /// <summary>Work started by a command that Battlement can poll and cancel.</summary>
    public interface IBattlementCommandOperation
    {
        /// <summary>Gets whether the operation has no natural completion.</summary>
        bool IsInfinite { get; }

        /// <summary>Advances the operation and reports whether it has completed.</summary>
        bool IsComplete(TimeSpan now);

        /// <summary>Cancels the operation without firing completion behavior.</summary>
        void Cancel();
    }

    /// <summary>Temporarily suspends an operation without completing or cancelling it.</summary>
    internal interface IBattlementPausableCommandOperation
    {
        void Pause(TimeSpan now);
        void Resume(TimeSpan now);
    }
}

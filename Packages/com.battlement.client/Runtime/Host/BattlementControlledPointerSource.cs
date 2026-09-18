#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

namespace Battlement
{
    internal sealed record BattlementControlledPointerLease(string Session, ulong Generation);

    internal sealed record BattlementControlledPointerSpace(
        int ViewportWidth,
        int ViewportHeight,
        string? CameraIdentity
    );

    internal sealed record BattlementControlledPointerSample(
        BattlementControlledPointerLease Lease,
        ulong Sequence,
        int PointerId,
        BattlementControlledPointerSpace Space,
        Vector2 Position,
        IReadOnlyCollection<PointerButton> Buttons,
        bool IsPresent,
        bool IsCancelled,
        ObjectId? ExpectedTarget,
        ulong PresentationBoundary
    );

    internal sealed record BattlementControlledPointerReceipt(
        string Session,
        ulong Generation,
        ulong Sequence,
        int PointerId,
        Vector2 Position,
        ObjectId? ExpectedTarget,
        ObjectId? ActualHit,
        ObjectId? CaptureOwner,
        string Route,
        ulong PresentationBoundary
    );

    /// <summary>Exclusively leases and orders bounded host pointer samples.</summary>
    internal sealed class BattlementControlledPointerSource
    {
        internal const int MaximumPendingSamples = 256;
        internal const int MaximumPendingReceipts = 512;

        private readonly Queue<BattlementControlledPointerSample> samples = new();
        private readonly Queue<BattlementControlledPointerReceipt> receipts = new();
        private BattlementControlledPointerLease? lease;
        private ulong generation;
        private ulong nextSequence;
        private ulong lastPresentationBoundary;
        private string? failure;

        internal bool IsActive => lease is not null;

        internal bool HasPendingSamples => samples.Count != 0;

        internal string? Failure => failure;

        internal BattlementControlledPointerLease Begin(string session)
        {
            if (string.IsNullOrWhiteSpace(session))
                throw new ArgumentException("Controlled input requires a session identity.");
            if (lease is not null)
                throw new InvalidOperationException(
                    $"Controlled input is already leased by {lease.Session}."
                );
            if (samples.Count != 0 || receipts.Count != 0)
                throw new InvalidOperationException(
                    "Controlled input retained prior session data."
                );
            generation = checked(generation + 1);
            nextSequence = 0;
            lastPresentationBoundary = 0;
            failure = null;
            lease = new BattlementControlledPointerLease(session, generation);
            return lease;
        }

        internal void Enqueue(BattlementControlledPointerSample sample)
        {
            RequireLease(sample.Lease);
            if (failure is not null)
                throw new InvalidOperationException(failure);
            if (sample.Sequence != nextSequence)
                throw new InvalidOperationException(
                    $"Controlled pointer sample {sample.Sequence} is out of order; "
                        + $"expected {nextSequence}."
                );
            if (sample.PointerId < 0)
                throw new ArgumentOutOfRangeException(
                    nameof(sample),
                    "Controlled pointer identities must be nonnegative."
                );
            if (sample.Space.ViewportWidth <= 0 || sample.Space.ViewportHeight <= 0)
                throw new ArgumentOutOfRangeException(
                    nameof(sample),
                    "Controlled pointer samples require a nonempty viewport."
                );
            if (!float.IsFinite(sample.Position.x) || !float.IsFinite(sample.Position.y))
                throw new ArgumentOutOfRangeException(
                    nameof(sample),
                    "Controlled pointer coordinates must be finite."
                );
            if (
                sample.Position.x < 0
                || sample.Position.x > sample.Space.ViewportWidth
                || sample.Position.y < 0
                || sample.Position.y > sample.Space.ViewportHeight
            )
                throw new ArgumentOutOfRangeException(
                    nameof(sample),
                    "Controlled pointer coordinates must be inside the declared viewport."
                );
            if (sample.PresentationBoundary == 0)
                throw new ArgumentOutOfRangeException(
                    nameof(sample),
                    "Controlled pointer samples require a presentation boundary."
                );
            if (sample.PresentationBoundary <= lastPresentationBoundary)
                throw new InvalidOperationException(
                    "Controlled pointer presentation boundaries must increase."
                );
            if (sample.IsCancelled && sample.Buttons.Count != 0)
                throw new ArgumentException(
                    "A cancelled controlled pointer sample cannot retain buttons."
                );
            if (sample.Buttons.Count != sample.Buttons.Distinct().Count())
                throw new ArgumentException("Controlled pointer buttons must be unique.");
            if (samples.Count == MaximumPendingSamples)
                throw new InvalidOperationException("The controlled pointer queue is full.");
            samples.Enqueue(sample);
            nextSequence = checked(nextSequence + 1);
            lastPresentationBoundary = sample.PresentationBoundary;
        }

        internal IReadOnlyList<BattlementControlledPointerSample> Drain()
        {
            var result = new List<BattlementControlledPointerSample>(samples.Count);
            while (samples.TryDequeue(out BattlementControlledPointerSample sample))
                result.Add(sample);
            return result;
        }

        internal void Record(BattlementControlledPointerReceipt receipt)
        {
            if (lease is null)
                throw new InvalidOperationException("Controlled input is not leased.");
            if (receipt.Session != lease.Session || receipt.Generation != lease.Generation)
                throw new InvalidOperationException("Controlled pointer receipt lease changed.");
            if (receipts.Count == MaximumPendingReceipts)
            {
                Fail("The controlled pointer receipt queue is full.");
                throw new InvalidOperationException(failure);
            }
            receipts.Enqueue(receipt);
        }

        internal IReadOnlyList<BattlementControlledPointerReceipt> TakeReceipts(
            BattlementControlledPointerLease owner
        )
        {
            RequireLease(owner);
            var result = new List<BattlementControlledPointerReceipt>(receipts.Count);
            while (receipts.TryDequeue(out BattlementControlledPointerReceipt receipt))
                result.Add(receipt);
            return result;
        }

        internal void Fail(string reason)
        {
            if (lease is null || failure is not null)
                return;
            failure = string.IsNullOrWhiteSpace(reason) ? "Controlled input failed." : reason;
            samples.Clear();
        }

        internal void End(BattlementControlledPointerLease owner)
        {
            RequireLease(owner);
            samples.Clear();
            receipts.Clear();
            lease = null;
            nextSequence = 0;
            lastPresentationBoundary = 0;
            failure = null;
        }

        private void RequireLease(BattlementControlledPointerLease owner)
        {
            if (lease is null)
                throw new InvalidOperationException("Controlled input is not leased.");
            if (owner.Session != lease.Session || owner.Generation != lease.Generation)
                throw new InvalidOperationException(
                    "The controlled input lease is stale or belongs to another session."
                );
        }
    }
}

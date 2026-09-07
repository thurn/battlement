#nullable enable

using System;

namespace Battlement
{
    internal sealed record DittoInputReceipt(
        string TransactionId,
        ObjectId Target,
        ulong ResolvedFrame,
        ulong AppliedInputFrame,
        ulong PresentedFrame,
        string Route
    );

    internal sealed class DittoInputTransaction
    {
        private readonly string id;
        private readonly ObjectId target;
        private readonly ulong resolvedFrame;
        private readonly ulong expectedInputFrame;
        private ulong appliedInputFrame;
        private ulong? deliveredInputFrame;
        private string? route;
        private string? rejection;

        public DittoInputTransaction(
            string transactionId,
            ObjectId expectedTarget,
            ulong frame,
            ulong inputFrame
        )
        {
            if (string.IsNullOrWhiteSpace(transactionId))
                throw new ArgumentException(
                    "An input transaction requires an identity.",
                    nameof(transactionId)
                );
            if (frame == 0)
                throw new ArgumentOutOfRangeException(
                    nameof(frame),
                    "Input requires a committed presentation."
                );
            if (inputFrame == 0)
                throw new ArgumentOutOfRangeException(nameof(inputFrame));
            id = transactionId;
            target = expectedTarget;
            resolvedFrame = frame;
            expectedInputFrame = inputFrame;
        }

        public string Id => id;

        public void Apply(ulong frame)
        {
            if (rejection is not null)
                return;
            appliedInputFrame = frame;
        }

        public void Observe(ObjectId deliveredTarget, string deliveredRoute)
        {
            if (route is not null || rejection is not null)
                return;
            if (deliveredTarget != target)
            {
                rejection =
                    $"Pointer transaction {id} reached {deliveredTarget.Value} "
                    + $"instead of {target.Value}.";
                return;
            }
            route = deliveredRoute;
            deliveredInputFrame = appliedInputFrame;
        }

        public void Reject(string reason) => rejection ??= reason;

        public bool Complete(
            ulong appliedInputFrame,
            ulong presentedFrame,
            out DittoInputReceipt? receipt,
            out string? diagnostic
        )
        {
            receipt = null;
            if (rejection is not null)
            {
                diagnostic = rejection;
                return false;
            }
            if (route is null)
            {
                diagnostic =
                    $"Pointer transaction {id} produced no semantic delivery receipt "
                    + $"for {target.Value}.";
                return false;
            }
            if (appliedInputFrame != expectedInputFrame)
            {
                diagnostic =
                    $"Pointer transaction {id} did not apply input frame {expectedInputFrame}.";
                return false;
            }
            if (deliveredInputFrame != expectedInputFrame)
            {
                diagnostic =
                    $"Pointer transaction {id} received its semantic delivery on frame "
                    + $"{deliveredInputFrame?.ToString() ?? "none"}, not {expectedInputFrame}.";
                return false;
            }
            if (presentedFrame <= resolvedFrame)
            {
                diagnostic =
                    $"Pointer transaction {id} produced no presentation commit after delivery.";
                return false;
            }
            receipt = new DittoInputReceipt(
                id,
                target,
                resolvedFrame,
                appliedInputFrame,
                presentedFrame,
                route
            );
            diagnostic = null;
            return true;
        }
    }
}

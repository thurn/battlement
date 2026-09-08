#nullable enable

using System;

namespace Battlement
{
    internal sealed record DittoActivationReceipt(
        string TransactionId,
        ObjectId Target,
        ActionId Action,
        ulong ResolvedFrame,
        ulong PresentedFrame,
        string Route
    );

    internal sealed class DittoActivationTransaction
    {
        private readonly string id;
        private readonly ObjectId target;
        private readonly ulong resolvedFrame;
        private ActionId? action;
        private string? causalRoute;
        private string? route;
        private string? rejection;

        public DittoActivationTransaction(string transactionId, ObjectId target, ulong frame)
        {
            if (string.IsNullOrWhiteSpace(transactionId))
                throw new ArgumentException(
                    "An activation transaction requires an identity.",
                    nameof(transactionId)
                );
            if (frame == 0)
                throw new ArgumentOutOfRangeException(
                    nameof(frame),
                    "Activation requires a committed presentation."
                );
            if (target.Value == Guid.Empty)
                throw new ArgumentException("Activation requires a target.", nameof(target));
            id = transactionId;
            this.target = target;
            resolvedFrame = frame;
        }

        public string Id => id;

        public bool HasDispatched => action is not null;

        public void BeginDispatch(ActionId actionId, string? receiptOnCausalBatch = null)
        {
            if (action is not null)
                throw new InvalidOperationException("Activation dispatch already began.");
            if (actionId.Value == Guid.Empty)
                throw new ArgumentException("Activation action identity must be nonzero.");
            action = actionId;
            causalRoute = receiptOnCausalBatch;
        }

        public bool TryBeginUiDispatch(
            ActionId actionId,
            ObjectId deliveredTarget,
            UiEventBody body,
            out string? deliveredRoute
        )
        {
            deliveredRoute = body switch
            {
                UiEventBody.AccessibilityAction => "ui-accessibility",
                UiEventBody.Click { Value: ClickEvent.NavigationSubmit } => "ui-navigation-submit",
                UiEventBody.ValueCommitted => "ui-value-commit",
                _ => null,
            };
            if (deliveredRoute is null || deliveredTarget != target || action is not null)
                return false;
            BeginDispatch(actionId, deliveredRoute == "ui-accessibility" ? null : deliveredRoute);
            return true;
        }

        public void ObserveHandled(
            ActionId actionId,
            ObjectId deliveredTarget,
            string deliveredRoute
        )
        {
            if (route is not null || rejection is not null)
                return;
            if (action != actionId)
            {
                rejection = $"Activation transaction {id} received another action.";
                return;
            }
            if (deliveredTarget != target)
            {
                rejection =
                    $"Activation transaction {id} reached {deliveredTarget.Value} "
                    + $"instead of {target.Value}.";
                return;
            }
            route = deliveredRoute;
        }

        public void ObserveCausalBatch(ActionId actionId)
        {
            if (action == actionId && causalRoute is string deliveredRoute)
                ObserveHandled(actionId, target, deliveredRoute);
        }

        public void Reject(string reason) => rejection ??= reason;

        public bool ValidateDelivery(out string? diagnostic)
        {
            if (rejection is not null)
            {
                diagnostic = rejection;
                return false;
            }
            if (action is null || route is null)
            {
                diagnostic =
                    $"Activation transaction {id} produced no semantic delivery receipt "
                    + $"for {target.Value}.";
                return false;
            }
            diagnostic = null;
            return true;
        }

        public bool Complete(
            ulong presentedFrame,
            out DittoActivationReceipt? receipt,
            out string? diagnostic
        )
        {
            receipt = null;
            if (!ValidateDelivery(out diagnostic))
                return false;
            ActionId actionId = action!.Value;
            if (presentedFrame <= resolvedFrame)
            {
                diagnostic = $"Activation transaction {id} produced no later presentation commit.";
                return false;
            }
            receipt = new DittoActivationReceipt(
                id,
                target,
                actionId,
                resolvedFrame,
                presentedFrame,
                route!
            );
            diagnostic = null;
            return true;
        }
    }
}

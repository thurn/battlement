#nullable enable

using System;
using UnityEngine;
using SystemAction = System.Action;

namespace Battlement
{
    internal sealed class BattlementFailureSurface : IDisposable
    {
        private const string RecoverableMessage =
            "The game ran into an unexpected problem. "
            + "Close this window to safely reload the current session.";
        private readonly BattlementErrorDialog? dialog;
        private readonly Func<bool> isFallbackSuppressed;
        private readonly IBattlementFailurePresenter? presenter;
        private bool completedInitialSnapshot;
        private bool presenterFailed;

        public BattlementFailureSurface(
            Transform parent,
            bool showFallback,
            IBattlementFailurePresenter? presenter,
            SystemAction continueAfterFailure,
            Func<bool> isFallbackSuppressed
        )
        {
            this.presenter = presenter;
            this.isFallbackSuppressed = isFallbackSuppressed;
            if (showFallback)
            {
                dialog = new BattlementErrorDialog(
                    parent,
                    "Player Error Dialog",
                    9_999,
                    BattlementErrorDialogVariant.Player
                );
                dialog.Close.clicked += continueAfterFailure;
                dialog.CopyId.clicked += CopyId;
            }
            Refresh(false);
        }

        public BattlementPlayerFailure? Current { get; private set; }

        public void Clear(IBattlementLogger logger)
        {
            Current = null;
            presenterFailed = false;
            if (presenter is not null)
            {
                try
                {
                    presenter.Hide();
                }
                catch (Exception exception)
                {
                    presenterFailed = true;
                    LogPresenterFailure(logger, "cleared", exception);
                }
            }
            Refresh(completedInitialSnapshot);
        }

        public void Show(BattlementPlayerFailure failure, IBattlementLogger logger)
        {
            Current = failure;
            if (presenter is not null)
            {
                try
                {
                    presenter.Show(failure);
                }
                catch (Exception exception)
                {
                    presenterFailed = true;
                    LogPresenterFailure(logger, "shown", exception);
                }
            }
            Refresh(completedInitialSnapshot);
        }

        public void Refresh(bool hasCompletedInitialSnapshot)
        {
            completedInitialSnapshot = hasCompletedInitialSnapshot;
            if (dialog is null)
            {
                return;
            }
            if (isFallbackSuppressed())
            {
                dialog.Hide();
                return;
            }
            if (presenter is not null && !presenterFailed)
            {
                dialog.Hide();
                return;
            }
            if (Current is not null)
            {
                ShowError(Current);
                return;
            }
            if (completedInitialSnapshot)
            {
                dialog.Hide();
                return;
            }
            dialog.ShowLoading();
        }

        public void Dispose() => dialog?.Dispose();

        private void ShowError(BattlementPlayerFailure failure)
        {
            if (dialog is null)
            {
                return;
            }

            dialog.Title.text = "Something went wrong";
            dialog.Summary.text =
                failure.Kind == BattlementPlayerFailureKind.ContinueAllowed
                    ? RecoverableMessage
                    : "The game can't safely continue. Please restart it.";
            dialog.ErrorId.text = $"Error ID  {failure.ErrorId}";
            dialog.ShowError(failure.Kind == BattlementPlayerFailureKind.ContinueAllowed);
        }

        private void CopyId()
        {
            if (Current is null || dialog is null)
            {
                return;
            }

            GUIUtility.systemCopyBuffer = Current.ErrorId;
            dialog.ShowCopyConfirmation();
        }

        private static void LogPresenterFailure(
            IBattlementLogger logger,
            string action,
            Exception exception
        ) =>
            logger.Log(
                new BattlementLogRecord(
                    BattlementLogSeverity.Warning,
                    "battlement.failure_presenter.failed",
                    $"The game-owned failure presenter could not be {action}.",
                    Exception: exception
                )
            );
    }
}

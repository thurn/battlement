#nullable enable

using System;
using System.Linq;
using System.Text;
using UnityEngine;
using SystemAction = System.Action;

namespace Battlement
{
    /// <summary>Detailed runtime diagnostics shown unless explicitly suppressed.</summary>
    internal sealed class BattlementDevelopmentDiagnostics : IDisposable
    {
        private const string ReloadSummary =
            "Close this window to discard the failed engine and reload the current session.";
        private const string RestartSummary =
            "The game cannot safely continue and must be restarted.";
        private readonly BattlementErrorDialog dialog;
        private readonly SystemAction continueAfterNativeFailure;
        private BattlementError? current;

        public BattlementDevelopmentDiagnostics(
            Transform parent,
            SystemAction continueAfterNativeFailure
        )
        {
            this.continueAfterNativeFailure = continueAfterNativeFailure;
            dialog = new BattlementErrorDialog(
                parent,
                "Development Error Dialog",
                10_000,
                BattlementErrorDialogVariant.Development
            );
            dialog.Close.clicked += Close;
            dialog.CopyId.clicked += CopyId;
            dialog.CopyDetails.clicked += CopyDetails;
        }

        public bool IsVisible => dialog.IsVisible;

        public void Show(BattlementError error)
        {
            if (
                IsVisible
                && current?.Source == BattlementErrorSource.Native
                && error.Source == BattlementErrorSource.Unity
            )
            {
                return;
            }

            current = error;
            dialog.Title.text =
                error.Source == BattlementErrorSource.Native ? "Rust panic" : "Unity exception";
            dialog.Summary.text = Summary(error);
            dialog.ErrorId.text = $"Error ID  {error.Id}";
            dialog.Details.text = DiagnosticText(error);
            dialog.ShowError(error.Type != BattlementErrorType.RestartRequired);
        }

        public void Hide() => dialog.Hide();

        public void Dispose() => dialog.Dispose();

        private static string DiagnosticText(BattlementError error)
        {
            var text = new StringBuilder();
            text.AppendLine($"[{error.EventName}] {error.Message}");
            text.AppendLine();
            foreach (var field in error.Fields.OrderBy(field => field.Key))
            {
                text.AppendLine($"{field.Key}: {field.Value}");
            }

            string diagnostic = error.Exception?.ToString() ?? error.StackTrace ?? string.Empty;
            if (!string.IsNullOrWhiteSpace(diagnostic))
            {
                text.AppendLine();
                text.Append(diagnostic);
            }
            return text.ToString();
        }

        private static string Summary(BattlementError error)
        {
            if (
                error.Source == BattlementErrorSource.Native
                && error.Type == BattlementErrorType.SessionFailed
            )
            {
                return $"{error.Message}\n{ReloadSummary}";
            }
            if (error.Type == BattlementErrorType.RestartRequired)
            {
                return $"{error.Message}\n{RestartSummary}";
            }
            return error.Message;
        }

        private void Close()
        {
            bool recoverNative =
                current?.Source == BattlementErrorSource.Native
                && current?.Type == BattlementErrorType.SessionFailed;
            Hide();
            if (recoverNative)
            {
                continueAfterNativeFailure();
            }
        }

        private void CopyId()
        {
            if (current is not null)
            {
                GUIUtility.systemCopyBuffer = current.Id;
            }
        }

        private void CopyDetails()
        {
            if (current is not null)
            {
                GUIUtility.systemCopyBuffer = DiagnosticText(current);
            }
        }
    }
}

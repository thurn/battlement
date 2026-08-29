#nullable enable

using System;
using System.Linq;
using System.Text;
using UnityEngine;
using SystemAction = System.Action;

namespace Battlement.Errors
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
                error.Source == BattlementErrorSource.Native ? "Rust panic" : "C# exception";
            dialog.Summary.text = Summary(error);
            dialog.ErrorId.text = $"Error ID  {error.Id}";
            dialog.Details.text = DiagnosticText(error, true);
            dialog.ShowError(error.Type != BattlementErrorType.RestartRequired);
        }

        public void Hide()
        {
            current = null;
            dialog.Hide();
        }

        public void Dispose() => dialog.Dispose();

        private static string DiagnosticText(BattlementError error, bool richText = false)
        {
            var text = new StringBuilder();
            string header = $"[{error.EventName}] {error.Message}";
            text.AppendLine(richText ? BattlementAnsiText.Escape(header) : header);
            text.AppendLine();
            foreach (var field in error.Fields.OrderBy(field => field.Key))
            {
                string value = $"{field.Key}: {field.Value}";
                text.AppendLine(richText ? BattlementAnsiText.Escape(value) : value);
            }

            string diagnostic = error.Exception?.ToString() ?? error.StackTrace ?? string.Empty;
            if (!string.IsNullOrWhiteSpace(diagnostic))
            {
                text.AppendLine();
                text.Append(richText ? RichDiagnostic(error, diagnostic) : diagnostic);
            }
            return text.ToString();
        }

        private static string RichDiagnostic(BattlementError error, string diagnostic)
        {
            if (error.AnsiStackTrace is not null)
            {
                return BattlementAnsiText.Format(error.AnsiStackTrace).RichText;
            }
            if (error.Source == BattlementErrorSource.Unity)
            {
                return BattlementCSharpExceptionText
                    .Format(error.Exception, error.StackTrace, error.Message)
                    .RichText;
            }
            return BattlementAnsiText.Escape(diagnostic);
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

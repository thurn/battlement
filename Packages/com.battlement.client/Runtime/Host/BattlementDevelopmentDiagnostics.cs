#nullable enable

using System;
using System.Linq;
using System.Text;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using SystemAction = System.Action;
using UnityColor = UnityEngine.Color;

namespace Battlement
{
    /// <summary>Detailed runtime diagnostics shown unless explicitly suppressed.</summary>
    internal sealed class BattlementDevelopmentDiagnostics : IDisposable
    {
        private const string ReloadSummary =
            "Close this window to discard the failed engine and reload the current session.";
        private const string RestartSummary =
            "The game cannot safely continue and must be restarted.";
        private readonly GameObject host;
        private readonly PanelSettings panelSettings;
        private readonly VisualElement overlay;
        private readonly Label title;
        private readonly Label subtitle;
        private readonly Label incidentId;
        private readonly Label details;
        private readonly Button close;
        private readonly SystemAction continueAfterNativeFailure;
        private BattlementIncident? current;

        public BattlementDevelopmentDiagnostics(
            Transform parent,
            SystemAction continueAfterNativeFailure
        )
        {
            this.continueAfterNativeFailure = continueAfterNativeFailure;
            host = new GameObject("Development Error Dialog");
            host.SetActive(false);
            host.transform.SetParent(parent, false);
            panelSettings = Resources.Load<PanelSettings>("BattlementDevelopmentPanelSettings");
            if (panelSettings == null)
            {
                throw new InvalidOperationException(
                    "Battlement development dialog panel settings are missing."
                );
            }

            UIDocument document = host.AddComponent<UIDocument>();
            document.panelSettings = panelSettings;
            document.sortingOrder = 10_000;
            host.SetActive(true);

            overlay = document.rootVisualElement;
            ConfigureOverlay(overlay);
            VisualElement dialog = CreateDialog();
            overlay.Add(dialog);

            VisualElement header = CreateHeader();
            dialog.Add(header);
            title = CreateLabel(20, FontStyle.Bold, UnityColor.white);
            title.name = "development-error-title";
            title.style.flexGrow = 1;
            header.Add(title);
            close = CreateButton("×", Close);
            close.name = "development-error-close";
            close.tooltip = "Close diagnostics";
            ConfigureCloseButton(close);
            dialog.Add(close);

            subtitle = CreateLabel(14, FontStyle.Normal, new UnityColor(0.82f, 0.84f, 0.88f));
            subtitle.name = "development-error-summary";
            subtitle.style.marginBottom = 12;
            dialog.Add(subtitle);

            ScrollView scroll = new(ScrollViewMode.Vertical);
            scroll.name = "development-error-scroll-view";
            scroll.style.flexGrow = 1;
            scroll.style.backgroundColor = new UnityColor(0.055f, 0.06f, 0.07f, 1);
            scroll.style.borderTopWidth = 1;
            scroll.style.borderRightWidth = 1;
            scroll.style.borderBottomWidth = 1;
            scroll.style.borderLeftWidth = 1;
            SetBorderColor(scroll, new UnityColor(0.22f, 0.24f, 0.28f, 1));
            scroll.style.paddingTop = 14;
            scroll.style.paddingRight = 14;
            scroll.style.paddingBottom = 14;
            scroll.style.paddingLeft = 14;
            dialog.Add(scroll);
            details = CreateLabel(13, FontStyle.Normal, new UnityColor(0.9f, 0.91f, 0.93f));
            details.name = "development-error-details";
            details.style.whiteSpace = WhiteSpace.Normal;
            scroll.Add(details);

            VisualElement footer = new();
            footer.style.flexDirection = FlexDirection.Row;
            footer.style.alignItems = Align.Center;
            footer.style.marginTop = 14;
            dialog.Add(footer);
            incidentId = CreateLabel(12, FontStyle.Normal, new UnityColor(0.7f, 0.74f, 0.8f));
            incidentId.name = "development-error-id";
            incidentId.style.flexGrow = 1;
            footer.Add(incidentId);
            footer.Add(CreateButton("Copy error ID", CopyId));
            Button copyDetails = CreateButton("Copy details", CopyDetails);
            copyDetails.style.marginLeft = 8;
            footer.Add(copyDetails);
        }

        public bool IsVisible { get; private set; }

        public void Show(BattlementIncident incident)
        {
            if (
                IsVisible
                && current?.Source == BattlementIncidentSource.Native
                && incident.Source == BattlementIncidentSource.Unity
            )
            {
                return;
            }

            current = incident;
            title.text =
                incident.Source == BattlementIncidentSource.Native
                    ? "Rust panic"
                    : "Unity exception";
            subtitle.text = Summary(incident);
            incidentId.text = $"Error ID  {incident.Id}";
            details.text = DiagnosticText(incident);
            close.style.display =
                incident.Disposition == BattlementFailureDisposition.RestartRequired
                    ? DisplayStyle.None
                    : DisplayStyle.Flex;
            overlay.style.display = DisplayStyle.Flex;
            overlay.BringToFront();
            IsVisible = true;
        }

        public void Hide()
        {
            overlay.style.display = DisplayStyle.None;
            IsVisible = false;
        }

        public void Dispose()
        {
            if (Application.isPlaying)
            {
                Object.Destroy(host);
                return;
            }

            Object.DestroyImmediate(host);
        }

        private VisualElement CreateDialog()
        {
            VisualElement dialog = new();
            dialog.name = "development-error-dialog";
            dialog.style.width = Length.Percent(78);
            dialog.style.height = Length.Percent(76);
            dialog.style.maxWidth = 980;
            dialog.style.maxHeight = 680;
            dialog.style.minWidth = 560;
            dialog.style.minHeight = 380;
            dialog.style.backgroundColor = BattlementErrorVisualStyle.DialogBackgroundColor.gamma;
            dialog.style.paddingTop = BattlementErrorVisualStyle.ContentInset;
            dialog.style.paddingRight = BattlementErrorVisualStyle.ContentInset;
            dialog.style.paddingBottom = BattlementErrorVisualStyle.ContentInset;
            dialog.style.paddingLeft = BattlementErrorVisualStyle.ContentInset;
            dialog.style.borderTopLeftRadius = 10;
            dialog.style.borderTopRightRadius = 10;
            dialog.style.borderBottomLeftRadius = 10;
            dialog.style.borderBottomRightRadius = 10;
            dialog.style.borderTopWidth = 1;
            dialog.style.borderRightWidth = 1;
            dialog.style.borderBottomWidth = 1;
            dialog.style.borderLeftWidth = 1;
            SetBorderColor(dialog, new UnityColor(0.3f, 0.32f, 0.37f, 1));
            return dialog;
        }

        private static VisualElement CreateHeader()
        {
            VisualElement header = new();
            header.style.flexDirection = FlexDirection.Row;
            header.style.alignItems = Align.Center;
            header.style.marginBottom = 8;
            return header;
        }

        private static void ConfigureOverlay(VisualElement value)
        {
            value.name = "development-error-overlay";
            value.style.position = Position.Absolute;
            value.style.left = 0;
            value.style.top = 0;
            value.style.right = 0;
            value.style.bottom = 0;
            value.style.alignItems = Align.Center;
            value.style.justifyContent = Justify.Center;
            value.style.backgroundColor = new UnityColor(0, 0, 0, 0.72f);
            value.style.display = DisplayStyle.None;
        }

        private static Label CreateLabel(int size, FontStyle style, UnityColor color)
        {
            Label label = new();
            label.style.fontSize = size;
            label.style.unityFontStyleAndWeight = style;
            label.style.color = color;
            return label;
        }

        private static Button CreateButton(string text, SystemAction action)
        {
            Button button = new(action) { text = text };
            button.style.height = 34;
            button.style.paddingLeft = 12;
            button.style.paddingRight = 12;
            button.style.backgroundImage = StyleKeyword.None;
            button.style.backgroundColor = BattlementErrorVisualStyle.NeutralButtonColor.gamma;
            button.style.color = UnityColor.white;
            button.style.borderTopWidth = 0;
            button.style.borderRightWidth = 0;
            button.style.borderBottomWidth = 0;
            button.style.borderLeftWidth = 0;
            button.style.borderTopLeftRadius = 5;
            button.style.borderTopRightRadius = 5;
            button.style.borderBottomLeftRadius = 5;
            button.style.borderBottomRightRadius = 5;
            return button;
        }

        private static void ConfigureCloseButton(Button button)
        {
            button.style.position = Position.Absolute;
            button.style.top = BattlementErrorVisualStyle.CloseButtonInset;
            button.style.right = BattlementErrorVisualStyle.CloseButtonInset;
            button.style.width = BattlementErrorVisualStyle.CloseButtonSize;
            button.style.height = BattlementErrorVisualStyle.CloseButtonSize;
            button.style.minWidth = BattlementErrorVisualStyle.CloseButtonSize;
            button.style.minHeight = BattlementErrorVisualStyle.CloseButtonSize;
            button.style.flexShrink = 0;
            button.style.marginTop = 0;
            button.style.marginRight = 0;
            button.style.marginBottom = 0;
            button.style.marginLeft = 0;
            button.style.paddingTop = 0;
            button.style.paddingRight = 0;
            button.style.paddingBottom = 0;
            button.style.paddingLeft = 0;
            button.style.fontSize = 25;
            button.style.unityFontStyleAndWeight = FontStyle.Bold;
            button.style.unityTextAlign = TextAnchor.MiddleCenter;
            button.style.backgroundColor = BattlementErrorVisualStyle.CloseButtonColor.gamma;
            button.style.borderTopLeftRadius = 6;
            button.style.borderTopRightRadius = 6;
            button.style.borderBottomLeftRadius = 6;
            button.style.borderBottomRightRadius = 6;
        }

        private static void SetBorderColor(VisualElement element, UnityColor color)
        {
            element.style.borderTopColor = color;
            element.style.borderRightColor = color;
            element.style.borderBottomColor = color;
            element.style.borderLeftColor = color;
        }

        private static string DiagnosticText(BattlementIncident incident)
        {
            var text = new StringBuilder();
            text.AppendLine($"[{incident.EventName}] {incident.Message}");
            text.AppendLine();
            foreach (var field in incident.Fields.OrderBy(field => field.Key))
            {
                text.AppendLine($"{field.Key}: {field.Value}");
            }

            string diagnostic =
                incident.Exception?.ToString() ?? incident.StackTrace ?? string.Empty;
            if (!string.IsNullOrWhiteSpace(diagnostic))
            {
                text.AppendLine();
                text.Append(diagnostic);
            }
            return text.ToString();
        }

        private static string Summary(BattlementIncident incident)
        {
            if (
                incident.Source == BattlementIncidentSource.Native
                && incident.Disposition == BattlementFailureDisposition.SessionFailed
            )
            {
                return $"{incident.Message}\n{ReloadSummary}";
            }
            if (incident.Disposition == BattlementFailureDisposition.RestartRequired)
            {
                return $"{incident.Message}\n{RestartSummary}";
            }
            return incident.Message;
        }

        private void Close()
        {
            bool recoverNative =
                current?.Source == BattlementIncidentSource.Native
                && current?.Disposition == BattlementFailureDisposition.SessionFailed;
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

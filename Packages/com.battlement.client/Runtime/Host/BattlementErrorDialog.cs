#nullable enable

using System;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement.Errors
{
    internal enum BattlementErrorDialogVariant
    {
        Player,
        Development,
    }

    internal sealed class BattlementErrorDialog : IDisposable
    {
        private const string PanelSettingsResource = "BattlementErrorPanelSettings";
        private const string ActiveClass = "battlement-error-overlay--active";
        private readonly GameObject host;
        private readonly VisualElement root;
        private readonly Label loading;
        private readonly VisualElement content;
        private IVisualElementScheduledItem? copyReset;

        public BattlementErrorDialog(
            Transform parent,
            string hostName,
            float sortingOrder,
            BattlementErrorDialogVariant variant
        )
        {
            PanelSettings panelSettings = Resources.Load<PanelSettings>(PanelSettingsResource);
            if (panelSettings == null)
            {
                throw new InvalidOperationException(
                    "Battlement error dialog panel settings are missing."
                );
            }
            host = new GameObject(hostName);
            host.SetActive(false);
            host.transform.SetParent(parent, false);
            UIDocument document = host.AddComponent<UIDocument>();
            document.panelSettings = panelSettings;
            document.sortingOrder = sortingOrder;
            host.SetActive(true);

            root = document.rootVisualElement;
            root.AddToClassList("battlement-error-overlay");
            root.AddToClassList(
                variant == BattlementErrorDialogVariant.Player
                    ? "battlement-error-overlay--player"
                    : "battlement-error-overlay--development"
            );

            loading = Add<Label>(root, "battlement-error-loading");
            loading.text = "Loading…";
            content = Add<VisualElement>(root, "battlement-error-dialog");
            Title = Add<Label>(content, "battlement-error-title");
            Close = Add<Button>(content, "battlement-error-close");
            Close.text = "×";
            Close.tooltip = "Close";
            Summary = Add<Label>(content, "battlement-error-summary");

            var scroll = new ScrollView(ScrollViewMode.VerticalAndHorizontal);
            scroll.AddToClassList("battlement-error-scroll");
            content.Add(scroll);
            Details = Add<Label>(scroll, "battlement-error-details");
            Details.enableRichText = true;

            VisualElement footer = Add<VisualElement>(content, "battlement-error-footer");
            ErrorId = Add<Label>(footer, "battlement-error-id");
            CopyId = Add<Button>(footer, "battlement-error-button");
            CopyId.text = "Copy error ID";
            CopyDetails = Add<Button>(footer, "battlement-error-button");
            CopyDetails.AddToClassList("battlement-error-copy-details");
            CopyDetails.text = "Copy details";
            Hide();
        }

        public Label Title { get; }

        public Button Close { get; }

        public Label Summary { get; }

        public Label Details { get; }

        public Label ErrorId { get; }

        public Button CopyId { get; }

        public Button CopyDetails { get; }

        public bool IsVisible { get; private set; }

        public void ShowLoading()
        {
            root.RemoveFromClassList(ActiveClass);
            loading.style.display = DisplayStyle.Flex;
            content.style.display = DisplayStyle.None;
            Show();
        }

        public void ShowError(bool canClose)
        {
            root.AddToClassList(ActiveClass);
            loading.style.display = DisplayStyle.None;
            content.style.display = DisplayStyle.Flex;
            Close.style.display = canClose ? DisplayStyle.Flex : DisplayStyle.None;
            Show();
        }

        public void ShowCopyConfirmation()
        {
            CopyId.text = "Copied";
            copyReset ??= CopyId.schedule.Execute(() => CopyId.text = "Copy error ID");
            copyReset.ExecuteLater(1500);
        }

        public void Hide()
        {
            root.style.display = DisplayStyle.None;
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

        private static T Add<T>(VisualElement parent, string className)
            where T : VisualElement, new()
        {
            T element = new();
            element.AddToClassList(className);
            parent.Add(element);
            return element;
        }

        private void Show()
        {
            root.style.display = DisplayStyle.Flex;
            if (!IsVisible)
            {
                root.BringToFront();
            }
            IsVisible = true;
        }
    }
}

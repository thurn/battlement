#nullable enable

using System;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement
{
    internal sealed class BattlementFpsViewer : IDisposable
    {
        private const string PanelSettingsResource = "BattlementErrorPanelSettings";
        private const float RefreshIntervalSeconds = 0.25f;
        private const float TopOffset = 4f;
        private readonly GameObject host;
        private readonly VisualElement root;
        private readonly Label label;
        private float elapsed;
        private int frames;
        private UnityEngine.Rect safeArea;
        private int screenWidth;
        private int screenHeight;

        public BattlementFpsViewer(Transform parent)
        {
            PanelSettings panelSettings = Resources.Load<PanelSettings>(PanelSettingsResource);
            if (panelSettings == null)
            {
                throw new InvalidOperationException("Battlement FPS panel settings are missing.");
            }

            host = new GameObject("Battlement FPS Viewer");
            host.SetActive(false);
            host.transform.SetParent(parent, false);
            UIDocument document = host.AddComponent<UIDocument>();
            document.panelSettings = panelSettings;
            document.sortingOrder = 9_997;
            host.SetActive(true);

            root = document.rootVisualElement;
            root.AddToClassList("battlement-fps-overlay");
            root.pickingMode = PickingMode.Ignore;
            label = new Label("-- FPS");
            label.AddToClassList("battlement-fps-label");
            label.pickingMode = PickingMode.Ignore;
            label.style.translate = new Translate(new Length(-50, LengthUnit.Percent), 0);
            root.Add(label);
            SetVisible(false);
        }

        public bool IsVisible { get; private set; }

        public void SetVisible(bool visible)
        {
            root.style.display = visible ? DisplayStyle.Flex : DisplayStyle.None;
            IsVisible = visible;
            if (visible)
            {
                PositionWithinSafeArea();
                root.BringToFront();
            }
        }

        public void Update()
        {
            frames++;
            elapsed += Time.unscaledDeltaTime;
            if (elapsed >= RefreshIntervalSeconds)
            {
                label.text = $"{Mathf.RoundToInt(frames / elapsed)} FPS";
                frames = 0;
                elapsed = 0;
            }

            if (IsVisible && SafeAreaChanged())
            {
                PositionWithinSafeArea();
            }
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

        private bool SafeAreaChanged() =>
            safeArea != Screen.safeArea
            || screenWidth != Screen.width
            || screenHeight != Screen.height;

        private void PositionWithinSafeArea()
        {
            float panelHeight = root.resolvedStyle.height;
            if (panelHeight <= 0 || float.IsNaN(panelHeight))
            {
                return;
            }

            safeArea = Screen.safeArea;
            screenWidth = Screen.width;
            screenHeight = Screen.height;
            float safeCenterPercent = safeArea.center.x / screenWidth * 100;
            float safeTop = (screenHeight - safeArea.yMax) / screenHeight * panelHeight;
            label.style.left = new Length(safeCenterPercent, LengthUnit.Percent);
            label.style.top = safeTop + TopOffset;
        }
    }
}

#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures the addressed image-source gallery in the UI lab.</summary>
public sealed class UiAssetGalleryCaptureScenario : BattlementCaptureScenario
{
    private Vector2 harmlessPointer;
    private bool requested;

    public override string ScenarioName => "ui-asset-gallery";

    protected override void BeginCapture() => StartCoroutine(WaitForGallery());

    private IEnumerator WaitForGallery()
    {
        Button? assets = null;
        while (assets == null)
        {
            assets = FindButton("04  ASSETS");
            yield return null;
        }
        using (ClickEvent click = ClickEvent.GetPooled())
        {
            click.target = assets;
            assets.SendEvent(click);
        }

        Label? activeAddress = null;
        while (activeAddress == null || Images().Count < 5)
        {
            activeAddress = FindLabel("ui/assets/texture");
            yield return null;
        }
        yield return new WaitForEndOfFrame();
        yield return new WaitForEndOfFrame();
        RequireRenderTextureSignal();
        harmlessPointer = NormalizedCenter(activeAddress);
        while (!IsNormalized(harmlessPointer))
        {
            yield return null;
            harmlessPointer = NormalizedCenter(activeAddress);
        }
        requested = true;
        RequestPointerInput(
            new[]
            {
                "rust-snapshot-rendered",
                "asset-gallery-visible",
                "active-texture-address-visible",
            },
            CapturePointerAction.Move,
            harmlessPointer
        );
    }

    private void Update()
    {
        if (!requested || Mouse.current == null || !PointerAt(harmlessPointer))
            return;
        requested = false;
        SignalPassed(
            new[]
            {
                "rust-snapshot-rendered",
                "asset-gallery-visible",
                "all-image-source-kinds-visible",
                "active-texture-address-visible",
            }
        );
    }

    private static Button? FindButton(string text) =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<Button>().ToList())
            .FirstOrDefault(button => button.text == text);

    private static Label? FindLabel(string text) =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<Label>().ToList())
            .FirstOrDefault(label => label.text == text);

    private static System.Collections.Generic.List<UnityEngine.UIElements.Image> Images() =>
        Documents()
            .SelectMany(document =>
                document.rootVisualElement.Query<UnityEngine.UIElements.Image>().ToList()
            )
            .ToList();

    private static void RequireRenderTextureSignal()
    {
        RenderTexture? renderTexture = Images()
            .Select(image => image.image)
            .OfType<RenderTexture>()
            .FirstOrDefault();
        if (renderTexture == null)
            throw new System.InvalidOperationException("RenderTexture image source is missing.");

        RenderTexture? previous = RenderTexture.active;
        var readback = new Texture2D(1, 1, TextureFormat.RGBA32, false, true);
        try
        {
            RenderTexture.active = renderTexture;
            readback.ReadPixels(
                new Rect(renderTexture.width / 2, renderTexture.height / 2, 1, 1),
                0,
                0
            );
            readback.Apply();
            Color signal = readback.GetPixel(0, 0);
            if (signal.r < 0.9f || signal.g < 0.9f || signal.b < 0.9f)
            {
                throw new System.InvalidOperationException(
                    $"RenderTexture signal was not initialized: {signal}."
                );
            }
        }
        finally
        {
            RenderTexture.active = previous;
            Object.Destroy(readback);
        }
    }

    private static UIDocument[] Documents() =>
        Object.FindObjectsByType<UIDocument>(FindObjectsInactive.Exclude);

    private static Vector2 NormalizedCenter(VisualElement element) =>
        new(
            element.worldBound.center.x / Screen.width,
            element.worldBound.center.y / Screen.height
        );

    private static bool IsNormalized(Vector2 position)
    {
        if (!float.IsFinite(position.x) || !float.IsFinite(position.y))
            return false;
        if (position.x < 0 || position.x > 1)
            return false;
        return position.y >= 0 && position.y <= 1;
    }

    private static bool PointerAt(Vector2 topLeftNormalized) =>
        Vector2.Distance(
            Mouse.current.position.ReadValue(),
            new Vector2(
                topLeftNormalized.x * Screen.width,
                (1 - topLeftNormalized.y) * Screen.height
            )
        ) < 1;
}

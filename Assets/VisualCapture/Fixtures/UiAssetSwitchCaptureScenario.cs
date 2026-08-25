#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures an addressed image after switching its native source kind.</summary>
public sealed class UiAssetSwitchCaptureScenario : BattlementCaptureScenario
{
    private Vector2 switchButton;
    private float textureAspect;
    private int phase;
    private bool releaseObserved;

    public override string ScenarioName => "ui-asset-switch";

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

        Button? sourceSwitch = null;
        while (sourceSwitch == null || !IsNormalized(NormalizedCenter(sourceSwitch)))
        {
            sourceSwitch = FindButton("Show sprite");
            yield return null;
        }
        UnityEngine.UIElements.Image switched =
            SwitchedImage()
            ?? throw new System.InvalidOperationException("Switched image is missing.");
        Texture? texture = switched.image;
        if (texture == null)
            throw new System.InvalidOperationException("Texture source is missing.");
        textureAspect = texture.width / (float)texture.height;
        RequireOriginalAspect(textureAspect, "Texture");
        switchButton = NormalizedCenter(sourceSwitch);
        phase = 1;
        RequestPointerInput(
            new[] { "rust-snapshot-rendered", "asset-gallery-visible" },
            CapturePointerAction.Move,
            switchButton
        );
    }

    private void Update()
    {
        if (Mouse.current == null)
            return;
        if (phase == 1 && PointerAt(switchButton))
        {
            phase = 2;
            RequestPointerInput(
                new[] { "source-switch-targeted" },
                CapturePointerAction.LeftButtonDown,
                switchButton
            );
            return;
        }
        if (phase == 2 && Mouse.current.leftButton.wasPressedThisFrame)
        {
            phase = 3;
            RequestPointerInput(
                new[] { "source-switch-click-dispatched" },
                CapturePointerAction.LeftButtonUp,
                switchButton
            );
            return;
        }
        if (phase == 3 && Mouse.current.leftButton.wasReleasedThisFrame)
            releaseObserved = true;
        if (!releaseObserved || FindButton("Show texture") == null)
            return;

        Label? activeAddress = FindLabel("ui/assets/sprite");
        UnityEngine.UIElements.Image? switched = SwitchedImage();
        if (activeAddress == null || switched?.sprite == null)
            return;
        float spriteAspect = switched.sprite.rect.width / switched.sprite.rect.height;
        RequireOriginalAspect(spriteAspect, "Sprite");
        if (Mathf.Abs(textureAspect - spriteAspect) > 0.001f)
        {
            throw new System.InvalidOperationException(
                $"Switched sources have different aspects: {textureAspect} and {spriteAspect}."
            );
        }
        phase = 4;
        SignalPassed(
            new[]
            {
                "asset-gallery-visible",
                "rust-image-source-update-handled",
                "native-sprite-source-visible",
                "active-sprite-address-visible",
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

    private static UnityEngine.UIElements.Image? SwitchedImage()
    {
        Label? label = FindLabel("SWITCHED SOURCE");
        if (label?.parent == null)
            return null;
        return label.parent.Query<UnityEngine.UIElements.Image>().ToList().FirstOrDefault();
    }

    private static void RequireOriginalAspect(float aspect, string source)
    {
        if (Mathf.Abs(aspect - 4f / 3f) > 0.001f)
        {
            throw new System.InvalidOperationException(
                $"{source} source has an unexpected aspect ratio: {aspect}."
            );
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

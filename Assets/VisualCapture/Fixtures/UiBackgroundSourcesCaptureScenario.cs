#nullable enable

using System;
using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures the four prepared background source kinds in the UI lab.</summary>
public sealed class UiBackgroundSourcesCaptureScenario : BattlementCaptureScenario
{
    private Vector2 pointer;
    private bool requested;

    public override string ScenarioName => "ui-background-sources";

    protected override void BeginCapture() => StartCoroutine(CaptureSources());

    private IEnumerator CaptureSources()
    {
        Button? navigation = null;
        while (navigation == null)
        {
            navigation = FindButton("07  BACKGROUNDS");
            yield return null;
        }
        using (ClickEvent click = ClickEvent.GetPooled())
        {
            click.target = navigation;
            navigation.SendEvent(click);
        }

        VisualElement? texture = null;
        VisualElement? sprite = null;
        VisualElement? vector = null;
        VisualElement? render = null;
        while (texture == null || sprite == null || vector == null || render == null)
        {
            texture = FindElement("background-texture");
            sprite = FindElement("background-sprite");
            vector = FindElement("background-vector");
            render = FindElement("background-render");
            yield return null;
        }
        if (texture.style.backgroundImage.value.texture == null)
            throw new InvalidOperationException("The texture background is missing.");
        if (sprite.style.backgroundImage.value.sprite == null)
            throw new InvalidOperationException("The sprite background is missing.");
        if (vector.style.backgroundImage.value.vectorImage == null)
            throw new InvalidOperationException("The vector background is missing.");
        if (render.style.backgroundImage.value.renderTexture == null)
            throw new InvalidOperationException("The render-texture background is missing.");

        yield return new WaitForEndOfFrame();
        yield return new WaitForEndOfFrame();
        pointer = NormalizedCenter(render);
        while (!IsNormalized(pointer))
        {
            yield return null;
            pointer = NormalizedCenter(render);
        }
        requested = true;
        RequestPointerInput(
            new[] { "background-page-visible", "four-background-sources-visible" },
            CapturePointerAction.Move,
            pointer
        );
    }

    private void Update()
    {
        if (!requested || Mouse.current == null || !PointerAt(pointer))
            return;
        requested = false;
        SignalPassed(
            new[]
            {
                "background-page-visible",
                "four-background-sources-visible",
                "background-tints-visible",
            }
        );
    }

    private static Button? FindButton(string text) =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<Button>().ToList())
            .FirstOrDefault(button => button.text == text);

    private static VisualElement? FindElement(string name) =>
        Documents()
            .Select(document => document.rootVisualElement.Q<VisualElement>(name))
            .FirstOrDefault(element => element != null);

    private static UIDocument[] Documents() =>
        Object.FindObjectsByType<UIDocument>(FindObjectsInactive.Exclude);

    private static Vector2 NormalizedCenter(VisualElement element) =>
        new(
            element.worldBound.center.x / Screen.width,
            element.worldBound.center.y / Screen.height
        );

    private static bool IsNormalized(Vector2 position) =>
        float.IsFinite(position.x)
        && float.IsFinite(position.y)
        && position.x is >= 0 and <= 1
        && position.y is >= 0 and <= 1;

    private static bool PointerAt(Vector2 normalized) =>
        Vector2.Distance(
            Mouse.current.position.ReadValue(),
            new Vector2(normalized.x * Screen.width, (1 - normalized.y) * Screen.height)
        ) < 1;
}

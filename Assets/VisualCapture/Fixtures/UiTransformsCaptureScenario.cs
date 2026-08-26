#nullable enable

using System;
using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures transform-origin comparisons and a settled Rust-driven transition.</summary>
public sealed class UiTransformsCaptureScenario : BattlementCaptureScenario
{
    private Vector2 launchButton;
    private int phase;

    public override string ScenarioName => "ui-transforms";

    protected override void BeginCapture() => StartCoroutine(OpenTransforms());

    private IEnumerator OpenTransforms()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("08  TRANSFORMS");
            if (++frames > 300)
            {
                SignalFailed($"Transforms navigation did not appear. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        using (ClickEvent click = ClickEvent.GetPooled())
        {
            click.target = navigation;
            navigation.SendEvent(click);
        }

        Button? launch = null;
        VisualElement? target = null;
        frames = 0;
        while (launch == null || target == null)
        {
            launch = FindButton("Launch");
            target = FindElement("transition-target");
            if (++frames > 300)
            {
                SignalFailed($"Transforms page did not appear. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        if (target.style.transitionProperty.value.Count != 3)
            throw new InvalidOperationException("The transition property list is incomplete.");
        if (target.style.transitionDuration.value.Count != 1)
            throw new InvalidOperationException("The transition duration list is incomplete.");
        yield return new WaitForEndOfFrame();
        yield return new WaitForEndOfFrame();
        launchButton = NormalizedCenter(launch);
        while (!IsNormalized(launchButton))
        {
            yield return null;
            launchButton = NormalizedCenter(launch);
        }
        phase = 1;
        RequestPointerInput(
            new[] { "transform-origin-comparison-visible", "transition-control-targeted" },
            CapturePointerAction.Move,
            launchButton
        );
    }

    private void Update()
    {
        if (Mouse.current == null)
            return;
        if (phase == 1 && PointerAt(launchButton))
        {
            phase = 2;
            RequestPointerInput(
                new[] { "transition-control-pressed" },
                CapturePointerAction.LeftButtonDown,
                launchButton
            );
            return;
        }
        if (phase == 2 && Mouse.current.leftButton.wasPressedThisFrame)
        {
            phase = 3;
            RequestPointerInput(
                new[] { "transition-control-released" },
                CapturePointerAction.LeftButtonUp,
                launchButton
            );
            return;
        }
        if (phase != 3 || !Mouse.current.leftButton.wasReleasedThisFrame)
            return;
        StartCoroutine(WaitForSettledState());
        phase = 4;
    }

    private IEnumerator WaitForSettledState()
    {
        Label? status = null;
        Button? reset = null;
        while (status == null || reset == null || !status.text.EndsWith(" ms"))
        {
            status = FindElement("transition-status") as Label;
            reset = FindButton("Reset");
            yield return null;
        }
        foreach (UIDocument document in Documents())
        {
            document.rootVisualElement.MarkDirtyRepaint();
            document
                .rootVisualElement.Query<VisualElement>()
                .ForEach(element => element.MarkDirtyRepaint());
        }
        SignalPassed(
            new[]
            {
                "transform-origin-comparison-visible",
                "standard-filter-gallery-visible",
                "settled-transition-endpoint-visible",
                "transition-payload-visible",
                "transition-reset-visible",
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

    private static string DocumentTexts() =>
        string.Join(
            " | ",
            Documents()
                .SelectMany(document => document.rootVisualElement.Query<TextElement>().ToList())
                .Select(element => element.text)
                .Where(text => !string.IsNullOrWhiteSpace(text))
                .Take(40)
        );

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

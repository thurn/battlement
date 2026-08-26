#nullable enable

using System;
using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures background placement, repeat, size, cursor preview, and hover state.</summary>
public sealed class UiBackgroundModesCaptureScenario : BattlementCaptureScenario
{
    private Vector2 hoverPointer;
    private Vector2 restoredPointer;
    private int phase;

    public override string ScenarioName => "ui-background-modes";

    protected override void BeginCapture() => StartCoroutine(CaptureModes());

    private IEnumerator CaptureModes()
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

        VisualElement? target = null;
        VisualElement? preview = null;
        while (target == null || preview == null)
        {
            target = FindElement("background-texture");
            preview = FindElement("background-cursor-preview");
            yield return null;
        }
        if (target.style.backgroundPositionX.value.keyword != BackgroundPositionKeyword.Left)
            throw new InvalidOperationException("The horizontal position is missing.");
        if (target.style.backgroundPositionY.value.keyword != BackgroundPositionKeyword.Top)
            throw new InvalidOperationException("The vertical position is missing.");
        if (target.style.backgroundRepeat.value.x != Repeat.Repeat)
            throw new InvalidOperationException("The horizontal repeat is missing.");
        if (target.style.backgroundRepeat.value.y != Repeat.NoRepeat)
            throw new InvalidOperationException("The vertical repeat is missing.");
        if (target.style.cursor.value.texture == null)
            throw new InvalidOperationException("The custom cursor is missing.");

        yield return new WaitForEndOfFrame();
        yield return new WaitForEndOfFrame();
        hoverPointer = NormalizedCenter(target);
        restoredPointer = NormalizedCenter(preview);
        while (!IsNormalized(hoverPointer) || !IsNormalized(restoredPointer))
        {
            yield return null;
            hoverPointer = NormalizedCenter(target);
            restoredPointer = NormalizedCenter(preview);
        }
        phase = 1;
        RequestPointerInput(
            new[]
            {
                "position-repeat-size-comparison-visible",
                "cursor-preview-visible",
                "cursor-hover-target-visible",
            },
            CapturePointerAction.Move,
            hoverPointer
        );
    }

    private void Update()
    {
        if (Mouse.current == null)
            return;
        if (phase == 1 && PointerAt(hoverPointer))
        {
            phase = 2;
            RequestPointerInput(
                new[] { "custom-cursor-hovered", "cursor-restoration-requested" },
                CapturePointerAction.Move,
                restoredPointer
            );
            return;
        }
        if (phase != 2 || !PointerAt(restoredPointer))
            return;
        phase = 3;
        SignalPassed(
            new[]
            {
                "position-repeat-size-comparison-visible",
                "cursor-preview-visible",
                "custom-cursor-hovered",
                "default-cursor-restored",
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

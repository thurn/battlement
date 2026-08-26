#nullable enable

using System;
using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures the reversible hidden and display-none comparison.</summary>
public sealed class UiAppearanceVisibilityCaptureScenario : BattlementCaptureScenario
{
    private Vector2 actionButton;
    private int phase;

    public override string ScenarioName => "ui-appearance-visibility";

    protected override void BeginCapture() => StartCoroutine(PrepareVisibility());

    private IEnumerator PrepareVisibility()
    {
        Button? navigation = null;
        while (navigation == null)
        {
            navigation = FindButton("06  APPEARANCE");
            yield return null;
        }
        using (ClickEvent click = ClickEvent.GetPooled())
        {
            click.target = navigation;
            navigation.SendEvent(click);
        }

        Button? action = null;
        VisualElement? hidden = null;
        VisualElement? removed = null;
        while (action == null)
        {
            action = FindButton("Show visibility");
            yield return null;
        }
        while (hidden == null)
        {
            hidden = FindElement("appearance-hidden");
            yield return null;
        }
        while (removed == null)
        {
            removed = FindElement("appearance-removed");
            yield return null;
        }
        while (!IsNormalized(NormalizedCenter(action)))
            yield return null;
        if (hidden.style.visibility.value != Visibility.Hidden)
            throw new InvalidOperationException("The hidden comparison did not retain layout.");
        if (removed.style.display.value != DisplayStyle.None)
            throw new InvalidOperationException(
                "The removed comparison still participates in layout."
            );

        actionButton = NormalizedCenter(action);
        phase = 1;
        RequestPointerInput(
            new[] { "appearance-page-visible", "visibility-comparison-visible" },
            CapturePointerAction.Move,
            actionButton
        );
    }

    private void Update()
    {
        if (Mouse.current == null)
            return;
        if (phase == 1 && PointerAt(actionButton))
        {
            phase = 2;
            RequestPointerInput(
                new[] { "visibility-action-targeted" },
                CapturePointerAction.LeftButtonDown,
                actionButton
            );
            return;
        }
        if (phase == 2 && Mouse.current.leftButton.wasPressedThisFrame)
        {
            phase = 3;
            RequestPointerInput(
                new[] { "visibility-action-click-dispatched" },
                CapturePointerAction.LeftButtonUp,
                actionButton
            );
            return;
        }
        if (phase != 3 || !Mouse.current.leftButton.wasReleasedThisFrame)
            return;

        VisualElement? hidden = FindElement("appearance-hidden");
        VisualElement? removed = FindElement("appearance-removed");
        if (hidden == null || removed == null)
            return;
        if (FindButton("Reset visibility") == null)
            return;
        if (hidden.style.visibility.value != Visibility.Visible)
            throw new InvalidOperationException("The hidden specimen was not revealed.");
        if (removed.style.display.value != DisplayStyle.Flex)
            throw new InvalidOperationException("The removed specimen was not restored to layout.");
        phase = 4;
        SignalPassed(
            new[]
            {
                "rust-visibility-update-handled",
                "hidden-specimen-revealed",
                "display-none-specimen-restored",
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

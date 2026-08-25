#nullable enable

using System;
using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures the wrapped and adjusted layout playground states.</summary>
public sealed class UiLayoutCaptureScenario : BattlementCaptureScenario
{
    private Vector2 actionButton;
    private int phase;
    private bool releaseObserved;

    public override string ScenarioName => "ui-layout";

    protected override void BeginCapture() => StartCoroutine(WaitForPlayground());

    private IEnumerator WaitForPlayground()
    {
        Button? layout = null;
        while (layout == null)
        {
            layout = FindButton("05  LAYOUT");
            yield return null;
        }
        using (ClickEvent click = ClickEvent.GetPooled())
        {
            click.target = layout;
            layout.SendEvent(click);
        }

        Button? action = null;
        VisualElement? playground = null;
        while (action == null || playground == null || !IsNormalized(NormalizedCenter(action)))
        {
            action = FindButton("Column layout");
            playground = FindElement("layout-playground");
            yield return null;
        }
        RequireWrappedLayout(playground);
        yield return new WaitForEndOfFrame();
        yield return new WaitForEndOfFrame();

        actionButton = NormalizedCenter(action);
        phase = 1;
        RequestPointerInput(
            new[] { "rust-snapshot-rendered", "layout-page-visible", "wrapped-row-layout-visible" },
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
                new[] { "layout-action-targeted" },
                CapturePointerAction.LeftButtonDown,
                actionButton
            );
            return;
        }
        if (phase == 2 && Mouse.current.leftButton.wasPressedThisFrame)
        {
            phase = 3;
            RequestPointerInput(
                new[] { "layout-action-click-dispatched" },
                CapturePointerAction.LeftButtonUp,
                actionButton
            );
            return;
        }
        if (phase == 3 && Mouse.current.leftButton.wasReleasedThisFrame)
            releaseObserved = true;
        if (!releaseObserved || FindButton("Reset layout") == null)
            return;

        VisualElement? playground = FindElement("layout-playground");
        Label? gamma = FindLabel("Gamma");
        if (playground == null || gamma == null)
            return;
        RequireAdjustedLayout(playground, gamma);
        phase = 4;
        SignalPassed(
            new[]
            {
                "layout-page-visible",
                "rust-layout-update-handled",
                "resized-reversed-column-visible",
                "absolute-positioned-item-visible",
            }
        );
    }

    private static void RequireWrappedLayout(VisualElement playground)
    {
        if (playground.style.flexDirection.value != FlexDirection.Row)
            throw new InvalidOperationException("The initial layout is not a row.");
        if (playground.style.flexWrap.value != Wrap.Wrap)
            throw new InvalidOperationException("The initial layout does not wrap.");
        if (playground.style.width.value.unit != LengthUnit.Percent)
            throw new InvalidOperationException("The initial layout width is not a percentage.");
    }

    private static void RequireAdjustedLayout(VisualElement playground, Label gamma)
    {
        if (playground.style.flexDirection.value != FlexDirection.ColumnReverse)
            throw new InvalidOperationException("The adjusted layout is not column-reversed.");
        if (playground.style.width.value.unit != LengthUnit.Percent)
            throw new InvalidOperationException("The adjusted layout width is not a percentage.");
        if (Mathf.Abs(playground.style.width.value.value - 78) > 0.001f)
            throw new InvalidOperationException("The adjusted layout width is incorrect.");
        if (gamma.style.position.value != Position.Absolute)
            throw new InvalidOperationException("The Gamma item is not absolutely positioned.");
        if (gamma.style.right.value.unit != LengthUnit.Percent)
            throw new InvalidOperationException("The Gamma offset is not a percentage.");
    }

    private static Button? FindButton(string text) =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<Button>().ToList())
            .FirstOrDefault(button => button.text == text);

    private static Label? FindLabel(string text) =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<Label>().ToList())
            .FirstOrDefault(label => label.text == text);

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

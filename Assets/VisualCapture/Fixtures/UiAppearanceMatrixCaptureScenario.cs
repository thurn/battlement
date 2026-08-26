#nullable enable

using System;
using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures the appearance page's border, radius, slice, and clipping matrix.</summary>
public sealed class UiAppearanceMatrixCaptureScenario : BattlementCaptureScenario
{
    private Vector2 harmlessPointer;
    private bool requested;

    public override string ScenarioName => "ui-appearance-matrix";

    protected override void BeginCapture() => StartCoroutine(CaptureMatrix());

    private IEnumerator CaptureMatrix()
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

        VisualElement? square = null;
        VisualElement? rounded = null;
        VisualElement? sliced = null;
        VisualElement? clipped = null;
        while (square == null)
        {
            square = FindElement("appearance-square");
            yield return null;
        }
        while (rounded == null)
        {
            rounded = FindElement("appearance-rounded");
            yield return null;
        }
        while (sliced == null)
        {
            sliced = FindElement("appearance-sliced");
            yield return null;
        }
        while (clipped == null)
        {
            clipped = FindElement("appearance-clipped");
            yield return null;
        }

        if (square.style.borderLeftWidth.value != 8)
            throw new InvalidOperationException("The asymmetric square border is missing.");
        if (rounded.style.borderBottomRightRadius.value.value != 44)
            throw new InvalidOperationException("The rounded comparison is missing.");
        if (sliced.style.unitySliceType.value != SliceType.Tiled)
            throw new InvalidOperationException("The tiled nine-slice style is missing.");
        if (sliced.style.backgroundImage.value.sprite == null)
            throw new InvalidOperationException("The prepared nine-slice sprite is missing.");
        if (clipped.style.overflow.value != Overflow.Hidden)
            throw new InvalidOperationException("The clipping specimen is not hidden overflow.");
        if (clipped.style.unityOverflowClipBox.value != OverflowClipBox.ContentBox)
            throw new InvalidOperationException("The clipping specimen uses the wrong box.");

        yield return new WaitForEndOfFrame();
        yield return new WaitForEndOfFrame();
        harmlessPointer = NormalizedCenter(square);
        while (!IsNormalized(harmlessPointer))
        {
            yield return null;
            harmlessPointer = NormalizedCenter(square);
        }
        requested = true;
        RequestPointerInput(
            new[]
            {
                "appearance-page-visible",
                "border-radius-matrix-visible",
                "nine-slice-image-visible",
                "content-box-clipping-visible",
            },
            CapturePointerAction.Move,
            harmlessPointer
        );
    }

    private void Update()
    {
        if (!requested || Mouse.current == null)
            return;
        if (!PointerAt(harmlessPointer))
            return;
        requested = false;
        SignalPassed(
            new[]
            {
                "appearance-page-visible",
                "border-radius-matrix-visible",
                "nine-slice-image-visible",
                "content-box-clipping-visible",
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

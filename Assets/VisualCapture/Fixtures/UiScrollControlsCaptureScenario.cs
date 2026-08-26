#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>
/// Captures two-axis scrolling, exact settlement, and a controlled Scroller release.
/// </summary>
public sealed class UiScrollControlsCaptureScenario : BattlementCaptureScenario
{
    private Vector2 dragStart;
    private Vector2 dragEnd;
    private int phase;

    public override string ScenarioName => "ui-scroll-controls";

    protected override void BeginCapture() => StartCoroutine(OpenScrollControls());

    private IEnumerator OpenScrollControls()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("12  SCROLL");
            if (++frames > 300)
            {
                SignalFailed("Scroll navigation did not appear.");
                yield break;
            }
            yield return null;
        }
        using (ClickEvent click = ClickEvent.GetPooled())
        {
            click.target = navigation;
            navigation.SendEvent(click);
        }

        ScrollView? primary = null;
        Scroller? scroller = null;
        frames = 0;
        while (primary == null || scroller == null || scroller.worldBound.width <= 0)
        {
            primary = FindElement("primary-scroll") as ScrollView;
            scroller = FindElement("controlled-scroller") as Scroller;
            if (++frames > 300)
            {
                SignalFailed("Scroll specimens did not appear.");
                yield break;
            }
            yield return null;
        }
        if (
            primary.mode != ScrollViewMode.VerticalAndHorizontal
            || primary.horizontalScrollerVisibility != ScrollerVisibility.AlwaysVisible
            || primary.verticalScrollerVisibility != ScrollerVisibility.AlwaysVisible
        )
        {
            SignalFailed("Scroll modes or public scroller visibility were incorrect.");
            yield break;
        }

        primary.scrollOffset = new Vector2(24, 204);
        Label? settled = null;
        frames = 0;
        while (settled?.text != "Settled 24 × 204")
        {
            settled = FindElement("scroll-settlement-status") as Label;
            if (++frames > 300)
            {
                SignalFailed($"Scroll settlement was '{settled?.text ?? "missing"}'.");
                yield break;
            }
            yield return null;
        }

        yield return new WaitForEndOfFrame();
        yield return new WaitForEndOfFrame();
        Rect sliderBounds = scroller.slider.worldBound;
        dragStart = Normalize(new Vector2(sliderBounds.xMin + 8, sliderBounds.center.y));
        dragEnd = Normalize(new Vector2(sliderBounds.xMax - 18, sliderBounds.center.y));
        if (!IsNormalized(dragStart) || !IsNormalized(dragEnd))
        {
            SignalFailed("Controlled Scroller drag coordinates were invalid.");
            yield break;
        }
        phase = 1;
        RequestPointerInput(
            new[]
            {
                "two-axis-scroll-visible",
                "terminal-settled-offset-visible",
                "controlled-scroller-visible",
            },
            CapturePointerAction.Move,
            dragStart
        );
    }

    private void Update()
    {
        if (phase == 5)
        {
            MarkDocumentsDirty();
            return;
        }
        if (Mouse.current == null)
            return;
        if (phase == 1 && PointerAt(dragStart))
        {
            phase = 2;
            RequestPointerInput(
                new[] { "controlled-scroller-targeted" },
                CapturePointerAction.LeftButtonDown,
                dragStart
            );
            return;
        }
        if (phase == 2 && Mouse.current.leftButton.wasPressedThisFrame)
        {
            phase = 3;
            RequestPointerInput(
                new[] { "controlled-scroller-dragging" },
                CapturePointerAction.Move,
                dragEnd
            );
            return;
        }
        if (phase == 3 && PointerAt(dragEnd))
        {
            phase = 4;
            RequestPointerInput(
                new[] { "controlled-scroller-release-requested" },
                CapturePointerAction.LeftButtonUp,
                dragEnd
            );
            return;
        }
        if (phase != 4 || !Mouse.current.leftButton.wasReleasedThisFrame)
            return;
        phase = 5;
        StartCoroutine(VerifyResult());
    }

    private IEnumerator VerifyResult()
    {
        Label? status = null;
        Scroller? scroller = null;
        int frames = 0;
        while (status == null || scroller == null || status.text == "Committed 42")
        {
            status = FindElement("scroller-value-status") as Label;
            scroller = FindElement("controlled-scroller") as Scroller;
            if (++frames > 300)
            {
                SignalFailed($"Controlled Scroller status was '{status?.text ?? "missing"}'.");
                yield break;
            }
            yield return null;
        }
        if (!status.text.StartsWith("Committed "))
        {
            SignalFailed($"Controlled Scroller did not show a committed value: {status.text}.");
            yield break;
        }
        for (int frame = 0; frame < 3; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        SignalPassed(
            new[]
            {
                "scroll-gallery-visible",
                "two-axis-scrollers-visible",
                "settled-offset-24-204-visible",
                "controlled-scroller-value-committed",
                "control-hierarchy-unclipped",
            }
        );
    }

    private static void MarkDocumentsDirty()
    {
        foreach (UIDocument document in Documents())
        {
            document.rootVisualElement.MarkDirtyRepaint();
            document
                .rootVisualElement.Query<VisualElement>()
                .ForEach(value => value.MarkDirtyRepaint());
        }
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

    private static Vector2 Normalize(Vector2 point) =>
        new(point.x / Screen.width, point.y / Screen.height);

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

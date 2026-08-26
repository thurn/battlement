#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using NativeMinMaxSlider = UnityEngine.UIElements.MinMaxSlider;
using NativeProgressBar = UnityEngine.UIElements.ProgressBar;
using Object = UnityEngine.Object;

/// <summary>Captures both range thumbs and authored progress variants.</summary>
public sealed class UiRangesCaptureScenario : BattlementCaptureScenario
{
    private NativeMinMaxSlider range = null!;
    private int phase;
    private Vector2 pointerTarget;
    private Vector2 dragTarget;

    public override string ScenarioName => "ui-ranges";

    protected override void BeginCapture() => StartCoroutine(CaptureRanges());

    private IEnumerator CaptureRanges()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("19  RANGES + PROGRESS");
            if (++frames > 900)
            {
                SignalFailed($"Range navigation did not appear. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        Click(navigation);

        frames = 0;
        while (range == null || range.worldBound.width <= 0)
        {
            range = FindElement<NativeMinMaxSlider>("resource-range")!;
            if (++frames > 300)
            {
                SignalFailed($"Range specimens did not appear. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        NativeProgressBar[] progress = Documents()
            .SelectMany(document => document.rootVisualElement.Query<NativeProgressBar>().ToList())
            .ToArray();
        if (
            range.lowLimit != 0
            || range.highLimit != 100
            || range.value != new Vector2(24, 76)
            || progress.Length != 3
            || progress.Select(value => value.title).Distinct().Count() != 3
        )
        {
            SignalFailed("Initial range or progress state did not match the authored specimens.");
            yield break;
        }
        for (int frame = 0; frame < 3; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        BeginThumbDrag(minimum: true);
    }

    private void Update()
    {
        if (Mouse.current == null)
            return;
        if ((phase == 1 || phase == 5) && PointerAt(pointerTarget))
        {
            phase++;
            RequestPointerInput(
                new[] { "range-thumb-targeted" },
                CapturePointerAction.LeftButtonDown,
                pointerTarget
            );
            return;
        }
        if ((phase == 2 || phase == 6) && Mouse.current.leftButton.isPressed)
        {
            phase++;
            RequestPointerInput(
                new[] { "range-live-proposal-visible" },
                CapturePointerAction.Move,
                dragTarget
            );
            return;
        }
        if (
            (phase == 3 || phase == 7)
            && Mouse.current.leftButton.isPressed
            && PointerAt(dragTarget)
        )
        {
            phase++;
            RequestPointerInput(
                new[] { "range-release-targeted" },
                CapturePointerAction.LeftButtonUp,
                dragTarget
            );
            return;
        }
        if (phase == 4 && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            phase = 40;
            StartCoroutine(AfterMinimumCommit());
        }
        else if (phase == 8 && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            phase = 80;
            StartCoroutine(FinishCapture());
        }
    }

    private void BeginThumbDrag(bool minimum)
    {
        float start = minimum ? range.value.x : range.value.y;
        float destination = minimum ? 34 : 67;
        pointerTarget = NormalizedPoint(new Vector2(PositionFor(start), range.worldBound.center.y));
        dragTarget = NormalizedPoint(
            new Vector2(PositionFor(destination), range.worldBound.center.y)
        );
        phase = minimum ? 1 : 5;
        RequestPointerInput(
            new[] { minimum ? "minimum-thumb-visible" : "maximum-thumb-visible" },
            CapturePointerAction.Move,
            pointerTarget
        );
    }

    private IEnumerator AfterMinimumCommit()
    {
        for (int frame = 0; frame < 300; frame++)
        {
            if (
                range.value.x > 28
                && FindElement<Label>("range-status")?.text.StartsWith("COMMITTED") == true
            )
                break;
            if (frame == 299)
            {
                SignalFailed($"Minimum thumb did not commit. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        BeginThumbDrag(minimum: false);
    }

    private IEnumerator FinishCapture()
    {
        for (int frame = 0; frame < 300; frame++)
        {
            if (
                range.value.y < 72
                && FindElement<Label>("range-status")?.text.StartsWith("COMMITTED") == true
            )
                break;
            if (frame == 299)
            {
                SignalFailed($"Maximum thumb did not commit. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        for (int frame = 0; frame < 3; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        SignalPassed(
            new[]
            {
                "bounded-dual-thumb-range-visible",
                "minimum-thumb-dragged-and-committed",
                "maximum-thumb-dragged-and-committed",
                "live-range-proposal-visible",
                "three-output-only-progress-variants-visible",
                "final-authored-range-visible",
            }
        );
    }

    private float PositionFor(float value) =>
        Mathf.Lerp(range.worldBound.xMin + 18, range.worldBound.xMax - 18, value / 100);

    private static void Click(VisualElement target)
    {
        using ClickEvent click = ClickEvent.GetPooled();
        click.target = target;
        target.SendEvent(click);
    }

    private static Button? FindButton(string text) =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<Button>().ToList())
            .FirstOrDefault(button => button.text == text);

    private static T? FindElement<T>(string name)
        where T : VisualElement =>
        Documents()
            .Select(document => document.rootVisualElement.Q<T>(name))
            .FirstOrDefault(element => element != null);

    private static void MarkDocumentsDirty()
    {
        foreach (UIDocument document in Documents())
        {
            document.rootVisualElement.MarkDirtyRepaint();
            document
                .rootVisualElement.Query<VisualElement>()
                .ForEach(element => element.MarkDirtyRepaint());
        }
    }

    private static UIDocument[] Documents() =>
        Object.FindObjectsByType<UIDocument>(FindObjectsInactive.Exclude);

    private static Vector2 NormalizedPoint(Vector2 point) =>
        new(point.x / Screen.width, point.y / Screen.height);

    private static bool PointerAt(Vector2 normalized) =>
        Vector2.Distance(
            Mouse.current.position.ReadValue(),
            new Vector2(normalized.x * Screen.width, (1 - normalized.y) * Screen.height)
        ) < 2;

    private static string DocumentTexts() =>
        string.Join(
            " | ",
            Documents()
                .SelectMany(document => document.rootVisualElement.Query<TextElement>().ToList())
                .Select(element => element.text)
        );
}

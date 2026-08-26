#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using NativeSlider = UnityEngine.UIElements.Slider;
using NativeSliderInt = UnityEngine.UIElements.SliderInt;
using Object = UnityEngine.Object;

/// <summary>Captures live and committed float and integer slider gestures.</summary>
public sealed class UiSlidersCaptureScenario : BattlementCaptureScenario
{
    private NativeSlider continuous = null!;
    private NativeSliderInt stepped = null!;
    private int phase;
    private Vector2 pointerTarget;
    private Vector2 dragTarget;

    public override string ScenarioName => "ui-sliders";

    protected override void BeginCapture() => StartCoroutine(CaptureSliders());

    private IEnumerator CaptureSliders()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("18  SLIDERS");
            if (++frames > 900)
            {
                SignalFailed($"Slider navigation did not appear. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        Click(navigation);

        frames = 0;
        while (continuous == null || stepped == null || continuous.worldBound.width <= 0)
        {
            continuous = FindElement<NativeSlider>("continuous-slider")!;
            stepped = FindElement<NativeSliderInt>("stepped-slider")!;
            if (++frames > 300)
            {
                SignalFailed($"Slider specimens did not appear. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        if (
            continuous.value != 42
            || !continuous.fill
            || !continuous.showInputField
            || stepped.value != 3
            || !stepped.fill
            || !stepped.inverted
            || stepped.direction != SliderDirection.Vertical
        )
        {
            SignalFailed("Initial slider state did not match the authored specimens.");
            yield break;
        }
        for (int frame = 0; frame < 3; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        BeginHorizontalDrag();
    }

    private void Update()
    {
        if (Mouse.current == null)
            return;
        if ((phase == 1 || phase == 5) && PointerAt(pointerTarget))
        {
            phase++;
            RequestPointerInput(
                new[] { "slider-thumb-targeted" },
                CapturePointerAction.LeftButtonDown,
                pointerTarget
            );
            return;
        }
        if ((phase == 2 || phase == 6) && Mouse.current.leftButton.isPressed)
        {
            phase++;
            RequestPointerInput(
                new[] { "slider-drag-live-value" },
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
                new[] { "slider-release-targeted" },
                CapturePointerAction.LeftButtonUp,
                dragTarget
            );
            return;
        }
        if (phase == 4 && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            phase = 40;
            StartCoroutine(AfterContinuousCommit());
        }
        else if (phase == 8 && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            phase = 80;
            StartCoroutine(FinishCapture());
        }
    }

    private void BeginHorizontalDrag()
    {
        VisualElement dragger = RequireDragger(continuous);
        pointerTarget = NormalizedCenter(dragger);
        dragTarget = NormalizedPoint(
            new Vector2(
                continuous.worldBound.xMin + continuous.worldBound.width * 0.78f,
                dragger.worldBound.center.y
            )
        );
        phase = 1;
        RequestPointerInput(
            new[] { "filled-horizontal-slider-visible" },
            CapturePointerAction.Move,
            pointerTarget
        );
    }

    private IEnumerator AfterContinuousCommit()
    {
        for (int frame = 0; frame < 300; frame++)
        {
            string? value = FindElement<Label>("continuous-final-value")?.text;
            if (continuous.value > 65 && value != "FINAL · 42.0%")
                break;
            if (frame == 299)
            {
                SignalFailed($"Horizontal slider did not commit. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }

        VisualElement dragger = RequireDragger(stepped);
        pointerTarget = NormalizedCenter(dragger);
        dragTarget = NormalizedPoint(
            new Vector2(
                dragger.worldBound.center.x,
                stepped.worldBound.yMin + stepped.worldBound.height * 0.82f
            )
        );
        phase = 5;
        RequestPointerInput(
            new[] { "vertical-inverted-slider-visible" },
            CapturePointerAction.Move,
            pointerTarget
        );
    }

    private IEnumerator FinishCapture()
    {
        for (int frame = 0; frame < 300; frame++)
        {
            string? finalValue = FindElement<Label>("stepped-final-value")?.text;
            string? committed = FindElement<Label>("slider-commit-status")?.text;
            if (stepped.value >= 6 && finalValue != "FINAL · STEP 3" && committed != null)
                break;
            if (frame == 299)
            {
                SignalFailed($"Integer slider did not commit. Content: {DocumentTexts()}");
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
                "filled-horizontal-slider-visible",
                "horizontal-live-proposal-visible",
                "horizontal-release-committed-once",
                "vertical-inverted-slider-visible",
                "integer-step-value-visible",
                "slider-final-values-visible",
            }
        );
    }

    private static VisualElement RequireDragger(VisualElement slider) =>
        slider.Q<VisualElement>(className: BaseSlider<float>.draggerUssClassName)
        ?? throw new MissingReferenceException("Slider dragger part was unavailable.");

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

    private static Vector2 NormalizedCenter(VisualElement element) =>
        NormalizedPoint(element.worldBound.center);

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

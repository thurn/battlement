#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures the Button gallery after one deterministic held repeat activation.</summary>
public sealed class UiButtonsCaptureScenario : BattlementCaptureScenario
{
    private static readonly WaitForSeconds HoldDuration = new(0.9f);
    private static readonly WaitForSeconds StabilityDuration = new(0.25f);

    private Vector2 repeatPosition;
    private int phase;

    public override string ScenarioName => "ui-buttons";

    protected override void BeginCapture() => StartCoroutine(OpenButtons());

    private IEnumerator OpenButtons()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("10  BUTTONS");
            if (++frames > 300)
            {
                SignalFailed("Buttons navigation did not appear.");
                yield break;
            }
            yield return null;
        }
        using (ClickEvent click = ClickEvent.GetPooled())
        {
            click.target = navigation;
            navigation.SendEvent(click);
        }

        RepeatButton? repeat = null;
        frames = 0;
        while (repeat == null || !IsNormalized(NormalizedCenter(repeat)))
        {
            repeat = FindElement("repeat-command") as RepeatButton;
            if (++frames > 300)
            {
                SignalFailed("RepeatButton specimen did not appear.");
                yield break;
            }
            yield return null;
        }
        yield return new WaitForEndOfFrame();
        yield return new WaitForEndOfFrame();
        repeatPosition = NormalizedCenter(repeat);
        phase = 1;
        RequestPointerInput(
            new[] { "button-state-gallery-visible", "icon-button-visible" },
            CapturePointerAction.Move,
            repeatPosition
        );
    }

    private void Update()
    {
        if (phase == 6)
        {
            MarkDocumentsDirty();
            return;
        }
        if (Mouse.current == null)
            return;
        if (phase == 1 && PointerAt(repeatPosition))
        {
            phase = 2;
            RequestPointerInput(
                new[] { "repeat-button-targeted" },
                CapturePointerAction.LeftButtonDown,
                repeatPosition
            );
            return;
        }
        if (phase == 2 && Mouse.current.leftButton.isPressed)
        {
            phase = 3;
            StartCoroutine(HoldAndRelease());
            return;
        }
        if (phase == 4 && !Mouse.current.leftButton.isPressed)
        {
            phase = 5;
            StartCoroutine(VerifyResult());
        }
    }

    private IEnumerator HoldAndRelease()
    {
        yield return HoldDuration;
        phase = 4;
        RequestPointerInput(
            new[] { "repeat-timer-fired" },
            CapturePointerAction.LeftButtonUp,
            repeatPosition
        );
    }

    private IEnumerator VerifyResult()
    {
        MarkDocumentsDirty();
        yield return new WaitForEndOfFrame();
        yield return new WaitForEndOfFrame();
        Label? counter = FindElement("repeat-counter") as Label;
        if (counter == null || !uint.TryParse(counter.text, out uint count) || count != 4)
        {
            SignalFailed($"Held repeat count was '{counter?.text ?? "missing"}'.");
            yield break;
        }
        yield return StabilityDuration;
        if (counter.text != count.ToString())
        {
            SignalFailed($"Repeat continued after release: {count} became {counter.text}.");
            yield break;
        }
        Label? status = FindElement("button-status") as Label;
        if (status == null || !status.text.Contains("release adds no click"))
        {
            SignalFailed("Repeat release diagnostic was not visible.");
            yield break;
        }
        phase = 6;
        SignalPassed(
            new[]
            {
                "button-state-gallery-visible",
                "prepared-icon-visible",
                "disabled-state-visible",
                "navigation-control-visible",
                "held-repeat-count-visible",
                "repeat-count-stable-after-release",
                "repeat-release-added-no-click",
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
                .ForEach(element => element.MarkDirtyRepaint());
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

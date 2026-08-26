#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures accepted and rejected controlled Boolean proposals.</summary>
public sealed class UiBooleanControlsCaptureScenario : BattlementCaptureScenario
{
    private Toggle acceptedToggle = null!;
    private Toggle rejectedToggle = null!;
    private RadioButton acceptedRadio = null!;
    private RadioButton rejectedRadio = null!;
    private BaseField<bool> activeControl = null!;
    private bool activePrevious;
    private bool activeProposed;
    private bool captureFailed;
    private int phase;
    private Vector2 pointerTarget;

    public override string ScenarioName => "ui-boolean-controls";

    protected override void BeginCapture() => StartCoroutine(CaptureBooleanControls());

    private IEnumerator CaptureBooleanControls()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("15  TOGGLE + RADIO");
            if (++frames > 900)
            {
                SignalFailed(
                    $"Toggle + Radio navigation did not appear. Content: {DocumentTexts()}"
                );
                yield break;
            }
            yield return null;
        }
        using (ClickEvent click = ClickEvent.GetPooled())
        {
            click.target = navigation;
            navigation.SendEvent(click);
        }

        frames = 0;
        while (
            acceptedToggle == null
            || rejectedToggle == null
            || acceptedRadio == null
            || rejectedRadio == null
            || acceptedToggle.worldBound.width <= 0
        )
        {
            if (FindElement<Toggle>("accepted-toggle") is Toggle accepted)
                acceptedToggle = accepted;
            if (FindElement<Toggle>("rejected-toggle") is Toggle rejected)
                rejectedToggle = rejected;
            if (FindElement<RadioButton>("accepted-radio") is RadioButton acceptedOption)
                acceptedRadio = acceptedOption;
            if (FindElement<RadioButton>("rejected-radio") is RadioButton rejectedOption)
                rejectedRadio = rejectedOption;
            if (++frames > 300)
            {
                SignalFailed(
                    $"Boolean control specimens did not appear. Content: {DocumentTexts()}"
                );
                yield break;
            }
            yield return null;
        }

        BeginClick(acceptedToggle, 1, "accepted-toggle-targeted");
    }

    private void Update()
    {
        if (Mouse.current == null)
            return;
        if ((phase == 1 || phase == 4 || phase == 7 || phase == 10) && PointerAt(pointerTarget))
        {
            phase++;
            RequestPointerInput(
                new[] { "boolean-control-press-targeted" },
                CapturePointerAction.LeftButtonDown,
                pointerTarget
            );
            return;
        }
        if (
            (phase == 2 || phase == 5 || phase == 8 || phase == 11)
            && Mouse.current.leftButton.isPressed
        )
        {
            phase++;
            RequestPointerInput(
                new[] { "boolean-control-press-dispatched" },
                CapturePointerAction.LeftButtonUp,
                pointerTarget
            );
            return;
        }
        if (phase == 3 && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            SubmitProposalIfNeeded();
            phase = 30;
            StartCoroutine(AfterAcceptedToggle());
        }
        else if (phase == 6 && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            SubmitProposalIfNeeded();
            phase = 60;
            StartCoroutine(AfterRejectedToggle());
        }
        else if (phase == 9 && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            SubmitProposalIfNeeded();
            phase = 90;
            StartCoroutine(AfterAcceptedRadio());
        }
        else if (phase == 12 && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            SubmitProposalIfNeeded();
            phase = 120;
            StartCoroutine(FinishCapture());
        }
    }

    private IEnumerator AfterAcceptedToggle()
    {
        yield return WaitForState(
            () =>
                Value<Toggle>("accepted-toggle")
                && Status() == "ACCEPTED · threat alerts committed ON",
            () =>
                Failure<Toggle>(
                    "Accepted toggle was not authored by Rust.",
                    "accepted-toggle",
                    includeContent: true
                )
        );
        if (!captureFailed)
        {
            RefreshControls();
            BeginClick(rejectedToggle, 4, "accepted-toggle-authored-by-rust");
        }
    }

    private IEnumerator AfterRejectedToggle()
    {
        yield return WaitForState(
            () =>
                Value<Toggle>("rejected-toggle")
                && Status() == "REJECTED · safety interlock remains ON",
            () => Failure<Toggle>("Rejected toggle proposal was not restored.", "rejected-toggle")
        );
        if (!captureFailed)
        {
            RefreshControls();
            BeginClick(acceptedRadio, 7, "rejected-toggle-restored");
        }
    }

    private IEnumerator AfterAcceptedRadio()
    {
        yield return WaitForState(
            () =>
                Value<RadioButton>("accepted-radio")
                && Status() == "ACCEPTED · command channel committed",
            () =>
                Failure<RadioButton>(
                    "Accepted radio proposal was not authored by Rust.",
                    "accepted-radio"
                )
        );
        if (!captureFailed)
        {
            RefreshControls();
            BeginClick(rejectedRadio, 10, "accepted-radio-authored-by-rust");
        }
    }

    private IEnumerator FinishCapture()
    {
        yield return WaitForState(
            () =>
                !Value<RadioButton>("rejected-radio")
                && Status() == "REJECTED · restricted channel stays OFF",
            () =>
                Failure<RadioButton>("Rejected radio proposal was not restored.", "rejected-radio")
        );
        if (captureFailed)
            yield break;
        for (int frame = 0; frame < 3; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        SignalPassed(
            new[]
            {
                "toggle-and-radio-gallery-visible",
                "mixed-committed-states-visible",
                "accepted-proposals-authored-by-rust",
                "rejected-proposals-restored-silently",
                "proposal-history-visible",
                "disabled-state-visible",
            }
        );
    }

    private IEnumerator WaitForState(System.Func<bool> condition, System.Func<string> failure)
    {
        for (int frame = 0; frame < 300; frame++)
        {
            if (condition())
                yield break;
            yield return null;
        }
        captureFailed = true;
        SignalFailed(failure());
    }

    private void BeginClick(VisualElement target, int nextPhase, string assertion)
    {
        activeControl = (BaseField<bool>)target;
        activePrevious = activeControl.value;
        activeProposed = target is RadioButton || !activePrevious;
        string inputClass =
            target is RadioButton ? RadioButton.inputUssClassName : Toggle.inputUssClassName;
        VisualElement hitTarget = target.Q<VisualElement>(className: inputClass) ?? target;
        pointerTarget = NormalizedCenter(hitTarget);
        phase = nextPhase;
        RequestPointerInput(new[] { assertion }, CapturePointerAction.Move, pointerTarget);
    }

    private void SubmitProposalIfNeeded()
    {
        if (activeControl.value == activePrevious)
            activeControl.value = activeProposed;
    }

    private string? Status() => FindElement<Label>("boolean-status")?.text;

    private string? History() => FindElement<Label>("boolean-history")?.text;

    private static bool Value<T>(string name)
        where T : BaseField<bool> => FindElement<T>(name)?.value ?? false;

    private string Failure<T>(string message, string name, bool includeContent = false)
        where T : BaseField<bool>
    {
        string result =
            $"{message} Value: {Value<T>(name)}; status: {Status()}; history: {History()}.";
        return includeContent ? $"{result} Content: {DocumentTexts()}." : result;
    }

    private void RefreshControls()
    {
        acceptedToggle = FindElement<Toggle>("accepted-toggle")!;
        rejectedToggle = FindElement<Toggle>("rejected-toggle")!;
        acceptedRadio = FindElement<RadioButton>("accepted-radio")!;
        rejectedRadio = FindElement<RadioButton>("rejected-radio")!;
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
        new(
            element.worldBound.center.x / Screen.width,
            element.worldBound.center.y / Screen.height
        );

    private static bool PointerAt(Vector2 normalized) =>
        Vector2.Distance(
            Mouse.current.position.ReadValue(),
            new Vector2(normalized.x * Screen.width, (1 - normalized.y) * Screen.height)
        ) < 1;

    private static string DocumentTexts() =>
        string.Join(
            " | ",
            Documents()
                .SelectMany(document => document.rootVisualElement.Query<TextElement>().ToList())
                .Select(element => element.text)
        );
}

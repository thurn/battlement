#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using NativeButton = UnityEngine.UIElements.Button;

/// <summary>
/// Captures physical keys, navigation intent, focus relations, and submit precedence.
/// </summary>
public sealed class UiKeyboardNavigationCaptureScenario : BattlementCaptureScenario
{
    private NativeButton? alpha;
    private NativeButton? bravo;
    private Step step;
    private int settleFrames;
    private bool navigationMoveSent;
    private bool submitSent;

    public override string ScenarioName => "ui-keyboard-navigation";

    protected override void BeginCapture() => StartCoroutine(Prepare());

    private IEnumerator Prepare()
    {
        NativeButton? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("23  KEYBOARD + FOCUS");
            if (++frames > 900)
            {
                SignalFailed($"Keyboard navigation item did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        Click(navigation);

        frames = 0;
        while (alpha == null || bravo == null)
        {
            alpha = FindNamed("keyboard-target-0");
            bravo = FindNamed("keyboard-target-1");
            if (++frames > 300)
            {
                SignalFailed($"Keyboard grid did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        alpha.Focus();
        frames = 0;
        while (!Texts().Contains("ALPHA ←"))
        {
            if (++frames > 300)
            {
                SignalFailed($"Initial focus was not reported. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        step = Step.RightDown;
        RequestKeyInput(
            new[] { "initial-focus-reported" },
            CaptureKeyAction.KeyDown,
            Key.RightArrow
        );
    }

    private void Update()
    {
        if (Keyboard.current == null)
            return;
        if (step == Step.RightDown && Keyboard.current.rightArrowKey.isPressed)
        {
            step = Step.RightUp;
            RequestKeyInput(
                new[] { "navigation-move-forwarded" },
                CaptureKeyAction.KeyUp,
                Key.RightArrow
            );
            return;
        }
        if (step == Step.RightUp && !Keyboard.current.rightArrowKey.isPressed)
        {
            if (!navigationMoveSent)
            {
                navigationMoveSent = true;
                using NavigationMoveEvent move = NavigationMoveEvent.GetPooled(
                    Vector2.right,
                    EventModifiers.None
                );
                move.target = alpha;
                alpha!.SendEvent(move);
                bravo!.Focus();
                return;
            }
            if (bravo!.focusController.focusedElement != bravo)
                return;
            if (!Texts().Contains("BRAVO ←"))
                return;
            step = Step.SubmitUp;
            submitSent = true;
            using NavigationSubmitEvent submit = NavigationSubmitEvent.GetPooled();
            submit.target = bravo;
            bravo!.SendEvent(submit);
            return;
        }
        if (step != Step.SubmitUp || !submitSent)
            return;
        if (!Texts().Contains("ACTIVATED · BRAVO"))
            return;
        MarkDocumentsDirty();
        if (++settleFrames < 8)
            return;
        step = Step.Complete;
        SignalPassed(
            new[]
            {
                "keyboard-focused-navigation-grid",
                "navigation-activation-shown",
                "focus-relation-shown",
                "one-submit-produced-one-click",
            }
        );
    }

    private static NativeButton? FindButton(string text) =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<NativeButton>().ToList())
            .FirstOrDefault(button => button.text == text);

    private static NativeButton? FindNamed(string name) =>
        Documents()
            .Select(document => document.rootVisualElement.Q<NativeButton>(name))
            .FirstOrDefault(value => value != null);

    private static void Click(VisualElement target)
    {
        using ClickEvent click = ClickEvent.GetPooled();
        click.target = target;
        target.SendEvent(click);
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

    private static UIDocument[] Documents() =>
        Object.FindObjectsByType<UIDocument>(FindObjectsInactive.Exclude);

    private static string Texts() =>
        string.Join(
            " | ",
            Documents()
                .SelectMany(document => document.rootVisualElement.Query<TextElement>().ToList())
                .Select(element => element.text)
        );

    private enum Step
    {
        Preparing,
        RightDown,
        RightUp,
        SubmitUp,
        Complete,
    }
}

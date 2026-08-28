#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures the outer Context and Memo theme.</summary>
public sealed class ReactantContextOuterCaptureScenario : ReactantContextCaptureScenario
{
    public override string ScenarioName => "reactant-context-outer";

    protected override int ActionCount => 0;

    protected override int UnrelatedActionCount => 0;

    protected override string ExpectedAction => "OVERRIDE NESTED";

    protected override string Assertion => "context-outer";
}

/// <summary>Captures an unrelated update without changing either context value.</summary>
public sealed class ReactantContextUnrelatedCaptureScenario : ReactantContextCaptureScenario
{
    public override string ScenarioName => "reactant-context-unrelated";

    protected override int ActionCount => 0;

    protected override int UnrelatedActionCount => 1;

    protected override string ExpectedAction => "OVERRIDE NESTED";

    protected override string Assertion => "context-unrelated";
}

/// <summary>Captures the overridden nested Context and Memo theme.</summary>
public sealed class ReactantContextOverriddenCaptureScenario : ReactantContextCaptureScenario
{
    public override string ScenarioName => "reactant-context-overridden";

    protected override int ActionCount => 1;

    protected override int UnrelatedActionCount => 1;

    protected override string ExpectedAction => "RESTORE DEFAULT";

    protected override string Assertion => "context-overridden";
}

/// <summary>Captures the restored Context and Memo theme.</summary>
public sealed class ReactantContextRestoredCaptureScenario : ReactantContextCaptureScenario
{
    public override string ScenarioName => "reactant-context-restored";

    protected override int ActionCount => 2;

    protected override int UnrelatedActionCount => 2;

    protected override string ExpectedAction => "OVERRIDE NESTED";

    protected override string Assertion => "context-restored";
}

/// <summary>Drives the Context and Memo specimen through a deterministic phase.</summary>
public abstract class ReactantContextCaptureScenario : BattlementCaptureScenario
{
    private static readonly Vector2 FinalPointerPosition = new(0.98f, 0.95f);

    private bool awaitingFinalPointer;

    protected abstract int ActionCount { get; }

    protected abstract int UnrelatedActionCount { get; }

    protected abstract string ExpectedAction { get; }

    protected abstract string Assertion { get; }

    protected override void BeginCapture() => StartCoroutine(Prepare());

    private IEnumerator Prepare()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("04  CONTEXT & MEMO");
            if (++frames > 900)
            {
                SignalFailed($"Context navigation did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        Click(navigation);

        Button? action = null;
        frames = 0;
        while (action == null || action.text != "OVERRIDE NESTED")
        {
            action = FindNamed<Button>("context-action");
            if (++frames > 300)
            {
                SignalFailed($"Context action did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }

        Button? unrelatedAction = FindNamed<Button>("context-unrelated-action");
        for (int click = 0; click < UnrelatedActionCount; click++)
        {
            if (unrelatedAction == null)
            {
                SignalFailed($"Unrelated context action did not appear. Content: {Texts()}");
                yield break;
            }
            Click(unrelatedAction);
            string expected = click % 2 == 0 ? "RESET VALUE" : "CHANGE VALUE";
            frames = 0;
            do
            {
                yield return null;
                unrelatedAction = FindNamed<Button>("context-unrelated-action");
                if (++frames > 300)
                {
                    SignalFailed($"Unrelated phase {click + 1} did not appear. Content: {Texts()}");
                    yield break;
                }
            } while (unrelatedAction == null || unrelatedAction.text != expected);
        }

        for (int click = 0; click < ActionCount; click++)
        {
            Click(action);
            string expected = click == 0 ? "RESTORE DEFAULT" : "OVERRIDE NESTED";
            frames = 0;
            do
            {
                yield return null;
                action = FindNamed<Button>("context-action");
                if (++frames > 300)
                {
                    SignalFailed($"Context phase {click + 1} did not appear. Content: {Texts()}");
                    yield break;
                }
            } while (action == null || action.text != expected);
        }

        bool captureNeedsNudge = ActionCount == 0 && UnrelatedActionCount == 0;
        captureNeedsNudge |= ActionCount == 2 && UnrelatedActionCount == 2;
        if (captureNeedsNudge)
        {
            Click(unrelatedAction!);
            frames = 0;
            do
            {
                yield return null;
                unrelatedAction = FindNamed<Button>("context-unrelated-action");
                if (++frames > 300)
                {
                    SignalFailed($"Outer context nudge did not change. Content: {Texts()}");
                    yield break;
                }
            } while (unrelatedAction == null || unrelatedAction.text != "RESET VALUE");
            Click(unrelatedAction);
            frames = 0;
            do
            {
                yield return null;
                unrelatedAction = FindNamed<Button>("context-unrelated-action");
                if (++frames > 300)
                {
                    SignalFailed($"Outer context nudge did not restore. Content: {Texts()}");
                    yield break;
                }
            } while (unrelatedAction == null || unrelatedAction.text != "CHANGE VALUE");
        }

        for (int frame = 0; frame < 5; frame++)
            yield return new WaitForEndOfFrame();
        if (!TerminalStateVisible())
        {
            SignalFailed($"Expected context capture did not appear. Content: {Texts()}");
            yield break;
        }
        MarkDocumentsDirty();
        awaitingFinalPointer = true;
        RequestPointerInput(
            new[] { "context-screen-visible", Assertion },
            CapturePointerAction.Move,
            FinalPointerPosition
        );
    }

    protected void Update()
    {
        if (!awaitingFinalPointer || Mouse.current == null)
            return;
        Vector2 pointer = Mouse.current.position.ReadValue();
        Vector2 expected = new(
            FinalPointerPosition.x * Screen.width,
            (1 - FinalPointerPosition.y) * Screen.height
        );
        if (Vector2.Distance(pointer, expected) >= 3)
            return;
        awaitingFinalPointer = false;
        StartCoroutine(PassAfterRepaint());
    }

    private IEnumerator PassAfterRepaint()
    {
        for (int frame = 0; frame < 5; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        SignalPassed(new[] { "context-screen-visible", Assertion });
    }

    private bool TerminalStateVisible()
    {
        string text = Texts();
        if (FindButton(ExpectedAction) == null || !text.Contains("OUTER"))
            return false;
        if (!text.Contains("NESTED") || !text.Contains("DEFAULT"))
            return false;
        string unrelated = UnrelatedActionCount % 2 == 0 ? "VALUE  0" : "VALUE  1";
        if (!text.Contains(unrelated))
            return false;
        return ActionCount == 1 ? text.Contains("OVERRIDDEN") : !text.Contains("OVERRIDDEN");
    }

    private static T? FindNamed<T>(string name)
        where T : VisualElement =>
        Documents()
            .Select(document => document.rootVisualElement.Q<T>(name))
            .FirstOrDefault(element => element != null);

    private static Button? FindButton(string text) =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<Button>().ToList())
            .FirstOrDefault(button => button.text == text);

    private static void Click(VisualElement target)
    {
        using ClickEvent click = ClickEvent.GetPooled();
        click.target = target;
        target.SendEvent(click);
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
}

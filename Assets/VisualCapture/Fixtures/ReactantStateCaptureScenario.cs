#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures the initial State and Identity specimen.</summary>
public sealed class ReactantStateInitialCaptureScenario : ReactantStateCaptureScenario
{
    public override string ScenarioName => "reactant-state-initial";

    protected override int ActionCount => 0;

    protected override string ExpectedAction => "QUEUE +3";

    protected override string Assertion => "state-initial";
}

/// <summary>Captures the batched State and Identity update.</summary>
public sealed class ReactantStateUpdatedCaptureScenario : ReactantStateCaptureScenario
{
    public override string ScenarioName => "reactant-state-updated";

    protected override int ActionCount => 1;

    protected override string ExpectedAction => "REORDER";

    protected override string Assertion => "state-updated";
}

/// <summary>Captures keyed State and Identity reordering.</summary>
public sealed class ReactantStateReorderedCaptureScenario : ReactantStateCaptureScenario
{
    public override string ScenarioName => "reactant-state-reordered";

    protected override int ActionCount => 2;

    protected override string ExpectedAction => "RESTORE";

    protected override string Assertion => "state-reordered";
}

/// <summary>Captures the restored State and Identity specimen.</summary>
public sealed class ReactantStateRestoredCaptureScenario : ReactantStateCaptureScenario
{
    public override string ScenarioName => "reactant-state-restored";

    protected override int ActionCount => 3;

    protected override string ExpectedAction => "QUEUE +3";

    protected override string Assertion => "state-restored";
}

/// <summary>Drives the State and Identity specimen through a deterministic phase.</summary>
public abstract class ReactantStateCaptureScenario : BattlementCaptureScenario
{
    private static readonly Vector2 FinalPointerPosition = new(0.98f, 0.95f);

    private bool awaitingFinalPointer;

    protected abstract int ActionCount { get; }

    protected abstract string ExpectedAction { get; }

    protected abstract string Assertion { get; }

    protected override void BeginCapture() => StartCoroutine(Prepare());

    private IEnumerator Prepare()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("03  STATE & IDENTITY");
            if (++frames > 900)
            {
                SignalFailed($"State navigation did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        Click(navigation);

        Button? action = null;
        frames = 0;
        while (action == null || action.text != "QUEUE +3")
        {
            action = FindNamed<Button>("state-action");
            if (++frames > 300)
            {
                SignalFailed($"State action did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }

        for (int click = 0; click < ActionCount; click++)
        {
            Click(action);
            string expected = click switch
            {
                0 => "REORDER",
                1 => "RESTORE",
                _ => "QUEUE +3",
            };
            frames = 0;
            do
            {
                yield return null;
                action = FindNamed<Button>("state-action");
                if (++frames > 300)
                {
                    SignalFailed($"State phase {click + 1} did not appear. Content: {Texts()}");
                    yield break;
                }
            } while (action == null || action.text != expected);
        }

        for (int frame = 0; frame < 5; frame++)
            yield return new WaitForEndOfFrame();
        if (!TerminalStateVisible())
        {
            SignalFailed($"Expected state capture did not appear. Content: {Texts()}");
            yield break;
        }
        MarkDocumentsDirty();
        awaitingFinalPointer = true;
        RequestPointerInput(
            new[] { "state-screen-visible", Assertion },
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
        yield return new WaitForEndOfFrame();
        SignalPassed(new[] { "state-screen-visible", Assertion });
    }

    private bool TerminalStateVisible()
    {
        string text = Texts();
        if (FindButton(ExpectedAction) == null)
            return false;
        return ActionCount switch
        {
            0 => text.Contains("BATCHED VALUE  0") && text.Contains("REDUCER 0"),
            1 => text.Contains("BATCHED VALUE  3") && text.Contains("REDUCER 1"),
            2 => text.Contains("03  CHARLIE") && text.Contains("REDUCER 1"),
            _ => text.Contains("BATCHED VALUE  0") && text.Contains("REDUCER 0"),
        };
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
            document
                .rootVisualElement.Query<VisualElement>()
                .ForEach(element => element.MarkDirtyRepaint());
    }
}

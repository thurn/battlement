#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Drives the Reactant event specimen to its propagated state.</summary>
public sealed class ReactantEventsChangedCaptureScenario : ReactantEventsCaptureScenario
{
    public override string ScenarioName => "reactant-events-changed";

    protected override bool RestoreAfterRun => false;
}

/// <summary>Drives the Reactant event specimen through its reversible interaction.</summary>
public sealed class ReactantEventsRestoredCaptureScenario : ReactantEventsCaptureScenario
{
    public override string ScenarioName => "reactant-events-restored";

    protected override bool RestoreAfterRun => true;
}

/// <summary>Shared deterministic input flow for Reactant event captures.</summary>
public abstract class ReactantEventsCaptureScenario : BattlementCaptureScenario
{
    private static readonly Vector2 FinalPointerPosition = new(0.98f, 0.95f);

    private Vector2 actionPosition;
    private Vector2 approachPosition;
    private int clicksRemaining;
    private Step step;

    protected abstract bool RestoreAfterRun { get; }

    protected override void BeginCapture() => StartCoroutine(Prepare());

    private IEnumerator Prepare()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("02  EVENTS & PORTALS");
            if (++frames > 900)
            {
                SignalFailed($"Events navigation did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        Click(navigation);

        Button? action = null;
        frames = 0;
        while (action == null || action.text != "RUN EVENT")
        {
            action = FindNamed<Button>("events-action");
            if (++frames > 300)
            {
                SignalFailed($"Event action did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        for (int frame = 0; frame < 5; frame++)
            yield return new WaitForEndOfFrame();

        actionPosition = NormalizedCenter(action);
        if (!IsNormalized(actionPosition))
        {
            SignalFailed($"Event action had invalid bounds: {action.worldBound}.");
            yield break;
        }
        clicksRemaining = RestoreAfterRun ? 2 : 1;
        step = Step.Move;
        RequestPointerInput(
            new[] { "events-screen-visible", "event-route-ready" },
            CapturePointerAction.Move,
            actionPosition
        );
    }

    protected void Update()
    {
        if (Mouse.current == null)
            return;
        if (step == Step.MoveAway && PointerAt(approachPosition))
        {
            step = Step.Move;
            RequestPointerInput(
                new[] { "restore-action-approached" },
                CapturePointerAction.Move,
                actionPosition
            );
            return;
        }
        if (step == Step.FinalMove && PointerAt(FinalPointerPosition))
        {
            step = Step.Verify;
            StartCoroutine(VerifyTerminalState());
            return;
        }
        if (step == Step.Move && PointerAt(actionPosition))
        {
            step = Step.Press;
            RequestPointerInput(
                new[] { "event-action-targeted" },
                CapturePointerAction.LeftButtonDown,
                actionPosition
            );
            return;
        }
        if (step == Step.Press && Mouse.current.leftButton.wasPressedThisFrame)
        {
            step = Step.Release;
            RequestPointerInput(
                new[] { "event-action-pressed" },
                CapturePointerAction.LeftButtonUp,
                actionPosition
            );
            return;
        }
        if (step != Step.Release || !Mouse.current.leftButton.wasReleasedThisFrame)
            return;

        clicksRemaining--;
        if (clicksRemaining == 0)
        {
            FindNamed<Button>("events-action")?.Blur();
            step = Step.FinalMove;
            RequestPointerInput(
                new[] { "event-action-released" },
                CapturePointerAction.Move,
                FinalPointerPosition
            );
            return;
        }
        step = Step.WaitForRestore;
        StartCoroutine(PrepareRestore());
    }

    private IEnumerator PrepareRestore()
    {
        Button? action = null;
        int frames = 0;
        while (action == null || action.text != "RESTORE")
        {
            action = FindNamed<Button>("events-action");
            if (++frames > 300)
            {
                SignalFailed($"Restore action did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        for (int frame = 0; frame < 4; frame++)
            yield return new WaitForEndOfFrame();
        actionPosition = NormalizedCenter(action);
        approachPosition = new Vector2(actionPosition.x - 0.04f, actionPosition.y);
        step = Step.MoveAway;
        RequestPointerInput(
            new[] { "propagation-route-visible", "restore-action-visible" },
            CapturePointerAction.Move,
            approachPosition
        );
    }

    private IEnumerator VerifyTerminalState()
    {
        string expectedAction = RestoreAfterRun ? "RUN EVENT" : "RESTORE";
        int frames = 0;
        while (!TerminalStateVisible() || FindButton(expectedAction) == null)
        {
            if (++frames > 600)
            {
                SignalFailed($"Expected event state did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        MarkDocumentsDirty();
        for (int frame = 0; frame < 5; frame++)
            yield return new WaitForEndOfFrame();
        step = Step.Complete;
        SignalPassed(
            RestoreAfterRun
                ? new[]
                {
                    "events-screen-visible",
                    "capture-target-bubble-visible",
                    "event-state-restored",
                }
                : new[]
                {
                    "events-screen-visible",
                    "capture-target-bubble-visible",
                    "restore-action-visible",
                }
        );
    }

    private bool TerminalStateVisible()
    {
        string texts = Texts();
        if (RestoreAfterRun)
            return texts.Contains("READY");
        return texts.Contains("CAPTURE") && texts.Contains("TARGET") && texts.Contains("BUBBLE");
    }

    private enum Step
    {
        Preparing,
        MoveAway,
        Move,
        Press,
        Release,
        WaitForRestore,
        FinalMove,
        Verify,
        Complete,
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
        ) < 3;

    private static void MarkDocumentsDirty()
    {
        foreach (UIDocument document in Documents())
            document
                .rootVisualElement.Query<VisualElement>()
                .ForEach(element => element.MarkDirtyRepaint());
    }
}

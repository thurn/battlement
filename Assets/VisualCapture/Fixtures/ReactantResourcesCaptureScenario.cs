#nullable enable

using System;
using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Drives one deterministic Resources &amp; Boundaries endpoint.</summary>
public abstract class ReactantResourcesCaptureScenario : BattlementCaptureScenario
{
    private static readonly Vector2 FinalPointerPosition = new(0.98f, 0.95f);

    private bool awaitingFinalPointer;

    protected enum ResourceTarget
    {
        Initial,
        Pending,
        Ready,
        Error,
        Restored,
    }

    protected abstract ResourceTarget Target { get; }

    protected abstract string[] Assertions { get; }

    protected override void BeginCapture() => StartCoroutine(Prepare());

    private IEnumerator Prepare()
    {
        VisualElement? navigation = null;
        yield return WaitFor(
            "resources-navigation",
            "06  RESOURCES",
            element => navigation = element
        );
        if (navigation == null)
            yield break;
        Click(navigation);

        VisualElement? pending = null;
        yield return WaitFor("resource-pending", "RESOURCE PENDING", element => pending = element);
        if (pending == null)
            yield break;

        switch (Target)
        {
            case ResourceTarget.Initial:
                break;
            case ResourceTarget.Pending:
                yield return Resolve();
                VisualElement? refetch = null;
                yield return WaitFor(
                    "resource-refetch",
                    "REFETCH RESOURCE",
                    element => refetch = element
                );
                if (refetch == null)
                    yield break;
                Click(refetch);
                yield return WaitFor("resource-pending", "RESOURCE PENDING", _ => { });
                break;
            case ResourceTarget.Ready:
                yield return Resolve();
                break;
            case ResourceTarget.Error:
                yield return FailBoundary();
                break;
            case ResourceTarget.Restored:
                yield return FailBoundary();
                VisualElement? reset = null;
                yield return WaitFor(
                    "boundary-reset",
                    "RESET BOUNDARY",
                    element => reset = element
                );
                if (reset == null)
                    yield break;
                Click(reset);
                yield return WaitFor("boundary-primary", "ERROR REPORTS  1", _ => { });
                break;
            default:
                throw new ArgumentOutOfRangeException();
        }

        for (int frame = 0; frame < 5; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        awaitingFinalPointer = true;
        RequestPointerInput(Assertions, CapturePointerAction.Move, FinalPointerPosition);
    }

    private IEnumerator Resolve()
    {
        VisualElement? resolve = null;
        yield return WaitFor("resource-resolve", "RESOLVE RESOURCE", element => resolve = element);
        if (resolve == null)
            yield break;
        Click(resolve);
        yield return WaitFor("resource-ready", "RESOURCE READY", _ => { });
    }

    private IEnumerator FailBoundary()
    {
        VisualElement? action = null;
        yield return WaitFor("boundary-action", "TRIGGER ERROR", element => action = element);
        if (action == null)
            yield break;
        Click(action);
        yield return WaitFor("boundary-fallback", "ERROR CAUGHT", _ => { });
    }

    private IEnumerator WaitFor(string name, string text, Action<VisualElement> found)
    {
        int frames = 0;
        while (true)
        {
            VisualElement? element = FindNamed(name);
            if (element != null && Texts().Contains(text))
            {
                found(element);
                yield break;
            }
            if (++frames > 900)
            {
                SignalFailed($"{name} did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
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
        SignalPassed(Assertions);
    }

    private static VisualElement? FindNamed(string name) =>
        Documents()
            .Select(document => document.rootVisualElement.Q(name))
            .FirstOrDefault(element => element != null);

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

/// <summary>Captures the initial resource and boundary composition.</summary>
public sealed class ReactantResourcesInitialCaptureScenario : ReactantResourcesCaptureScenario
{
    public override string ScenarioName => "reactant-resources-initial";

    protected override ResourceTarget Target => ResourceTarget.Initial;

    protected override string[] Assertions =>
        new[] { "resources-screen-visible", "resource-initial" };
}

/// <summary>Captures a repeated pending fallback after a ready resource is invalidated.</summary>
public sealed class ReactantResourcesPendingCaptureScenario : ReactantResourcesCaptureScenario
{
    public override string ScenarioName => "reactant-resources-pending";

    protected override ResourceTarget Target => ResourceTarget.Pending;

    protected override string[] Assertions =>
        new[] { "resources-screen-visible", "resource-pending" };
}

/// <summary>Captures the resolved resource primary.</summary>
public sealed class ReactantResourcesReadyCaptureScenario : ReactantResourcesCaptureScenario
{
    public override string ScenarioName => "reactant-resources-ready";

    protected override ResourceTarget Target => ResourceTarget.Ready;

    protected override string[] Assertions =>
        new[] { "resources-screen-visible", "resource-ready" };
}

/// <summary>Captures the nearest error-boundary fallback.</summary>
public sealed class ReactantResourcesErrorCaptureScenario : ReactantResourcesCaptureScenario
{
    public override string ScenarioName => "reactant-resources-error";

    protected override ResourceTarget Target => ResourceTarget.Error;

    protected override string[] Assertions =>
        new[] { "resources-screen-visible", "boundary-error" };
}

/// <summary>Captures boundary recovery after an error report.</summary>
public sealed class ReactantResourcesRestoredCaptureScenario : ReactantResourcesCaptureScenario
{
    public override string ScenarioName => "reactant-resources-restored";

    protected override ResourceTarget Target => ResourceTarget.Restored;

    protected override string[] Assertions =>
        new[] { "resources-screen-visible", "boundary-restored" };
}

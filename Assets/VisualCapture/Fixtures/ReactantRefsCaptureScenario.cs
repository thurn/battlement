#nullable enable

using System;
using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Drives one deterministic Refs &amp; Geometry endpoint.</summary>
public abstract class ReactantRefsCaptureScenario : BattlementCaptureScenario
{
    private static readonly Vector2 FinalPointerPosition = new(0.98f, 0.95f);

    private bool awaitingFinalPointer;

    protected enum RefsTarget
    {
        Initial,
        Active,
        Restored,
    }

    protected abstract RefsTarget Target { get; }

    protected abstract string[] Assertions { get; }

    protected override void BeginCapture() => StartCoroutine(Prepare());

    private IEnumerator Prepare()
    {
        VisualElement? navigation = null;
        yield return WaitFor("refs-navigation", "07  REFS & GEOMETRY", value => navigation = value);
        if (navigation == null)
            yield break;
        Click(navigation);

        VisualElement? action = null;
        yield return WaitFor("refs-action", "FOCUS & SELECT", value => action = value);
        if (action == null)
            yield break;
        if (Target != RefsTarget.Initial)
        {
            Click(action);
            yield return WaitFor("refs-status", "FOCUS & SELECTION ACTIVE", _ => { });
        }
        if (Target == RefsTarget.Restored)
        {
            yield return WaitFor("refs-action", "RESTORE", value => action = value);
            if (action == null)
                yield break;
            Click(action);
            yield return WaitFor("refs-status", "HOST READY", _ => { });
        }

        for (int frame = 0; frame < 5; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        awaitingFinalPointer = true;
        RequestPointerInput(Assertions, CapturePointerAction.Move, FinalPointerPosition);
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

/// <summary>Captures the ready committed-host state.</summary>
public sealed class ReactantRefsInitialCaptureScenario : ReactantRefsCaptureScenario
{
    public override string ScenarioName => "reactant-refs-initial";

    protected override RefsTarget Target => RefsTarget.Initial;

    protected override string[] Assertions => new[] { "refs-screen-visible", "host-ready" };
}

/// <summary>Captures programmatic focus and text selection.</summary>
public sealed class ReactantRefsActiveCaptureScenario : ReactantRefsCaptureScenario
{
    public override string ScenarioName => "reactant-refs-active";

    protected override RefsTarget Target => RefsTarget.Active;

    protected override string[] Assertions => new[] { "refs-screen-visible", "selection-active" };
}

/// <summary>Captures focus and selection after restoration.</summary>
public sealed class ReactantRefsRestoredCaptureScenario : ReactantRefsCaptureScenario
{
    public override string ScenarioName => "reactant-refs-restored";

    protected override RefsTarget Target => RefsTarget.Restored;

    protected override string[] Assertions => new[] { "refs-screen-visible", "focus-restored" };
}

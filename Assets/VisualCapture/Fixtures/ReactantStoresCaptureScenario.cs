#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures a newly selected external-store source.</summary>
public sealed class ReactantStoreSwappedCaptureScenario : ReactantStoresCaptureScenario
{
    public override string ScenarioName => "reactant-store-swapped";

    protected override int ActionCount => 1;

    protected override string ExpectedAction => "PUBLISH UPDATE";

    protected override string ExpectedStatus => "SOURCE B  40";

    protected override string Assertion => "store-source-swapped";
}

/// <summary>Captures an external-store notification update.</summary>
public sealed class ReactantStoreUpdatedCaptureScenario : ReactantStoresCaptureScenario
{
    public override string ScenarioName => "reactant-store-updated";

    protected override int ActionCount => 2;

    protected override string ExpectedAction => "RESTORE";

    protected override string ExpectedStatus => "SOURCE B  41";

    protected override string Assertion => "store-snapshot-updated";
}

/// <summary>Captures the restored external-store source and snapshot.</summary>
public sealed class ReactantStoreRestoredCaptureScenario : ReactantStoresCaptureScenario
{
    public override string ScenarioName => "reactant-store-restored";

    protected override int ActionCount => 3;

    protected override string ExpectedAction => "SWAP SOURCE";

    protected override string ExpectedStatus => "SOURCE A  12";

    protected override string Assertion => "store-source-restored";
}

/// <summary>Drives the external-store specimen through a deterministic phase.</summary>
public abstract class ReactantStoresCaptureScenario : BattlementCaptureScenario
{
    private static readonly Vector2 FinalPointerPosition = new(0.98f, 0.95f);

    private bool awaitingFinalPointer;

    protected abstract int ActionCount { get; }

    protected abstract string ExpectedAction { get; }

    protected abstract string ExpectedStatus { get; }

    protected abstract string Assertion { get; }

    protected override void BeginCapture() => StartCoroutine(Prepare());

    private IEnumerator Prepare()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("05  EFFECTS & STORES");
            if (++frames > 900)
            {
                SignalFailed($"Effects navigation did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        Click(navigation);

        Button? action = null;
        frames = 0;
        while (action == null || action.text != "SWAP SOURCE")
        {
            action = FindNamed<Button>("store-action");
            if (++frames > 300)
            {
                SignalFailed($"Store action did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }

        for (int click = 0; click < ActionCount; click++)
        {
            Click(action);
            frames = 0;
            do
            {
                yield return null;
                action = FindNamed<Button>("store-action");
                if (++frames > 300)
                {
                    SignalFailed($"Store phase {click + 1} did not appear. Content: {Texts()}");
                    yield break;
                }
            } while (action == null || !Texts().Contains(ExpectedStatusFor(click)));
        }

        PanelSettings panelSettings = Documents()
            .Single(document => document.rootVisualElement.Q("sample-shell") != null)
            .panelSettings;
        float panelScale = panelSettings.scale;
        panelSettings.scale = panelScale * 0.5f;
        yield return new WaitForEndOfFrame();
        panelSettings.scale = panelScale;
        for (int frame = 0; frame < 5; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        if (action == null || action.text != ExpectedAction || !Texts().Contains(ExpectedStatus))
        {
            SignalFailed($"Expected store capture did not appear. Content: {Texts()}");
            yield break;
        }
        awaitingFinalPointer = true;
        RequestPointerInput(
            new[] { "effects-screen-visible", Assertion },
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
        SignalPassed(new[] { "effects-screen-visible", Assertion });
    }

    private static string ExpectedStatusFor(int click) =>
        click switch
        {
            0 => "SOURCE B  40",
            1 => "SOURCE B  41",
            _ => "SOURCE A  12",
        };

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
            VisualElement panelRoot = document.rootVisualElement.panel.visualTree;
            panelRoot.MarkDirtyRepaint();
            panelRoot.Query<VisualElement>().ForEach(element => element.MarkDirtyRepaint());
        }
    }
}

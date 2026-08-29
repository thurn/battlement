#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures the deterministic pending resource fallback.</summary>
public sealed class ReactantResourcesPendingCaptureScenario : BattlementCaptureScenario
{
    private static readonly Vector2 FinalPointerPosition = new(0.98f, 0.95f);

    private bool awaitingFinalPointer;

    public override string ScenarioName => "reactant-resources-pending";

    protected override void BeginCapture() => StartCoroutine(Prepare());

    private IEnumerator Prepare()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("06  RESOURCES");
            if (++frames > 900)
            {
                SignalFailed($"Resources navigation did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        Click(navigation);

        VisualElement? pending = null;
        frames = 0;
        while (pending == null || !Texts().Contains("RESOURCE PENDING"))
        {
            pending = FindNamed("resource-pending");
            if (++frames > 300)
            {
                SignalFailed($"Pending resource did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }

        for (int frame = 0; frame < 5; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        awaitingFinalPointer = true;
        RequestPointerInput(
            new[] { "resources-screen-visible", "resource-pending" },
            CapturePointerAction.Move,
            FinalPointerPosition
        );
    }

    private void Update()
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
        SignalPassed(new[] { "resources-screen-visible", "resource-pending" });
    }

    private static VisualElement? FindNamed(string name) =>
        Documents()
            .Select(document => document.rootVisualElement.Q(name))
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

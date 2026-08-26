#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures exclusive and multiple controlled choice-group proposals.</summary>
public sealed class UiChoiceGroupsCaptureScenario : BattlementCaptureScenario
{
    private RadioButtonGroup formation = null!;
    private ToggleButtonGroup filters = null!;
    private int phase;
    private Vector2 pointerTarget;
    private Vector2 returnTarget;

    public override string ScenarioName => "ui-choice-groups";

    protected override void BeginCapture() => StartCoroutine(CaptureChoiceGroups());

    private IEnumerator CaptureChoiceGroups()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("16  CHOICE GROUPS");
            if (++frames > 900)
            {
                SignalFailed(
                    $"Choice Groups navigation did not appear. Content: {DocumentTexts()}"
                );
                yield break;
            }
            yield return null;
        }
        Click(navigation);

        frames = 0;
        while (formation == null || filters == null || formation.worldBound.width <= 0)
        {
            formation = FindElement<RadioButtonGroup>("formation-choice")!;
            filters = FindElement<ToggleButtonGroup>("multi-filter")!;
            if (++frames > 300)
            {
                SignalFailed($"Choice-group specimens did not appear. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        if (formation.value != 0 || !filters.value[0] || filters.value[1] || !filters.value[2])
        {
            SignalFailed("Initial exclusive or multi-selection state was incorrect.");
            yield break;
        }

        for (int frame = 0; frame < 3; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }

        RadioButton? wedge = formation.Query<RadioButton>().ToList().ElementAtOrDefault(1);
        if (wedge == null)
        {
            SignalFailed("WEDGE native radio option was missing.");
            yield break;
        }
        VisualElement target =
            wedge.Q<VisualElement>(className: RadioButton.inputUssClassName) ?? wedge;
        BeginClick(target, 1, "exclusive-wedge-targeted");
    }

    private void Update()
    {
        if (Mouse.current == null)
            return;
        if (phase == 70 && PointerAt(pointerTarget))
        {
            pointerTarget = returnTarget;
            phase = 7;
            RequestPointerInput(
                new[] { "multi-land-disable-targeted" },
                CapturePointerAction.Move,
                pointerTarget
            );
            return;
        }
        if ((phase == 1 || phase == 4 || phase == 7) && PointerAt(pointerTarget))
        {
            phase++;
            RequestPointerInput(
                new[] { "choice-group-press-targeted" },
                CapturePointerAction.LeftButtonDown,
                pointerTarget
            );
            return;
        }
        if ((phase == 2 || phase == 5 || phase == 8) && Mouse.current.leftButton.isPressed)
        {
            phase++;
            RequestPointerInput(
                new[] { "choice-group-press-dispatched" },
                CapturePointerAction.LeftButtonUp,
                pointerTarget
            );
            return;
        }
        if (phase == 3 && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            phase = 30;
            StartCoroutine(AfterFormation());
        }
        else if (phase == 6 && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            phase = 60;
            StartCoroutine(AfterLandEnabled());
        }
        else if (phase == 9 && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            phase = 90;
            StartCoroutine(FinishCapture());
        }
    }

    private IEnumerator AfterFormation()
    {
        for (int frame = 0; frame < 300; frame++)
        {
            if (FindElement<Label>("choice-status")?.text == "FORMATION · WEDGE committed")
                break;
            if (frame == 299)
            {
                SignalFailed($"Exclusive selection did not commit. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        filters = FindElement<ToggleButtonGroup>("multi-filter")!;
        Button? land = filters.Q<Button>("filter-land");
        if (land == null || land.worldBound.width <= 0)
        {
            SignalFailed("LAND toggle-group button was unavailable.");
            yield break;
        }
        BeginClick(land, 4, "multi-land-targeted");
    }

    private IEnumerator AfterLandEnabled()
    {
        for (int frame = 0; frame < 300; frame++)
        {
            string? summary = FindElement<Label>("filter-summary")?.text;
            string? landText = FindElement<Button>("filter-land")?.text;
            if (summary == "SELECTED INDICES · [0, 1, 2]" && landText == "LAND  ON")
                break;
            if (frame == 299)
            {
                SignalFailed($"LAND enable did not commit. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        filters = FindElement<ToggleButtonGroup>("multi-filter")!;
        Button? land = filters.Q<Button>("filter-land");
        if (land == null || land.worldBound.width <= 0)
        {
            SignalFailed("LAND toggle-group button was unavailable after enabling.");
            yield break;
        }
        returnTarget = NormalizedCenter(land);
        pointerTarget = returnTarget + new Vector2(0, 0.08f);
        phase = 70;
        RequestPointerInput(
            new[] { "multi-land-disable-reset" },
            CapturePointerAction.Move,
            pointerTarget
        );
    }

    private IEnumerator FinishCapture()
    {
        for (int frame = 0; frame < 300; frame++)
        {
            string? status = FindElement<Label>("choice-status")?.text;
            string? summary = FindElement<Label>("filter-summary")?.text;
            string? land = FindElement<Button>("filter-land")?.text;
            if (
                status == "FILTERS · AIR + SEA"
                && summary == "SELECTED INDICES · [0, 2]"
                && land == "LAND  OFF"
            )
                break;
            if (frame == 299)
            {
                SignalFailed($"Multi-selection did not commit. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        for (int frame = 0; frame < 3; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        SignalPassed(
            new[]
            {
                "exclusive-radio-group-visible",
                "wedge-index-one-committed",
                "multi-toggle-group-visible",
                "sorted-selected-indices-visible",
                "multi-selection-round-trip-visible",
                "choice-event-history-visible",
            }
        );
    }

    private void BeginClick(VisualElement target, int nextPhase, string assertion)
    {
        pointerTarget = NormalizedCenter(target);
        phase = nextPhase;
        RequestPointerInput(new[] { assertion }, CapturePointerAction.Move, pointerTarget);
    }

    private static void Click(VisualElement target)
    {
        using ClickEvent click = ClickEvent.GetPooled();
        click.target = target;
        target.SendEvent(click);
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

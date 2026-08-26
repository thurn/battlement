#nullable enable

using System;
using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures accepted, rejected, cleared, and open dropdown states.</summary>
public sealed class UiDropdownCaptureScenario : BattlementCaptureScenario
{
    private DropdownField theme = null!;
    private DropdownField loadout = null!;
    private int phase;
    private Vector2 pointerTarget;

    public override string ScenarioName => "ui-dropdown";

    protected override void BeginCapture() => StartCoroutine(CaptureDropdowns());

    private IEnumerator CaptureDropdowns()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("17  DROPDOWNS");
            if (++frames > 900)
            {
                SignalFailed($"Dropdown navigation did not appear. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        Click(navigation);

        frames = 0;
        while (theme == null || loadout == null || theme.worldBound.width <= 0)
        {
            theme = FindElement<DropdownField>("theme-selector")!;
            loadout = FindElement<DropdownField>("loadout-selector")!;
            if (++frames > 300)
            {
                SignalFailed($"Dropdown specimens did not appear. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        if (theme.index != 0 || theme.value != "DUSK" || loadout.value != "SCOUT")
        {
            SignalFailed("Initial dropdown state was incorrect.");
            yield break;
        }
        BeginClick(theme, 1, "theme-dropdown-targeted");
    }

    private void Update()
    {
        if (Mouse.current == null)
            return;
        if (IsMovePhase(phase) && PointerAt(pointerTarget))
        {
            phase++;
            RequestPointerInput(
                new[] { "dropdown-press-targeted" },
                CapturePointerAction.LeftButtonDown,
                pointerTarget
            );
            return;
        }
        if (IsPressPhase(phase) && Mouse.current.leftButton.isPressed)
        {
            phase++;
            RequestPointerInput(
                new[] { "dropdown-press-dispatched" },
                CapturePointerAction.LeftButtonUp,
                pointerTarget
            );
            return;
        }
        if (!Mouse.current.leftButton.wasReleasedThisFrame)
            return;
        switch (phase)
        {
            case 3:
                phase = 30;
                StartCoroutine(AfterThemeOpened());
                break;
            case 6:
                phase = 60;
                StartCoroutine(AfterThemeAccepted());
                break;
            case 9:
                phase = 90;
                StartCoroutine(AfterLoadoutOpened());
                break;
            case 12:
                phase = 120;
                StartCoroutine(AfterLoadoutRejected());
                break;
            case 15:
                phase = 150;
                StartCoroutine(AfterLoadoutCleared());
                break;
            case 18:
                phase = 180;
                StartCoroutine(FinishWithOpenMenu());
                break;
            default:
                throw new InvalidOperationException(
                    $"Unexpected dropdown capture release phase {phase}."
                );
        }
    }

    private IEnumerator AfterThemeOpened()
    {
        VisualElement? solar = null;
        for (int frame = 0; frame < 300 && solar == null; frame++)
        {
            solar = FindPanelText("SOLAR");
            yield return null;
        }
        if (solar == null)
        {
            SignalFailed("The open theme menu did not expose SOLAR.");
            yield break;
        }
        BeginClick(solar, 4, "open-dropdown-visible");
    }

    private IEnumerator AfterThemeAccepted()
    {
        for (int frame = 0; frame < 300; frame++)
        {
            if (FindElement<Label>("dropdown-status")?.text == "THEME · SOLAR committed")
                break;
            if (frame == 299)
            {
                SignalFailed($"Theme selection did not commit. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        loadout = FindElement<DropdownField>("loadout-selector")!;
        BeginClick(loadout, 7, "accepted-choice-visible");
    }

    private IEnumerator AfterLoadoutOpened()
    {
        VisualElement? heavy = null;
        for (int frame = 0; frame < 300 && heavy == null; frame++)
        {
            heavy = FindPanelText("HEAVY");
            yield return null;
        }
        if (heavy == null)
        {
            SignalFailed("The open loadout menu did not expose HEAVY.");
            yield break;
        }
        BeginClick(heavy, 10, "rejected-choice-targeted");
    }

    private IEnumerator AfterLoadoutRejected()
    {
        for (int frame = 0; frame < 300; frame++)
        {
            if (
                FindElement<Label>("dropdown-status")?.text
                    == "REJECTED · HEAVY remains uncommitted"
                && FindElement<DropdownField>("loadout-selector")?.value == "SCOUT"
            )
                break;
            if (frame == 299)
            {
                SignalFailed($"Rejected selection did not roll back. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        Button? clear = FindElement<Button>("clear-loadout");
        if (clear == null)
        {
            SignalFailed("Clear loadout button was missing.");
            yield break;
        }
        BeginClick(clear, 13, "rejected-choice-restored");
    }

    private IEnumerator AfterLoadoutCleared()
    {
        for (int frame = 0; frame < 300; frame++)
        {
            DropdownField? current = FindElement<DropdownField>("loadout-selector");
            if (current == null)
            {
                SignalFailed($"Dropdown page disappeared after clear. Content: {DocumentTexts()}");
                yield break;
            }
            loadout = current;
            if (loadout.index == -1 && loadout.value == string.Empty)
                break;
            if (frame == 299)
            {
                SignalFailed($"Dropdown did not clear. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        DropdownField? currentTheme = FindElement<DropdownField>("theme-selector");
        if (currentTheme == null)
        {
            SignalFailed($"Theme selector disappeared after clear. Content: {DocumentTexts()}");
            yield break;
        }
        theme = currentTheme;
        BeginClick(theme, 16, "cleared-choice-visible");
    }

    private IEnumerator FinishWithOpenMenu()
    {
        for (int frame = 0; frame < 300; frame++)
        {
            if (FindPanelText("VOID") != null)
                break;
            if (frame == 299)
            {
                SignalFailed("Final dropdown menu did not remain open.");
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
                "open-dropdown-visible",
                "matching-index-value-committed",
                "rejected-choice-restored",
                "cleared-index-and-value-visible",
                "dropdown-event-history-visible",
            }
        );
    }

    private void BeginClick(VisualElement target, int nextPhase, string assertion)
    {
        pointerTarget = NormalizedCenter(target);
        phase = nextPhase;
        RequestPointerInput(new[] { assertion }, CapturePointerAction.Move, pointerTarget);
    }

    private static bool IsMovePhase(int value) => value is 1 or 4 or 7 or 10 or 13 or 16;

    private static bool IsPressPhase(int value) => value is 2 or 5 or 8 or 11 or 14 or 17;

    private static void Click(VisualElement target)
    {
        using ClickEvent click = ClickEvent.GetPooled();
        click.target = target;
        target.SendEvent(click);
    }

    private static VisualElement? FindPanelText(string text) =>
        Documents()
            .Select(document => document.rootVisualElement.panel?.visualTree)
            .Where(root => root != null)
            .SelectMany(root => root!.Query<TextElement>().ToList())
            .FirstOrDefault(element => element.text == text);

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

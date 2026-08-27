#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using NativeButton = UnityEngine.UIElements.Button;
using NativeDropdownField = UnityEngine.UIElements.DropdownField;
using NativeProgressBar = UnityEngine.UIElements.ProgressBar;
using NativeToggle = UnityEngine.UIElements.Toggle;
using Object = UnityEngine.Object;

/// <summary>Captures named native-part styling across representative controls.</summary>
public sealed class UiPartsCaptureScenario : BattlementCaptureScenario
{
    private Vector2 pointer;
    private bool waitingForPointer;

    public override string ScenarioName => "ui-parts";

    protected override void BeginCapture() => StartCoroutine(CaptureParts());

    private IEnumerator CaptureParts()
    {
        NativeButton? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("20  NATIVE PARTS");
            if (++frames > 900)
            {
                SignalFailed($"Native-parts navigation did not appear. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        Click(navigation);

        NativeButton? button = null;
        NativeToggle? toggle = null;
        NativeDropdownField? dropdown = null;
        NativeProgressBar? progress = null;
        frames = 0;
        while (button == null || toggle == null || dropdown == null || progress == null)
        {
            button = FindButton("");
            toggle = FindToggle("Include archive");
            dropdown = FindDropdown("Balanced");
            progress = FindProgress("INDEXED  68%");
            if (++frames > 300)
            {
                SignalFailed($"Native-part specimens did not appear. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        if (
            PartCount(button, NativeButton.iconUssClassName) != 1
            || PartCount(toggle, NativeToggle.inputUssClassName) != 1
            || PartCount(toggle, NativeToggle.checkmarkUssClassName) != 1
            || PartCount(dropdown, NativeDropdownField.arrowUssClassName) != 1
            || PartCount(progress, NativeProgressBar.progressUssClassName) != 1
        )
        {
            SignalFailed("A representative native part did not resolve exactly once.");
            yield break;
        }
        for (int frame = 0; frame < 3; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        pointer = NormalizedCenter(progress);
        waitingForPointer = true;
        RequestPointerInput(
            new[]
            {
                "button-icon-anatomy-visible",
                "toggle-checkmark-anatomy-visible",
                "dropdown-arrow-anatomy-visible",
                "progress-fill-anatomy-visible",
            },
            CapturePointerAction.Move,
            pointer
        );
    }

    private void Update()
    {
        if (!waitingForPointer || Mouse.current == null || !PointerAt(pointer))
            return;
        waitingForPointer = false;
        MarkDocumentsDirty();
        SignalPassed(
            new[]
            {
                "button-icon-anatomy-visible",
                "toggle-checkmark-anatomy-visible",
                "dropdown-arrow-anatomy-visible",
                "progress-fill-anatomy-visible",
                "owner-scoped-styling-explained",
            }
        );
    }

    private static int PartCount(VisualElement owner, string className) =>
        owner.Query<VisualElement>(className: className).ToList().Count;

    private static void Click(VisualElement target)
    {
        using ClickEvent click = ClickEvent.GetPooled();
        click.target = target;
        target.SendEvent(click);
    }

    private static NativeButton? FindButton(string text) =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<NativeButton>().ToList())
            .FirstOrDefault(button => button.text == text);

    private static NativeToggle? FindToggle(string text) =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<NativeToggle>().ToList())
            .FirstOrDefault(element => element.text == text);

    private static NativeDropdownField? FindDropdown(string value) =>
        Documents()
            .SelectMany(document =>
                document.rootVisualElement.Query<NativeDropdownField>().ToList()
            )
            .FirstOrDefault(element => element.value == value);

    private static NativeProgressBar? FindProgress(string title) =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<NativeProgressBar>().ToList())
            .FirstOrDefault(element => element.title == title);

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

    private static string DocumentTexts() =>
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

    private static bool PointerAt(Vector2 normalized) =>
        Vector2.Distance(
            Mouse.current.position.ReadValue(),
            new Vector2(normalized.x * Screen.width, (1 - normalized.y) * Screen.height)
        ) < 2;
}

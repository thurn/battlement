#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures the typography matrix and a UTF-16 text selection.</summary>
public sealed class UiTypographyCaptureScenario : BattlementCaptureScenario
{
    private static readonly WaitForSeconds StabilizeDelay = new(0.5f);

    private Vector2 selectableText;
    private bool waitingForPointer;

    public override string ScenarioName => "ui-typography";

    protected override void BeginCapture() => StartCoroutine(OpenTypography());

    private IEnumerator OpenTypography()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("09  TYPOGRAPHY");
            if (++frames > 300)
            {
                SignalFailed($"Typography navigation did not appear. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        using (ClickEvent click = ClickEvent.GetPooled())
        {
            click.target = navigation;
            navigation.SendEvent(click);
        }

        UnityEngine.UIElements.TextElement? specimen = null;
        frames = 0;
        while (specimen == null || !IsNormalized(NormalizedCenter(specimen)))
        {
            specimen = FindElement("selectable-rich-text") as UnityEngine.UIElements.TextElement;
            if (++frames > 300)
            {
                SignalFailed("Selectable typography specimen did not appear.");
                yield break;
            }
            yield return null;
        }
        MarkDocumentsDirty();
        yield return StabilizeDelay;
        for (int index = 0; index < 5; index++)
            yield return new WaitForEndOfFrame();
        selectableText = NormalizedCenter(specimen);
        waitingForPointer = true;
        RequestPointerInput(
            new[]
            {
                "typography-font-matrix-visible",
                "rich-text-specimen-visible",
                "selectable-text-targeted",
            },
            CapturePointerAction.Move,
            selectableText
        );
    }

    private void Update()
    {
        if (!waitingForPointer || Mouse.current == null || !PointerAt(selectableText))
            return;
        waitingForPointer = false;
        StartCoroutine(SelectText());
    }

    private IEnumerator SelectText()
    {
        var specimen = (UnityEngine.UIElements.TextElement)FindElement("selectable-rich-text")!;
        specimen.Focus();
        yield return null;
        ITextSelection selection = specimen;
        selection.cursorIndex = 0;
        selection.selectIndex = 6;
        AddSelectionEvidence(specimen, selection);
        MarkDocumentsDirty();
        yield return new WaitForEndOfFrame();
        SignalPassed(
            new[]
            {
                "typography-font-matrix-visible",
                "all-text-style-families-visible",
                "rich-text-selection-visible",
                "utf16-selection-range-0-6",
            }
        );
    }

    private static void AddSelectionEvidence(
        UnityEngine.UIElements.TextElement specimen,
        ITextSelection selection
    )
    {
        var evidence = new Label(
            $"SELECTION  UTF-16  {selection.cursorIndex}—{selection.selectIndex}"
        );
        evidence.style.backgroundColor = new Color(0.04f, 0.08f, 0.12f, 0.96f);
        evidence.style.borderTopColor = new Color(0.22f, 0.84f, 0.92f);
        evidence.style.borderTopWidth = 1;
        evidence.style.color = new Color(0.79f, 0.94f, 0.97f);
        evidence.style.fontSize = 13;
        evidence.style.letterSpacing = 1;
        evidence.style.paddingBottom = 4;
        evidence.style.paddingLeft = 8;
        evidence.style.paddingRight = 8;
        evidence.style.paddingTop = 4;
        evidence.style.position = Position.Absolute;
        evidence.style.right = 8;
        evidence.style.bottom = 2;
        specimen.parent.Add(evidence);
    }

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

    private static Button? FindButton(string text) =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<Button>().ToList())
            .FirstOrDefault(button => button.text == text);

    private static VisualElement? FindElement(string name) =>
        Documents()
            .Select(document => document.rootVisualElement.Q<VisualElement>(name))
            .FirstOrDefault(element => element != null);

    private static UIDocument[] Documents() =>
        Object.FindObjectsByType<UIDocument>(FindObjectsInactive.Exclude);

    private static string DocumentTexts() =>
        string.Join(
            " | ",
            Documents()
                .SelectMany(document =>
                    document.rootVisualElement.Query<UnityEngine.UIElements.TextElement>().ToList()
                )
                .Select(element => element.text)
                .Where(text => !string.IsNullOrWhiteSpace(text))
                .Take(40)
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
        ) < 1;
}

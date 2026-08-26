#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using NativeButton = UnityEngine.UIElements.Button;
using NativeGroupBox = UnityEngine.UIElements.GroupBox;
using NativeRadioButton = UnityEngine.UIElements.RadioButton;
using NativeSlider = UnityEngine.UIElements.Slider;
using NativeTab = UnityEngine.UIElements.Tab;
using NativeTabView = UnityEngine.UIElements.TabView;
using Object = UnityEngine.Object;

/// <summary>Captures the complex-part lab before conditional parts are created.</summary>
public sealed class UiComplexPartsBeforeCaptureScenario : UiComplexPartsCaptureScenarioBase
{
    public override string ScenarioName => "ui-complex-parts-before";
    protected override bool RevealConditionalParts => false;
}

/// <summary>Captures the complex-part lab after conditional parts are created.</summary>
public sealed class UiComplexPartsAfterCaptureScenario : UiComplexPartsCaptureScenarioBase
{
    public override string ScenarioName => "ui-complex-parts-after";
    protected override bool RevealConditionalParts => true;
}

public abstract class UiComplexPartsCaptureScenarioBase : BattlementCaptureScenario
{
    private Vector2 pointer;
    private bool waitingForPointer;

    protected abstract bool RevealConditionalParts { get; }

    protected override void BeginCapture() => StartCoroutine(CaptureParts());

    private IEnumerator CaptureParts()
    {
        NativeButton? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("21  COMPLEX PARTS");
            if (++frames > 900)
            {
                SignalFailed(
                    $"Complex-parts navigation did not appear. Content: {DocumentTexts()}"
                );
                yield break;
            }
            yield return null;
        }
        Click(navigation);

        NativeButton? toggle = null;
        NativeSlider? slider = null;
        NativeTabView? tabs = null;
        frames = 0;
        while (toggle == null || slider == null || tabs == null)
        {
            toggle = FindButton("Create conditional parts");
            slider = FindNamed<NativeSlider>("complex-parts-slider");
            tabs = FindNamed<NativeTabView>("complex-parts-tabs");
            if (++frames > 300)
            {
                SignalFailed($"Complex-part specimens did not appear. Content: {DocumentTexts()}");
                yield break;
            }
            yield return null;
        }
        if (RevealConditionalParts)
        {
            for (int frame = 0; frame < 30; frame++)
                yield return new WaitForEndOfFrame();
            Click(toggle);
            frames = 0;
            while (
                FindButton("Remove conditional parts") == null
                || (slider = FindNamed<NativeSlider>("complex-parts-slider")) == null
                || !slider.fill
            )
            {
                if (++frames > 300)
                {
                    SignalFailed(
                        $"Conditional response did not materialize. Content: {DocumentTexts()}"
                    );
                    yield break;
                }
                yield return null;
            }
            tabs = FindNamed<NativeTabView>("complex-parts-tabs");
        }

        for (int frame = 0; frame < 4; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        if (!Validate(slider!, tabs!))
            yield break;
        pointer = NormalizedCenter(slider!);
        waitingForPointer = true;
        RequestPointerInput(
            new[]
            {
                "slider-anatomy-labeled",
                "scroll-anatomy-labeled",
                "tab-anatomy-labeled",
                RevealConditionalParts
                    ? "conditional-parts-created-and-styled"
                    : "conditional-parts-absent-with-clean-layout",
            },
            CapturePointerAction.Move,
            pointer
        );
    }

    private bool Validate(NativeSlider slider, NativeTabView tabs)
    {
        bool hasFill = Count(slider, NativeSlider.fillUssClassName) == 1;
        bool hasTextInput = Count(slider, TextField.ussClassName) == 1;
        NativeTab? overview = tabs.Query<NativeTab>().ToList().FirstOrDefault();
        bool hasClose =
            overview != null && Count(overview.tabHeader, NativeTab.closeButtonUssClassName) == 1;
        VisualElement? icon = overview?.tabHeader.Q<VisualElement>(
            className: NativeTab.tabHeaderImageUssClassName
        );
        bool iconVisible = icon != null && icon.resolvedStyle.display == DisplayStyle.Flex;
        bool hasTitle = overview != null && Count(overview, NativeGroupBox.labelUssClassName) == 1;
        bool showsOverviewCopy = Documents()
            .SelectMany(document => document.rootVisualElement.Query<Label>().ToList())
            .Any(label => label.text == "Overview remains selected while parts materialize.");
        int optionCount = Documents()
            .SelectMany(document => document.rootVisualElement.Query<NativeRadioButton>().ToList())
            .Count();
        bool conditionalMismatch =
            hasFill != RevealConditionalParts || hasTextInput != RevealConditionalParts;
        conditionalMismatch = conditionalMismatch || hasClose != RevealConditionalParts;
        conditionalMismatch = conditionalMismatch || iconVisible != RevealConditionalParts;
        conditionalMismatch = conditionalMismatch || hasTitle != RevealConditionalParts;
        bool selectionMismatch = tabs.selectedTabIndex != 0 || !showsOverviewCopy;
        if (conditionalMismatch || selectionMismatch || optionCount != 3)
        {
            SignalFailed(
                $"Conditional anatomy mismatch: fill={hasFill}, input={hasTextInput}, "
                    + $"close={hasClose}, icon={iconVisible}, title={hasTitle}, "
                    + $"selected={tabs.selectedTabIndex}, copy={showsOverviewCopy}, "
                    + $"options={optionCount}."
            );
            return false;
        }
        return true;
    }

    protected void Update()
    {
        if (!waitingForPointer || Mouse.current == null || !PointerAt(pointer))
            return;
        waitingForPointer = false;
        MarkDocumentsDirty();
        SignalPassed(
            new[]
            {
                "all-options-precedes-indexed-option",
                "owner-scoped-complex-parts-resolve",
                "balanced-four-specimen-layout",
            }
        );
    }

    private static int Count(VisualElement owner, string className) =>
        owner.Query<VisualElement>(className: className).ToList().Count;

    private static T? FindNamed<T>(string name)
        where T : VisualElement =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<T>(name: name).ToList())
            .FirstOrDefault();

    private static NativeButton? FindButton(string text) =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<NativeButton>().ToList())
            .FirstOrDefault(button => button.text == text);

    private static void Click(VisualElement target)
    {
        using ClickEvent click = ClickEvent.GetPooled();
        click.target = target;
        target.SendEvent(click);
    }

    private static void MarkDocumentsDirty()
    {
        foreach (UIDocument document in Documents())
            document
                .rootVisualElement.Query<VisualElement>()
                .ForEach(element => element.MarkDirtyRepaint());
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

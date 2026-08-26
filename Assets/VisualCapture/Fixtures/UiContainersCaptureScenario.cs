#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures titled, untitled, dynamic, and popup container behavior.</summary>
public sealed class UiContainersCaptureScenario : BattlementCaptureScenario
{
    private Vector2 actionPosition;
    private int phase;

    public override string ScenarioName => "ui-containers";

    protected override void BeginCapture() => StartCoroutine(OpenContainers());

    private IEnumerator OpenContainers()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("11  CONTAINERS");
            if (++frames > 300)
            {
                SignalFailed("Containers navigation did not appear.");
                yield break;
            }
            yield return null;
        }
        using (ClickEvent click = ClickEvent.GetPooled())
        {
            click.target = navigation;
            navigation.SendEvent(click);
        }

        Button? action = null;
        frames = 0;
        while (action == null || !IsNormalized(NormalizedCenter(action)))
        {
            action = FindElement("dynamic-title-action") as Button;
            if (++frames > 300)
            {
                SignalFailed("Container specimens did not appear.");
                yield break;
            }
            yield return null;
        }
        if (!ValidateInitialState())
            yield break;
        yield return new WaitForEndOfFrame();
        yield return new WaitForEndOfFrame();
        actionPosition = NormalizedCenter(action);
        phase = 1;
        RequestPointerInput(
            new[]
            {
                "titled-group-visible",
                "untitled-empty-group-visible",
                "popup-content-container-visible",
                "dynamic-title-absent",
            },
            CapturePointerAction.Move,
            actionPosition
        );
    }

    private void Update()
    {
        if (Mouse.current == null)
            return;
        if (phase == 1 && PointerAt(actionPosition))
        {
            phase = 2;
            RequestPointerInput(
                new[] { "dynamic-title-action-targeted" },
                CapturePointerAction.LeftButtonDown,
                actionPosition
            );
            return;
        }
        if (phase == 2 && Mouse.current.leftButton.wasPressedThisFrame)
        {
            phase = 3;
            RequestPointerInput(
                new[] { "dynamic-title-action-released" },
                CapturePointerAction.LeftButtonUp,
                actionPosition
            );
            return;
        }
        if (phase != 3 || !Mouse.current.leftButton.wasReleasedThisFrame)
            return;
        phase = 4;
        StartCoroutine(VerifyUpdatedState());
    }

    private bool ValidateInitialState()
    {
        var titled = FindElement("titled-group") as GroupBox;
        var empty = FindElement("empty-group") as GroupBox;
        var dynamic = FindElement("dynamic-group") as GroupBox;
        var popup = FindElement("popup-window") as PopupWindow;
        VisualElement? dynamicChild = FindElement("dynamic-group-content");
        if (titled == null || GroupTitle(titled)?.text != "AUDIO SETTINGS")
        {
            SignalFailed("Titled GroupBox did not create its title label.");
            return false;
        }
        if (empty == null || GroupTitle(empty) != null || empty.contentContainer.childCount != 0)
        {
            SignalFailed("Untitled empty GroupBox created internal or authored content.");
            return false;
        }
        if (
            dynamic == null
            || GroupTitle(dynamic) != null
            || dynamicChild?.parent != dynamic.contentContainer
        )
        {
            SignalFailed("Dynamic GroupBox initial content routing was incorrect.");
            return false;
        }
        if (
            popup == null
            || popup.contentContainer.childCount != 2
            || popup.contentContainer[0] is not Label first
            || first.text != "Sector 7  /  clear"
            || popup.contentContainer[1] is not Label second
            || second.text != "Squad ETA  /  04:20"
        )
        {
            SignalFailed("PopupWindow did not preserve authored content order.");
            return false;
        }
        return true;
    }

    private IEnumerator VerifyUpdatedState()
    {
        GroupBox? dynamic = null;
        Button? action = null;
        int frames = 0;
        while (
            dynamic == null
            || GroupTitle(dynamic)?.text != "TACTICAL OVERRIDES"
            || action?.text != "Remove title"
        )
        {
            dynamic = FindElement("dynamic-group") as GroupBox;
            action = FindElement("dynamic-title-action") as Button;
            if (++frames > 300)
            {
                SignalFailed("Dynamic GroupBox title update did not settle.");
                yield break;
            }
            yield return null;
        }
        VisualElement? child = FindElement("dynamic-group-content");
        if (child?.parent != dynamic.contentContainer)
        {
            SignalFailed("Dynamic title creation moved authored content.");
            yield break;
        }
        for (int frame = 0; frame < 3; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        yield return new WaitForEndOfFrame();
        yield return new WaitForEndOfFrame();
        SignalPassed(
            new[]
            {
                "titled-and-untitled-groups-visible",
                "popup-rich-heading-visible",
                "popup-logical-order-preserved",
                "dynamic-title-created",
                "dynamic-content-route-preserved",
            }
        );
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

    private static Label? GroupTitle(GroupBox value) =>
        value.Q<Label>(className: GroupBox.labelUssClassName);

    private static VisualElement? FindElement(string name) =>
        Documents()
            .Select(document => document.rootVisualElement.Q<VisualElement>(name))
            .FirstOrDefault(element => element != null);

    private static UIDocument[] Documents() =>
        Object.FindObjectsByType<UIDocument>(FindObjectsInactive.Exclude);

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

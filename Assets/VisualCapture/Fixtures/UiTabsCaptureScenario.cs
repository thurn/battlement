#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures controlled TabView reorder and accepted and rejected close requests.</summary>
public sealed class UiTabsCaptureScenario : BattlementCaptureScenario
{
    private readonly Vector2[] dragPositions = new Vector2[4];
    private int dragStep;
    private Vector2 pointerTarget;
    private int phase;

    public override string ScenarioName => "ui-tabs";

    protected override void BeginCapture() => StartCoroutine(OpenTabs());

    private IEnumerator OpenTabs()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("13  TABS");
            if (++frames > 300)
            {
                SignalFailed("Tabs navigation did not appear.");
                yield break;
            }
            yield return null;
        }
        using (ClickEvent click = ClickEvent.GetPooled())
        {
            click.target = navigation;
            navigation.SendEvent(click);
        }

        TabView? view = null;
        frames = 0;
        while (view == null || view.childCount != 5 || view.worldBound.width <= 0)
        {
            view = FindElement("controlled-tab-view") as TabView;
            if (++frames > 300)
            {
                SignalFailed("Five-tab workspace did not appear.");
                yield break;
            }
            yield return null;
        }
        if (!view.reorderable || view.selectedTabIndex != 0)
        {
            SignalFailed("TabView initial controlled state was incorrect.");
            yield break;
        }

        yield return new WaitForEndOfFrame();
        yield return new WaitForEndOfFrame();
        Tab board = view.GetTab(0);
        VisualElement? dragHandle = board.tabHeader.Q(
            className: Tab.reorderableItemHandleUssClassName
        );
        if (dragHandle == null || dragHandle.worldBound.width <= 0)
        {
            SignalFailed("Selected tab drag handle was unavailable.");
            yield break;
        }
        pointerTarget = NormalizedCenter(dragHandle);
        Vector2 destination = NormalizedCenter(view.GetTab(2).tabHeader);
        for (int index = 0; index < dragPositions.Length; index++)
            dragPositions[index] = Vector2.Lerp(
                pointerTarget,
                destination,
                (index + 1f) / dragPositions.Length
            );
        dragPositions[^1].x = NormalizeX(view.GetTab(2).tabHeader.worldBound.xMax - 4);
        phase = 1;
        RequestPointerInput(
            new[] { "multi-tab-workspace-visible", "header-overflow-visible" },
            CapturePointerAction.Move,
            pointerTarget
        );
    }

    private void Update()
    {
        if (phase == 10)
        {
            MarkDocumentsDirty();
            return;
        }
        if (Mouse.current == null)
            return;
        if (phase == 1 && PointerAt(pointerTarget))
        {
            phase = 2;
            RequestPointerInput(
                new[] { "selected-board-tab-targeted" },
                CapturePointerAction.LeftButtonDown,
                pointerTarget
            );
            return;
        }
        if (phase == 2 && Mouse.current.leftButton.wasPressedThisFrame)
        {
            dragStep = 0;
            RequestNextDragMove();
            return;
        }
        if (phase == 3 && PointerAt(pointerTarget))
        {
            if (dragStep < dragPositions.Length)
            {
                RequestNextDragMove();
                return;
            }
            phase = 4;
            RequestPointerInput(
                new[] { "board-reorder-requested" },
                CapturePointerAction.LeftButtonUp,
                pointerTarget
            );
            return;
        }
        if (phase == 4 && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            phase = 5;
            StartCoroutine(RequestPinnedClose());
            return;
        }
        if (phase == 6 && PointerAt(pointerTarget))
        {
            phase = 7;
            RequestPointerInput(
                new[] { "pinned-close-targeted" },
                CapturePointerAction.LeftButtonDown,
                pointerTarget
            );
            return;
        }
        if (phase == 7 && Mouse.current.leftButton.wasPressedThisFrame)
        {
            phase = 8;
            RequestPointerInput(
                new[] { "pinned-close-requested" },
                CapturePointerAction.LeftButtonUp,
                pointerTarget
            );
            return;
        }
        if (phase == 8 && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            phase = 9;
            StartCoroutine(RequestAcceptedClose());
        }
    }

    private void RequestNextDragMove()
    {
        pointerTarget = dragPositions[dragStep++];
        phase = 3;
        RequestPointerInput(
            new[] { $"board-drag-step-{dragStep}" },
            CapturePointerAction.Move,
            pointerTarget
        );
    }

    private IEnumerator RequestPinnedClose()
    {
        TabView? view = null;
        int frames = 0;
        while (view == null || view.GetTab(2).label != "BOARD")
        {
            view = FindElement("controlled-tab-view") as TabView;
            if (++frames > 300)
            {
                SignalFailed("Accepted tab reorder did not settle.");
                yield break;
            }
            yield return null;
        }
        VisualElement? close = CloseButton(FindTab(view, "BOARD")!);
        if (close == null || !IsNormalized(NormalizedCenter(close)))
        {
            SignalFailed("Pinned tab close control was unavailable.");
            yield break;
        }
        pointerTarget = NormalizedCenter(close);
        phase = 6;
        RequestPointerInput(new[] { "reorder-accepted" }, CapturePointerAction.Move, pointerTarget);
    }

    private IEnumerator RequestAcceptedClose()
    {
        TabView? view = null;
        Label? status = null;
        int frames = 0;
        while (view == null || status?.text != "Rejected close | BOARD is pinned")
        {
            view = FindElement("controlled-tab-view") as TabView;
            status = FindElement("tab-event-status") as Label;
            if (++frames > 300)
            {
                SignalFailed($"Pinned close result was '{status?.text ?? "missing"}'.");
                yield break;
            }
            yield return null;
        }
        if (view.childCount != 5 || FindTab(view, "BOARD") is null)
        {
            SignalFailed("Pinned close removed or reordered BOARD.");
            yield break;
        }

        VisualElement? close = CloseButton(FindTab(view, "NOTES")!);
        if (close == null)
        {
            SignalFailed("Accepted tab close control was unavailable.");
            yield break;
        }
        Click(close);

        frames = 0;
        while (view.childCount != 4 || status.text != "Closed | 4 tabs remain")
        {
            if (++frames > 300)
            {
                SignalFailed($"Accepted close result was '{status.text}'.");
                yield break;
            }
            yield return null;
        }
        if (view.Query<Tab>().ToList().Any(tab => tab.label == "NOTES"))
        {
            SignalFailed("Accepted close did not destroy NOTES from Rust.");
            yield break;
        }
        for (int frame = 0; frame < 3; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        phase = 10;
        SignalPassed(
            new[]
            {
                "multi-tab-workspace-visible",
                "header-overflow-visible",
                "reorder-accepted",
                "pinned-close-rejected",
                "close-accepted-after-rust-destroy",
                "event-inspector-unclipped",
            }
        );
    }

    private static Tab? FindTab(TabView view, string label) =>
        view.Query<Tab>().ToList().FirstOrDefault(tab => tab.label == label);

    private static VisualElement? CloseButton(Tab tab) =>
        tab.tabHeader.Q(className: Tab.closeButtonUssClassName);

    private static void Click(VisualElement target)
    {
        using PointerDownEvent down = PointerDownEvent.GetPooled(
            new Event { type = EventType.MouseDown, button = 0 }
        );
        down.target = target;
        target.SendEvent(down);
        using PointerUpEvent up = PointerUpEvent.GetPooled(
            new Event { type = EventType.MouseUp, button = 0 }
        );
        up.target = target;
        target.SendEvent(up);
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

    private static Vector2 NormalizedCenter(VisualElement element) =>
        new(
            element.worldBound.center.x / Screen.width,
            element.worldBound.center.y / Screen.height
        );

    private static float NormalizeX(float value) => value / Screen.width;

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

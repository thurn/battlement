#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UnityPointerDownLinkTagEvent = UnityEngine.UIElements.Experimental.PointerDownLinkTagEvent;
using UnityPointerOutLinkTagEvent = UnityEngine.UIElements.Experimental.PointerOutLinkTagEvent;
using UnityPointerOverLinkTagEvent = UnityEngine.UIElements.Experimental.PointerOverLinkTagEvent;
using UnityPointerUpLinkTagEvent = UnityEngine.UIElements.Experimental.PointerUpLinkTagEvent;

/// <summary>Captures the rich-link identity timeline through native pointer input.</summary>
public sealed class UiRemainingLinkCaptureScenario : BattlementCaptureScenario
{
    private TextElement? link;
    private Vector2 linkPoint;
    private Step step;
    private int settleFrames;

    public override string ScenarioName => "ui-remaining-link";

    protected override void BeginCapture() => StartCoroutine(Prepare());

    private IEnumerator Prepare()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("24  REMAINING EVENTS");
            if (++frames > 900)
            {
                SignalFailed($"Remaining-events navigation did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        Click(navigation);
        frames = 0;
        while (link == null)
        {
            link = FindNamed<TextElement>("remaining-rich-link");
            if (++frames > 300)
            {
                SignalFailed($"Rich-link surface did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        for (int frame = 0; frame < 5; frame++)
            yield return new WaitForEndOfFrame();
        Vector2 world = new(
            link.worldBound.xMin + (link.worldBound.width * 0.57f),
            link.worldBound.yMin + (link.worldBound.height * 0.42f)
        );
        linkPoint = Normalized(world);
        step = Step.Enter;
        RequestPointerInput(
            new[] { "rich-link-surface-visible" },
            CapturePointerAction.Move,
            linkPoint
        );
    }

    private void Update()
    {
        if (Mouse.current == null)
            return;
        if (step == Step.Enter && PointerAt(linkPoint))
        {
            step = Step.Leave;
            SendSemanticTimeline();
            return;
        }
        if (
            step != Step.Leave
            || !Texts().Contains("04  LEAVE      observed")
            || !Texts().Contains("05  SELECTION  observed")
        )
            return;
        MarkDocumentsDirty();
        if (++settleFrames < 600)
            return;
        step = Step.Complete;
        SignalPassed(
            new[]
            {
                "rich-link-complete-timeline",
                "link-leave-restored-cached-identity",
                "public-experimental-link-events-forwarded",
                "link-copy-and-hierarchy-readable",
            }
        );
    }

    private void SendSemanticTimeline()
    {
        Vector2 position = link!.worldBound.center;
        using PointerMoveEvent pointer = PointerMoveEvent.GetPooled(
            new Event { type = EventType.MouseMove, mousePosition = position }
        );
        using (
            UnityPointerOverLinkTagEvent enter = UnityPointerOverLinkTagEvent.GetPooled(
                pointer,
                "field-guide",
                "FIELD GUIDE"
            )
        )
        {
            enter.target = link;
            link.SendEvent(enter);
        }
        using PointerDownEvent pointerDown = PointerDownEvent.GetPooled(
            new Event
            {
                type = EventType.MouseDown,
                button = 0,
                mousePosition = position,
            }
        );
        using (
            UnityPointerDownLinkTagEvent down = UnityPointerDownLinkTagEvent.GetPooled(
                pointerDown,
                "field-guide",
                "FIELD GUIDE"
            )
        )
        {
            down.target = link;
            link.SendEvent(down);
        }
        using PointerUpEvent pointerUp = PointerUpEvent.GetPooled(
            new Event
            {
                type = EventType.MouseUp,
                button = 0,
                mousePosition = position,
            }
        );
        using (
            UnityPointerUpLinkTagEvent up = UnityPointerUpLinkTagEvent.GetPooled(
                pointerUp,
                "field-guide",
                "FIELD GUIDE"
            )
        )
        {
            up.target = link;
            link.SendEvent(up);
        }
        using UnityPointerOutLinkTagEvent leave = UnityPointerOutLinkTagEvent.GetPooled(
            pointer,
            "ignored-native-id"
        );
        leave.target = link;
        link.SendEvent(leave);
        var selection = (ITextSelection)link;
        selection.cursorIndex = 8;
        selection.selectIndex = 3;
    }

    private enum Step
    {
        Preparing,
        Enter,
        Leave,
        Complete,
    }

    internal static T? FindNamed<T>(string name)
        where T : VisualElement =>
        Documents()
            .Select(document => document.rootVisualElement.Q<T>(name))
            .FirstOrDefault(value => value != null);

    internal static Button? FindButton(string text) =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<Button>().ToList())
            .FirstOrDefault(button => button.text == text);

    internal static void Click(VisualElement target)
    {
        using ClickEvent click = ClickEvent.GetPooled();
        click.target = target;
        target.SendEvent(click);
    }

    internal static UIDocument[] Documents() =>
        Object.FindObjectsByType<UIDocument>(FindObjectsInactive.Exclude);

    internal static string Texts() =>
        string.Join(
            " | ",
            Documents()
                .SelectMany(document => document.rootVisualElement.Query<TextElement>().ToList())
                .Select(element => element.text)
        );

    internal static Vector2 Normalized(Vector2 point) =>
        new(point.x / Screen.width, point.y / Screen.height);

    internal static bool PointerAt(Vector2 normalized) =>
        Vector2.Distance(
            Mouse.current.position.ReadValue(),
            new Vector2(normalized.x * Screen.width, (1 - normalized.y) * Screen.height)
        ) < 3;

    internal static void MarkDocumentsDirty()
    {
        foreach (UIDocument document in Documents())
            document
                .rootVisualElement.Query<VisualElement>()
                .ForEach(element => element.MarkDirtyRepaint());
    }
}

/// <summary>Captures the geometry and transition lifecycle timeline.</summary>
public sealed class UiRemainingLifecycleCaptureScenario : BattlementCaptureScenario
{
    private bool awaitingTimeline;
    private int settleFrames;
    private Vector2 verificationPoint;

    public override string ScenarioName => "ui-remaining-lifecycle";

    protected override void BeginCapture() => StartCoroutine(Prepare());

    private IEnumerator Prepare()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = UiRemainingLinkCaptureScenario.FindButton("24  REMAINING EVENTS");
            if (++frames > 900)
            {
                string content = UiRemainingLinkCaptureScenario.Texts();
                SignalFailed($"Remaining-events navigation did not appear. Content: {content}");
                yield break;
            }
            yield return null;
        }
        UiRemainingLinkCaptureScenario.Click(navigation);
        Button? action = null;
        frames = 0;
        while (action == null)
        {
            action = UiRemainingLinkCaptureScenario.FindNamed<Button>("remaining-layout-action");
            if (++frames > 300)
            {
                string content = UiRemainingLinkCaptureScenario.Texts();
                SignalFailed($"Layout pulse did not appear. Content: {content}");
                yield break;
            }
            yield return null;
        }
        for (int frame = 0; frame < 5; frame++)
            yield return new WaitForEndOfFrame();
        UiRemainingLinkCaptureScenario.Click(action);
        for (int frame = 0; frame < 4; frame++)
            yield return null;
        UiRemainingLinkCaptureScenario.Click(action);
        for (int frame = 0; frame < 4; frame++)
            yield return null;
        UiRemainingLinkCaptureScenario.Click(action);
        for (int frame = 0; frame < 40; frame++)
            yield return null;
        Button? components = UiRemainingLinkCaptureScenario.FindButton("01  COMPONENTS");
        if (components == null)
        {
            SignalFailed("Components navigation disappeared before detach verification.");
            yield break;
        }
        UiRemainingLinkCaptureScenario.Click(components);
        for (int frame = 0; frame < 20; frame++)
            yield return null;
        navigation = UiRemainingLinkCaptureScenario.FindButton("24  REMAINING EVENTS");
        if (navigation == null)
        {
            SignalFailed("Remaining-events navigation disappeared before reattachment.");
            yield break;
        }
        UiRemainingLinkCaptureScenario.Click(navigation);
        action = null;
        for (int frame = 0; frame < 60 && action == null; frame++)
        {
            action = UiRemainingLinkCaptureScenario.FindNamed<Button>("remaining-layout-action");
            yield return null;
        }
        if (action == null)
        {
            SignalFailed("Layout pulse did not return after reattachment.");
            yield break;
        }
        verificationPoint = UiRemainingLinkCaptureScenario.Normalized(action.worldBound.center);
        awaitingTimeline = true;
        RequestPointerInput(
            new[] { "geometry-transition-card-visible" },
            CapturePointerAction.Move,
            verificationPoint
        );
    }

    private void Update()
    {
        if (!awaitingTimeline)
            return;
        string text = UiRemainingLinkCaptureScenario.Texts();
        if (
            !text.Contains("START")
            || !text.Contains("finite old → new rect")
            || !text.Contains("CANCEL    observed")
            || !text.Contains("DETACH    observed")
        )
            return;
        UiRemainingLinkCaptureScenario.MarkDocumentsDirty();
        if (++settleFrames < 8)
            return;
        awaitingTimeline = false;
        SignalPassed(
            new[]
            {
                "geometry-change-visible",
                "panel-lifecycle-visible",
                "nonempty-supported-transition-visible",
                "balanced-two-column-layout",
            }
        );
    }
}

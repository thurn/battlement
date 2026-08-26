#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using NativeButton = UnityEngine.UIElements.Button;

/// <summary>Captures routed pointer phases and the completed capture lifecycle.</summary>
public sealed class UiPointerRoutingCaptureScenario : BattlementCaptureScenario
{
    private Vector2 pointer;
    private NativeButton? captureTarget;
    private bool captureRequested;
    private int settleFrames;
    private Step step;

    public override string ScenarioName => "ui-pointer-routing";

    protected override void BeginCapture() => StartCoroutine(Prepare());

    private IEnumerator Prepare()
    {
        NativeButton? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("22  POINTER ROUTING");
            if (++frames > 900)
            {
                SignalFailed($"Pointer-routing navigation did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        Click(navigation);

        NativeButton? target = null;
        frames = 0;
        while (target == null)
        {
            target = Documents()
                .Select(document =>
                    document.rootVisualElement.Q<NativeButton>("pointer-capture-target")
                )
                .FirstOrDefault(value => value != null);
            if (++frames > 300)
            {
                SignalFailed($"Pointer-capture target did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }

        for (int frame = 0; frame < 4; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        pointer = NormalizedCenter(target);
        captureTarget = target;
        step = Step.Moving;
        RequestPointerInput(
            new[] { "nested-route-visible", "complete-payload-inspector-visible" },
            CapturePointerAction.Move,
            pointer
        );
    }

    private void Update()
    {
        if (Mouse.current == null)
            return;
        if (step == Step.Complete)
        {
            MarkDocumentsDirty();
            if (++settleFrames < 8)
                return;
            step = Step.Passed;
            SignalPassed(
                new[]
                {
                    "five-route-phases-highlighted",
                    "one-native-event-produces-one-rust-action",
                    "capture-lifecycle-complete",
                    "full-pointer-payload-readable",
                }
            );
            return;
        }
        if (step == Step.Moving && PointerAt(pointer))
        {
            step = Step.Pressing;
            RequestPointerInput(
                new[] { "pointer-down-targets-one-logical-element" },
                CapturePointerAction.LeftButtonDown,
                pointer
            );
            return;
        }
        if (step == Step.Pressing && Mouse.current.leftButton.isPressed)
        {
            if (!captureRequested)
            {
                captureRequested = true;
                captureTarget!.CapturePointer(PointerId.mousePointerId);
            }
            if (!Texts().Contains("CAPTURED · POINTER OWNED BY TARGET"))
                return;
            step = Step.Releasing;
            RequestPointerInput(
                new[] { "pointer-capture-observed-before-release" },
                CapturePointerAction.LeftButtonUp,
                pointer
            );
            return;
        }
        if (step != Step.Releasing || Mouse.current.leftButton.isPressed)
            return;
        if (captureTarget!.HasPointerCapture(PointerId.mousePointerId))
        {
            captureTarget.ReleasePointer(PointerId.mousePointerId);
            return;
        }
        if (!Texts().Contains("ACTIVE CAPTURE OBSERVED"))
            return;
        step = Step.Complete;
    }

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
        ) < 2;

    private static string Texts() =>
        string.Join(
            " | ",
            Documents()
                .SelectMany(document => document.rootVisualElement.Query<Label>().ToList())
                .Select(label => label.text)
        );

    private enum Step
    {
        Starting,
        Moving,
        Pressing,
        Releasing,
        Complete,
        Passed,
    }
}

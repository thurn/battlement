#nullable enable

using System.Collections;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.UIElements;

/// <summary>Captures the complete action console after deferred execution.</summary>
public sealed class UiActionsConsoleCaptureScenario : BattlementCaptureScenario
{
    public override string ScenarioName => "ui-actions-console";

    protected override void BeginCapture() => StartCoroutine(Capture());

    private IEnumerator Capture()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = UiRemainingLinkCaptureScenario.FindButton("25  ACTIONS + AUTHORITY");
            if (++frames > 900)
            {
                SignalFailed($"Actions navigation did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        UiRemainingLinkCaptureScenario.Click(navigation);
        Button? action = null;
        frames = 0;
        while (action == null)
        {
            action = UiRemainingLinkCaptureScenario.FindButton("Run all six actions");
            if (++frames > 300)
            {
                SignalFailed($"Action console did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        for (int frame = 0; frame < 8; frame++)
            yield return new WaitForEndOfFrame();
        RequestPointerInput(
            new[] { "action-console-ready" },
            CapturePointerAction.Move,
            NormalizedCenter(action)
        );
        UiRemainingLinkCaptureScenario.Click(action);
        frames = 0;
        while (!Texts().Contains("PASSED  Focus/Blur > ScrollTo > SelectText > Capture/Release"))
        {
            if (++frames > 300)
            {
                SignalFailed($"Action response did not settle. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        UiRemainingLinkCaptureScenario.MarkDocumentsDirty();
        for (int frame = 0; frame < 600; frame++)
            yield return null;
        SignalPassed(
            new[]
            {
                "all-six-actions-visible",
                "scroll-to-destination-visible",
                "utf16-selection-applied",
                "deferred-response-visible-before-repaint",
            }
        );
    }

    private static string Texts() => UiRemainingLinkCaptureScenario.Texts();

    private static Vector2 NormalizedCenter(VisualElement element) =>
        new(
            element.worldBound.center.x / Screen.width,
            element.worldBound.center.y / Screen.height
        );
}

/// <summary>Captures accepted, rejected, and input-disabled cleanup behavior.</summary>
public sealed class UiInputCleanupCaptureScenario : BattlementCaptureScenario
{
    public override string ScenarioName => "ui-input-cleanup";

    protected override void BeginCapture() => StartCoroutine(Capture());

    private IEnumerator Capture()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = UiRemainingLinkCaptureScenario.FindButton("25  ACTIONS + AUTHORITY");
            if (++frames > 900)
            {
                SignalFailed($"Actions navigation did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        UiRemainingLinkCaptureScenario.Click(navigation);
        Toggle? accepted = null;
        Toggle? rejected = null;
        TextField? draft = null;
        Slider? drag = null;
        Button? cleanup = null;
        frames = 0;
        while (cleanup == null)
        {
            accepted = UiRemainingLinkCaptureScenario.FindNamed<Toggle>("action-accepted");
            rejected = UiRemainingLinkCaptureScenario.FindNamed<Toggle>("action-rejected");
            draft = UiRemainingLinkCaptureScenario.FindNamed<TextField>("action-draft");
            drag = UiRemainingLinkCaptureScenario.FindNamed<Slider>("action-drag");
            cleanup = UiRemainingLinkCaptureScenario.FindButton("Reset cleanup proof");
            if (++frames > 300)
            {
                SignalFailed($"Controlled console did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        if (accepted == null || rejected == null || draft == null || drag == null)
        {
            SignalFailed($"Controlled inputs were incomplete. Content: {Texts()}");
            yield break;
        }
        for (int frame = 0; frame < 8; frame++)
            yield return new WaitForEndOfFrame();
        RequestPointerInput(
            new[] { "controlled-cleanup-ready" },
            CapturePointerAction.Move,
            NormalizedCenter(cleanup)
        );
        accepted.value = true;
        rejected.value = false;
        TextElement input = draft.Q<VisualElement>(TextField.textInputUssName).Q<TextElement>();
        draft.Focus();
        ((INotifyValueChanged<string>)input).value = "Uncommitted local draft";
        frames = 0;
        while (!Texts().Contains("DRAFT CLEANED"))
        {
            if (++frames > 300)
            {
                SignalFailed($"Draft cleanup did not settle. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        if (input.text != "Committed: North Gate")
        {
            SignalFailed($"Draft was not restored: {input.text}");
            yield break;
        }
        drag.CapturePointer(PointerId.mousePointerId);
        using (
            PointerCaptureEvent capture = PointerCaptureEvent.GetPooled(
                drag,
                null,
                PointerId.mousePointerId
            )
        )
            drag.SendEvent(capture);
        if (!drag.HasPointerCapture(PointerId.mousePointerId))
        {
            SignalFailed("Slider did not own the proof pointer capture.");
            yield break;
        }
        drag.value = 82;
        frames = 0;
        while (!Texts().Contains("CLEANED  draft + drag restored"))
        {
            if (++frames > 300)
            {
                SignalFailed($"Input cleanup did not settle. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        if (drag.value != 38 || drag.HasPointerCapture(PointerId.mousePointerId))
        {
            bool captured = drag.HasPointerCapture(PointerId.mousePointerId);
            SignalFailed($"Drag cleanup was incomplete: value={drag.value}, captured={captured}");
            yield break;
        }
        Focusable? focused = draft.panel?.focusController.focusedElement;
        if (focused == draft || focused == input)
        {
            SignalFailed("Draft focus was not released by input shutdown.");
            yield break;
        }
        UiRemainingLinkCaptureScenario.MarkDocumentsDirty();
        for (int frame = 0; frame < 600; frame++)
            yield return null;
        if (!Texts().Contains("0 cleanup events") || Texts().Contains("FAILED"))
        {
            SignalFailed($"Cleanup emitted a later event. Content: {Texts()}");
            yield break;
        }
        SignalPassed(
            new[]
            {
                "accepted-controlled-value-visible",
                "rejected-controlled-value-visible",
                "draft-and-drag-restored",
                "focus-and-capture-released-without-events",
            }
        );
    }

    private static string Texts() => UiRemainingLinkCaptureScenario.Texts();

    private static Vector2 NormalizedCenter(VisualElement element) =>
        new(
            element.worldBound.center.x / Screen.width,
            element.worldBound.center.y / Screen.height
        );
}

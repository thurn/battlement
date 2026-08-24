#nullable enable

using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

public sealed class BattlementLogViewerCapture : BattlementCaptureScenario
{
    private const string TestMessage = "Log viewer capture record";
    private CaptureState state;

    public override string ScenarioName => "battlement-log-viewer";

    protected override void BeginCapture()
    {
        BattlementCaptureShell shell = Object.FindAnyObjectByType<BattlementCaptureShell>();
        shell.SetTitle("Battlement in-game logs");
        shell.SetPhase("Opening with Command + Shift + L");
        shell.SetLegend("Modal viewer", "Incremental JSONL records", "Source and severity filters");
        Debug.Log(TestMessage);
        state = CaptureState.PressCommand;
        RequestKeyInput(
            new[] { "logging-bootstrap-ready" },
            CaptureKeyAction.KeyDown,
            Key.LeftMeta
        );
    }

    private void Update()
    {
        Keyboard? keyboard = Keyboard.current;
        if (keyboard == null)
        {
            return;
        }

        switch (state)
        {
            case CaptureState.PressCommand when keyboard.leftMetaKey.isPressed:
                state = CaptureState.PressShift;
                RequestKeyInput(new[] { "command-held" }, CaptureKeyAction.KeyDown, Key.LeftShift);
                break;
            case CaptureState.PressShift when keyboard.leftShiftKey.isPressed:
                state = CaptureState.PressL;
                RequestKeyInput(new[] { "command-shift-held" }, CaptureKeyAction.KeyDown, Key.L);
                break;
            case CaptureState.PressL when ViewerContainsTestRecord():
                state = CaptureState.ReleaseL;
                RequestKeyInput(
                    new[] { "log-viewer-open", "capture-record-visible" },
                    CaptureKeyAction.KeyUp,
                    Key.L
                );
                break;
            case CaptureState.ReleaseL when !keyboard.lKey.isPressed:
                state = CaptureState.ReleaseShift;
                RequestKeyInput(
                    new[] { "shortcut-letter-released" },
                    CaptureKeyAction.KeyUp,
                    Key.LeftShift
                );
                break;
            case CaptureState.ReleaseShift when !keyboard.leftShiftKey.isPressed:
                state = CaptureState.ReleaseCommand;
                RequestKeyInput(
                    new[] { "shortcut-shift-released" },
                    CaptureKeyAction.KeyUp,
                    Key.LeftMeta
                );
                break;
            case CaptureState.ReleaseCommand when !keyboard.leftMetaKey.isPressed:
                state = CaptureState.Complete;
                Object
                    .FindAnyObjectByType<BattlementCaptureShell>()
                    .SetPhase("Log viewer opened from the global shortcut");
                SignalPassed(
                    new[] { "log-viewer-open", "capture-record-visible", "shortcut-input-balanced" }
                );
                break;
            case CaptureState.PressCommand:
            case CaptureState.PressShift:
            case CaptureState.PressL:
            case CaptureState.ReleaseL:
            case CaptureState.ReleaseShift:
            case CaptureState.ReleaseCommand:
            case CaptureState.Complete:
            default:
                break;
        }
    }

    private static bool ViewerContainsTestRecord()
    {
        foreach (UIDocument document in Object.FindObjectsByType<UIDocument>())
        {
            VisualElement overlay = document.rootVisualElement.Q(
                className: "battlement-log-overlay"
            );
            Label details = document.rootVisualElement.Q<Label>(
                className: "battlement-log-details"
            );
            TextField search = document.rootVisualElement.Q<TextField>(
                className: "battlement-log-search"
            );
            if (
                overlay?.resolvedStyle.display == DisplayStyle.Flex
                && search != null
                && string.IsNullOrEmpty(search.value)
            )
            {
                search.value = TestMessage;
                return false;
            }
            if (
                overlay != null
                && overlay.resolvedStyle.display == DisplayStyle.Flex
                && details?.text.Contains(TestMessage) == true
            )
            {
                return true;
            }
        }

        return false;
    }

    private enum CaptureState
    {
        PressCommand,
        PressShift,
        PressL,
        ReleaseL,
        ReleaseShift,
        ReleaseCommand,
        Complete,
    }
}

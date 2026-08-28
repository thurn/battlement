#nullable enable

using Battlement.VisualCapture;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

public sealed class BattlementFpsViewerCaptureScenario : BattlementCaptureScenario
{
    private CaptureState state;

    public override string ScenarioName => "fps-viewer";

    protected override void BeginCapture()
    {
        BattlementCaptureShell? shell = Object.FindAnyObjectByType<BattlementCaptureShell>();
        if (shell != null)
        {
            shell.SetTitle("Battlement FPS viewer");
            shell.SetPhase("Opening with Command + Shift + F");
            shell.SetLegend("Safe-area aware", "Quarter-second refresh", "Debug UI command ready");
        }

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
                state = CaptureState.PressF;
                RequestKeyInput(new[] { "command-shift-held" }, CaptureKeyAction.KeyDown, Key.F);
                break;
            case CaptureState.PressF when FpsViewerReady():
                state = CaptureState.ReleaseF;
                RequestKeyInput(
                    new[] { "fps-viewer-open", "fps-value-visible" },
                    CaptureKeyAction.KeyUp,
                    Key.F
                );
                break;
            case CaptureState.ReleaseF when !keyboard.fKey.isPressed:
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
                BattlementCaptureShell shell = Object.FindAnyObjectByType<BattlementCaptureShell>();
                if (shell != null)
                {
                    shell.SetPhase("FPS viewer opened from the global shortcut");
                }
                SignalPassed(
                    new[] { "fps-viewer-open", "fps-value-visible", "shortcut-input-balanced" }
                );
                break;
            case CaptureState.PressCommand:
            case CaptureState.PressShift:
            case CaptureState.PressF:
            case CaptureState.ReleaseF:
            case CaptureState.ReleaseShift:
            case CaptureState.ReleaseCommand:
            case CaptureState.Complete:
            default:
                break;
        }
    }

    private static bool FpsViewerReady()
    {
        foreach (UIDocument document in Object.FindObjectsByType<UIDocument>())
        {
            VisualElement root = document.rootVisualElement.Q(className: "battlement-fps-overlay");
            Label label = document.rootVisualElement.Q<Label>(className: "battlement-fps-label");
            if (root?.resolvedStyle.display != DisplayStyle.Flex || label == null)
            {
                continue;
            }
            if (label.text.StartsWith("--") || !label.text.EndsWith(" FPS"))
            {
                continue;
            }

            return true;
        }

        return false;
    }

    private enum CaptureState
    {
        PressCommand,
        PressShift,
        PressF,
        ReleaseF,
        ReleaseShift,
        ReleaseCommand,
        Complete,
    }
}

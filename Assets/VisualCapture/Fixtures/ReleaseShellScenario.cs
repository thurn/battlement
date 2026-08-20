#nullable enable

using Masonry.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;

public sealed class ReleaseShellScenario : MasonryCaptureScenario
{
    private bool awaitingMove;
    private bool awaitingPress;
    private bool awaitingDrag;
    private bool awaitingRelease;
    private bool awaitingKeyDown;
    private bool awaitingKeyUp;

    public override string ScenarioName => "release-shell-fixture";

    protected override void BeginCapture()
    {
        Object.FindAnyObjectByType<MasonryCaptureShell>().SetPhase("Release colors ready");
        awaitingMove = true;
        RequestPointerInput(
            new[] { "primary-accent-success-visible" },
            CapturePointerAction.Move,
            new Vector2(0.25f, 0.25f)
        );
    }

    private void Update()
    {
        if (awaitingMove && PointerAt(new Vector2(0.25f, 0.25f)))
        {
            awaitingMove = false;
            awaitingPress = true;
            RequestPointerInput(
                new[] { "primary-accent-success-visible", "requested-move-observed" },
                CapturePointerAction.LeftButtonDown,
                new Vector2(0.25f, 0.25f)
            );
            return;
        }

        if (awaitingPress && Mouse.current.leftButton.wasPressedThisFrame)
        {
            awaitingPress = false;
            awaitingDrag = true;
            RequestPointerInput(
                new[]
                {
                    "primary-accent-success-visible",
                    "requested-move-observed",
                    "requested-press-observed",
                },
                CapturePointerAction.Move,
                new Vector2(0.75f, 0.75f)
            );
            return;
        }

        if (
            awaitingDrag
            && Mouse.current.leftButton.isPressed
            && PointerAt(new Vector2(0.75f, 0.75f))
        )
        {
            awaitingDrag = false;
            awaitingRelease = true;
            RequestPointerInput(
                new[]
                {
                    "primary-accent-success-visible",
                    "requested-move-observed",
                    "requested-press-observed",
                    "requested-drag-observed",
                },
                CapturePointerAction.LeftButtonUp,
                new Vector2(0.75f, 0.75f)
            );
            return;
        }

        if (awaitingRelease && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            awaitingRelease = false;
            awaitingKeyDown = true;
            RequestKeyInput(
                new[]
                {
                    "primary-accent-success-visible",
                    "requested-move-observed",
                    "requested-press-observed",
                    "requested-drag-observed",
                    "requested-release-observed",
                },
                CaptureKeyAction.KeyDown,
                Key.A
            );
            return;
        }

        if (awaitingKeyDown && Keyboard.current.aKey.wasPressedThisFrame)
        {
            awaitingKeyDown = false;
            awaitingKeyUp = true;
            RequestKeyInput(
                new[]
                {
                    "primary-accent-success-visible",
                    "requested-move-observed",
                    "requested-press-observed",
                    "requested-drag-observed",
                    "requested-release-observed",
                    "requested-key-down-observed",
                },
                CaptureKeyAction.KeyUp,
                Key.A
            );
            return;
        }

        if (!awaitingKeyUp || !Keyboard.current.aKey.wasReleasedThisFrame)
        {
            return;
        }

        awaitingKeyUp = false;
        Object.FindAnyObjectByType<MasonryCaptureShell>().SetPhase("Release input passed");
        SignalPassed(
            new[]
            {
                "primary-accent-success-visible",
                "requested-move-observed",
                "requested-press-observed",
                "requested-drag-observed",
                "requested-release-observed",
                "requested-key-down-observed",
                "requested-key-up-observed",
            }
        );
    }

    private static bool PointerAt(Vector2 topLeftNormalized) =>
        Vector2.Distance(
            Mouse.current.position.ReadValue(),
            new Vector2(
                topLeftNormalized.x * Screen.width,
                (1 - topLeftNormalized.y) * Screen.height
            )
        ) < 1;
}

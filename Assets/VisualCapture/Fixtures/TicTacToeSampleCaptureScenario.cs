#nullable enable

using System.Collections;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;

/// <summary>Captures a real human move and delayed AI move in the Tic-Tac-Toe sample.</summary>
public sealed class TicTacToeSampleCaptureScenario : BattlementCaptureScenario
{
    private static readonly Vector2 BoardCenter = new(0.5f, 0.5625f);

    private bool awaitingMove;
    private bool awaitingPress;
    private bool awaitingRelease;
    private bool releaseObserved;
    private int initialRendererCount;

    public override string ScenarioName => "tictactoe-sample";

    protected override void BeginCapture() => StartCoroutine(WaitForBoard());

    private IEnumerator WaitForBoard()
    {
        while (RendererCount() < 1)
        {
            yield return null;
        }

        yield return null;
        initialRendererCount = RendererCount();
        awaitingMove = true;
        RequestPointerInput(
            new[] { "rust-snapshot-rendered", "board-image-visible" },
            CapturePointerAction.Move,
            BoardCenter
        );
    }

    private void Update()
    {
        if (awaitingMove && PointerAt(BoardCenter))
        {
            awaitingMove = false;
            awaitingPress = true;
            RequestPointerInput(
                new[] { "rust-snapshot-rendered", "board-image-visible", "board-targeted" },
                CapturePointerAction.LeftButtonDown,
                BoardCenter
            );
            return;
        }

        if (awaitingPress && Mouse.current.leftButton.wasPressedThisFrame)
        {
            awaitingPress = false;
            awaitingRelease = true;
            RequestPointerInput(
                new[] { "rust-snapshot-rendered", "board-image-visible", "board-pressed" },
                CapturePointerAction.LeftButtonUp,
                BoardCenter
            );
            return;
        }

        if (awaitingRelease && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            releaseObserved = true;
        }

        if (!awaitingRelease || !releaseObserved || RendererCount() < initialRendererCount + 2)
        {
            return;
        }

        awaitingRelease = false;
        SignalPassed(
            new[]
            {
                "rust-snapshot-rendered",
                "board-image-visible",
                "human-x-rendered",
                "delayed-ai-o-rendered",
            }
        );
    }

    private static int RendererCount() =>
        Object.FindObjectsByType<MeshRenderer>(FindObjectsInactive.Exclude).Length;

    private static bool PointerAt(Vector2 topLeftNormalized) =>
        Vector2.Distance(
            Mouse.current.position.ReadValue(),
            new Vector2(
                topLeftNormalized.x * Screen.width,
                (1 - topLeftNormalized.y) * Screen.height
            )
        ) < 1;
}

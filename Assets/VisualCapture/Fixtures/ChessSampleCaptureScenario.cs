#nullable enable

using System.Collections;
using System.Linq;
using Masonry;
using Masonry.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;

/// <summary>Captures all Masonry-controlled pieces in the chess starting position.</summary>
public sealed class ChessSampleCaptureScenario : MasonryCaptureScenario
{
    private static readonly Vector2 BoardCenter = new(0.5f, 0.5f);

    private bool awaitingMove;

    public override string ScenarioName => "chess-sample";

    protected override void BeginCapture() => StartCoroutine(WaitForPieces());

    private IEnumerator WaitForPieces()
    {
        while (PieceCount() < 32)
        {
            yield return null;
        }

        yield return new WaitForEndOfFrame();
        awaitingMove = true;
        RequestPointerInput(
            new[]
            {
                "rust-snapshot-rendered",
                "decorated-board-scene-visible",
                "all-32-chess-pieces-rendered",
            },
            CapturePointerAction.Move,
            BoardCenter
        );
    }

    private void Update()
    {
        if (!awaitingMove || !PointerAt(BoardCenter) || PieceCount() != 32)
        {
            return;
        }

        awaitingMove = false;
        SignalPassed(
            new[]
            {
                "rust-snapshot-rendered",
                "decorated-board-scene-visible",
                "all-32-chess-pieces-rendered",
                "capture-frame-stable",
            }
        );
    }

    private static int PieceCount() =>
        Object
            .FindObjectsByType<MasonryIdentity>(FindObjectsInactive.Exclude)
            .Count(identity => identity.GetComponentInChildren<Renderer>() != null);

    private static bool PointerAt(Vector2 topLeftNormalized) =>
        Vector2.Distance(
            Mouse.current.position.ReadValue(),
            new Vector2(
                topLeftNormalized.x * Screen.width,
                (1 - topLeftNormalized.y) * Screen.height
            )
        ) < 1;
}

#nullable enable

using System.Collections;
using System.Linq;
using Masonry;
using Masonry.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;

/// <summary>Captures legal-square highlights while picking up a chess piece.</summary>
public sealed class ChessSampleCaptureScenario : MasonryCaptureScenario
{
    private static readonly System.Guid PlayButton = new("4cf7cb75-ec8f-44ec-88c9-c83ca3869f43");

    private MasonryIdentity? playButton;
    private Vector2 buttonPointer;
    private Vector2 pawnPointer;
    private bool awaitingHighlights;
    private bool awaitingMove;
    private bool awaitingPawnMove;
    private bool awaitingPawnPress;
    private bool awaitingPawnRelease;
    private bool awaitingPress;
    private bool awaitingRelease;
    private bool highlightsObserved;
    private bool pawnReleaseRequested;
    private float highlightsObservedAt;
    private bool releaseObserved;

    public override string ScenarioName => "chess-sample";

    protected override void BeginCapture() => StartCoroutine(WaitForPlayButton());

    private IEnumerator WaitForPlayButton()
    {
        while (playButton == null)
        {
            playButton = Object
                .FindObjectsByType<MasonryIdentity>(FindObjectsInactive.Exclude)
                .SingleOrDefault(identity => identity.Id == PlayButton);
            yield return null;
        }

        yield return new WaitForEndOfFrame();
        UnityEngine.Vector3 screen = Object
            .FindAnyObjectByType<Camera>()
            .WorldToScreenPoint(playButton.transform.position);
        buttonPointer = new Vector2(screen.x / Screen.width, 1 - screen.y / Screen.height);
        awaitingMove = true;
        RequestPointerInput(
            new[]
            {
                "rust-snapshot-rendered",
                "decorated-board-scene-visible",
                "play-button-visible",
            },
            CapturePointerAction.Move,
            buttonPointer
        );
    }

    private void Update()
    {
        if (awaitingMove && PointerAt(buttonPointer))
        {
            awaitingMove = false;
            awaitingPress = true;
            RequestPointerInput(
                new[] { "play-button-visible", "play-button-targeted" },
                CapturePointerAction.LeftButtonDown,
                buttonPointer
            );
            return;
        }

        if (awaitingPress && Mouse.current.leftButton.wasPressedThisFrame)
        {
            awaitingPress = false;
            awaitingRelease = true;
            RequestPointerInput(
                new[] { "play-button-visible", "play-button-pressed" },
                CapturePointerAction.LeftButtonUp,
                buttonPointer
            );
            return;
        }

        if (awaitingRelease && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            releaseObserved = true;
        }

        if (awaitingRelease && releaseObserved && PieceCount() == 32)
        {
            awaitingRelease = false;
            UnityEngine.Vector3 screen = Object
                .FindAnyObjectByType<Camera>()
                .WorldToScreenPoint(
                    PieceAt(new UnityEngine.Vector3(0.5f, 0, -2.5f)).transform.position
                );
            pawnPointer = new Vector2(screen.x / Screen.width, 1 - screen.y / Screen.height);
            awaitingPawnMove = true;
            RequestPointerInput(
                new[] { "all-32-chess-pieces-rendered", "white-pawn-visible" },
                CapturePointerAction.Move,
                pawnPointer
            );
            return;
        }

        if (awaitingPawnMove && PointerAt(pawnPointer))
        {
            awaitingPawnMove = false;
            awaitingPawnPress = true;
            RequestPointerInput(
                new[] { "white-pawn-visible", "white-pawn-targeted" },
                CapturePointerAction.LeftButtonDown,
                pawnPointer
            );
            return;
        }

        if (awaitingPawnPress && Mouse.current.leftButton.wasPressedThisFrame)
        {
            awaitingPawnPress = false;
            awaitingHighlights = true;
        }

        if (awaitingHighlights && HighlightCount() == 2)
        {
            awaitingHighlights = false;
            highlightsObserved = true;
            highlightsObservedAt = Time.unscaledTime;
        }

        if (
            highlightsObserved
            && !pawnReleaseRequested
            && Time.unscaledTime - highlightsObservedAt >= 1
        )
        {
            pawnReleaseRequested = true;
            awaitingPawnRelease = true;
            RequestPointerInput(
                new[] { "white-pawn-picked-up", "two-legal-squares-highlighted-green" },
                CapturePointerAction.LeftButtonUp,
                pawnPointer
            );
            return;
        }

        if (awaitingPawnRelease && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            awaitingPawnRelease = false;
        }

        if (pawnReleaseRequested && !awaitingPawnRelease && HighlightCount() == 0)
        {
            highlightsObserved = false;
            SignalPassed(
                new[]
                {
                    "rust-snapshot-rendered",
                    "decorated-board-scene-visible",
                    "play-button-clicked",
                    "all-32-chess-pieces-rendered",
                    "white-pawn-picked-up",
                    "two-legal-squares-highlighted-green",
                    "capture-frame-stable",
                }
            );
        }
    }

    private static int PieceCount() =>
        Object
            .FindObjectsByType<MasonryIdentity>(FindObjectsInactive.Exclude)
            .Count(identity => identity.GetComponentInChildren<Renderer>() != null);

    private static MasonryIdentity PieceAt(UnityEngine.Vector3 position) =>
        Object
            .FindObjectsByType<MasonryIdentity>(FindObjectsInactive.Exclude)
            .Single(identity =>
                UnityEngine.Vector3.Distance(identity.transform.position, position) < 0.01f
                && identity.GetComponentInChildren<Renderer>() != null
            );

    private static int HighlightCount() =>
        Object
            .FindObjectsByType<MasonryIdentity>(FindObjectsInactive.Exclude)
            .Count(identity =>
            {
                Renderer? renderer = identity.GetComponent<Renderer>();
                return renderer != null && renderer.sharedMaterial.name == "Legal Square";
            });

    private static bool PointerAt(Vector2 topLeftNormalized) =>
        Vector2.Distance(
            Mouse.current.position.ReadValue(),
            new Vector2(
                topLeftNormalized.x * Screen.width,
                (1 - topLeftNormalized.y) * Screen.height
            )
        ) < 1;
}

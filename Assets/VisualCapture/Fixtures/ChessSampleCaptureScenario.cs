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
    private static readonly System.Guid PlayButton = new("4cf7cb75-ec8f-44ec-88c9-c83ca3869f43");

    private MasonryIdentity? playButton;
    private Vector2 buttonPointer;
    private bool awaitingMove;
    private bool awaitingPress;
    private bool awaitingRelease;
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

        if (!awaitingRelease || !releaseObserved || PieceCount() != 32)
        {
            return;
        }

        awaitingRelease = false;
        SignalPassed(
            new[]
            {
                "rust-snapshot-rendered",
                "decorated-board-scene-visible",
                "play-button-clicked",
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

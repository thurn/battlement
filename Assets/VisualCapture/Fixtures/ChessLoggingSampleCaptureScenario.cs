#nullable enable

using System;
using System.Collections;
using System.IO;
using System.Linq;
using Battlement;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Moves a chess piece and displays the Rust events in the in-game log viewer.</summary>
public sealed class ChessLoggingSampleCaptureScenario : BattlementCaptureScenario
{
    private static readonly Guid PlayButton = new("4cf7cb75-ec8f-44ec-88c9-c83ca3869f43");
    private CaptureState state;
    private Vector2 playPointer;
    private Vector2 pawnPointer;
    private Vector2 targetPointer;
    private bool pawnPressObserved;
    private bool playReleaseObserved;
    private bool releaseObserved;

    public override string ScenarioName => "chess-logging-sample";

    [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.BeforeSceneLoad)]
    private static void ResetSavedGame()
    {
        if (CaptureArguments.Value("-battlementCaptureScenario") != "chess-logging-sample")
        {
            return;
        }

        File.Delete(Path.Combine(Application.persistentDataPath, "chess-game.json"));
    }

    protected override void BeginCapture() => StartCoroutine(WaitForPlayButton());

    private IEnumerator WaitForPlayButton()
    {
        BattlementIdentity? playButton = null;
        while (playButton == null)
        {
            playButton = Object
                .FindObjectsByType<BattlementIdentity>(FindObjectsInactive.Exclude)
                .SingleOrDefault(identity => identity.Id == PlayButton);
            yield return null;
        }

        yield return new WaitForEndOfFrame();
        playPointer = PointerFor(playButton.transform.position);
        state = CaptureState.MoveToPlay;
        RequestPointerInput(
            new[] { "real-chess-scene-visible", "play-button-visible" },
            CapturePointerAction.Move,
            playPointer
        );
    }

    private void Update()
    {
        Keyboard? keyboard = Keyboard.current;
        switch (state)
        {
            case CaptureState.MoveToPlay when PointerAt(playPointer):
                state = CaptureState.PressPlay;
                RequestPointerInput(
                    new[] { "play-button-targeted" },
                    CapturePointerAction.LeftButtonDown,
                    playPointer
                );
                break;
            case CaptureState.PressPlay when Mouse.current.leftButton.wasPressedThisFrame:
                state = CaptureState.ReleasePlay;
                RequestPointerInput(
                    new[] { "play-button-pressed" },
                    CapturePointerAction.LeftButtonUp,
                    playPointer
                );
                break;
            case CaptureState.ReleasePlay when Mouse.current.leftButton.wasReleasedThisFrame:
                playReleaseObserved = true;
                break;
            case CaptureState.ReleasePlay when playReleaseObserved && PieceCount() == 32:
                pawnPointer = PointerFor(
                    PieceAt(new UnityEngine.Vector3(0.5f, 0, -2.5f)).transform.position
                );
                targetPointer = PointerFor(new UnityEngine.Vector3(0.5f, 0, -0.5f));
                state = CaptureState.MoveToPawn;
                RequestPointerInput(
                    new[] { "new-game-started", "all-32-chess-pieces-rendered" },
                    CapturePointerAction.Move,
                    pawnPointer
                );
                break;
            case CaptureState.MoveToPawn when PointerAt(pawnPointer):
                state = CaptureState.PressPawn;
                RequestPointerInput(
                    new[] { "white-e-pawn-targeted" },
                    CapturePointerAction.LeftButtonDown,
                    pawnPointer
                );
                break;
            case CaptureState.PressPawn when Mouse.current.leftButton.wasPressedThisFrame:
                pawnPressObserved = true;
                break;
            case CaptureState.PressPawn when pawnPressObserved && HighlightCount() == 2:
                state = CaptureState.DragPawn;
                RequestPointerInput(
                    new[] { "white-e-pawn-picked-up", "two-legal-squares-highlighted" },
                    CapturePointerAction.Move,
                    targetPointer
                );
                break;
            case CaptureState.DragPawn when PointerAt(targetPointer):
                state = CaptureState.ReleasePawn;
                RequestPointerInput(
                    new[] { "white-e-pawn-dragged-to-e4" },
                    CapturePointerAction.LeftButtonUp,
                    targetPointer
                );
                break;
            case CaptureState.ReleasePawn when Mouse.current.leftButton.wasReleasedThisFrame:
                releaseObserved = true;
                break;
            case CaptureState.ReleasePawn
                when releaseObserved && PieceExistsAt(new UnityEngine.Vector3(0.5f, 0, -0.5f)):
                state = CaptureState.PressCommand;
                RequestKeyInput(
                    new[] { "white-e-pawn-moved-to-e4", "rust-move-event-written" },
                    CaptureKeyAction.KeyDown,
                    Key.LeftMeta
                );
                break;
            case CaptureState.PressCommand when keyboard?.leftMetaKey.isPressed == true:
                state = CaptureState.PressShift;
                RequestKeyInput(new[] { "command-held" }, CaptureKeyAction.KeyDown, Key.LeftShift);
                break;
            case CaptureState.PressShift when keyboard?.leftShiftKey.isPressed == true:
                state = CaptureState.PressL;
                RequestKeyInput(new[] { "command-shift-held" }, CaptureKeyAction.KeyDown, Key.L);
                break;
            case CaptureState.PressL when ViewerShowsChessEvents():
                state = CaptureState.ReleaseL;
                RequestKeyInput(
                    new[] { "log-viewer-open", "rust-chess-events-visible" },
                    CaptureKeyAction.KeyUp,
                    Key.L
                );
                break;
            case CaptureState.ReleaseL when keyboard?.lKey.isPressed == false:
                state = CaptureState.ReleaseShift;
                RequestKeyInput(
                    new[] { "shortcut-letter-released" },
                    CaptureKeyAction.KeyUp,
                    Key.LeftShift
                );
                break;
            case CaptureState.ReleaseShift when keyboard?.leftShiftKey.isPressed == false:
                state = CaptureState.ReleaseCommand;
                RequestKeyInput(
                    new[] { "shortcut-shift-released" },
                    CaptureKeyAction.KeyUp,
                    Key.LeftMeta
                );
                break;
            case CaptureState.ReleaseCommand when keyboard?.leftMetaKey.isPressed == false:
                state = CaptureState.Complete;
                SignalPassed(
                    new[]
                    {
                        "real-chess-scene-visible",
                        "white-e-pawn-moved-to-e4",
                        "log-viewer-open",
                        "rust-chess-events-visible",
                        "shortcut-input-balanced",
                    }
                );
                break;
            case CaptureState.Waiting:
            case CaptureState.MoveToPlay:
            case CaptureState.PressPlay:
            case CaptureState.ReleasePlay:
            case CaptureState.MoveToPawn:
            case CaptureState.PressPawn:
            case CaptureState.DragPawn:
            case CaptureState.ReleasePawn:
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

    private static bool ViewerShowsChessEvents()
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
            if (overlay?.resolvedStyle.display != DisplayStyle.Flex || search == null)
            {
                continue;
            }
            if (search.value != "chess.")
            {
                search.value = "chess.";
                return false;
            }
            if (
                details?.text.Contains("[rust/information]") == true
                && details.text.Contains("chess.game.started")
                && details.text.Contains("chess.move.applied")
                && details.text.Contains("chess.ai.search_started")
            )
            {
                return true;
            }
        }

        return false;
    }

    private static int PieceCount() =>
        Object
            .FindObjectsByType<BattlementIdentity>(FindObjectsInactive.Exclude)
            .Count(identity => identity.GetComponentInChildren<Renderer>() != null);

    private static BattlementIdentity PieceAt(UnityEngine.Vector3 position) =>
        Object
            .FindObjectsByType<BattlementIdentity>(FindObjectsInactive.Exclude)
            .Single(identity =>
                UnityEngine.Vector3.Distance(identity.transform.position, position) < 0.01f
                && identity.GetComponentInChildren<Renderer>() != null
            );

    private static bool PieceExistsAt(UnityEngine.Vector3 position) =>
        Object
            .FindObjectsByType<BattlementIdentity>(FindObjectsInactive.Exclude)
            .Any(identity =>
                UnityEngine.Vector3.Distance(identity.transform.position, position) < 0.01f
                && identity.GetComponentInChildren<Renderer>() != null
            );

    private static int HighlightCount() =>
        Object
            .FindObjectsByType<BattlementIdentity>(FindObjectsInactive.Exclude)
            .Count(identity => IsLegalSquare(identity.GetComponent<Renderer>()));

    private static bool IsLegalSquare(Renderer? renderer) =>
        renderer != null && renderer.sharedMaterial.name == "Legal Square";

    private static Vector2 PointerFor(UnityEngine.Vector3 worldPosition)
    {
        UnityEngine.Vector3 screen = Object
            .FindAnyObjectByType<Camera>()
            .WorldToScreenPoint(worldPosition);
        return new Vector2(screen.x / Screen.width, 1 - screen.y / Screen.height);
    }

    private static bool PointerAt(Vector2 topLeftNormalized) =>
        Vector2.Distance(
            Mouse.current.position.ReadValue(),
            new Vector2(
                topLeftNormalized.x * Screen.width,
                (1 - topLeftNormalized.y) * Screen.height
            )
        ) < 1;

    private enum CaptureState
    {
        Waiting,
        MoveToPlay,
        PressPlay,
        ReleasePlay,
        MoveToPawn,
        PressPawn,
        DragPawn,
        ReleasePawn,
        PressCommand,
        PressShift,
        PressL,
        ReleaseL,
        ReleaseShift,
        ReleaseCommand,
        Complete,
    }
}

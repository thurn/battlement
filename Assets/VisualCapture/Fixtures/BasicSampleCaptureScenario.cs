#nullable enable

using System.Collections;
using System.Linq;
using Battlement;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityVector3 = UnityEngine.Vector3;

/// <summary>Captures a completed snap-to-pointer drag in the basic sample.</summary>
public sealed class BasicSampleCaptureScenario : BattlementCaptureScenario
{
    private static readonly System.Guid CubeA = new("00000000-0000-0000-0000-000000000064");
    private static readonly UnityVector3 Destination = new(-0.5f, 1.25f, 0);

    private BattlementIdentity? cube;
    private Vector2 destinationPointer;
    private Vector2 pickupPointer;
    private bool awaitingMove;
    private bool awaitingPress;
    private bool awaitingDrag;
    private bool awaitingRelease;

    public override string ScenarioName => "basic-sample";

    protected override void BeginCapture() => StartCoroutine(WaitForCube());

    private IEnumerator WaitForCube()
    {
        while (cube == null)
        {
            cube = Object
                .FindObjectsByType<BattlementIdentity>(FindObjectsInactive.Exclude)
                .SingleOrDefault(identity => identity.Id == CubeA);
            yield return null;
        }

        yield return new WaitForEndOfFrame();
        Camera camera = Object.FindAnyObjectByType<Camera>();
        pickupPointer = Normalize(camera.WorldToScreenPoint(cube.transform.position));
        destinationPointer = Normalize(camera.WorldToScreenPoint(Destination));
        awaitingMove = true;
        RequestPointerInput(
            new[] { "rust-snapshot-rendered", "draggable-cubes-visible" },
            CapturePointerAction.Move,
            pickupPointer
        );
    }

    private void Update()
    {
        if (awaitingMove && PointerAt(pickupPointer))
        {
            awaitingMove = false;
            awaitingPress = true;
            RequestPointerInput(
                new[] { "rust-snapshot-rendered", "cube-a-targeted" },
                CapturePointerAction.LeftButtonDown,
                pickupPointer
            );
            return;
        }

        if (awaitingPress && Mouse.current.leftButton.wasPressedThisFrame)
        {
            awaitingPress = false;
            awaitingDrag = true;
            RequestPointerInput(
                new[] { "rust-drag-start-submitted", "cube-a-captured" },
                CapturePointerAction.Move,
                destinationPointer
            );
            return;
        }

        if (awaitingDrag && PointerAt(destinationPointer) && AtDestination())
        {
            awaitingDrag = false;
            awaitingRelease = true;
            RequestPointerInput(
                new[] { "cube-a-followed-pointer", "snap-to-pointer-visible" },
                CapturePointerAction.LeftButtonUp,
                destinationPointer
            );
            return;
        }

        if (!awaitingRelease || !Mouse.current.leftButton.wasReleasedThisFrame)
        {
            return;
        }

        awaitingRelease = false;
        SignalPassed(
            new[]
            {
                "rust-drag-start-submitted",
                "cube-a-followed-pointer",
                "rust-drag-end-submitted",
                "final-world-position-committed",
            }
        );
    }

    private bool AtDestination() =>
        UnityVector3.Distance(cube!.transform.position, Destination) < 0.02f;

    private static Vector2 Normalize(UnityVector3 screen) =>
        new(screen.x / Screen.width, 1 - screen.y / Screen.height);

    private static bool PointerAt(Vector2 topLeftNormalized) =>
        Vector2.Distance(
            Mouse.current.position.ReadValue(),
            new Vector2(
                topLeftNormalized.x * Screen.width,
                (1 - topLeftNormalized.y) * Screen.height
            )
        ) < 1;
}

#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;

/// <summary>Captures the first Rust-authored Battlement UI lab shell.</summary>
public sealed class UiSampleCaptureScenario : BattlementCaptureScenario
{
    private static readonly Vector2 CapturePointer = new(0.98f, 0.98f);

    private bool awaitingPointer;

    public override string ScenarioName => "ui-sample";

    protected override void BeginCapture() => StartCoroutine(WaitForShell());

    private IEnumerator WaitForShell()
    {
        Label? specimen = null;
        while (specimen == null)
        {
            specimen = Object
                .FindObjectsByType<UIDocument>(FindObjectsInactive.Exclude)
                .SelectMany(document => document.rootVisualElement.Query<Label>().ToList())
                .FirstOrDefault(label => label.text == "FIRST RUST-AUTHORED LABEL");
            yield return null;
        }

        yield return new WaitForEndOfFrame();
        yield return new WaitForEndOfFrame();
        awaitingPointer = true;
        RequestPointerInput(
            new[] { "rust-snapshot-rendered", "command-deck-shell-visible" },
            CapturePointerAction.Move,
            CapturePointer
        );
    }

    private void Update()
    {
        if (!awaitingPointer || Mouse.current == null)
        {
            return;
        }
        Vector2 expected = new(
            CapturePointer.x * Screen.width,
            (1 - CapturePointer.y) * Screen.height
        );
        if (Vector2.Distance(Mouse.current.position.ReadValue(), expected) >= 1)
        {
            return;
        }

        awaitingPointer = false;
        SignalPassed(
            new[]
            {
                "rust-snapshot-rendered",
                "command-deck-shell-visible",
                "document-root-inspector-visible",
            }
        );
    }
}

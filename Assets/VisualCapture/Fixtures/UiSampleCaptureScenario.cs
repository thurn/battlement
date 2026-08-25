#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures Rust-handled hierarchy changes in the UI lab.</summary>
public sealed class UiSampleCaptureScenario : BattlementCaptureScenario
{
    private Vector2 mutationButton;
    private int phase;
    private bool mutationReleaseObserved;

    public override string ScenarioName => "ui-sample";

    protected override void BeginCapture() => StartCoroutine(WaitForShell());

    private IEnumerator WaitForShell()
    {
        Button? hierarchy = null;
        while (hierarchy == null)
        {
            hierarchy = FindButton("03  HIERARCHY");
            yield return null;
        }

        yield return new WaitForEndOfFrame();
        yield return new WaitForEndOfFrame();
        using (ClickEvent click = ClickEvent.GetPooled())
        {
            click.target = hierarchy;
            hierarchy.SendEvent(click);
        }

        Button? mutation = null;
        while (mutation == null || !IsNormalized(NormalizedCenter(mutation)))
        {
            mutation = FindButton("Reorder children");
            yield return null;
        }

        mutationButton = NormalizedCenter(mutation);
        phase = 1;
        RequestPointerInput(
            new[]
            {
                "rust-snapshot-rendered",
                "rust-navigation-click-handled",
                "hierarchy-page-visible",
            },
            CapturePointerAction.Move,
            mutationButton
        );
    }

    private void Update()
    {
        if (Mouse.current == null)
        {
            return;
        }

        if (phase == 1 && PointerAt(mutationButton))
        {
            phase = 2;
            RequestPointerInput(
                new[] { "hierarchy-mutation-button-targeted" },
                CapturePointerAction.LeftButtonDown,
                mutationButton
            );
            return;
        }

        if (phase == 2 && Mouse.current.leftButton.wasPressedThisFrame)
        {
            phase = 3;
            RequestPointerInput(
                new[] { "hierarchy-mutation-click-dispatched" },
                CapturePointerAction.LeftButtonUp,
                mutationButton
            );
            return;
        }

        if (phase == 3 && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            mutationReleaseObserved = true;
        }

        if (phase != 3 || !mutationReleaseObserved)
        {
            return;
        }
        Label? alpha = FindLabel("Alpha");
        Label? beta = FindLabel("Beta");
        Label? movable = FindLabel("Move");
        if (alpha == null || beta == null || movable == null)
        {
            return;
        }
        if (alpha.enabledInHierarchy || FindButton("Reset") == null)
        {
            return;
        }
        if (
            alpha.parent != beta.parent
            || alpha.parent.IndexOf(beta) >= alpha.parent.IndexOf(alpha)
        )
        {
            return;
        }
        if (movable.parent == alpha.parent)
        {
            return;
        }

        phase = 4;
        SignalPassed(
            new[]
            {
                "rust-snapshot-rendered",
                "rust-navigation-click-handled",
                "hierarchy-page-visible",
                "rust-hierarchy-mutation-handled",
                "reordered-disabled-hierarchy-visible",
            }
        );
    }

    private static Button? FindButton(string text) =>
        Object
            .FindObjectsByType<UIDocument>(FindObjectsInactive.Exclude)
            .SelectMany(document => document.rootVisualElement.Query<Button>().ToList())
            .FirstOrDefault(button => button.text == text);

    private static Label? FindLabel(string text) =>
        Object
            .FindObjectsByType<UIDocument>(FindObjectsInactive.Exclude)
            .SelectMany(document => document.rootVisualElement.Query<Label>().ToList())
            .FirstOrDefault(label => label.text == text);

    private static Vector2 NormalizedCenter(VisualElement element) =>
        new(
            element.worldBound.center.x / Screen.width,
            element.worldBound.center.y / Screen.height
        );

    private static bool IsNormalized(Vector2 position)
    {
        if (!float.IsFinite(position.x) || !float.IsFinite(position.y))
        {
            return false;
        }
        if (position.x < 0 || position.x > 1)
        {
            return false;
        }
        return position.y >= 0 && position.y <= 1;
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

#nullable enable

using Masonry.VisualCapture;
using UnityEngine;

public sealed class ReleaseShellScenario : MasonryCaptureScenario
{
    private bool awaitingPress;
    private bool awaitingRelease;

    public override string ScenarioName => "release-shell-fixture";

    protected override void BeginCapture()
    {
        Object.FindAnyObjectByType<MasonryCaptureShell>().SetPhase("Release colors ready");
        awaitingPress = true;
        RequestInput(
            new[] { "primary-accent-success-visible" },
            CaptureInput.PointerLeftButtonDown,
            new Vector2(0.5f, 0.5f)
        );
    }

    private void Update()
    {
        if (awaitingPress && Input.GetMouseButtonDown(0))
        {
            awaitingPress = false;
            awaitingRelease = true;
            RequestInput(
                new[] { "primary-accent-success-visible", "requested-press-observed" },
                CaptureInput.PointerLeftButtonUp,
                new Vector2(0.5f, 0.5f)
            );
            return;
        }

        if (!awaitingRelease || !Input.GetMouseButtonUp(0))
        {
            return;
        }

        awaitingRelease = false;
        Object.FindAnyObjectByType<MasonryCaptureShell>().SetPhase("Release input passed");
        SignalPassed(
            new[]
            {
                "primary-accent-success-visible",
                "requested-press-observed",
                "requested-release-observed",
            }
        );
    }
}

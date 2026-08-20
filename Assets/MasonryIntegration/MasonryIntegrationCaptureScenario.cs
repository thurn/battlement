#nullable enable

using System.Collections;
using Masonry.VisualCapture;
using UnityEngine;

namespace Masonry.Integration
{
    /// <summary>Drives the packaged integration fixture through real pointer actions.</summary>
    public sealed class MasonryIntegrationCaptureScenario : MasonryCaptureScenario
    {
        private MasonryIntegrationFixture fixture = null!;
        private bool awaitingMove;
        private bool awaitingDown;
        private bool awaitingUp;
        private bool releaseObserved;
        private Vector2 pointer;

        public override string ScenarioName => "masonry-integration-fixture";

        protected override void BeginCapture()
        {
            fixture = FindAnyObjectByType<MasonryIntegrationFixture>();
            StartCoroutine(WaitForFixture());
        }

        private IEnumerator WaitForFixture()
        {
            while (!fixture.IsReadyForClick && fixture.Failure.Length == 0)
            {
                yield return null;
            }

            if (fixture.Failure.Length > 0)
            {
                SignalFailed(fixture.Failure);
                yield break;
            }

            pointer = fixture.ClickTargetPosition();
            awaitingMove = true;
            RequestPointerInput(
                new[] { "real-addressables-loaded", "rust-snapshot-rendered" },
                CapturePointerAction.Move,
                pointer
            );
        }

        private void Update()
        {
            if (fixture == null || fixture.Failure.Length > 0)
            {
                if (fixture != null)
                {
                    SignalFailed(fixture.Failure);
                }
                return;
            }

            if (awaitingMove && PointerAtTarget())
            {
                awaitingMove = false;
                awaitingDown = true;
                RequestPointerInput(
                    new[]
                    {
                        "real-addressables-loaded",
                        "rust-snapshot-rendered",
                        "pointer-targeted",
                    },
                    CapturePointerAction.LeftButtonDown,
                    pointer
                );
                return;
            }

            if (
                awaitingDown && UnityEngine.InputSystem.Mouse.current.leftButton.wasPressedThisFrame
            )
            {
                awaitingDown = false;
                awaitingUp = true;
                RequestPointerInput(
                    new[]
                    {
                        "real-addressables-loaded",
                        "rust-snapshot-rendered",
                        "pointer-targeted",
                        "pointer-pressed",
                    },
                    CapturePointerAction.LeftButtonUp,
                    pointer
                );
                return;
            }

            if (awaitingUp && UnityEngine.InputSystem.Mouse.current.leftButton.wasReleasedThisFrame)
            {
                releaseObserved = true;
            }

            if (awaitingUp && releaseObserved && fixture.HasPassed)
            {
                awaitingUp = false;
                SignalPassed(
                    new[]
                    {
                        "real-addressables-loaded",
                        "rust-snapshot-rendered",
                        "pointer-click-submitted",
                        "rust-command-applied",
                    }
                );
            }
        }

        private bool PointerAtTarget() =>
            Vector2.Distance(
                UnityEngine.InputSystem.Mouse.current.position.ReadValue(),
                new Vector2(pointer.x * Screen.width, (1 - pointer.y) * Screen.height)
            ) < 1;
    }
}

#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using UnityEngine;

namespace Masonry.VisualCapture
{
    /// <summary>A low-level host input requested by a visual-capture scenario.</summary>
    public enum CaptureInput
    {
        /// <summary>Moves the pointer without changing button state.</summary>
        PointerMove,

        /// <summary>Presses the primary pointer button.</summary>
        PointerLeftButtonDown,

        /// <summary>Releases the primary pointer button.</summary>
        PointerLeftButtonUp,
    }

    /// <summary>Base API implemented by one deterministic visual-capture scenario.</summary>
    public abstract class MasonryCaptureScenario : MonoBehaviour
    {
        private string statusPath = string.Empty;
        private CapturePhase phase;
        private int requestId;

        /// <summary>Gets the stable command-line name of this scenario.</summary>
        public abstract string ScenarioName { get; }

        /// <summary>Whether capture-only diagnostics were requested.</summary>
        protected bool ShowCaptureOverlay { get; private set; }

        /// <summary>Begins the selected scenario after command-line setup.</summary>
        protected abstract void BeginCapture();

        /// <summary>
        /// Requests one host input at a top-left-origin normalized pointer position.
        /// </summary>
        protected void RequestInput(
            IEnumerable<string> assertions,
            CaptureInput input,
            Vector2 normalizedPointerPosition
        )
        {
            if (phase == CapturePhase.Passed || phase == CapturePhase.Failed)
            {
                throw new InvalidOperationException(
                    "A completed capture scenario cannot request more input."
                );
            }

            if (
                !float.IsFinite(normalizedPointerPosition.x)
                || !float.IsFinite(normalizedPointerPosition.y)
                || normalizedPointerPosition.x < 0
                || normalizedPointerPosition.x > 1
                || normalizedPointerPosition.y < 0
                || normalizedPointerPosition.y > 1
            )
            {
                throw new ArgumentOutOfRangeException(
                    nameof(normalizedPointerPosition),
                    "Capture pointer coordinates must be finite normalized values."
                );
            }

            phase = CapturePhase.Ready;
            requestId++;
            Publish(
                "ready",
                NormalizeAssertions(assertions),
                requestId,
                InputName(input),
                normalizedPointerPosition
            );
        }

        /// <summary>Signals that all scenario interactions and assertions passed.</summary>
        protected void SignalPassed(IEnumerable<string> assertions)
        {
            if (phase != CapturePhase.Ready)
            {
                throw new InvalidOperationException(
                    "A capture scenario must be ready before passing."
                );
            }

            phase = CapturePhase.Passed;
            Publish("passed", NormalizeAssertions(assertions), requestId, string.Empty, null);
        }

        /// <summary>Publishes a terminal scenario failure for the capture command.</summary>
        protected void SignalFailed(string failure)
        {
            if (phase == CapturePhase.Passed || phase == CapturePhase.Failed)
            {
                return;
            }

            phase = CapturePhase.Failed;
            Publish(
                "failed",
                Array.Empty<string>(),
                requestId,
                string.Empty,
                null,
                string.IsNullOrWhiteSpace(failure) ? "Unknown scenario failure." : failure
            );
        }

        private void Start()
        {
            string? selectedScenario = ArgumentValue("-masonryCaptureScenario");
            if (!string.Equals(selectedScenario, ScenarioName, StringComparison.Ordinal))
            {
                enabled = false;
                return;
            }

            statusPath = ArgumentValue("-masonryCaptureStatus") ?? string.Empty;
            ShowCaptureOverlay = HasArgument("-masonryCaptureOverlay");
            try
            {
                BeginCapture();
            }
            catch (Exception exception)
            {
                SignalFailed(exception.Message);
            }
        }

        private void Publish(
            string status,
            string[] assertions,
            int publishedRequestId,
            string input,
            Vector2? pointerPosition,
            string? failure = null
        )
        {
            if (string.IsNullOrEmpty(statusPath))
            {
                throw new InvalidOperationException("The capture status path was not supplied.");
            }

            string? directory = Path.GetDirectoryName(statusPath);
            if (string.IsNullOrEmpty(directory))
            {
                throw new InvalidOperationException("The capture status path has no directory.");
            }

            Directory.CreateDirectory(directory);
            string temporaryPath = statusPath + ".new";
            File.WriteAllText(
                temporaryPath,
                JsonUtility.ToJson(
                    new CaptureStatus(
                        status,
                        ScenarioName,
                        assertions,
                        publishedRequestId,
                        input,
                        pointerPosition?.x ?? -1,
                        pointerPosition?.y ?? -1,
                        failure
                    ),
                    true
                )
            );
            if (File.Exists(statusPath))
            {
                File.Delete(statusPath);
            }

            File.Move(temporaryPath, statusPath);
        }

        private static string[] NormalizeAssertions(IEnumerable<string> assertions)
        {
            string[] normalized = (
                assertions ?? throw new ArgumentNullException(nameof(assertions))
            )
                .Select(assertion => assertion?.Trim() ?? string.Empty)
                .ToArray();
            if (
                normalized.Any(string.IsNullOrEmpty)
                || normalized.Distinct(StringComparer.Ordinal).Count() != normalized.Length
            )
            {
                throw new ArgumentException(
                    "Capture assertions must be nonempty and unique.",
                    nameof(assertions)
                );
            }

            return normalized;
        }

        private static string InputName(CaptureInput input) =>
            input switch
            {
                CaptureInput.PointerMove => "pointer-move",
                CaptureInput.PointerLeftButtonDown => "pointer-left-button-down",
                CaptureInput.PointerLeftButtonUp => "pointer-left-button-up",
                _ => throw new ArgumentOutOfRangeException(nameof(input)),
            };

        private static string? ArgumentValue(string name)
        {
            string[] arguments = Environment.GetCommandLineArgs();
            for (int index = 0; index < arguments.Length - 1; index++)
            {
                if (arguments[index] == name)
                {
                    return arguments[index + 1];
                }
            }

            return null;
        }

        private static bool HasArgument(string name) =>
            Array.IndexOf(Environment.GetCommandLineArgs(), name) >= 0;

        private enum CapturePhase
        {
            Starting,
            Ready,
            Passed,
            Failed,
        }

        [Serializable]
        private sealed class CaptureStatus
        {
            public CaptureStatus(
                string phase,
                string scenario,
                string[] assertions,
                int requestId,
                string input,
                float pointerX,
                float pointerY,
                string? failure
            )
            {
                this.phase = phase;
                this.scenario = scenario;
                this.assertions = assertions;
                this.requestId = requestId;
                this.input = input;
                this.pointerX = pointerX;
                this.pointerY = pointerY;
                this.failure = failure;
            }

            [SerializeField]
            private string phase;

            [SerializeField]
            private string scenario;

            [SerializeField]
            private string[] assertions;

            [SerializeField]
            private int requestId;

            [SerializeField]
            private string input;

            [SerializeField]
            private float pointerX;

            [SerializeField]
            private float pointerY;

            [SerializeField]
            private string? failure;
        }
    }
}

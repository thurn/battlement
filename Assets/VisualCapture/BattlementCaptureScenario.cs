#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using UnityEngine;
using UnityEngine.InputSystem;

namespace Battlement.VisualCapture
{
    /// <summary>A pointer transition requested by a visual-capture scenario.</summary>
    public enum CapturePointerAction
    {
        Move,
        LeftButtonDown,
        LeftButtonUp,
    }

    /// <summary>A keyboard transition requested by a visual-capture scenario.</summary>
    public enum CaptureKeyAction
    {
        KeyDown,
        KeyUp,
    }

    /// <summary>Base API implemented by one deterministic visual-capture scenario.</summary>
    public abstract class BattlementCaptureScenario : MonoBehaviour
    {
        private CaptureRequest? currentRequest;
        private string statusPath = string.Empty;
        private CapturePhase phase;
        private bool requestDispatched;
        private int requestId;

        /// <summary>Gets the stable command-line name of this scenario.</summary>
        public abstract string ScenarioName { get; }

        /// <summary>Whether capture-only diagnostics were requested.</summary>
        protected bool ShowCaptureOverlay { get; private set; }

        /// <summary>Begins the selected scenario after command-line setup.</summary>
        protected abstract void BeginCapture();

        /// <summary>Requests pointer input at a top-left-origin normalized position.</summary>
        protected void RequestPointerInput(
            IEnumerable<string> assertions,
            CapturePointerAction action,
            Vector2 normalizedPosition
        )
        {
            if (
                !float.IsFinite(normalizedPosition.x)
                || !float.IsFinite(normalizedPosition.y)
                || normalizedPosition.x < 0
                || normalizedPosition.x > 1
                || normalizedPosition.y < 0
                || normalizedPosition.y > 1
            )
            {
                throw new ArgumentOutOfRangeException(
                    nameof(normalizedPosition),
                    "Capture pointer coordinates must be finite normalized values."
                );
            }

            Request(
                assertions,
                new CaptureRequest(
                    ++requestId,
                    "pointer",
                    action switch
                    {
                        CapturePointerAction.Move => "pointer-move",
                        CapturePointerAction.LeftButtonDown => "pointer-left-button-down",
                        CapturePointerAction.LeftButtonUp => "pointer-left-button-up",
                        _ => throw new ArgumentOutOfRangeException(nameof(action)),
                    },
                    normalizedPosition,
                    string.Empty
                )
            );
        }

        /// <summary>Requests one virtual keyboard transition.</summary>
        protected void RequestKeyInput(
            IEnumerable<string> assertions,
            CaptureKeyAction action,
            Key key
        )
        {
            if (key == Key.None)
            {
                throw new ArgumentOutOfRangeException(nameof(key));
            }

            Request(
                assertions,
                new CaptureRequest(
                    ++requestId,
                    "keyboard",
                    action switch
                    {
                        CaptureKeyAction.KeyDown => "key-down",
                        CaptureKeyAction.KeyUp => "key-up",
                        _ => throw new ArgumentOutOfRangeException(nameof(action)),
                    },
                    new Vector2(-1, -1),
                    key.ToString()
                )
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

            BattlementCaptureController.RequireBalancedInput();
            phase = CapturePhase.Passed;
            Publish("passed", NormalizeAssertions(assertions), null);
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
                currentRequest,
                string.IsNullOrWhiteSpace(failure) ? "Unknown scenario failure." : failure
            );
        }

        internal void DispatchRequest(int publishedRequestId)
        {
            if (phase != CapturePhase.Ready || currentRequest?.RequestId != publishedRequestId)
            {
                throw new InvalidOperationException(
                    $"Capture input request {publishedRequestId} is not the current request."
                );
            }

            requestDispatched = true;
            BattlementCaptureController.Dispatch(currentRequest);
        }

        protected void Start()
        {
            string? selectedScenario = CaptureArguments.Value("-battlementCaptureScenario");
            if (!string.Equals(selectedScenario, ScenarioName, StringComparison.Ordinal))
            {
                enabled = false;
                return;
            }

            statusPath = CaptureArguments.Value("-battlementCaptureStatus") ?? string.Empty;
            ShowCaptureOverlay = CaptureArguments.Has("-battlementCaptureOverlay");
            try
            {
                BattlementCaptureController.Attach(this);
                BeginCapture();
            }
            catch (Exception exception)
            {
                SignalFailed(exception.Message);
            }
        }

        private void Request(IEnumerable<string> assertions, CaptureRequest request)
        {
            if (phase == CapturePhase.Passed || phase == CapturePhase.Failed)
            {
                throw new InvalidOperationException("A completed scenario cannot request input.");
            }
            if (phase == CapturePhase.Ready && !requestDispatched)
            {
                throw new InvalidOperationException(
                    "A capture scenario cannot replace an undispatched input request."
                );
            }

            phase = CapturePhase.Ready;
            currentRequest = request;
            requestDispatched = false;
            Publish("ready", NormalizeAssertions(assertions), request);
        }

        private void Publish(
            string status,
            string[] assertions,
            CaptureRequest? request,
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
            CaptureFiles.WriteJson(
                statusPath,
                new CaptureStatus(status, ScenarioName, assertions, request, failure)
            );
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
                CaptureRequest? request,
                string? failure
            )
            {
                this.phase = phase;
                this.scenario = scenario;
                this.assertions = assertions;
                requestId = request?.RequestId ?? 0;
                inputDevice = request?.Device ?? string.Empty;
                input = request?.Action ?? string.Empty;
                pointerX = request?.Position.x ?? -1;
                pointerY = request?.Position.y ?? -1;
                key = request?.Key ?? string.Empty;
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
            private string inputDevice;

            [SerializeField]
            private string input;

            [SerializeField]
            private float pointerX;

            [SerializeField]
            private float pointerY;

            [SerializeField]
            private string key;

            [SerializeField]
            private string? failure;
        }
    }
}

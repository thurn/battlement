#nullable enable

using System;
using System.Collections.Generic;

namespace Battlement
{
    internal delegate void DittoFailureFrameCapture(
        ulong committedFrame,
        Action<DittoNativeCaptureResult> completion
    );

    internal sealed class DittoScenarioContext : IDisposable
    {
        private readonly List<DittoReachedArtifact> artifacts = new();
        private readonly List<string> observedErrorRefs = new();
        private readonly DittoJob job;
        private readonly DittoResolvedScenario scenario;
        private readonly DittoLogDelivery delivery;
        private readonly DittoFunctionalErrorGate errorGate;
        private readonly Func<DittoErrorCode, string, string> allocateError;
        private string? engineSessionId;
        private uint? currentStepIndex;
        private uint? failureStepIndex;
        private DittoPlayerFailureFrame? failureFrame;
        private bool begun;
        private bool engineEnded;

        public DittoScenarioContext(
            DittoJob job,
            DittoResolvedScenario scenario,
            DittoLogDelivery delivery,
            BattlementLogObserver errorObserver,
            Func<DittoErrorCode, string, string> errorAllocator
        )
        {
            this.job = job ?? throw new ArgumentNullException(nameof(job));
            this.scenario = scenario ?? throw new ArgumentNullException(nameof(scenario));
            this.delivery = delivery ?? throw new ArgumentNullException(nameof(delivery));
            errorGate = new DittoFunctionalErrorGate(errorObserver);
            allocateError =
                errorAllocator ?? throw new ArgumentNullException(nameof(errorAllocator));
        }

        public IReadOnlyList<string> ObservedErrorRefs => observedErrorRefs;

        public void Begin()
        {
            if (begun)
            {
                throw new InvalidOperationException("The scenario context is already open.");
            }
            begun = true;
            delivery.EmitContext(new DittoContext.ScenarioStarted(scenario.Id), "scenario started");
            errorGate.Open();
        }

        public void EngineStarted(string engineId)
        {
            RequireBegun();
            DittoLifecycleValidation.Identifier("engine_session_id", engineId);
            if (engineSessionId is not null)
            {
                throw new InvalidOperationException("The scenario engine is already started.");
            }
            engineSessionId = engineId;
            delivery.EmitContext(
                new DittoContext.EngineStarted(engineId, scenario.Id),
                "engine started"
            );
        }

        public void StepStarted(DittoResolvedStep step)
        {
            RequireBegun();
            currentStepIndex = step.Index;
            delivery.EmitContext(
                new DittoContext.StepStarted(scenario.Id, step.Index),
                "step started"
            );
        }

        public void StepEnded(DittoPlayerStepResult result, Action<bool> completion) =>
            delivery.EmitContext(
                new DittoContext.StepEnded(scenario.Id, result),
                "step ended",
                succeeded =>
                {
                    currentStepIndex = null;
                    completion(succeeded);
                }
            );

        public string ReportFunctionalError(DittoErrorCode code, string message) =>
            Observe(code, DittoErrorSource.DittoPlayer, null, null, message);

        public string? PollFailure()
        {
            DittoDetectedFailure? failure = errorGate.Poll();
            return failure is null
                ? null
                : Observe(
                    failure.Code,
                    failure.Source,
                    failure.RecordSequence,
                    failure.BattlementErrorId,
                    failure.Message
                );
        }

        public void CloseForBoundary() => errorGate.Close();

        public void EngineEnded(DittoExecutionStatus status)
        {
            if (engineSessionId is null || engineEnded)
            {
                return;
            }
            engineEnded = true;
            delivery.EmitContext(
                new DittoContext.EngineEnded(engineSessionId, status),
                "engine ended"
            );
        }

        public void AddArtifact(DittoReachedArtifact artifact) => artifacts.Add(artifact);

        public void CaptureFailureFrame(
            ulong committedFrame,
            DittoFailureFrameCapture capture,
            Action<bool> completion
        )
        {
            if (committedFrame == 0)
            {
                failureFrame = new DittoPlayerFailureFrame.Unavailable(
                    "No committed framebuffer was available.",
                    null
                );
                completion(true);
                return;
            }
            capture(
                committedFrame,
                result =>
                {
                    if (result is DittoNativeCaptureResult.Unavailable unavailable)
                    {
                        string errorRef = Observe(
                            unavailable.Failure.Code,
                            DittoErrorSource.DittoPlayer,
                            null,
                            null,
                            unavailable.Failure.Reason
                        );
                        failureFrame = new DittoPlayerFailureFrame.Unavailable(
                            unavailable.Failure.Reason,
                            errorRef
                        );
                        completion(true);
                        return;
                    }
                    var captured = (DittoNativeCaptureResult.Captured)result;
                    string artifactId = Guid.NewGuid().ToString("D");
                    delivery.UploadArtifact(
                        new DittoPngArtifact(
                            scenario.Id,
                            failureStepIndex,
                            artifactId,
                            new DittoArtifactKind.FailureFrame(),
                            captured.Width,
                            captured.Height,
                            captured.Png
                        ),
                        succeeded =>
                        {
                            if (succeeded)
                            {
                                failureFrame = new DittoPlayerFailureFrame.Captured(artifactId);
                                artifacts.Add(
                                    new DittoReachedArtifact(
                                        artifactId,
                                        failureStepIndex,
                                        new DittoArtifactKind.FailureFrame()
                                    )
                                );
                            }
                            completion(succeeded);
                        }
                    );
                }
            );
        }

        public void AcceptUploadedFailureFrame(string artifactId, Action<bool> completion)
        {
            var kind = new DittoArtifactKind.FailureFrame();
            delivery.ConfirmUploadedArtifact(
                scenario.Id,
                failureStepIndex,
                artifactId,
                kind,
                succeeded =>
                {
                    if (succeeded)
                    {
                        failureFrame = new DittoPlayerFailureFrame.Captured(artifactId);
                        artifacts.Add(new DittoReachedArtifact(artifactId, failureStepIndex, kind));
                    }
                    completion(succeeded);
                }
            );
        }

        public void RecordUnavailableFailureFrame(string reason, string? errorRef) =>
            failureFrame = new DittoPlayerFailureFrame.Unavailable(reason, errorRef);

        public void Complete(
            DittoScenarioExecution execution,
            DittoPlayerResetFailure? resetFailure,
            ulong boundaryDurationMs,
            IReadOnlyList<DittoNativeVideoInput> videoInputs,
            Action<DittoScenarioComplete?> completion
        )
        {
            CloseForBoundary();
            DittoScenarioBoundary boundary = resetFailure is null
                ? new DittoScenarioBoundary.Passed(boundaryDurationMs)
                : FailedBoundary(resetFailure, boundaryDurationMs);
            if (engineSessionId is not null && !engineEnded)
            {
                throw new InvalidOperationException("The engine end context is missing.");
            }
            var ended = new DittoContext.ScenarioEnded(
                scenario.Id,
                execution.Status,
                failureFrame,
                videoInputs,
                execution.ExecutionDurationMs,
                execution.StartupDurationMs,
                execution.SettleDurationMs,
                execution.CaptureDurationMs,
                boundary,
                execution.PrimaryErrorRef
            );
            delivery.EmitContext(
                ended,
                "scenario ended",
                succeeded =>
                {
                    if (!succeeded)
                    {
                        completion(null);
                        return;
                    }
                    var result = new DittoScenarioComplete(
                        scenario.Id,
                        execution.Status,
                        execution.Steps,
                        artifacts.ToArray(),
                        failureFrame,
                        videoInputs,
                        delivery.LastLogSequence!.Value,
                        execution.ExecutionDurationMs,
                        execution.StartupDurationMs,
                        execution.SettleDurationMs,
                        execution.CaptureDurationMs,
                        boundary,
                        execution.PrimaryErrorRef
                    );
                    DittoCompletionValidation.ValidateScenarioComplete(
                        result,
                        job,
                        observedErrorRefs
                    );
                    completion(result);
                }
            );
        }

        public void Dispose() => errorGate.Dispose();

        private string Observe(
            DittoErrorCode code,
            DittoErrorSource source,
            ulong? recordSequence,
            string? battlementErrorId,
            string message
        )
        {
            string errorRef = allocateError(code, message);
            DittoLifecycleValidation.PlayerErrorRef(errorRef);
            observedErrorRefs.Add(errorRef);
            failureStepIndex ??= currentStepIndex;
            delivery.EmitContext(
                new DittoContext.ErrorObserved(
                    scenario.Id,
                    currentStepIndex,
                    errorRef,
                    code,
                    source,
                    recordSequence,
                    battlementErrorId
                ),
                "error observed"
            );
            return errorRef;
        }

        private DittoScenarioBoundary FailedBoundary(
            DittoPlayerResetFailure failure,
            ulong durationMs
        )
        {
            DittoErrorCode code =
                failure.Stage == DittoBoundaryStage.Destroy
                    ? DittoErrorCode.RuntimeDestroyFailed
                    : DittoErrorCode.RuntimeResetFailed;
            string errorRef = Observe(
                code,
                DittoErrorSource.DittoPlayer,
                null,
                null,
                failure.Diagnostic
            );
            return new DittoScenarioBoundary.Failed(durationMs, failure.Stage, errorRef);
        }

        private void RequireBegun()
        {
            if (!begun)
            {
                throw new InvalidOperationException("The scenario context is not open.");
            }
        }
    }
}

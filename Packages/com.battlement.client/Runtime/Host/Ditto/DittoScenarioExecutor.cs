#nullable enable

using System;
using System.Collections.Generic;

namespace Battlement
{
    internal delegate void DittoScreenshotCapture(
        DittoResolvedStep step,
        DittoRenderCommit renderCommit,
        System.Action<DittoScreenshotStepOutcome> completion
    );

    internal delegate void DittoStepBoundary(
        DittoPlayerStepResult result,
        System.Action<bool> completion
    );

    internal sealed record DittoScreenshotStepOutcome(
        string? ArtifactId,
        string? ErrorRef,
        bool ContinueScenario,
        DittoStepStatus FailureStatus = DittoStepStatus.Failed
    );

    internal sealed record DittoScenarioExecution(
        DittoExecutionStatus Status,
        IReadOnlyList<DittoPlayerStepResult> Steps,
        ulong StartupDurationMs,
        ulong ExecutionDurationMs,
        ulong SettleDurationMs,
        ulong CaptureDurationMs,
        DittoDeadlineKind? ExpiredDeadline,
        string? PrimaryErrorRef
    );

    internal sealed class DittoScenarioExecutor : IDisposable
    {
        private const int AcceleratedFrameBurst = 256;

        private enum Phase
        {
            None,
            StartupSettle,
            ProfileIdle,
            PointerPressPresentation,
            ActionPresentation,
            Settle,
            ScreenshotSettle,
            ScreenshotCapture,
            FrameAdvance,
            ObjectWait,
        }

        private readonly BattlementRunner runner;
        private readonly DittoResolvedScenario scenario;
        private readonly DittoMotionController motion;
        private readonly DittoInputTargets targets;
        private readonly Func<TimeSpan> now;
        private readonly DittoScreenshotCapture capture;
        private readonly Func<DittoErrorCode, string, string> reportError;
        private readonly Func<string?> pollFailure;
        private readonly Action<DittoResolvedStep> onStepStarted;
        private readonly DittoStepBoundary onStepEnded;
        private readonly DittoNativeVideoRecorder? videoRecorder;
        private readonly Func<DittoRenderCommit, byte[]>? captureVideoFrame;
        private readonly DittoCapturePixelLayout? videoLayout;
        private readonly System.Action setup;
        private readonly ulong runTimeoutMs;
        private readonly List<DittoPlayerStepResult> results = new();
        private TimeSpan scenarioStarted;
        private TimeSpan executionStarted;
        private TimeSpan stepStarted;
        private TimeSpan phaseStarted;
        private Phase phase;
        private ulong settleDurationMs;
        private ulong captureDurationMs;
        private uint advanceFrames;
        private DittoObjectCondition? waitCondition;
        private DittoScreenshotStepOutcome? screenshotOutcome;
        private DittoDeadlineKind? scenarioExpiry;
        private string? primaryErrorRef;
        private int nextStep;
        private bool started;
        private bool complete;
        private bool disposed;
        private bool presentationReady;
        private ulong committedFrame;
        private ulong fallbackRenderGeneration;
        private DittoRenderCommit? renderCommit;
        private bool stepActive;
        private bool boundaryPending;
        private bool? boundarySucceeded;
        private bool completeAfterBoundary;
        private bool videoMotionOverridden;
        private bool presentationChanged;
        private bool preserveAdvanceState;
        private int startupQuietFrames;
        private bool awaitingPresentation;
        private string? activationTransactionId;
        private DittoStepPerformanceRecorder? performanceRecorder;
        private uint profileIdleFrames;
        private ObjectId? pointerClickTarget;

        public DittoScenarioExecutor(
            BattlementRunner runner,
            DittoResolvedScenario scenario,
            DittoPlatform platform,
            uint width,
            uint height,
            IReadOnlyDictionary<string, ObjectId> aliases,
            ulong remainingRunTimeoutMs,
            Func<TimeSpan> currentTime,
            Func<DittoResolvedStep, DittoScreenshotStepOutcome> captureScreenshot,
            Func<DittoErrorCode, string, string> errorReporter,
            System.Action? setupScenario = null,
            Func<string?>? observeFailure = null,
            Action<DittoResolvedStep>? stepStarted = null,
            DittoStepBoundary? stepEnded = null,
            DittoNativeVideoRecorder? video = null,
            Func<DittoRenderCommit, byte[]>? videoFrame = null,
            DittoCapturePixelLayout? nativeVideoLayout = null
        )
            : this(
                runner,
                scenario,
                platform,
                width,
                height,
                aliases,
                remainingRunTimeoutMs,
                currentTime,
                Wrap(captureScreenshot),
                errorReporter,
                setupScenario,
                observeFailure,
                stepStarted,
                stepEnded,
                video,
                videoFrame,
                nativeVideoLayout
            ) { }

        public DittoScenarioExecutor(
            BattlementRunner runner,
            DittoResolvedScenario scenario,
            DittoPlatform platform,
            uint width,
            uint height,
            IReadOnlyDictionary<string, ObjectId> aliases,
            ulong remainingRunTimeoutMs,
            Func<TimeSpan> currentTime,
            DittoScreenshotCapture captureScreenshot,
            Func<DittoErrorCode, string, string> errorReporter,
            System.Action? setupScenario = null,
            Func<string?>? observeFailure = null,
            Action<DittoResolvedStep>? stepStarted = null,
            DittoStepBoundary? stepEnded = null,
            DittoNativeVideoRecorder? video = null,
            Func<DittoRenderCommit, byte[]>? videoFrame = null,
            DittoCapturePixelLayout? nativeVideoLayout = null
        )
        {
            if (runner == null)
            {
                throw new ArgumentNullException(nameof(runner));
            }
            this.runner = runner;
            this.scenario = scenario ?? throw new ArgumentNullException(nameof(scenario));
            now = currentTime ?? throw new ArgumentNullException(nameof(currentTime));
            capture =
                captureScreenshot ?? throw new ArgumentNullException(nameof(captureScreenshot));
            reportError = errorReporter ?? throw new ArgumentNullException(nameof(errorReporter));
            setup = setupScenario ?? (() => { });
            pollFailure = observeFailure ?? (() => null);
            onStepStarted = stepStarted ?? (_ => { });
            onStepEnded = stepEnded ?? ((_, completion) => completion(true));
            videoRecorder = video;
            captureVideoFrame = videoFrame;
            videoLayout = nativeVideoLayout;
            runTimeoutMs = remainingRunTimeoutMs;
            motion = new DittoMotionController(runner);
            _ = platform;
            targets = new DittoInputTargets(runner, aliases, width, height);
            runner.BeginDittoInput();
        }

        public DittoScenarioExecution? Result { get; private set; }

        public uint? CurrentStepIndex => stepActive ? scenario.Steps[nextStep].Index : null;

        public ulong LastCommittedFrame => committedFrame;

        public DittoRenderCommit? LastRenderCommit => renderCommit;

        public ulong NextPresentedFrame => motion.NextFrameIndex;

        public bool AwaitingPresentation => awaitingPresentation;

        public bool RequiresPaintObservation => phase != Phase.ObjectWait;

        public bool Advance()
        {
            ThrowIfDisposed();
            for (var index = 0; index < AcceleratedFrameBurst; index++)
            {
                ulong priorFrame = committedFrame;
                int priorStep = nextStep;
                presentationChanged = false;
                if (AdvanceOnce())
                {
                    return true;
                }
                if (
                    motion.Motion == DittoMotion.RealTime
                    || phase is not Phase.FrameAdvance and not Phase.ObjectWait
                )
                {
                    return false;
                }
                if (committedFrame == priorFrame || nextStep != priorStep || presentationChanged)
                {
                    return false;
                }
            }
            return false;
        }

        private bool AdvanceOnce()
        {
            if (complete)
            {
                return true;
            }
            if (awaitingPresentation)
            {
                return false;
            }
            if (AdvanceBoundary())
            {
                return complete;
            }
            if (!started)
            {
                Begin();
            }

            while (!complete)
            {
                if (TryFreezeObserved())
                {
                    return complete;
                }
                if (AdvanceBoundary())
                {
                    return complete;
                }
                if (phase != Phase.None)
                {
                    if (phase == Phase.ScreenshotCapture)
                    {
                        AdvanceScreenshotCapture();
                        if (phase == Phase.None)
                        {
                            continue;
                        }
                        return complete;
                    }
                    PrepareFrame();
                    AdvanceBoundary();
                    return complete;
                }
                if (nextStep == scenario.Steps.Count)
                {
                    Complete();
                    return true;
                }
                if (TryExpireBeforeStep())
                {
                    return true;
                }
                StartStep(scenario.Steps[nextStep]);
            }
            return true;
        }

        public void Dispose()
        {
            if (disposed)
            {
                return;
            }
            disposed = true;
            if (videoRecorder?.IsActive == true)
            {
                videoRecorder.TruncateForRuntimeFailure();
            }
            runner.EndDittoInput();
        }

        public void Freeze(string errorRef)
        {
            ThrowIfDisposed();
            if (complete)
            {
                return;
            }
            FinalizeVideoFailure();
            if (!started)
            {
                started = true;
                scenarioStarted = now();
                executionStarted = scenarioStarted;
            }
            if (stepActive)
            {
                FinishStep(
                    scenario.Steps[nextStep],
                    DittoStepStatus.Failed,
                    null,
                    errorRef,
                    null,
                    null
                );
                return;
            }
            primaryErrorRef ??= errorRef;
            AddNotRunSteps();
            Complete();
        }

        private void Begin()
        {
            started = true;
            scenarioStarted = now();
            motion.Begin(scenario.Motion);
            if (scenario.Performance is not null && !UnityEngine.Application.isFocused)
                throw new InvalidOperationException("Performance run lost application focus.");
            setup();
            phase = Phase.StartupSettle;
            phaseStarted = now();
            executionStarted = phaseStarted;
            DittoDeadlineKind? expired = Expired(null);
            if (expired.HasValue)
            {
                scenarioExpiry = expired;
                FailRemaining(expired.Value, "Scenario setup exceeded its deadline.");
            }
        }

        private void StartStep(DittoResolvedStep step)
        {
            stepStarted = now();
            stepActive = true;
            if (step.Action is not DittoStepAction.Screenshot)
            {
                preserveAdvanceState = false;
            }
            onStepStarted(step);
            switch (step.Action)
            {
                case DittoStepAction.Click click:
                    presentationReady = false;
                    if (TryResolve(click.Target, step, out DittoInputResolution? resolution))
                    {
                        activationTransactionId = $"{scenario.Id}:{step.Index}";
                        if (
                            targets.Activate(
                                resolution!,
                                activationTransactionId,
                                committedFrame,
                                out string? activationDiagnostic
                            )
                        )
                        {
                            phase = Phase.ActionPresentation;
                            phaseStarted = now();
                        }
                        else
                        {
                            activationTransactionId = null;
                            FailStep(step, DittoErrorCode.InputUnreachable, activationDiagnostic!);
                        }
                    }
                    break;
                case DittoStepAction.Hover:
                case DittoStepAction.Drag:
                    FailStep(
                        step,
                        DittoErrorCode.InputUnreachable,
                        "Hover and drag have no deterministic semantic delivery contract."
                    );
                    break;
                case DittoStepAction.Key:
                    FailStep(
                        step,
                        DittoErrorCode.InputUnreachable,
                        "Physical key input has no deterministic semantic delivery contract."
                    );
                    break;
                case DittoStepAction.Advance advance:
                    advanceFrames = advance.Frames;
                    phase = Phase.FrameAdvance;
                    break;
                case DittoStepAction.Wait wait:
                    waitCondition = wait.Condition;
                    phase = Phase.ObjectWait;
                    EvaluateObjectWait(step);
                    break;
                case DittoStepAction.Assert assertion:
                    Assert(step, assertion.Condition);
                    break;
                case DittoStepAction.AccessibilityAssert assertion:
                    AccessibilityAssert(step, assertion.Value);
                    break;
                case DittoStepAction.AccessibilityAction action:
                    presentationReady = false;
                    activationTransactionId = $"{scenario.Id}:{step.Index}";
                    if (
                        targets.AccessibilityAction(
                            action.Target,
                            action.Action,
                            activationTransactionId,
                            committedFrame,
                            out string? diagnostic
                        )
                    )
                    {
                        phase = Phase.ActionPresentation;
                        phaseStarted = now();
                    }
                    else
                    {
                        activationTransactionId = null;
                        FailStep(step, DittoErrorCode.InputUnreachable, diagnostic!);
                    }
                    break;
                case DittoStepAction.PointerAction action:
                    presentationReady = false;
                    if (step.Measure && scenario.Performance is not null)
                        performanceRecorder = new DittoStepPerformanceRecorder(
                            scenario.Performance.TargetFps
                        );
                    bool pointerDispatched = action.Action switch
                    {
                        DittoPointerAction.Click => targets.BeginPointerClick(
                            action.Target,
                            out pointerClickTarget,
                            out _
                        ),
                        DittoPointerAction.Hover => targets.Hover(action.Target, out _),
                        _ => false,
                    };
                    if (pointerDispatched)
                    {
                        phase =
                            action.Action == DittoPointerAction.Click
                                ? Phase.PointerPressPresentation
                                : Phase.ActionPresentation;
                        phaseStarted = now();
                    }
                    else
                    {
                        performanceRecorder = null;
                        FailStep(
                            step,
                            DittoErrorCode.InputUnreachable,
                            $"Pointer {action.Action} could not be delivered to "
                                + $"{action.Target.Name}."
                        );
                    }
                    break;
                case DittoStepAction.Screenshot:
                    if (presentationReady)
                    {
                        Capture(step);
                    }
                    else
                    {
                        phase = Phase.ScreenshotSettle;
                        phaseStarted = now();
                    }
                    break;
                case DittoStepAction.Video { Value: DittoVideo.Start start }:
                    StartVideo(step, start);
                    break;
                case DittoStepAction.Video { Value: DittoVideo.Stop }:
                    StopVideo(step);
                    break;
                default:
                    throw new InvalidOperationException("Unknown Ditto step action.");
            }
        }

        private void PrepareFrame()
        {
            motion.PrepareFrame(
                phase == Phase.FrameAdvance,
                preserveAdvanceState && phase == Phase.ScreenshotSettle
            );
            runner.RunFrame();
            runner.CompleteNativeFrame();
            awaitingPresentation = true;
        }

        public void CompletePresentedFrame()
        {
            CompletePresentedFrame(
                new DittoRenderCommit(NextPresentedFrame, checked(++fallbackRenderGeneration), 0)
            );
        }

        public void CompletePresentedFrame(DittoRenderCommit commit)
        {
            ThrowIfDisposed();
            if (!awaitingPresentation)
            {
                throw new InvalidOperationException("No Ditto frame is awaiting presentation.");
            }
            awaitingPresentation = false;
            DittoCommittedFrame frame = motion.ObserveCommittedFrame(commit.PixelFingerprint);
            if (scenario.Performance is not null && !UnityEngine.Application.isFocused)
                throw new InvalidOperationException("Performance run lost application focus.");
            if (commit.Frame != frame.Index)
            {
                throw new InvalidOperationException(
                    $"Render commit frame {commit.Frame} does not match "
                        + $"presented frame {frame.Index}."
                );
            }
            if (
                renderCommit is not null
                && commit.RenderGeneration <= renderCommit.RenderGeneration
            )
            {
                throw new InvalidOperationException("Render commit generations must increase.");
            }
            committedFrame = frame.Index;
            renderCommit = commit;
            presentationChanged = frame.LayoutChanged || frame.PaintChanged;
            performanceRecorder?.Presented(frame.LayoutChanged || frame.PaintChanged);
            CaptureVideoFrame(frame, commit);
            if (TryFreezeObserved())
            {
                return;
            }
            if (phase == Phase.StartupSettle)
            {
                if (Expired(null).HasValue)
                {
                    scenarioExpiry = Expired(null);
                    FailRemaining(
                        scenarioExpiry!.Value,
                        $"Scenario setup exceeded its deadline ({motion.PendingDiagnostic()})."
                    );
                    return;
                }
                startupQuietFrames = frame.IsSettled ? startupQuietFrames + 1 : 0;
                if (startupQuietFrames == 2)
                {
                    settleDurationMs += PhaseDuration();
                    presentationReady = true;
                    executionStarted = now();
                    profileIdleFrames = scenario.Performance?.IdleFrames ?? 0;
                    phase = profileIdleFrames == 0 ? Phase.None : Phase.ProfileIdle;
                }
                return;
            }
            if (phase == Phase.ProfileIdle)
            {
                profileIdleFrames--;
                if (profileIdleFrames == 0)
                    phase = Phase.None;
                return;
            }
            DittoResolvedStep step = scenario.Steps[nextStep];
            if (TryExpireStep(step))
            {
                return;
            }

            switch (phase)
            {
                case Phase.PointerPressPresentation:
                    string? pointerDiagnostic = null;
                    if (
                        pointerClickTarget is not ObjectId target
                        || !targets.FinishPointerClick(target, out pointerDiagnostic)
                    )
                    {
                        pointerClickTarget = null;
                        FailInfrastructureStep(
                            step,
                            DittoErrorCode.InputUnreachable,
                            pointerDiagnostic ?? "Pointer target disappeared before release."
                        );
                        return;
                    }
                    pointerClickTarget = null;
                    phase = Phase.ActionPresentation;
                    phaseStarted = now();
                    break;
                case Phase.ActionPresentation:
                    if (activationTransactionId is string transactionId)
                    {
                        if (
                            !runner.ValidateDittoActivationDelivery(
                                transactionId,
                                out string? deliveryDiagnostic
                            )
                        )
                        {
                            activationTransactionId = null;
                            runner.CancelDittoActivationTransaction();
                            FailInfrastructureStep(
                                step,
                                DittoErrorCode.InputUnreachable,
                                deliveryDiagnostic!
                            );
                            return;
                        }
                        activationTransactionId = null;
                        if (
                            !runner.CompleteDittoActivationTransaction(
                                transactionId,
                                frame.Index,
                                out DittoActivationReceipt? receipt,
                                out string? diagnostic
                            )
                        )
                        {
                            FailInfrastructureStep(
                                step,
                                DittoErrorCode.InputUnreachable,
                                diagnostic!
                            );
                            return;
                        }
                        UnityEngine.Debug.Log(
                            "[Battlement/Ditto] activation-receipt "
                                + $"transaction={receipt!.TransactionId} "
                                + $"target={receipt.Target.Value} route={receipt.Route} "
                                + $"action={receipt.Action.Value} "
                                + $"resolved-frame={receipt.ResolvedFrame} "
                                + $"presented-frame={receipt.PresentedFrame}"
                        );
                        if (!NextStepIsAdvance() && !frame.IsSettled)
                        {
                            phase = Phase.Settle;
                            phaseStarted = now();
                            motion.RestartQuietWindow();
                            return;
                        }
                    }
                    if (activationTransactionId is null && !NextStepIsAdvance() && !frame.IsSettled)
                    {
                        phase = Phase.Settle;
                        phaseStarted = now();
                        motion.RestartQuietWindow();
                        return;
                    }
                    if (NextStepIsAdvance())
                    {
                        PassStep(step);
                    }
                    else
                    {
                        settleDurationMs += PhaseDuration();
                        presentationReady = true;
                        PassStep(step);
                    }
                    break;
                case Phase.Settle when frame.IsSettled:
                    settleDurationMs += PhaseDuration();
                    presentationReady = true;
                    PassStep(step);
                    break;
                case Phase.ScreenshotSettle when frame.IsSettled:
                    settleDurationMs += PhaseDuration();
                    presentationReady = true;
                    preserveAdvanceState = false;
                    phase = Phase.None;
                    Capture(step);
                    break;
                case Phase.FrameAdvance:
                    advanceFrames--;
                    if (advanceFrames == 0)
                    {
                        motion.PreserveExactAdvanceState();
                        presentationReady = false;
                        preserveAdvanceState = true;
                        PassStep(step);
                    }
                    break;
                case Phase.ObjectWait:
                    EvaluateObjectWait(step);
                    break;
                case Phase.StartupSettle:
                case Phase.ProfileIdle:
                case Phase.Settle:
                case Phase.ScreenshotSettle:
                    break;
                case Phase.ScreenshotCapture:
                case Phase.None:
                default:
                    throw new InvalidOperationException("No frame-driven step is active.");
            }
        }

        private void EvaluateObjectWait(DittoResolvedStep step)
        {
            DittoConditionResult condition = targets.Evaluate(waitCondition!);
            if (!condition.IsSupported)
            {
                FailStep(
                    step,
                    DittoErrorCode.ConditionUnsupported,
                    condition.Diagnostic ?? "The object condition is unsupported."
                );
            }
            else if (condition.Matches)
            {
                motion.RestartQuietWindow();
                phase = Phase.Settle;
                phaseStarted = now();
            }
        }

        private bool NextStepIsAdvance() =>
            nextStep + 1 < scenario.Steps.Count
            && scenario.Steps[nextStep + 1].Action is DittoStepAction.Advance;

        private void Assert(DittoResolvedStep step, DittoObjectCondition condition)
        {
            DittoConditionResult observed = targets.Evaluate(condition);
            var assertion = new DittoAssertionResult(
                condition.Object,
                condition.State,
                true,
                observed.Matches,
                observed.Matches
            );
            if (!observed.IsSupported)
            {
                FailStep(
                    step,
                    DittoErrorCode.ConditionUnsupported,
                    observed.Diagnostic ?? "The object condition is unsupported.",
                    assertion
                );
            }
            else if (!observed.Matches)
            {
                FailStep(step, DittoErrorCode.AssertionFailed, "Assertion failed.", assertion);
            }
            else
            {
                FinishStep(step, DittoStepStatus.Passed, null, null, assertion, null);
            }
        }

        private void AccessibilityAssert(
            DittoResolvedStep step,
            DittoAccessibilityAssertion assertion
        )
        {
            DittoConditionResult observed = targets.Evaluate(assertion);
            if (!observed.Matches)
            {
                FailStep(
                    step,
                    DittoErrorCode.AssertionFailed,
                    observed.Diagnostic ?? "Accessibility assertion failed."
                );
            }
            else
            {
                PassStep(step);
            }
        }

        private void Capture(DittoResolvedStep step)
        {
            phase = Phase.ScreenshotCapture;
            phaseStarted = now();
            capture(step, renderCommit!, outcome => screenshotOutcome = outcome);
            if (screenshotOutcome is not null)
            {
                AdvanceScreenshotCapture();
            }
        }

        private void AdvanceScreenshotCapture()
        {
            DittoResolvedStep step = scenario.Steps[nextStep];
            if (screenshotOutcome is null)
            {
                TryExpireStep(step);
                return;
            }
            DittoScreenshotStepOutcome outcome = screenshotOutcome;
            captureDurationMs += PhaseDuration();
            screenshotOutcome = null;
            if (outcome.ErrorRef is null)
            {
                FinishStep(step, DittoStepStatus.Passed, null, null, null, outcome.ArtifactId);
                return;
            }
            FinishStep(
                step,
                outcome.FailureStatus,
                null,
                outcome.ErrorRef,
                null,
                outcome.ArtifactId,
                outcome.ContinueScenario
            );
        }

        private bool TryResolve(
            DittoInputTarget target,
            DittoResolvedStep step,
            out DittoInputResolution? resolved
        )
        {
            DittoInputResolution resolution = targets.Resolve(target);
            resolved = resolution;
            UnityEngine.Debug.Log(
                $"[Battlement/Ditto-trace] target-resolved reachable={resolution.IsReachable} "
                    + $"object={resolution.ObjectId?.Value.ToString() ?? "coordinates"} "
                    + $"position={resolution.Position} "
                    + $"bounds={resolution.Bounds?.ToString() ?? "none"} "
                    + $"candidates={resolution.Candidates.Count} committed={committedFrame}"
            );
            if (resolution.IsReachable && resolution.ObjectId is not null)
            {
                return true;
            }
            string diagnostic = resolution.ObjectId is ObjectId id
                ? $"Input target {id.Value} is unreachable."
                : "Coordinate input has no deterministic semantic delivery contract.";
            FailStep(step, DittoErrorCode.InputUnreachable, diagnostic);
            resolved = null;
            return false;
        }

        private bool TryExpireBeforeStep()
        {
            DittoDeadlineKind? expired = Expired(null);
            if (!expired.HasValue)
            {
                return false;
            }
            scenarioExpiry = expired;
            FailRemaining(expired.Value, "Scenario execution exceeded its deadline.");
            return true;
        }

        private bool TryExpireStep(DittoResolvedStep step)
        {
            DittoDeadlineKind? expired = Expired(step);
            if (!expired.HasValue)
            {
                return false;
            }
            if (expired != DittoDeadlineKind.Step)
            {
                scenarioExpiry = expired;
            }
            FailInfrastructureStep(
                step,
                DittoErrorCode.DeadlineExpired,
                $"The {expired.Value.ToString().ToLowerInvariant()} deadline expired "
                    + $"({motion.PendingDiagnostic()}).",
                expired: expired
            );
            return true;
        }

        private DittoDeadlineKind? Expired(DittoResolvedStep? step)
        {
            TimeSpan current = now();
            TimeSpan runDeadline = scenarioStarted + TimeSpan.FromMilliseconds(runTimeoutMs);
            if (current >= runDeadline)
            {
                return DittoDeadlineKind.Run;
            }
            TimeSpan scenarioDeadline =
                scenarioStarted + TimeSpan.FromMilliseconds(scenario.TimeoutMs);
            if (current >= scenarioDeadline)
            {
                return DittoDeadlineKind.Scenario;
            }
            if (step is not null)
            {
                TimeSpan stepDeadline = stepStarted + TimeSpan.FromMilliseconds(step.TimeoutMs);
                if (current >= stepDeadline)
                {
                    return DittoDeadlineKind.Step;
                }
            }
            return null;
        }

        private void PassStep(DittoResolvedStep step) =>
            FinishStep(step, DittoStepStatus.Passed, null, null, null, null);

        private void StartVideo(DittoResolvedStep step, DittoVideo.Start start)
        {
            RequireVideoRecorder();
            string inputId = videoRecorder!.Begin(
                step.Index,
                start.MaxDurationMs,
                runner.DittoElapsed
            );
            motion.Begin(start.Motion);
            videoMotionOverridden = true;
            FinishStep(step, DittoStepStatus.Passed, null, null, null, null, videoInputId: inputId);
        }

        private void StopVideo(DittoResolvedStep step)
        {
            RequireVideoRecorder();
            videoRecorder!.Stop();
            RestoreScenarioMotion();
            PassStep(step);
        }

        private void CaptureVideoFrame(DittoCommittedFrame frame, DittoRenderCommit commit)
        {
            if (videoRecorder?.IsActive != true)
            {
                return;
            }
            if (captureVideoFrame is null || videoLayout is null)
            {
                throw new InvalidOperationException(
                    "Native video frame capture is not configured."
                );
            }
            if (videoRecorder.AppendFrame(captureVideoFrame(commit), videoLayout, frame.Elapsed))
            {
                RestoreScenarioMotion();
            }
        }

        private void FinalizeVideoFailure()
        {
            if (videoRecorder?.IsActive != true)
            {
                RestoreScenarioMotion();
                return;
            }
            if (!videoRecorder.TruncateForRuntimeFailure())
            {
                for (int index = results.Count - 1; index >= 0; index--)
                {
                    if (results[index].VideoInputId is null)
                    {
                        continue;
                    }
                    results[index] = results[index] with { VideoInputId = null };
                    break;
                }
            }
            RestoreScenarioMotion();
        }

        private void RestoreScenarioMotion()
        {
            if (!videoMotionOverridden)
            {
                return;
            }
            motion.Begin(scenario.Motion);
            videoMotionOverridden = false;
        }

        private void RequireVideoRecorder()
        {
            if (videoRecorder is null)
            {
                throw new InvalidOperationException("Native video recording is not configured.");
            }
        }

        private void FailStep(
            DittoResolvedStep step,
            DittoErrorCode code,
            string diagnostic,
            DittoAssertionResult? assertion = null,
            DittoDeadlineKind? expired = null
        ) =>
            FinishStep(
                step,
                DittoStepStatus.Failed,
                expired,
                reportError(code, diagnostic),
                assertion,
                null
            );

        private void FailInfrastructureStep(
            DittoResolvedStep step,
            DittoErrorCode code,
            string diagnostic,
            DittoDeadlineKind? expired = null
        ) =>
            FinishStep(
                step,
                DittoStepStatus.InfrastructureError,
                expired,
                reportError(code, diagnostic),
                null,
                null
            );

        private void FinishStep(
            DittoResolvedStep step,
            DittoStepStatus status,
            DittoDeadlineKind? expired,
            string? errorRef,
            DittoAssertionResult? assertion,
            string? artifactId,
            bool continueScenario = false,
            string? videoInputId = null
        )
        {
            if (status == DittoStepStatus.Passed && pollFailure() is string observed)
            {
                status = DittoStepStatus.Failed;
                errorRef = observed;
                continueScenario = false;
            }
            if (status != DittoStepStatus.Passed && !continueScenario)
            {
                FinalizeVideoFailure();
            }
            var result = new DittoPlayerStepResult(
                step.Index,
                step.Name,
                DittoLifecycleValidation.StepName(step.Action),
                status,
                Duration(stepStarted, now(), step.TimeoutMs),
                expired,
                errorRef is null ? Array.Empty<string>() : new[] { errorRef },
                assertion,
                artifactId,
                videoInputId,
                performanceRecorder?.Finish()
            );
            performanceRecorder = null;
            pointerClickTarget = null;
            results.Add(result);
            phase = Phase.None;
            waitCondition = null;
            screenshotOutcome = null;
            stepActive = false;
            nextStep++;
            primaryErrorRef ??= errorRef;
            completeAfterBoundary = errorRef is not null && !continueScenario;
            if (completeAfterBoundary)
            {
                AddNotRunSteps();
            }
            boundaryPending = true;
            onStepEnded(result, succeeded => boundarySucceeded = succeeded);
        }

        private void FailRemaining(DittoDeadlineKind expired, string diagnostic)
        {
            primaryErrorRef = reportError(DittoErrorCode.DeadlineExpired, diagnostic);
            if (nextStep < scenario.Steps.Count)
            {
                DittoResolvedStep step = scenario.Steps[nextStep++];
                results.Add(
                    new DittoPlayerStepResult(
                        step.Index,
                        step.Name,
                        DittoLifecycleValidation.StepName(step.Action),
                        DittoStepStatus.InfrastructureError,
                        0,
                        expired,
                        new[] { primaryErrorRef! },
                        null,
                        null,
                        null,
                        null
                    )
                );
            }
            AddNotRunSteps();
            Complete();
        }

        private void AddNotRunSteps()
        {
            while (nextStep < scenario.Steps.Count)
            {
                DittoResolvedStep step = scenario.Steps[nextStep++];
                results.Add(
                    new DittoPlayerStepResult(
                        step.Index,
                        step.Name,
                        DittoLifecycleValidation.StepName(step.Action),
                        DittoStepStatus.NotRun,
                        0,
                        null,
                        Array.Empty<string>(),
                        null,
                        null,
                        null,
                        null
                    )
                );
            }
        }

        private void Complete()
        {
            complete = true;
            ulong totalDuration = OverallDuration(now());
            ulong startupDuration = OverallDuration(executionStarted);
            Result = new DittoScenarioExecution(
                primaryErrorRef is null ? DittoExecutionStatus.Passed : DittoExecutionStatus.Failed,
                results.ToArray(),
                startupDuration,
                totalDuration - startupDuration,
                settleDurationMs,
                captureDurationMs,
                scenarioExpiry,
                primaryErrorRef
            );
        }

        private bool AdvanceBoundary()
        {
            if (!boundaryPending)
            {
                return false;
            }
            if (!boundarySucceeded.HasValue)
            {
                return true;
            }
            bool succeeded = boundarySucceeded.Value;
            boundaryPending = false;
            boundarySucceeded = null;
            if (!succeeded)
            {
                AddNotRunSteps();
                complete = true;
                return true;
            }
            if (completeAfterBoundary)
            {
                completeAfterBoundary = false;
                Complete();
                return true;
            }
            return false;
        }

        private bool TryFreezeObserved()
        {
            string? errorRef = pollFailure();
            if (errorRef is null)
            {
                return false;
            }
            Freeze(errorRef);
            return true;
        }

        private static ulong Duration(TimeSpan start, TimeSpan end, ulong capMs)
        {
            double milliseconds = Math.Max(0, (end - start).TotalMilliseconds);
            return Math.Min(capMs, checked((ulong)Math.Floor(milliseconds)));
        }

        private ulong OverallDuration(TimeSpan end) =>
            Duration(scenarioStarted, end, Math.Min(scenario.TimeoutMs, runTimeoutMs));

        private ulong PhaseDuration() =>
            Duration(phaseStarted, now(), scenario.Steps[nextStep].TimeoutMs);

        private void ThrowIfDisposed()
        {
            if (disposed)
            {
                throw new ObjectDisposedException(nameof(DittoScenarioExecutor));
            }
        }

        private static DittoScreenshotCapture Wrap(
            Func<DittoResolvedStep, DittoScreenshotStepOutcome> captureScreenshot
        )
        {
            if (captureScreenshot is null)
            {
                throw new ArgumentNullException(nameof(captureScreenshot));
            }
            return (step, _, completion) => completion(captureScreenshot(step));
        }
    }
}

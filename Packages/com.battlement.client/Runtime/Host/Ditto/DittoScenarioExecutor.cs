#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine.InputSystem;

namespace Battlement
{
    internal delegate void DittoScreenshotCapture(
        DittoResolvedStep step,
        ulong committedFrame,
        System.Action<DittoScreenshotStepOutcome> completion
    );

    internal delegate void DittoStepBoundary(
        DittoPlayerStepResult result,
        System.Action<bool> completion
    );

    internal sealed record DittoScreenshotStepOutcome(
        string? ArtifactId,
        string? ErrorRef,
        bool ContinueScenario
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
            Input,
            Settle,
            ScreenshotSettle,
            ScreenshotCapture,
            FrameWait,
            ObjectWait,
        }

        private readonly BattlementRunner runner;
        private readonly DittoResolvedScenario scenario;
        private readonly DittoMotionController motion;
        private readonly DittoVirtualInput input;
        private readonly DittoInputTargets targets;
        private readonly Func<TimeSpan> now;
        private readonly DittoScreenshotCapture capture;
        private readonly Func<DittoErrorCode, string, string> reportError;
        private readonly Func<string?> pollFailure;
        private readonly Action<DittoResolvedStep> onStepStarted;
        private readonly DittoStepBoundary onStepEnded;
        private readonly DittoNativeVideoRecorder? videoRecorder;
        private readonly Func<ulong, byte[]>? captureVideoFrame;
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
        private uint waitFrames;
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
        private bool stepActive;
        private bool boundaryPending;
        private bool? boundarySucceeded;
        private bool completeAfterBoundary;
        private bool videoMotionOverridden;
        private bool presentationChanged;
        private int startupQuietFrames;

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
            Func<ulong, byte[]>? videoFrame = null,
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
            Func<ulong, byte[]>? videoFrame = null,
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
            input = new DittoVirtualInput(platform, width, height);
            targets = new DittoInputTargets(runner, aliases, width, height);
        }

        public DittoScenarioExecution? Result { get; private set; }

        public uint? CurrentStepIndex => stepActive ? scenario.Steps[nextStep].Index : null;

        public ulong LastCommittedFrame => committedFrame;

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
                    || phase is not Phase.FrameWait and not Phase.ObjectWait
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
                    AdvanceFrame();
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
            input.Dispose();
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
            setup();
            motion.Begin(scenario.Motion);
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
            onStepStarted(step);
            switch (step.Action)
            {
                case DittoStepAction.Click click:
                    presentationReady = false;
                    if (TryResolve(click.Target, step, out UnityEngine.Vector2 clickPosition))
                    {
                        input.Click(clickPosition);
                        phase = Phase.Input;
                    }
                    break;
                case DittoStepAction.Hover hover:
                    presentationReady = false;
                    if (TryResolve(hover.Target, step, out UnityEngine.Vector2 hoverPosition))
                    {
                        if (input.Hover(hoverPosition))
                        {
                            phase = Phase.Input;
                        }
                        else
                        {
                            FailStep(
                                step,
                                DittoErrorCode.InputUnreachable,
                                "Hover is unsupported."
                            );
                        }
                    }
                    break;
                case DittoStepAction.Drag drag:
                    presentationReady = false;
                    if (!TryResolve(drag.From, step, out UnityEngine.Vector2 from))
                    {
                        break;
                    }
                    if (TryResolve(drag.To, step, out UnityEngine.Vector2 to))
                    {
                        input.Drag(from, to);
                        phase = Phase.Input;
                    }
                    break;
                case DittoStepAction.Key key:
                    presentationReady = false;
                    input.Key(key.Value, key.Action);
                    phase = Phase.Input;
                    break;
                case DittoStepAction.Wait { Value: DittoWait.Frames frames }:
                    waitFrames = frames.Count;
                    phase = Phase.FrameWait;
                    break;
                case DittoStepAction.Wait { Value: DittoWait.Object condition }:
                    waitCondition = condition.Condition;
                    phase = Phase.ObjectWait;
                    EvaluateObjectWait(step);
                    break;
                case DittoStepAction.Assert assertion:
                    Assert(step, assertion.Condition);
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

        private void AdvanceFrame()
        {
            motion.PrepareFrame();
            if (phase == Phase.Input)
            {
                input.QueueNextFrame();
                InputSystem.Update();
            }
            runner.RunFrame();
            runner.CompleteNativeFrame();
            DittoCommittedFrame frame = motion.ObserveCommittedFrame();
            committedFrame = frame.Index;
            presentationChanged = frame.LayoutChanged;
            CaptureVideoFrame(frame);
            if (TryFreezeObserved())
            {
                return;
            }
            if (phase == Phase.StartupSettle)
            {
                if (Expired(null).HasValue)
                {
                    scenarioExpiry = Expired(null);
                    FailRemaining(scenarioExpiry!.Value, "Scenario setup exceeded its deadline.");
                    return;
                }
                startupQuietFrames = frame.IsSettled ? startupQuietFrames + 1 : 0;
                if (startupQuietFrames == 2)
                {
                    settleDurationMs += PhaseDuration();
                    presentationReady = true;
                    executionStarted = now();
                    phase = Phase.None;
                }
                return;
            }
            DittoResolvedStep step = scenario.Steps[nextStep];
            if (TryExpireStep(step))
            {
                return;
            }

            switch (phase)
            {
                case Phase.Input when input.PendingFrameCount == 0:
                    if (step.Action is DittoStepAction.Click { Settle: false })
                    {
                        presentationReady = true;
                        PassStep(step);
                    }
                    else
                    {
                        phase = Phase.Settle;
                        phaseStarted = now();
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
                    phase = Phase.None;
                    Capture(step);
                    break;
                case Phase.FrameWait:
                    waitFrames--;
                    if (waitFrames == 0)
                    {
                        motion.PreserveExactWaitState();
                        presentationReady = true;
                        PassStep(step);
                    }
                    break;
                case Phase.ObjectWait:
                    EvaluateObjectWait(step);
                    break;
                case Phase.Input:
                case Phase.StartupSettle:
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
                motion.PreserveExactWaitState();
                presentationReady = true;
                PassStep(step);
            }
        }

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

        private void Capture(DittoResolvedStep step)
        {
            phase = Phase.ScreenshotCapture;
            phaseStarted = now();
            capture(step, committedFrame, outcome => screenshotOutcome = outcome);
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
                DittoStepStatus.Failed,
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
            out UnityEngine.Vector2 position
        )
        {
            DittoInputResolution resolution = targets.Resolve(target);
            position = resolution.Position;
            if (resolution.IsReachable)
            {
                return true;
            }
            string diagnostic = resolution.ObjectId is ObjectId id
                ? $"Input target {id.Value} is unreachable."
                : "Input target is unreachable.";
            FailStep(step, DittoErrorCode.InputUnreachable, diagnostic);
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
            FailStep(
                step,
                DittoErrorCode.DeadlineExpired,
                $"The {expired.Value.ToString().ToLowerInvariant()} deadline expired.",
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

        private void CaptureVideoFrame(DittoCommittedFrame frame)
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
            if (
                videoRecorder.AppendFrame(
                    captureVideoFrame(frame.Index),
                    videoLayout,
                    frame.Elapsed
                )
            )
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
                videoInputId
            );
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

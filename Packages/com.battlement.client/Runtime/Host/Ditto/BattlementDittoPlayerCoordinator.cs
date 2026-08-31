#nullable enable

#if BATTLEMENT_DITTO_DIAGNOSTICS
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Newtonsoft.Json.Linq;
using UnityEngine;

namespace Battlement
{
    internal sealed class BattlementDittoPlayerCoordinator : MonoBehaviour
    {
        private enum Phase
        {
            Idle,
            WaitingForRunner,
            Probing,
            Starting,
            Executing,
            CapturingFailure,
            Resetting,
            CompletingScenario,
            CompletingJob,
            Complete,
        }

        private readonly List<string> executedScenarioIds = new();
        private DittoUnityWebRequestTransport? transport;
        private DittoLogDelivery? delivery;
        private DittoJob? job;
        private BattlementRunner? runner;
        private DittoWebCaptureAdapter? webCapture;
        private DittoNativeCaptureAdapter? nativeCapture;
        private DittoScenarioContext? scenarioContext;
        private DittoScenarioExecutor? executor;
        private DittoNativeEngineSession? engine;
        private DittoPlayerStateReset? reset;
        private DittoScenarioExecution? execution;
        private DittoNativeVideoRecorder? videoRecorder;
        private DittoCaptureProbeResult.Passed? nativeProbe;
        private DittoWebProbeResult.Passed? webProbe;
        private DittoPlayerInfrastructureFailure? startupFailure;
        private Phase phase;
        private int scenarioIndex;
        private int errorIndex;
        private DittoOrientation? preparedIosOrientation;
        private int preparedIosFrame;
        private uint preparedMacosWidth;
        private uint preparedMacosHeight;
        private int preparedMacosFrame;
        private double jobStartedAt;
        private bool warmJob;

        private void Awake() => BattlementDittoPlayerBootstrap.JobAvailable += ReceiveJob;

        private void Update()
        {
            if (phase == Phase.WaitingForRunner)
            {
                TryPreparePlayer();
            }
            else if (phase == Phase.Executing)
            {
                AdvanceScenario();
            }
            else if (phase == Phase.Resetting && reset!.Advance())
            {
                CompleteScenarioBoundary();
            }
        }

        private void ReceiveJob(DittoJob value)
        {
            if (phase is not Phase.Idle and not Phase.Complete)
            {
                LogFailure("ditto.job-overlap", "A Ditto job arrived while another was active.");
                return;
            }
            warmJob = phase == Phase.Complete;
            job = value;
            scenarioIndex = 0;
            errorIndex = 0;
            executedScenarioIds.Clear();
            jobStartedAt = Time.realtimeSinceStartupAsDouble;
            transport ??= new DittoUnityWebRequestTransport(
                this,
                BattlementDittoPlayerBootstrap.SessionUrl
            );
            delivery ??= new DittoLogDelivery(
                BattlementDittoPlayerBootstrap.BootstrapLogs ?? BattlementLogStore.Observe(),
                transport
            );
            if (warmJob)
            {
                delivery.BindWarmJob(
                    value,
                    BattlementDittoPlayerBootstrap.PlayerSessionId,
                    RouteRedactions()
                );
            }
            else
            {
                delivery.BindFirstJob(
                    value,
                    BattlementDittoPlayerBootstrap.PlayerSessionId,
                    RouteRedactions()
                );
            }
            phase = Phase.WaitingForRunner;
        }

        private void TryPreparePlayer()
        {
            runner ??= FindAnyObjectByType<BattlementRunner>();
            if (runner?.IsDittoConfigured != true)
            {
                return;
            }
            phase = Phase.Probing;
            DittoResolvedProfile profile = job!.Profile;
            if (
                profile.Platform == DittoPlatform.IosSimulator
                && !PrepareIosOrientation(profile.Display.Orientation)
            )
            {
                phase = Phase.WaitingForRunner;
                return;
            }
            if (
                profile.Platform == DittoPlatform.Macos
                && !PrepareMacosDisplay(profile.Display.Width, profile.Display.Height)
            )
            {
                phase = Phase.WaitingForRunner;
                return;
            }
            if (profile.Platform == DittoPlatform.Webgl)
            {
                var captureHost = new GameObject("Battlement Ditto Web Capture");
                DontDestroyOnLoad(captureHost);
                webCapture ??= DittoWebCaptureAdapter.Attach(
                    captureHost,
                    profile.Display.Width,
                    profile.Display.Height
                );
                webCapture.Probe(CompleteWebProbe);
                return;
            }
            var nativeCaptureHost = new GameObject("Battlement Ditto Native Capture");
            DontDestroyOnLoad(nativeCaptureHost);
            nativeCapture ??= DittoNativeCaptureAdapter.Attach(
                nativeCaptureHost,
                profile.Platform,
                profile.Display.Width,
                profile.Display.Height,
                profile.Display.Orientation
            );
            nativeCapture.Probe(CompleteNativeProbe);
        }

        private void CompleteWebProbe(DittoWebProbeResult result)
        {
            if (result is DittoWebProbeResult.Passed passed)
            {
                webProbe = passed;
            }
            else
            {
                DittoCaptureFailure failure = ((DittoWebProbeResult.Failed)result).Failure;
                startupFailure = new DittoPlayerInfrastructureFailure(failure.Code, failure.Reason);
            }
            PostStarted();
        }

        private void CompleteNativeProbe(DittoCaptureProbeResult result)
        {
            if (result is DittoCaptureProbeResult.Passed passed)
            {
                nativeProbe = passed;
            }
            else
            {
                DittoCaptureFailure failure = ((DittoCaptureProbeResult.Failed)result).Failure;
                Debug.Log(
                    $"[Battlement/Ditto-player][ditto.capture-probe-failed] {failure.Reason}"
                );
                startupFailure = new DittoPlayerInfrastructureFailure(failure.Code, failure.Reason);
            }
            PostStarted();
        }

        private void PostStarted()
        {
            phase = Phase.Starting;
            DittoStartupIdentity identity = warmJob
                ? new DittoStartupIdentity.Accepted(BattlementDittoPlayerBootstrap.PlayerSessionId)
                : new DittoStartupIdentity.Report(StartupReport());
            var started = new DittoStarted(
                job!.JobId,
                job.RunId,
                BattlementDittoPlayerBootstrap.PlayerSessionId,
                delivery!.FirstLogSequence,
                startupFailure,
                delivery.Failure,
                identity
            );
            PostJson<DittoStarted, DittoScenarioDecision>(
                $"jobs/{job.JobId}/started",
                started,
                decision =>
                {
                    if (decision is null || decision.Action != DittoNextAction.Continue)
                    {
                        CompleteJob(DittoTerminalReason.InfrastructureError);
                        return;
                    }
                    StartScenario();
                }
            );
        }

        private DittoStartupReport StartupReport()
        {
            JObject identity = JObject.Parse(
                Resources.Load<TextAsset>("BattlementDittoBuildIdentity")?.text
                    ?? throw new InvalidOperationException(
                        "The immutable Ditto build identity is missing."
                    )
            );
            DittoResolvedProfile profile = job!.Profile;
            string adapter =
                webProbe?.Adapter
                ?? nativeProbe?.Adapter
                ?? identity.Value<string>("capture_adapter")
                ?? "unavailable";
            uint width = webProbe?.Width ?? nativeProbe?.Width ?? profile.Display.Width;
            uint height = webProbe?.Height ?? nativeProbe?.Height ?? profile.Display.Height;
            DittoDisplay display = profile.Display with { Width = width, Height = height };
            if (profile.Platform == DittoPlatform.IosSimulator)
            {
                UnityEngine.Rect safeArea = Screen.safeArea;
                display = display with
                {
                    Width = checked((uint)Screen.width),
                    Height = checked((uint)Screen.height),
                    Orientation = CurrentOrientation() ?? profile.Display.Orientation,
                    SafeArea = new uint[]
                    {
                        checked((uint)Math.Round(safeArea.x)),
                        checked((uint)Math.Round(safeArea.y)),
                        checked((uint)Math.Round(safeArea.width)),
                        checked((uint)Math.Round(safeArea.height)),
                    },
                };
            }
            return new DittoStartupReport(
                ParsePlatform(identity.Value<string>("platform")),
                adapter,
                RequiredIdentity(identity, "build_fingerprint"),
                RequiredIdentity(identity, "source_fingerprint"),
                RequiredIdentity(identity, "unity_version"),
                identity.Value<bool?>("diagnostics") ?? false,
                display,
                ActualCapabilities(profile.Platform)
            );
        }

        private bool PrepareIosOrientation(DittoOrientation? orientation)
        {
            ScreenOrientation requested = orientation switch
            {
                DittoOrientation.Portrait => ScreenOrientation.Portrait,
                DittoOrientation.PortraitUpsideDown => ScreenOrientation.PortraitUpsideDown,
                DittoOrientation.LandscapeLeft => ScreenOrientation.LandscapeLeft,
                DittoOrientation.LandscapeRight => ScreenOrientation.LandscapeRight,
                _ => throw new InvalidOperationException(
                    "The iOS Simulator profile omitted its orientation."
                ),
            };
            if (preparedIosOrientation != orientation)
            {
                Screen.orientation = requested;
                preparedIosOrientation = orientation;
                preparedIosFrame = Time.frameCount;
                return false;
            }
            if (Time.frameCount <= preparedIosFrame)
            {
                return false;
            }
            bool portrait =
                orientation is DittoOrientation.Portrait or DittoOrientation.PortraitUpsideDown;
            return portrait ? Screen.height >= Screen.width : Screen.width >= Screen.height;
        }

        private bool PrepareMacosDisplay(uint width, uint height)
        {
            if (preparedMacosWidth != width || preparedMacosHeight != height)
            {
                Screen.SetResolution(
                    checked((int)width),
                    checked((int)height),
                    FullScreenMode.Windowed
                );
                preparedMacosWidth = width;
                preparedMacosHeight = height;
                preparedMacosFrame = Time.frameCount;
                return false;
            }
            if (Time.frameCount <= preparedMacosFrame)
            {
                return false;
            }
            return Screen.width == checked((int)width) && Screen.height == checked((int)height);
        }

        private static DittoOrientation? CurrentOrientation() =>
            Screen.orientation switch
            {
                ScreenOrientation.Portrait => DittoOrientation.Portrait,
                ScreenOrientation.PortraitUpsideDown => DittoOrientation.PortraitUpsideDown,
                ScreenOrientation.LandscapeLeft => DittoOrientation.LandscapeLeft,
                ScreenOrientation.LandscapeRight => DittoOrientation.LandscapeRight,
                _ => null,
            };

        private void StartScenario()
        {
            if (scenarioIndex >= job!.Scenarios.Count)
            {
                CompleteJob(DittoTerminalReason.Completed);
                return;
            }
            DittoResolvedScenario scenario = job.Scenarios[scenarioIndex];
            scenarioContext = new DittoScenarioContext(
                job,
                scenario,
                delivery!,
                BattlementLogStore.Observe(),
                AllocateError
            );
            scenarioContext.Begin();
            engine = DittoNativeEngineSession.Create(
                runner!.DittoNativeTransport,
                out BattlementTransportResult creation,
                scenario.Fixture
            );
            executor = NewExecutor(scenario);
            if (engine is null)
            {
                executor.Freeze(
                    scenarioContext.ReportFunctionalError(
                        DittoErrorCode.RuntimeFatal,
                        creation.Diagnostic ?? "The scenario engine could not be created."
                    )
                );
                BeginFailureFrameOrBoundary();
                return;
            }
            scenarioContext.EngineStarted(engine.Id);
            runner.Connect();
            phase = Phase.Executing;
        }

        private DittoScenarioExecutor NewExecutor(DittoResolvedScenario scenario)
        {
            if (
                nativeCapture is not null
                && scenario.Steps.Any(step => step.Action is DittoStepAction.Video)
            )
            {
                string directory = Path.Combine(Application.persistentDataPath, "BattlementDitto");
                string reportedDirectory =
                    job!.Profile.Platform == DittoPlatform.IosSimulator
                        ? Path.Combine("Documents", "BattlementDitto")
                        : directory;
                videoRecorder = new DittoNativeVideoRecorder(
                    directory,
                    job.Profile.Display.Width,
                    job.Profile.Display.Height,
                    reportedDirectory
                );
            }
            Func<ulong, byte[]>? captureVideoFrame = nativeCapture is null
                ? null
                : new Func<ulong, byte[]>(nativeCapture.CaptureVideoFrame);
            DittoScenarioContext context = scenarioContext!;
            return new DittoScenarioExecutor(
                runner!,
                scenario,
                job!.Profile.Platform,
                job.Profile.Display.Width,
                job.Profile.Display.Height,
                new Dictionary<string, ObjectId>(),
                job.RemainingRunTimeoutMs,
                () => TimeSpan.FromSeconds(Time.realtimeSinceStartupAsDouble),
                (DittoScreenshotCapture)CaptureScreenshot,
                context.ReportFunctionalError,
                observeFailure: context.PollFailure,
                stepStarted: context.StepStarted,
                stepEnded: context.StepEnded,
                video: videoRecorder,
                videoFrame: captureVideoFrame,
                nativeVideoLayout: nativeCapture?.VideoLayout
            );
        }

        private void AdvanceScenario()
        {
            try
            {
                if (executor!.Advance())
                {
                    BeginFailureFrameOrBoundary();
                }
            }
            catch (Exception exception)
            {
                executor!.Freeze(
                    scenarioContext!.ReportFunctionalError(
                        DittoErrorCode.RuntimeFatal,
                        exception.Message
                    )
                );
                if (executor.Result is not null)
                {
                    BeginFailureFrameOrBoundary();
                }
            }
        }

        private void CaptureScreenshot(
            DittoResolvedStep step,
            ulong frame,
            Action<DittoScreenshotStepOutcome> completion
        )
        {
            var screenshot = (DittoStepAction.Screenshot)step.Action;
            string artifactId = Guid.NewGuid().ToString("D");
            var kind = new DittoArtifactKind.Screenshot(screenshot.Value.Name);
            if (webCapture is not null)
            {
                webCapture.UploadCommittedFrame(
                    ArtifactUrl(artifactId),
                    artifactId,
                    frame,
                    result => CompleteWebScreenshot(step, kind, result, completion)
                );
                return;
            }
            nativeCapture!.CaptureCommittedFrame(
                frame,
                result => CompleteNativeScreenshot(step, artifactId, kind, result, completion)
            );
        }

        private void CompleteWebScreenshot(
            DittoResolvedStep step,
            DittoArtifactKind kind,
            DittoWebCaptureResult result,
            Action<DittoScreenshotStepOutcome> completion
        )
        {
            if (result is DittoWebCaptureResult.Unavailable unavailable)
            {
                CompleteCaptureFailure(unavailable.Failure, completion);
                return;
            }
            string artifactId = ((DittoWebCaptureResult.Uploaded)result).ArtifactId;
            delivery!.ConfirmUploadedArtifact(
                job!.Scenarios[scenarioIndex].Id,
                step.Index,
                artifactId,
                kind,
                succeeded => FinishScreenshot(step, artifactId, kind, succeeded, completion)
            );
        }

        private void CompleteNativeScreenshot(
            DittoResolvedStep step,
            string artifactId,
            DittoArtifactKind kind,
            DittoNativeCaptureResult result,
            Action<DittoScreenshotStepOutcome> completion
        )
        {
            if (result is DittoNativeCaptureResult.Unavailable unavailable)
            {
                CompleteCaptureFailure(unavailable.Failure, completion);
                return;
            }
            var captured = (DittoNativeCaptureResult.Captured)result;
            delivery!.UploadArtifact(
                new DittoPngArtifact(
                    job!.Scenarios[scenarioIndex].Id,
                    step.Index,
                    artifactId,
                    kind,
                    captured.Width,
                    captured.Height,
                    captured.Png
                ),
                succeeded => FinishScreenshot(step, artifactId, kind, succeeded, completion)
            );
        }

        private void FinishScreenshot(
            DittoResolvedStep step,
            string artifactId,
            DittoArtifactKind kind,
            bool succeeded,
            Action<DittoScreenshotStepOutcome> completion
        )
        {
            if (succeeded)
            {
                scenarioContext!.AddArtifact(
                    new DittoReachedArtifact(artifactId, step.Index, kind)
                );
                completion(new DittoScreenshotStepOutcome(artifactId, null, true));
                return;
            }
            CompleteCaptureFailure(
                delivery!.Failure is DittoPlayerInfrastructureFailure failure
                    ? new DittoCaptureFailure(failure.Code, failure.Message)
                    : new DittoCaptureFailure(
                        DittoErrorCode.TransportRequestFailed,
                        "The screenshot acknowledgement failed."
                    ),
                completion
            );
        }

        private void CompleteCaptureFailure(
            DittoCaptureFailure failure,
            Action<DittoScreenshotStepOutcome> completion
        ) =>
            completion(
                new DittoScreenshotStepOutcome(
                    null,
                    scenarioContext!.ReportFunctionalError(failure.Code, failure.Reason),
                    false
                )
            );

        private void BeginFailureFrameOrBoundary()
        {
            execution =
                executor!.Result
                ?? throw new InvalidOperationException("The scenario result is missing.");
            if (execution.Status == DittoExecutionStatus.Passed)
            {
                BeginBoundary();
                return;
            }
            phase = Phase.CapturingFailure;
            if (webCapture is not null)
            {
                CaptureWebFailureFrame();
                return;
            }
            scenarioContext!.CaptureFailureFrame(
                executor.LastCommittedFrame,
                nativeCapture!.CaptureCommittedFrame,
                _ => BeginBoundary()
            );
        }

        private void CaptureWebFailureFrame()
        {
            if (executor!.LastCommittedFrame == 0)
            {
                scenarioContext!.RecordUnavailableFailureFrame(
                    "No committed framebuffer was available.",
                    null
                );
                BeginBoundary();
                return;
            }
            string artifactId = Guid.NewGuid().ToString("D");
            webCapture!.UploadCommittedFrame(
                ArtifactUrl(artifactId),
                artifactId,
                executor.LastCommittedFrame,
                result =>
                {
                    if (result is DittoWebCaptureResult.Uploaded uploaded)
                    {
                        scenarioContext!.AcceptUploadedFailureFrame(
                            uploaded.ArtifactId,
                            _ => BeginBoundary()
                        );
                        return;
                    }
                    DittoCaptureFailure failure = (
                        (DittoWebCaptureResult.Unavailable)result
                    ).Failure;
                    string errorRef = scenarioContext!.ReportFunctionalError(
                        failure.Code,
                        failure.Reason
                    );
                    scenarioContext.RecordUnavailableFailureFrame(failure.Reason, errorRef);
                    BeginBoundary();
                }
            );
        }

        private void BeginBoundary()
        {
            scenarioContext!.CloseForBoundary();
            reset = new DittoPlayerStateReset(
                runner!,
                engine,
                () => TimeSpan.FromSeconds(Time.realtimeSinceStartupAsDouble),
                onEngineDestroyed: _ => scenarioContext.EngineEnded(execution!.Status)
            );
            phase = Phase.Resetting;
            reset.Begin();
            if (reset.IsComplete)
            {
                CompleteScenarioBoundary();
            }
        }

        private void CompleteScenarioBoundary()
        {
            phase = Phase.CompletingScenario;
            scenarioContext!.Complete(
                execution!,
                reset!.Failure,
                reset.DurationMs,
                videoRecorder?.Inputs.ToArray() ?? Array.Empty<DittoNativeVideoInput>(),
                complete =>
                {
                    if (complete is null)
                    {
                        CompleteJob(DittoTerminalReason.InfrastructureError);
                        return;
                    }
                    PostJson<DittoScenarioComplete, DittoScenarioDecision>(
                        $"jobs/{job!.JobId}/scenarios/{complete.ScenarioId}/complete",
                        complete,
                        FinishScenario
                    );
                }
            );
        }

        private void FinishScenario(DittoScenarioDecision? decision)
        {
            executedScenarioIds.Add(job!.Scenarios[scenarioIndex].Id);
            executor!.Dispose();
            videoRecorder?.Dispose();
            scenarioContext!.Dispose();
            executor = null;
            videoRecorder = null;
            scenarioContext = null;
            engine = null;
            reset = null;
            execution = null;
            scenarioIndex++;
            if (decision?.Action == DittoNextAction.Continue)
            {
                StartScenario();
                return;
            }
            CompleteJob(
                scenarioIndex == job.Scenarios.Count
                    ? DittoTerminalReason.Completed
                    : DittoTerminalReason.Bail
            );
        }

        private void CompleteJob(DittoTerminalReason reason)
        {
            if (phase == Phase.CompletingJob || phase == Phase.Complete)
            {
                return;
            }
            phase = Phase.CompletingJob;
            delivery!.EmitContext(
                new DittoContext.JobEnded(reason),
                "job ended",
                succeeded =>
                {
                    if (!succeeded)
                    {
                        PostFailed(delivery.Failure!);
                        return;
                    }
                    var complete = new DittoJobComplete(
                        job!.JobId,
                        delivery.LastLogSequence!.Value,
                        executedScenarioIds.ToArray(),
                        UnstartedScenarios(),
                        reason,
                        ElapsedMs()
                    );
                    PostJson<DittoJobComplete, DittoJobCompleteAck>(
                        $"jobs/{job.JobId}/complete",
                        complete,
                        _ => FinishJob()
                    );
                }
            );
        }

        private void PostFailed(DittoPlayerInfrastructureFailure failure)
        {
            var failed = new DittoJobFailed(
                job!.JobId,
                failure,
                delivery!.LastLogSequence,
                executedScenarioIds.ToArray(),
                UnstartedScenarios()
            );
            PostJson<DittoJobFailed, DittoJobFailedAck>(
                $"jobs/{job.JobId}/failed",
                failed,
                _ => FinishJob()
            );
        }

        private void FinishJob()
        {
            delivery!.CloseAfterTerminalAcknowledgement();
            phase = Phase.Complete;
        }

        private void PostJson<TRequest, TResponse>(
            string path,
            TRequest value,
            Action<TResponse?> completion
        )
        {
            var request = new DittoDeliveryRequest(
                "POST",
                path,
                "application/json",
                new Dictionary<string, string>(),
                DittoLifecycleCodec.Encode(value)
            );
            SendWithRetry(
                request,
                response =>
                {
                    if (response is DittoDeliveryResponse.Accepted accepted)
                    {
                        try
                        {
                            completion(DittoLifecycleCodec.Decode<TResponse>(accepted.Body));
                        }
                        catch (Exception exception)
                        {
                            LogFailure("ditto.response-invalid", exception.Message);
                            completion(default);
                        }
                        return;
                    }
                    string message = response switch
                    {
                        DittoDeliveryResponse.Rejected rejected =>
                            $"Ditto returned HTTP {rejected.Status}.",
                        DittoDeliveryResponse.Uncertain uncertain => uncertain.Reason,
                        _ => "The Ditto request failed.",
                    };
                    LogFailure("ditto.request-failed", message);
                    completion(default);
                }
            );
        }

        private void SendWithRetry(
            DittoDeliveryRequest request,
            Action<DittoDeliveryResponse> completion,
            bool retried = false
        ) =>
            transport!.Send(
                request,
                response =>
                {
                    if (response is DittoDeliveryResponse.Uncertain && !retried)
                    {
                        transport.SendAfter(
                            TimeSpan.FromMilliseconds(100),
                            request,
                            repeated => completion(repeated)
                        );
                        return;
                    }
                    completion(response);
                }
            );

        private string AllocateError(DittoErrorCode code, string message)
        {
            errorIndex++;
            return $"P{errorIndex:0000}";
        }

        private IReadOnlyList<DittoUnstartedScenario> UnstartedScenarios() =>
            job!
                .Scenarios.Skip(scenarioIndex)
                .Select(scenario => new DittoUnstartedScenario(scenario.Id, "host stopped"))
                .ToArray();

        private string ArtifactUrl(string artifactId) =>
            $"{BattlementDittoPlayerBootstrap.SessionUrl}/jobs/{job!.JobId}/artifacts/{artifactId}";

        private string[] RouteRedactions() => new[] { BattlementDittoPlayerBootstrap.SessionUrl };

        private ulong ElapsedMs() =>
            checked(
                (ulong)
                    Math.Floor(Math.Max(0, Time.realtimeSinceStartupAsDouble - jobStartedAt) * 1000)
            );

        private static DittoPlatform ParsePlatform(string? value) =>
            value switch
            {
                "macos" => DittoPlatform.Macos,
                "webgl" => DittoPlatform.Webgl,
                "ios-simulator" => DittoPlatform.IosSimulator,
                _ => throw new InvalidOperationException("The Ditto build platform is invalid."),
            };

        private static IReadOnlyList<DittoCapability> ActualCapabilities(DittoPlatform platform) =>
            platform switch
            {
                DittoPlatform.Webgl => new[]
                {
                    DittoCapability.Click,
                    DittoCapability.Hover,
                    DittoCapability.Drag,
                    DittoCapability.Key,
                    DittoCapability.Png,
                },
                DittoPlatform.Macos => new[]
                {
                    DittoCapability.Click,
                    DittoCapability.Hover,
                    DittoCapability.Drag,
                    DittoCapability.Key,
                    DittoCapability.Png,
                    DittoCapability.Video,
                },
                DittoPlatform.IosSimulator => new[]
                {
                    DittoCapability.Click,
                    DittoCapability.Drag,
                    DittoCapability.Key,
                    DittoCapability.Png,
                    DittoCapability.Video,
                },
                _ => throw new ArgumentOutOfRangeException(nameof(platform)),
            };

        private static string RequiredIdentity(JObject identity, string name) =>
            identity.Value<string>(name)
            ?? throw new InvalidOperationException($"The Ditto build identity omitted {name}.");

        private static void LogFailure(string eventName, string message) =>
            BattlementUnityLogging.Log(
                "ditto-player",
                new BattlementLogRecord(BattlementLogSeverity.Error, eventName, message)
            );

        private void OnDestroy()
        {
            BattlementDittoPlayerBootstrap.JobAvailable -= ReceiveJob;
            executor?.Dispose();
            videoRecorder?.Dispose();
            scenarioContext?.Dispose();
            delivery?.Dispose();
        }
    }
}
#endif

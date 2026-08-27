#nullable enable

using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using System.Threading;
using Battlement.Errors;
using Battlement.UI;
using Newtonsoft.Json;
using UnityEngine;

namespace Battlement
{
    /// <summary>Scene-authored host for one Battlement session.</summary>
    [DisallowMultipleComponent]
    public sealed class BattlementRunner
        : MonoBehaviour,
            IDisposable,
            IBattlementObjectLookup,
            IBattlementPreparedAssetLookup,
            IBattlementUiAssetLookup
    {
        [SerializeField]
        private BattlementTransportKind transportKind;

        [SerializeField]
        private BattlementNativeTransportConfiguration nativeTransport = new();

        [SerializeField]
        private BattlementHttpTransportConfiguration httpTransport = new();

        [SerializeField]
        private bool showLoadingSurface = true;

        private BattlementRunnerOptions? options;
        private BattlementWorld? world;
        private BattlementPreparedAssets? preparedAssets;
        private BattlementScenes? scenes;
        private BattlementSnapshotReplacement? snapshotReplacement;
        private BattlementBatchScheduler? batchScheduler;
        private BattlementParticleEffects? particleEffects;
        private BattlementAudioSources? audioSources;
        private BattlementPointerInput? pointerInput;
        private BattlementKeyboardInput? keyboardInput;
        private BattlementControllerInput? controllerInput;
        private BattlementCustomCommands? customCommands;
        private BattlementTweenAdapter? tweens;
        private BattlementUiDocuments? uiDocuments;
        private readonly BattlementResponseStream responses = new();
        private readonly BattlementSessionState session = new();
        private readonly BattlementBatchAdmission batchAdmission = new();
        private readonly ConcurrentQueue<BattlementCapturedUnityError> unityErrors = new();
        private BattlementDevelopmentDiagnostics? developmentDiagnostics;
        private BattlementFailureSurface? failureSurface;
        private BattlementErrorReporter? errors;
        private IDisposable? unityErrorSubscription;
        private bool wasPaused;
        private bool hasApplicationFocus = true;
        private bool isDisposed;
        private bool isNativePanicRecovery;
        private bool isRuntimePoisoned;
        private bool completedInitialSnapshot;
        private int mainThreadId;
        private int uiDispatchDepth;

        private const int MaximumDiagnosticBytes = 65_536;
        private static readonly TimeSpan SlowFrameThreshold = TimeSpan.FromMilliseconds(16.67);

        public BattlementTransportKind TransportKind => transportKind;

        public BattlementNativeTransportConfiguration NativeTransport => nativeTransport;

        public BattlementHttpTransportConfiguration HttpTransport => httpTransport;

        /// <summary>Whether Battlement renders its built-in loading and failure surface.</summary>
        public bool ShowLoadingSurface => showLoadingSurface;

        /// <summary>
        /// Whether Battlement may currently emit pointer, keyboard, and controller input.
        /// </summary>
        public bool IsInputAvailable => session.IsInputAvailable;

        /// <summary>Current nontechnical failure presentation, if the session failed.</summary>
        public BattlementPlayerFailure? CurrentFailure => failureSurface?.Current;

        /// <summary>Whether an unknown runtime failure requires an application restart.</summary>
        public bool IsRestartRequired => isRuntimePoisoned;

        /// <summary>Returns whether a global physical key is selected for input dispatch.</summary>
        public bool IsGlobalKeyEnabled(PhysicalKey key) => world?.IsGlobalKeyEnabled(key) == true;

        /// <summary>Injects the dependencies owned by this runner.</summary>
        public void Configure(BattlementRunnerOptions runnerOptions)
        {
            BattlementRunnerOptions checkedOptions = Preconditions.CheckNotNull(
                runnerOptions,
                nameof(runnerOptions)
            );

            if (options is not null)
            {
                throw new InvalidOperationException("The runner is already configured.");
            }

            if (isDisposed)
            {
                throw new ObjectDisposedException(nameof(BattlementRunner));
            }

            options = checkedOptions;
            if (!checkedOptions.SuppressDevelopmentErrorDialogs)
            {
                developmentDiagnostics = new BattlementDevelopmentDiagnostics(
                    transform,
                    ContinueAfterFailure
                );
            }
            System.Action<BattlementError>? showDevelopmentError = developmentDiagnostics is null
                ? null
                : developmentDiagnostics.Show;
            errors = new BattlementErrorReporter(
                checkedOptions.Logger,
                checkedOptions.ErrorSink,
                showDevelopmentError
            );
            failureSurface = new BattlementFailureSurface(
                transform,
                showLoadingSurface,
                checkedOptions.FailurePresenter,
                ContinueAfterFailure,
                () => developmentDiagnostics?.IsVisible == true
            );
            unityErrorSubscription = BattlementUnityErrors.Subscribe(unityErrors.Enqueue);
            mainThreadId = Environment.CurrentManagedThreadId;
            preparedAssets = new BattlementPreparedAssets(checkedOptions.AssetStorage);
            world = new BattlementWorld(gameObject.scene, preparedAssets);
            pointerInput = new BattlementPointerInput(transform, EmitAction);
            keyboardInput = new BattlementKeyboardInput(IsGlobalKeyEnabled, EmitAction);
            controllerInput = new BattlementControllerInput(
                () => world.ControllerInput,
                () => pointerInput.NavigationTiming,
                EmitAction
            );
            world.InputCameraChanged += pointerInput.SetCamera;
            particleEffects = new BattlementParticleEffects(world, preparedAssets);
            audioSources = new BattlementAudioSources(world, preparedAssets, transform);
            scenes = new BattlementScenes(checkedOptions.AssetStorage, preparedAssets, world);
            uiDocuments = new BattlementUiDocuments(
                EmitUiEvent,
                world.ContainsLiveObject,
                world.ReserveUiIdentities,
                world.ReleaseUiIdentities,
                this,
                () => checkedOptions.Clock.Elapsed
            );
            snapshotReplacement = new BattlementSnapshotReplacement(
                preparedAssets,
                scenes,
                world,
                uiDocuments
            );
            var operations = new BattlementOperationRegistry(
                (failure, exception) => ReportOperationFailure(failure, exception),
                ReportCustomOperationFailure
            );
            tweens = new BattlementTweenAdapter(
                checkedOptions.UseInstantAnimations,
                checkedOptions.Clock is not UnityBattlementClock
            );
            customCommands = new BattlementCustomCommands(now => CreateCommandContext(now));
            var commandExecutor = new BattlementCommandExecutor(
                world,
                preparedAssets,
                scenes,
                operations,
                tweens,
                particleEffects,
                audioSources,
                controllerInput,
                customCommands,
                SetInputEnabled,
                uiDocuments
            );
            batchScheduler = new BattlementBatchScheduler(
                checkedOptions.Clock,
                commandExecutor,
                operations,
                (failure, exception) => ReportBatchFailure(failure, exception),
                ReportCustomBatchFailure
            );
        }

        /// <summary>Registers one game-owned command type before connecting.</summary>
        public void RegisterCommand<TPayload, TError>(
            string type,
            IBattlementCommandHandler<TPayload> handler,
            JsonConverter<TPayload>? payloadConverter = null,
            JsonConverter<TError>? errorConverter = null
        )
        {
            RequireConfiguredAndStopped();
            customCommands!.Register(type, handler, payloadConverter, errorConverter);
        }

        /// <summary>Emits a typed game-owned action through the active transport.</summary>
        public ActionId EmitCustomAction<TPayload>(
            string type,
            TPayload payload,
            JsonConverter<TPayload>? payloadConverter = null
        )
        {
            EnsureMainThread();
            BattlementCustomCommands.RequireNamespaced(type);
            SessionId currentSession =
                session.LastSession
                ?? throw new InvalidOperationException("No Battlement session is active.");
            if (session.Phase != BattlementSessionPhase.Running)
            {
                throw new InvalidOperationException(
                    "Custom actions may only be emitted while the runner is active."
                );
            }

            var actionId = new ActionId(Guid.NewGuid());
            Submit(
                RequireExtensionCodec()
                    .SerializeCustomAction(
                        new CustomAction<TPayload>(actionId, currentSession, type, payload),
                        payloadConverter
                    )
            );
            return actionId;
        }

        /// <summary>Starts the configured host session.</summary>
        public void Connect()
        {
            BattlementRunnerOptions configured = RequireOptions();
            RequireHealthyRuntime();
            if (session.Phase != BattlementSessionPhase.Stopped)
            {
                throw new InvalidOperationException("The runner is already connected.");
            }

            StartSession(configured, false);
        }

        /// <summary>Stops the current session and starts a new one on the same transport.</summary>
        public void Reconnect()
        {
            BattlementRunnerOptions configured = RequireOptions();
            RequireHealthyRuntime();
            bool recoveringPanic =
                failureSurface?.Current?.Kind == BattlementPlayerFailureKind.ContinueAllowed;
            SessionId? previousSession = session.LastSession;
            isNativePanicRecovery = recoveringPanic;
            try
            {
                if (session.Phase != BattlementSessionPhase.Stopped)
                {
                    StopSession(configured, false);
                }

                StartSession(configured, true, previousSession);
            }
            finally
            {
                isNativePanicRecovery = false;
            }
        }

        /// <summary>Attempts recovery for the currently dismissible player failure.</summary>
        public void ContinueAfterFailure()
        {
            EnsureMainThread();
            if (failureSurface?.Current?.Kind != BattlementPlayerFailureKind.ContinueAllowed)
            {
                throw new InvalidOperationException(
                    "There is no player-visible failure that can be continued."
                );
            }

            Reconnect();
        }

        /// <summary>Stops the active session. Repeated calls are no-ops.</summary>
        public void Stop()
        {
            if (session.Phase == BattlementSessionPhase.Stopped || options is null)
            {
                return;
            }

            StopSession(options, true);
        }

        /// <summary>
        /// Reports an exception caught outside Battlement's own execution boundaries.
        /// </summary>
        public void ReportUnhandledException(Exception exception)
        {
            Preconditions.CheckNotNull(exception, nameof(exception));
            RequireOptions();
            EnsureMainThread();
            unityErrors.Enqueue(
                new BattlementCapturedUnityError(
                    exception.Message,
                    exception.StackTrace ?? string.Empty,
                    LogType.Exception,
                    exception,
                    true
                )
            );
            DrainUnityErrors();
        }

        /// <summary>Submits one already encoded client message to the active session.</summary>
        public void Submit(ReadOnlyMemory<byte> json)
        {
            BattlementRunnerOptions configured = RequireOptions();
            if (session.Phase != BattlementSessionPhase.Running)
            {
                throw new InvalidOperationException(
                    "Client messages may only be submitted while the runner is active."
                );
            }
            if (json.Length > BattlementProtocolLimits.MaximumMessageBytes)
            {
                FailSession(
                    configured,
                    $"A client message cannot exceed "
                        + $"{BattlementProtocolLimits.MaximumMessageBytes} bytes.",
                    payloadBytes: json.Length
                );
                return;
            }

            TimeSpan started = configured.Clock.Elapsed;
            try
            {
                BattlementTransportResult result;
                using (BattlementProfiler.Transport.Auto())
                {
                    result = configured.Transport.Submit(json);
                }

                ProcessTransportResult(
                    configured,
                    result,
                    "Submit failed.",
                    duration: configured.Clock.Elapsed - started
                );
            }
            catch (Exception exception)
            {
                FailSession(
                    configured,
                    $"Submit response failed: {exception.Message}",
                    duration: configured.Clock.Elapsed - started,
                    payloadBytes: json.Length,
                    exception: exception
                );
            }
        }

        /// <summary>Reports a recoverable core failure that stopped one batch.</summary>
        public void ReportBatchFailure(BatchFailed<CoreErrorCode> failure) =>
            ReportBatchFailure(failure, null);

        private void ReportBatchFailure(BatchFailed<CoreErrorCode> failure, Exception? exception)
        {
            Preconditions.CheckNotNull(failure, nameof(failure));
            BattlementRunnerOptions configured = RequireRunningSession();
            BatchFailed<CoreErrorCode> bounded = failure with
            {
                Message = BoundDiagnostic(failure.Message),
            };
            SubmitFailure(
                configured,
                () => configured.ProtocolCodec.SerializeBatchFailure(bounded),
                "battlement.batch.failed",
                bounded.Message,
                bounded.SessionId,
                bounded.BatchId,
                bounded.CommandId,
                bounded.ErrorCode,
                exception
            );
        }

        /// <summary>Reports a late failure from one nonblocking core operation.</summary>
        public void ReportOperationFailure(OperationFailed<CoreErrorCode> failure) =>
            ReportOperationFailure(failure, null);

        private void ReportOperationFailure(
            OperationFailed<CoreErrorCode> failure,
            Exception? exception
        )
        {
            Preconditions.CheckNotNull(failure, nameof(failure));
            BattlementRunnerOptions configured = RequireRunningSession();
            OperationFailed<CoreErrorCode> bounded = failure with
            {
                Message = BoundDiagnostic(failure.Message),
            };
            SubmitFailure(
                configured,
                () => configured.ProtocolCodec.SerializeOperationFailure(bounded),
                "battlement.operation.failed",
                bounded.Message,
                bounded.SessionId,
                bounded.BatchId,
                bounded.CommandId,
                bounded.ErrorCode,
                exception
            );
        }

        private void ReportCustomBatchFailure(
            BattlementRegisteredCommandException exception,
            SessionId sessionId,
            BatchId batchId,
            CommandId? commandId
        )
        {
            BattlementRunnerOptions configured = RequireRunningSession();
            string message = BoundDiagnostic(exception.Message);
            SubmitFailure(
                configured,
                () =>
                    exception.Registration.SerializeBatchFailure(
                        RequireExtensionCodec(),
                        sessionId,
                        batchId,
                        commandId,
                        exception.ErrorCode,
                        message
                    ),
                "battlement.batch.failed",
                message,
                sessionId,
                batchId,
                commandId,
                exception.ErrorCode
            );
        }

        private void ReportCustomOperationFailure(
            BattlementRegisteredCommandException exception,
            SessionId sessionId,
            BatchId batchId,
            CommandId commandId
        )
        {
            BattlementRunnerOptions configured = RequireRunningSession();
            string message = BoundDiagnostic(exception.Message);
            SubmitFailure(
                configured,
                () =>
                    exception.Registration.SerializeOperationFailure(
                        RequireExtensionCodec(),
                        sessionId,
                        batchId,
                        commandId,
                        exception.ErrorCode,
                        message
                    ),
                "battlement.operation.failed",
                message,
                sessionId,
                batchId,
                commandId,
                exception.ErrorCode
            );
        }

        /// <summary>Looks up an asset without starting an implicit load.</summary>
        public bool TryGetPreparedAsset(PreparedAsset asset, out object? value) =>
            preparedAssets?.TryGet(asset, out value) ?? ReturnMissing(out value);

        bool IBattlementPreparedAssetLookup.TryGet(PreparedAsset asset, out object? value) =>
            TryGetPreparedAsset(asset, out value);

        IBattlementUiAssetLease IBattlementUiAssetLookup.Acquire(PreparedAsset asset) =>
            AcquirePreparedAsset(asset);

        /// <summary>
        /// Acquires a usage lease that must be disposed when the asset is no longer referenced.
        /// </summary>
        public IBattlementAssetLease AcquirePreparedAsset(PreparedAsset asset) =>
            preparedAssets?.Acquire(asset)
            ?? throw new InvalidOperationException("The runner is not configured.");

        /// <summary>Looks up a live Battlement-controlled Unity object.</summary>
        public bool TryGetObject(ObjectId id, out GameObject? gameObject) =>
            world?.TryGetObject(id, out gameObject) ?? ReturnMissingObject(out gameObject);

        /// <summary>Stops the session and releases the runner's injected dependencies.</summary>
        public void Dispose()
        {
            if (isDisposed)
            {
                return;
            }

            Stop();
            try
            {
                scenes?.Dispose();
            }
            finally
            {
                try
                {
                    preparedAssets?.Dispose();
                }
                finally
                {
                    try
                    {
                        options?.AssetStorage.Dispose();
                    }
                    finally
                    {
                        try
                        {
                            options?.Transport.Dispose();
                        }
                        finally
                        {
                            try
                            {
                                particleEffects?.Dispose();
                            }
                            finally
                            {
                                try
                                {
                                    audioSources?.Dispose();
                                }
                                finally
                                {
                                    try
                                    {
                                        pointerInput?.Dispose();
                                    }
                                    finally
                                    {
                                        try
                                        {
                                            controllerInput?.Dispose();
                                        }
                                        finally
                                        {
                                            try
                                            {
                                                world?.Dispose();
                                            }
                                            finally
                                            {
                                                try
                                                {
                                                    unityErrorSubscription?.Dispose();
                                                }
                                                finally
                                                {
                                                    try
                                                    {
                                                        failureSurface?.Dispose();
                                                    }
                                                    finally
                                                    {
                                                        developmentDiagnostics?.Dispose();
                                                        isDisposed = true;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        /// <summary>Advances Battlement work for the current Unity frame.</summary>
        public void RunFrame()
        {
            if (options is null)
            {
                return;
            }

            BattlementRunnerOptions configured = options;
            DrainUnityErrors();
            if (session.Phase == BattlementSessionPhase.Stopped)
            {
                return;
            }
            AdvanceSnapshotPreparation(configured);
            if (session.Phase == BattlementSessionPhase.Stopped)
            {
                return;
            }

            if (uiDispatchDepth == 0)
                DrainResponses(configured);
            if (session.Phase == BattlementSessionPhase.Stopped)
            {
                return;
            }

            batchScheduler?.Advance();
            uiDocuments?.Advance();
            pointerInput?.Update(CanEmitInput);
            keyboardInput?.Update(CanEmitInput);
            controllerInput?.Update(CanEmitInput, configured.Clock.Elapsed);
            if (session.Phase == BattlementSessionPhase.Stopped)
            {
                return;
            }

            TimeSpan started = configured.Clock.Elapsed;
            TimeSpan previous = session.PreviousStepTime ?? started;
            if (started < previous)
            {
                throw new InvalidOperationException("The Battlement clock must be monotonic.");
            }

            int payloadBytes = 0;
            using (BattlementProfiler.Frame.Auto())
            {
                try
                {
                    BattlementTransportResult result;
                    TimeSpan pollStarted = configured.Clock.Elapsed;
                    using (BattlementProfiler.Poll.Auto())
                    using (BattlementProfiler.Transport.Auto())
                    {
                        result = configured.Transport.Poll();
                    }

                    payloadBytes = result.Payload.Length;
                    if (result.Status != BattlementTransportStatus.NoMessage)
                    {
                        ProcessTransportResult(
                            configured,
                            result,
                            "Poll failed.",
                            duration: configured.Clock.Elapsed - pollStarted
                        );
                    }
                }
                catch (Exception exception)
                {
                    FailSession(
                        configured,
                        $"Poll response failed: {exception.Message}",
                        duration: configured.Clock.Elapsed - started,
                        payloadBytes: payloadBytes,
                        exception: exception
                    );
                }
            }

            TimeSpan finished = configured.Clock.Elapsed;
            session.PreviousStepTime = finished;
            TimeSpan frameDuration = finished - previous;
            if (frameDuration > SlowFrameThreshold)
            {
                LogSlowFrame(configured, frameDuration, finished - started, payloadBytes);
            }

            if (!Application.isPlaying)
            {
                world?.UpdateBillboards();
            }
        }

        private void Update() => RunFrame();

        private void LateUpdate()
        {
            if (options is BattlementRunnerOptions configured)
            {
                DrainResponses(configured);
                batchScheduler?.Advance();
            }
            world?.UpdateBillboards();
            failureSurface?.Refresh(completedInitialSnapshot);
        }

        private void OnApplicationPause(bool pauseStatus)
        {
            if (pauseStatus)
            {
                wasPaused = true;
            }
            else if (wasPaused)
            {
                wasPaused = false;
                Stop();
            }
        }

        private void OnApplicationFocus(bool hasFocus)
        {
            hasApplicationFocus = hasFocus;
            if (!hasFocus && session.Phase != BattlementSessionPhase.Stopped)
            {
                pointerInput?.CancelPresses();
                keyboardInput?.Reset();
                controllerInput?.Reset();
                Log(
                    BattlementLogSeverity.Information,
                    "battlement.input.pointer_presses_cancelled",
                    "Pointer presses were cancelled after application focus loss."
                );
            }
        }

        private void OnApplicationQuit() => Stop();

        private void OnDestroy() => Dispose();

        private void StartSession(
            BattlementRunnerOptions configured,
            bool reconnecting,
            SessionId? previousSession = null
        )
        {
            developmentDiagnostics?.Hide();
            failureSurface!.Clear(errors!);
            batchAdmission.BeginSession();
            batchScheduler?.BeginSession();
            scenes?.BeginSession();
            world?.BeginSession();
            session.BeginConnection(configured.Clock.Elapsed, reconnecting);

            TimeSpan started = configured.Clock.Elapsed;
            try
            {
                Connect connect = BuildConnect(configured);
                byte[] bytes;
                using (BattlementProfiler.Serialization.Auto())
                {
                    bytes = configured.ProtocolCodec.SerializeConnect(connect);
                }
                if (bytes.Length > BattlementProtocolLimits.MaximumMessageBytes)
                {
                    FailSession(
                        configured,
                        $"A connect request cannot exceed "
                            + $"{BattlementProtocolLimits.MaximumMessageBytes} bytes.",
                        duration: configured.Clock.Elapsed - started,
                        payloadBytes: bytes.Length
                    );
                    return;
                }

                BattlementTransportResult result;
                using (BattlementProfiler.Transport.Auto())
                {
                    result = configured.Transport.Connect(bytes);
                }

                if (result.Status != BattlementTransportStatus.Success)
                {
                    FailSession(
                        configured,
                        "Connect failed.",
                        result,
                        configured.Clock.Elapsed - started
                    );
                    return;
                }

                ProcessTransportResult(
                    configured,
                    result,
                    "Connect failed.",
                    true,
                    previousSession,
                    configured.Clock.Elapsed - started
                );
                if (session.Phase == BattlementSessionPhase.Running)
                {
                    LogPendingConnection();
                }
            }
            catch (Exception exception)
            {
                FailSession(
                    configured,
                    $"Connect response failed: {exception.Message}",
                    duration: configured.Clock.Elapsed - started,
                    exception: exception
                );
            }
        }

        private void ProcessTransportResult(
            BattlementRunnerOptions configured,
            BattlementTransportResult result,
            string failureMessage,
            bool isInitial = false,
            SessionId? previousSession = null,
            TimeSpan? duration = null
        )
        {
            if (result.Status != BattlementTransportStatus.Success)
            {
                FailSession(configured, failureMessage, result, duration);
                return;
            }

            responses.Enqueue(
                payload => DecodeResponse(configured, payload),
                result.Payload,
                isInitial,
                previousSession
            );
            if (uiDispatchDepth == 0)
                DrainResponses(configured);
        }

        private Response<ICommand> DecodeResponse(
            BattlementRunnerOptions configured,
            ReadOnlyMemory<byte> payload
        )
        {
            if (customCommands is not null && customCommands.Types.Count > 0)
            {
                return RequireExtensionCodec().DeserializeResponse(payload, customCommands.Read);
            }

            Response core = configured.ProtocolCodec.DeserializeResponse(payload);
            var messages = new ResponseMessage<ICommand>[core.Messages.Count];
            for (int index = 0; index < messages.Length; index++)
            {
                messages[index] = core.Messages[index] switch
                {
                    ResponseMessage<Command>.SnapshotMessage snapshot =>
                        new ResponseMessage<ICommand>.SnapshotMessage(snapshot.Snapshot),
                    ResponseMessage<Command>.BatchMessage batch =>
                        new ResponseMessage<ICommand>.BatchMessage(ToAnyBatch(batch.Batch)),
                    _ => throw new InvalidDataException("Unknown core response message."),
                };
            }

            return new Response<ICommand>(core.SessionId, messages);
        }

        private static Batch<ICommand> ToAnyBatch(Batch<Command> batch)
        {
            var groups = new ParallelCommandGroup<ICommand>[batch.Groups.Count];
            for (int groupIndex = 0; groupIndex < groups.Length; groupIndex++)
            {
                IReadOnlyList<Command> commands = batch.Groups[groupIndex].Commands;
                var anyCommands = new ICommand[commands.Count];
                for (int commandIndex = 0; commandIndex < anyCommands.Length; commandIndex++)
                {
                    anyCommands[commandIndex] = commands[commandIndex];
                }

                groups[groupIndex] = new ParallelCommandGroup<ICommand>(anyCommands);
            }

            return new Batch<ICommand>(
                batch.Id,
                batch.SessionId,
                groups,
                batch.CausedByActionId,
                batch.Start
            );
        }

        private void DrainResponses(BattlementRunnerOptions configured) =>
            responses.Drain(
                (response, isInitial, previousSession) =>
                    ValidateResponse(configured, response, isInitial, previousSession),
                (session, message) => ApplyMessage(configured, session, message),
                () => session.Phase == BattlementSessionPhase.ApplyingSnapshot,
                () => session.Phase == BattlementSessionPhase.Stopped
            );

        private bool ValidateResponse(
            BattlementRunnerOptions configured,
            Response<ICommand> response,
            bool isInitial,
            SessionId? previousSession
        )
        {
            if (response.SessionId.Value == Guid.Empty || response.Messages is null)
            {
                FailSession(configured, "The response did not contain orderable identity fields.");
                return false;
            }

            if (response.Messages.Count > 256)
            {
                FailSession(configured, "A response cannot contain more than 256 messages.");
                return false;
            }

            if (previousSession is not null && response.SessionId == previousSession.Value)
            {
                Log(
                    BattlementLogSeverity.Error,
                    "battlement.response.wrong_session",
                    "Discarded a response from the previous session."
                );
                return false;
            }

            if (
                isInitial
                && (
                    response.Messages.Count == 0
                    || response.Messages[0] is not ResponseMessage<ICommand>.SnapshotMessage
                )
            )
            {
                FailSession(configured, "The first current-session message was not a snapshot.");
                return false;
            }

            if (
                !isInitial
                && session.LastSession is not null
                && response.SessionId != session.LastSession.Value
            )
            {
                Log(
                    BattlementLogSeverity.Error,
                    "battlement.response.wrong_session",
                    "Discarded a response from a different session."
                );
                return false;
            }

            return true;
        }

        private void ApplyMessage(
            BattlementRunnerOptions configured,
            SessionId responseSession,
            ResponseMessage<ICommand> message
        )
        {
            if (message is ResponseMessage<ICommand>.SnapshotMessage snapshotMessage)
            {
                ApplySnapshot(configured, responseSession, snapshotMessage.Snapshot);
            }
            else if (message is ResponseMessage<ICommand>.BatchMessage batchMessage)
            {
                ApplyBatch(configured, responseSession, batchMessage.Batch);
            }
            else
            {
                FailSession(configured, "The response contained an unknown message kind.");
            }
        }

        private void ApplyBatch(
            BattlementRunnerOptions configured,
            SessionId responseSession,
            Batch<ICommand> batch
        )
        {
            try
            {
                BattlementBatchAdmissionResult result = batchAdmission.Admit(
                    responseSession,
                    Preconditions.CheckNotNull(batch, nameof(batch))
                );
                var fields = new Dictionary<string, string>
                {
                    ["batch_id"] = batch.Id.ToString(),
                    ["session_id"] = responseSession.ToString(),
                };
                if (result.IsDuplicate)
                {
                    Log(
                        BattlementLogSeverity.Warning,
                        "battlement.batch.duplicate",
                        "Ignored a duplicate batch UUID.",
                        fields
                    );
                    return;
                }

                fields["sequence"] = result.Sequence.ToString(CultureInfo.InvariantCulture);
                fields["start"] = batch.Start.ToString();
                if (result.WaitsThroughSequence is long dependency)
                {
                    fields["waits_through_sequence"] = dependency.ToString(
                        CultureInfo.InvariantCulture
                    );
                }

                Log(
                    BattlementLogSeverity.Trace,
                    "battlement.batch.admitted",
                    "Admitted a command batch for scheduling.",
                    fields
                );
                batchScheduler!.Schedule(responseSession, batch, result);
            }
            catch (BattlementBatchAdmissionException exception)
            {
                ReportBatchFailure(
                    new BatchFailed<CoreErrorCode>(
                        responseSession,
                        batch.Id,
                        exception.ErrorCode,
                        exception.Message,
                        exception.CommandId
                    )
                );
            }
            catch (BattlementUnorderableBatchException exception)
            {
                FailSession(configured, exception.Message, exception: exception);
            }
        }

        private Connect BuildConnect(BattlementRunnerOptions configured)
        {
            bool native = configured.Transport.Kind == BattlementTransportKind.Native;
            var commandTypes = new SortedSet<string>(
                configured.CustomCommandTypes,
                StringComparer.Ordinal
            );
            if (customCommands is not null)
            {
                commandTypes.UnionWith(customCommands.Types);
            }

            return new Connect(
                PlatformName(Application.platform),
                Application.unityVersion,
                new ScreenSize(checked((uint)Screen.width), checked((uint)Screen.height)),
                new List<string>(commandTypes),
                native ? Path.GetFullPath(Application.persistentDataPath) : null,
                native ? Path.GetFullPath(Application.streamingAssetsPath) : null
            );
        }

        private void ApplySnapshot(
            BattlementRunnerOptions configured,
            SessionId responseSession,
            Snapshot snapshot
        )
        {
            try
            {
                pointerInput?.Suspend();
                keyboardInput?.Reset();
                controllerInput?.Reset();
                batchScheduler?.CancelForSnapshot();
                particleEffects?.ClearInactive();
                session.BeginSnapshot(responseSession);
                snapshotReplacement!.Begin(responseSession, snapshot);
                AdvanceSnapshotPreparation(configured);
            }
            catch (BattlementSnapshotReplacementException exception)
            {
                FailSession(configured, exception.Message, exception: exception);
            }
        }

        private void AdvanceSnapshotPreparation(BattlementRunnerOptions configured)
        {
            if (session.Phase != BattlementSessionPhase.ApplyingSnapshot)
            {
                return;
            }

            try
            {
                if (!snapshotReplacement!.TryComplete(out bool inputDisabled))
                {
                    return;
                }

                session.CompleteSnapshot(inputDisabled);
                uiDocuments?.SetInputEnabled(!inputDisabled);
                completedInitialSnapshot = true;
                LogPendingConnection();
            }
            catch (BattlementSnapshotReplacementException exception)
            {
                FailSession(configured, exception.Message, exception: exception);
            }
        }

        private void FailSession(
            BattlementRunnerOptions configured,
            string message,
            BattlementTransportResult? result = null,
            TimeSpan? duration = null,
            int? payloadBytes = null,
            Exception? exception = null
        )
        {
            var fields = new Dictionary<string, string>();
            AddSessionField(fields);
            if (duration is not null)
            {
                fields["duration_ms"] = Milliseconds(duration.Value);
            }

            if (result is not null)
            {
                fields["status"] = result.Status.ToString();
                fields["payload_bytes"] = result.Payload.Length.ToString(
                    CultureInfo.InvariantCulture
                );
                if (
                    result.Status != BattlementTransportStatus.Panic
                    && !string.IsNullOrEmpty(result.Diagnostic)
                )
                {
                    fields["diagnostic"] = result.Diagnostic!;
                }
            }
            else if (payloadBytes is not null)
            {
                fields["payload_bytes"] = payloadBytes.Value.ToString(CultureInfo.InvariantCulture);
            }

            bool nativePanic = result?.Status == BattlementTransportStatus.Panic;
            BattlementFormattedText? panicDiagnostic = nativePanic
                ? BattlementAnsiText.Format(result?.Diagnostic)
                : null;
            bool restartRequired = nativePanic && isNativePanicRecovery;
            BattlementError error = errors!.Report(
                restartRequired
                    ? BattlementErrorType.RestartRequired
                    : BattlementErrorType.SessionFailed,
                Source(result, exception),
                "battlement.session.failed",
                message,
                exception,
                stackTrace: panicDiagnostic?.PlainText,
                ansiStackTrace: nativePanic ? result?.Diagnostic : null,
                fields: fields
            );
            isRuntimePoisoned |= restartRequired;
            if (nativePanic)
            {
                failureSurface!.Show(
                    new BattlementPlayerFailure(
                        restartRequired
                            ? BattlementPlayerFailureKind.RestartRequired
                            : BattlementPlayerFailureKind.ContinueAllowed,
                        error.Id
                    ),
                    errors
                );
            }
            StopSession(configured, false);
        }

        private void DrainUnityErrors()
        {
            while (unityErrors.TryDequeue(out BattlementCapturedUnityError error))
            {
                if (!Application.isPlaying && !error.IsExplicit)
                {
                    continue;
                }
                if (isRuntimePoisoned)
                {
                    continue;
                }

                var fields = new Dictionary<string, string>
                {
                    ["log_type"] = error.Type.ToString(),
                };
                AddSessionField(fields);
                errors!.Report(
                    BattlementErrorType.Logged,
                    BattlementErrorSource.Unity,
                    "battlement.unhandled_unity_exception",
                    UnityErrorMessage(error),
                    error.Exception,
                    stackTrace: error.StackTrace,
                    fields: fields
                );
            }
        }

        private static string UnityErrorMessage(BattlementCapturedUnityError error)
        {
            if (error.Exception is not null)
            {
                return error.Condition;
            }

            int end = error.Condition.IndexOfAny(new[] { '\r', '\n' });
            return (end < 0 ? error.Condition : error.Condition.Substring(0, end)).Trim();
        }

        private static BattlementErrorSource Source(
            BattlementTransportResult? result,
            Exception? exception
        )
        {
            if (result?.Status == BattlementTransportStatus.Panic)
            {
                return BattlementErrorSource.Native;
            }
            if (result is not null)
            {
                return BattlementErrorSource.Transport;
            }

            return exception is null ? BattlementErrorSource.Protocol : BattlementErrorSource.Unity;
        }

        private void SubmitFailure(
            BattlementRunnerOptions configured,
            Func<byte[]> serialize,
            string eventName,
            string message,
            SessionId sessionId,
            BatchId batchId,
            CommandId? commandId,
            object errorCode,
            Exception? exception = null
        )
        {
            var fields = new Dictionary<string, string>
            {
                ["batch_id"] = batchId.Value.ToString(),
                ["error_code"] = errorCode.ToString() ?? string.Empty,
                ["session_id"] = sessionId.Value.ToString(),
            };
            if (commandId is not null)
            {
                fields["command_id"] = commandId.Value.Value.ToString();
            }

            if (exception is null)
            {
                Log(BattlementLogSeverity.Error, eventName, message, fields);
            }
            else
            {
                errors!.Report(
                    BattlementErrorType.CommandFailed,
                    BattlementErrorSource.Unity,
                    eventName,
                    message,
                    exception,
                    fields: fields
                );
            }

            TimeSpan started = configured.Clock.Elapsed;
            int payloadBytes = 0;
            try
            {
                BattlementTransportResult result;
                byte[] json;
                using (BattlementProfiler.Serialization.Auto())
                {
                    json = serialize();
                    payloadBytes = json.Length;
                }
                if (json.Length > BattlementProtocolLimits.MaximumMessageBytes)
                {
                    FailSession(
                        configured,
                        $"A failure message cannot exceed "
                            + $"{BattlementProtocolLimits.MaximumMessageBytes} bytes.",
                        duration: configured.Clock.Elapsed - started,
                        payloadBytes: payloadBytes
                    );
                    return;
                }

                using (BattlementProfiler.Transport.Auto())
                {
                    result = configured.Transport.Submit(json);
                }

                ProcessTransportResult(
                    configured,
                    result,
                    "Failure submission failed.",
                    duration: configured.Clock.Elapsed - started
                );
            }
            catch (Exception submissionException)
            {
                FailSession(
                    configured,
                    $"Failure submission response failed: {submissionException.Message}",
                    duration: configured.Clock.Elapsed - started,
                    payloadBytes: payloadBytes,
                    exception: submissionException
                );
            }
        }

        private static string BoundDiagnostic(string? message)
        {
            if (string.IsNullOrEmpty(message))
            {
                return string.Empty;
            }

            if (Encoding.UTF8.GetByteCount(message) <= MaximumDiagnosticBytes)
            {
                return message;
            }

            int low = 0;
            int high = Math.Min(message.Length, MaximumDiagnosticBytes);
            while (low < high)
            {
                int candidate = low + ((high - low + 1) / 2);
                if (Encoding.UTF8.GetByteCount(message, 0, candidate) <= MaximumDiagnosticBytes)
                {
                    low = candidate;
                }
                else
                {
                    high = candidate - 1;
                }
            }

            if (low > 0 && char.IsHighSurrogate(message[low - 1]))
            {
                low--;
            }

            return message.Substring(0, low);
        }

        private void LogSlowFrame(
            BattlementRunnerOptions configured,
            TimeSpan frameDuration,
            TimeSpan battlementDuration,
            int payloadBytes
        )
        {
            var fields = new Dictionary<string, string>
            {
                ["duration_ms"] = Milliseconds(frameDuration),
                ["battlement_duration_ms"] = Milliseconds(battlementDuration),
                ["payload_bytes"] = payloadBytes.ToString(CultureInfo.InvariantCulture),
            };
            AddSessionField(fields);
            errors!.Log(
                new BattlementLogRecord(
                    BattlementLogSeverity.Warning,
                    "battlement.frame.slow",
                    "Battlement did work during a slow Unity frame.",
                    fields
                )
            );
        }

        private void AddSessionField(IDictionary<string, string> fields)
        {
            if (session.LastSession is not null)
            {
                fields["session_id"] = session.LastSession.Value.Value.ToString();
            }
        }

        private static string Milliseconds(TimeSpan duration) =>
            duration.TotalMilliseconds.ToString("F3", CultureInfo.InvariantCulture);

        private void StopSession(BattlementRunnerOptions configured, bool log)
        {
            pointerInput?.Reset();
            keyboardInput?.Reset();
            controllerInput?.Reset();
            controllerInput?.StopHaptics();
            batchScheduler?.BeginSession();
            particleEffects?.ClearInactive();
            preparedAssets?.CancelPending();
            scenes?.BeginSession();
            world?.BeginSession();
            snapshotReplacement?.Cancel();
            configured.Transport.Stop();
            session.Stop();
            batchAdmission.BeginSession();
            responses.Clear();
            if (log)
            {
                Log(BattlementLogSeverity.Information, "battlement.host.stopped", "Host stopped.");
            }
        }

        private void SetInputEnabled(bool isEnabled)
        {
            session.SetInputEnabled(isEnabled);
            uiDocuments?.SetInputEnabled(isEnabled);
            if (!isEnabled)
            {
                pointerInput?.CancelPresses();
                keyboardInput?.Reset();
                controllerInput?.Reset();
            }
        }

        private bool EmitAction(ActionBody body)
        {
            if (!CanEmitInput || session.LastSession is not SessionId currentSession)
            {
                return false;
            }

            BattlementRunnerOptions configured = RequireOptions();
            byte[] message;
            using (BattlementProfiler.Serialization.Auto())
            {
                message = configured.ProtocolCodec.SerializeAction(
                    new Action(new ActionId(Guid.NewGuid()), currentSession, body)
                );
            }
            Submit(message);
            return CanEmitInput && session.LastSession == currentSession;
        }

        private bool EmitUiEvent(UiEvent value)
        {
            uiDispatchDepth++;
            try
            {
                return EmitAction(new ActionBody.VisualElement(value.TargetId, value.Body));
            }
            finally
            {
                uiDispatchDepth--;
            }
        }

        private bool CanEmitInput =>
            session.IsInputAvailable && (hasApplicationFocus || Application.isBatchMode);

        private void LogPendingConnection()
        {
            (string EventName, string Message)? connection = session.TakeConnectionLog();
            if (connection is null)
            {
                return;
            }

            Log(
                BattlementLogSeverity.Information,
                connection.Value.EventName,
                connection.Value.Message
            );
        }

        private static string PlatformName(RuntimePlatform platform) =>
            platform switch
            {
                RuntimePlatform.OSXEditor or RuntimePlatform.OSXPlayer => "macOS",
                RuntimePlatform.WindowsEditor or RuntimePlatform.WindowsPlayer => "Windows",
                RuntimePlatform.LinuxEditor or RuntimePlatform.LinuxPlayer => "Linux",
                RuntimePlatform.IPhonePlayer => "iOS",
                RuntimePlatform.Android => "Android",
                _ => platform.ToString(),
            };

        private BattlementRunnerOptions RequireOptions()
        {
            if (isDisposed)
            {
                throw new ObjectDisposedException(nameof(BattlementRunner));
            }

            return options
                ?? throw new InvalidOperationException(
                    "Configure the runner with public host dependencies before use."
                );
        }

        private BattlementRunnerOptions RequireRunningSession()
        {
            BattlementRunnerOptions configured = RequireOptions();
            if (session.Phase != BattlementSessionPhase.Running || session.LastSession is null)
            {
                throw new InvalidOperationException(
                    "Failures may only be reported while the runner is active."
                );
            }

            return configured;
        }

        private IBattlementExtensionProtocolCodec RequireExtensionCodec() =>
            RequireOptions().ProtocolCodec as IBattlementExtensionProtocolCodec
            ?? throw new InvalidOperationException(
                "Registered custom code requires an extension-capable protocol codec."
            );

        private void RequireConfiguredAndStopped()
        {
            RequireOptions();
            EnsureMainThread();
            if (session.Phase != BattlementSessionPhase.Stopped)
            {
                throw new InvalidOperationException(
                    "Custom command handlers must be registered before connecting."
                );
            }
        }

        private void EnsureMainThread()
        {
            if (Environment.CurrentManagedThreadId != mainThreadId)
            {
                throw new InvalidOperationException(
                    "Battlement custom code must run on Unity's main thread."
                );
            }
        }

        private void RequireHealthyRuntime()
        {
            if (isRuntimePoisoned)
            {
                throw new InvalidOperationException(
                    "The runtime cannot reconnect after a fatal error. Restart the application."
                );
            }
        }

        private BattlementCommandContext CreateCommandContext(TimeSpan now)
        {
            EnsureMainThread();
            BattlementRunnerOptions configured = RequireOptions();
            return new BattlementCommandContext(
                CancellationToken.None,
                configured.Logger,
                this,
                this,
                new BattlementTweenHelpers(
                    tweens ?? throw new InvalidOperationException("Tween helpers are unavailable."),
                    now
                )
            );
        }

        private void Log(
            BattlementLogSeverity severity,
            string eventName,
            string message,
            IReadOnlyDictionary<string, string>? fields = null
        ) => errors!.Log(new BattlementLogRecord(severity, eventName, message, fields));

        private static bool ReturnMissing(out object? value)
        {
            value = null;
            return false;
        }

        private static bool ReturnMissingObject(out GameObject? value)
        {
            value = null;
            return false;
        }
    }
}

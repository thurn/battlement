#nullable enable

using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using System.Threading;
using MessagePack.Formatters;
using UnityEngine;

namespace Masonry
{
    /// <summary>Scene-authored host for one Masonry session.</summary>
    [DisallowMultipleComponent]
    public sealed class MasonryRunner
        : MonoBehaviour,
            IDisposable,
            IMasonryObjectLookup,
            IMasonryPreparedAssetLookup
    {
        [SerializeField]
        private MasonryTransportKind transportKind;

        [SerializeField]
        private MasonryNativeTransportConfiguration nativeTransport = new();

        [SerializeField]
        private MasonryHttpTransportConfiguration httpTransport = new();

        private MasonryRunnerOptions? options;
        private MasonryWorld? world;
        private MasonryPreparedAssets? preparedAssets;
        private MasonryScenes? scenes;
        private MasonrySnapshotReplacement? snapshotReplacement;
        private MasonryBatchScheduler? batchScheduler;
        private MasonryParticleEffects? particleEffects;
        private MasonryAudioSources? audioSources;
        private MasonryPointerInput? pointerInput;
        private MasonryKeyboardInput? keyboardInput;
        private MasonryCustomCommands? customCommands;
        private MasonryTweenAdapter? tweens;
        private readonly MasonryResponseStream responses = new();
        private readonly MasonrySessionState session = new();
        private readonly MasonryBatchAdmission batchAdmission = new();
        private bool wasPaused;
        private bool hasApplicationFocus = true;
        private bool isDisposed;
        private int mainThreadId;

        private const int MaximumDiagnosticBytes = 65_536;
        private static readonly TimeSpan SlowFrameThreshold = TimeSpan.FromMilliseconds(16.67);

        public MasonryTransportKind TransportKind => transportKind;

        public MasonryNativeTransportConfiguration NativeTransport => nativeTransport;

        public MasonryHttpTransportConfiguration HttpTransport => httpTransport;

        /// <summary>Whether Masonry may currently emit pointer and keyboard input.</summary>
        public bool IsInputAvailable => session.IsInputAvailable;

        /// <summary>Returns whether a global physical key is selected for input dispatch.</summary>
        public bool IsGlobalKeyEnabled(KeyCode key) => world?.IsGlobalKeyEnabled(key) == true;

        /// <summary>Injects the dependencies owned by this runner.</summary>
        public void Configure(MasonryRunnerOptions runnerOptions)
        {
            MasonryRunnerOptions checkedOptions = Errors.CheckNotNull(
                runnerOptions,
                nameof(runnerOptions)
            );

            if (options is not null)
            {
                throw new InvalidOperationException("The runner is already configured.");
            }

            if (isDisposed)
            {
                throw new ObjectDisposedException(nameof(MasonryRunner));
            }

            options = checkedOptions;
            mainThreadId = Environment.CurrentManagedThreadId;
            preparedAssets = new MasonryPreparedAssets(checkedOptions.AssetStorage);
            world = new MasonryWorld(gameObject.scene, preparedAssets);
            pointerInput = new MasonryPointerInput(transform, EmitAction);
            keyboardInput = new MasonryKeyboardInput(IsGlobalKeyEnabled, EmitAction);
            world.InputCameraChanged += pointerInput.SetCamera;
            particleEffects = new MasonryParticleEffects(world, preparedAssets);
            audioSources = new MasonryAudioSources(world, preparedAssets, transform);
            scenes = new MasonryScenes(checkedOptions.AssetStorage, preparedAssets, world);
            snapshotReplacement = new MasonrySnapshotReplacement(preparedAssets, scenes, world);
            var operations = new MasonryOperationRegistry(
                ReportOperationFailure,
                ReportCustomOperationFailure
            );
            tweens = new MasonryTweenAdapter(
                checkedOptions.UseInstantAnimations,
                checkedOptions.Clock is not UnityMasonryClock
            );
            customCommands = new MasonryCustomCommands(now => CreateCommandContext(now));
            var commandExecutor = new MasonryCommandExecutor(
                world,
                preparedAssets,
                scenes,
                operations,
                tweens,
                particleEffects,
                audioSources,
                customCommands,
                SetInputEnabled
            );
            batchScheduler = new MasonryBatchScheduler(
                checkedOptions.Clock,
                commandExecutor,
                operations,
                ReportBatchFailure,
                ReportCustomBatchFailure
            );
        }

        /// <summary>Registers one game-owned command type before connecting.</summary>
        public void RegisterCommand<TPayload, TError>(
            string type,
            IMasonryCommandHandler<TPayload> handler,
            IMessagePackFormatter<TPayload> payloadFormatter,
            IMessagePackFormatter<TError> errorFormatter
        )
        {
            RequireConfiguredAndStopped();
            customCommands!.Register(type, handler, payloadFormatter, errorFormatter);
        }

        /// <summary>Emits a typed game-owned action through the active transport.</summary>
        public ActionId EmitCustomAction<TPayload>(
            string type,
            TPayload payload,
            IMessagePackFormatter<TPayload> payloadFormatter
        )
        {
            EnsureMainThread();
            MasonryCustomCommands.RequireNamespaced(type);
            SessionId currentSession =
                session.LastSession
                ?? throw new InvalidOperationException("No Masonry session is active.");
            if (session.Phase != MasonrySessionPhase.Running)
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
                        Errors.CheckNotNull(payloadFormatter, nameof(payloadFormatter))
                    )
            );
            return actionId;
        }

        /// <summary>Starts the configured host session.</summary>
        public void Connect()
        {
            MasonryRunnerOptions configured = RequireOptions();
            if (session.Phase != MasonrySessionPhase.Stopped)
            {
                throw new InvalidOperationException("The runner is already connected.");
            }

            StartSession(configured, false);
        }

        /// <summary>Stops the current session and starts a new one on the same transport.</summary>
        public void Reconnect()
        {
            MasonryRunnerOptions configured = RequireOptions();
            SessionId? previousSession = session.LastSession;
            if (session.Phase != MasonrySessionPhase.Stopped)
            {
                StopSession(configured, false);
            }

            StartSession(configured, true, previousSession);
        }

        /// <summary>Stops the active session. Repeated calls are no-ops.</summary>
        public void Stop()
        {
            if (session.Phase == MasonrySessionPhase.Stopped || options is null)
            {
                return;
            }

            StopSession(options, true);
        }

        /// <summary>Submits one already encoded client message to the active session.</summary>
        public void Submit(ReadOnlyMemory<byte> messagePack)
        {
            MasonryRunnerOptions configured = RequireOptions();
            if (session.Phase != MasonrySessionPhase.Running)
            {
                throw new InvalidOperationException(
                    "Client messages may only be submitted while the runner is active."
                );
            }

            TimeSpan started = configured.Clock.Elapsed;
            try
            {
                MasonryTransportResult result;
                using (MasonryProfiler.Transport.Auto())
                {
                    result = configured.Transport.Submit(messagePack);
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
                    payloadBytes: messagePack.Length
                );
            }
        }

        /// <summary>Reports a recoverable core failure that stopped one batch.</summary>
        public void ReportBatchFailure(BatchFailed<CoreErrorCode> failure)
        {
            Errors.CheckNotNull(failure, nameof(failure));
            MasonryRunnerOptions configured = RequireRunningSession();
            BatchFailed<CoreErrorCode> bounded = failure with
            {
                Message = BoundDiagnostic(failure.Message),
            };
            SubmitFailure(
                configured,
                () => configured.ProtocolCodec.SerializeBatchFailure(bounded),
                "masonry.batch.failed",
                bounded.Message,
                bounded.SessionId,
                bounded.BatchId,
                bounded.CommandId,
                bounded.ErrorCode
            );
            ThrowReportedFailureInEditor(bounded.Message);
        }

        /// <summary>Reports a late failure from one nonblocking core operation.</summary>
        public void ReportOperationFailure(OperationFailed<CoreErrorCode> failure)
        {
            Errors.CheckNotNull(failure, nameof(failure));
            MasonryRunnerOptions configured = RequireRunningSession();
            OperationFailed<CoreErrorCode> bounded = failure with
            {
                Message = BoundDiagnostic(failure.Message),
            };
            SubmitFailure(
                configured,
                () => configured.ProtocolCodec.SerializeOperationFailure(bounded),
                "masonry.operation.failed",
                bounded.Message,
                bounded.SessionId,
                bounded.BatchId,
                bounded.CommandId,
                bounded.ErrorCode
            );
            ThrowReportedFailureInEditor(bounded.Message);
        }

        private void ReportCustomBatchFailure(
            MasonryRegisteredCommandException exception,
            SessionId sessionId,
            BatchId batchId,
            CommandId? commandId
        )
        {
            MasonryRunnerOptions configured = RequireRunningSession();
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
                "masonry.batch.failed",
                message,
                sessionId,
                batchId,
                commandId,
                exception.ErrorCode
            );
            ThrowReportedFailureInEditor(message);
        }

        private void ReportCustomOperationFailure(
            MasonryRegisteredCommandException exception,
            SessionId sessionId,
            BatchId batchId,
            CommandId commandId
        )
        {
            MasonryRunnerOptions configured = RequireRunningSession();
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
                "masonry.operation.failed",
                message,
                sessionId,
                batchId,
                commandId,
                exception.ErrorCode
            );
            ThrowReportedFailureInEditor(message);
        }

        /// <summary>Looks up an asset without starting an implicit load.</summary>
        public bool TryGetPreparedAsset(PreparedAsset asset, out object? value) =>
            preparedAssets?.TryGet(asset, out value) ?? ReturnMissing(out value);

        bool IMasonryPreparedAssetLookup.TryGet(PreparedAsset asset, out object? value) =>
            TryGetPreparedAsset(asset, out value);

        /// <summary>
        /// Acquires a usage lease that must be disposed when the asset is no longer referenced.
        /// </summary>
        public IMasonryAssetLease AcquirePreparedAsset(PreparedAsset asset) =>
            preparedAssets?.Acquire(asset)
            ?? throw new InvalidOperationException("The runner is not configured.");

        /// <summary>Looks up a live Masonry-controlled Unity object.</summary>
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
                                        world?.Dispose();
                                        isDisposed = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        /// <summary>Advances Masonry work for the current Unity frame.</summary>
        public void RunFrame()
        {
            if (session.Phase == MasonrySessionPhase.Stopped || options is null)
            {
                return;
            }

            MasonryRunnerOptions configured = options;
            AdvanceSnapshotPreparation(configured);
            if (session.Phase == MasonrySessionPhase.Stopped)
            {
                return;
            }

            DrainResponses(configured);
            if (session.Phase == MasonrySessionPhase.Stopped)
            {
                return;
            }

            batchScheduler?.Advance();
            pointerInput?.Update(CanEmitInput);
            keyboardInput?.Update(CanEmitInput);
            if (session.Phase == MasonrySessionPhase.Stopped)
            {
                return;
            }

            TimeSpan started = configured.Clock.Elapsed;
            TimeSpan previous = session.PreviousStepTime ?? started;
            if (started < previous)
            {
                throw new InvalidOperationException("The Masonry clock must be monotonic.");
            }

            int payloadBytes = 0;
            using (MasonryProfiler.Frame.Auto())
            {
                try
                {
                    MasonryTransportResult result;
                    TimeSpan pollStarted = configured.Clock.Elapsed;
                    using (MasonryProfiler.Poll.Auto())
                    using (MasonryProfiler.Transport.Auto())
                    {
                        result = configured.Transport.Poll();
                    }

                    payloadBytes = result.Payload.Length;
                    if (result.Status != MasonryTransportStatus.NoMessage)
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
                        payloadBytes: payloadBytes
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

        private void LateUpdate() => world?.UpdateBillboards();

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
            if (!hasFocus && session.Phase != MasonrySessionPhase.Stopped)
            {
                pointerInput?.CancelPresses();
                keyboardInput?.Reset();
                Log(
                    MasonryLogSeverity.Information,
                    "masonry.input.pointer_presses_cancelled",
                    "Pointer presses were cancelled after application focus loss."
                );
            }
        }

        private void OnApplicationQuit() => Stop();

        private void OnDestroy() => Dispose();

        private void StartSession(
            MasonryRunnerOptions configured,
            bool reconnecting,
            SessionId? previousSession = null
        )
        {
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
                using (MasonryProfiler.Serialization.Auto())
                {
                    bytes = configured.ProtocolCodec.SerializeConnect(connect);
                }

                MasonryTransportResult result;
                using (MasonryProfiler.Transport.Auto())
                {
                    result = configured.Transport.Connect(bytes);
                }

                if (result.Status != MasonryTransportStatus.Success)
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
                if (session.Phase == MasonrySessionPhase.Running)
                {
                    LogPendingConnection();
                }
            }
            catch (Exception exception)
            {
                FailSession(
                    configured,
                    $"Connect response failed: {exception.Message}",
                    duration: configured.Clock.Elapsed - started
                );
            }
        }

        private void ProcessTransportResult(
            MasonryRunnerOptions configured,
            MasonryTransportResult result,
            string failureMessage,
            bool isInitial = false,
            SessionId? previousSession = null,
            TimeSpan? duration = null
        )
        {
            if (result.Status != MasonryTransportStatus.Success)
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
            DrainResponses(configured);
        }

        private Response<ICommand> DecodeResponse(
            MasonryRunnerOptions configured,
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

        private void DrainResponses(MasonryRunnerOptions configured) =>
            responses.Drain(
                (response, isInitial, previousSession) =>
                    ValidateResponse(configured, response, isInitial, previousSession),
                (session, message) => ApplyMessage(configured, session, message),
                () => session.Phase == MasonrySessionPhase.ApplyingSnapshot,
                () => session.Phase == MasonrySessionPhase.Stopped
            );

        private bool ValidateResponse(
            MasonryRunnerOptions configured,
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
                    MasonryLogSeverity.Error,
                    "masonry.response.wrong_session",
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
                    MasonryLogSeverity.Error,
                    "masonry.response.wrong_session",
                    "Discarded a response from a different session."
                );
                return false;
            }

            return true;
        }

        private void ApplyMessage(
            MasonryRunnerOptions configured,
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
            MasonryRunnerOptions configured,
            SessionId responseSession,
            Batch<ICommand> batch
        )
        {
            try
            {
                MasonryBatchAdmissionResult result = batchAdmission.Admit(
                    responseSession,
                    Errors.CheckNotNull(batch, nameof(batch))
                );
                var fields = new Dictionary<string, string>
                {
                    ["batch_id"] = batch.Id.ToString(),
                    ["session_id"] = responseSession.ToString(),
                };
                if (result.IsDuplicate)
                {
                    Log(
                        MasonryLogSeverity.Warning,
                        "masonry.batch.duplicate",
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
                    MasonryLogSeverity.Trace,
                    "masonry.batch.admitted",
                    "Admitted a command batch for scheduling.",
                    fields
                );
                batchScheduler!.Schedule(responseSession, batch, result);
            }
            catch (MasonryBatchAdmissionException exception)
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
            catch (MasonryUnorderableBatchException exception)
            {
                FailSession(configured, exception.Message);
            }
        }

        private Connect BuildConnect(MasonryRunnerOptions configured)
        {
            bool native = configured.Transport.Kind == MasonryTransportKind.Native;
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
            MasonryRunnerOptions configured,
            SessionId responseSession,
            Snapshot snapshot
        )
        {
            try
            {
                pointerInput?.Suspend();
                keyboardInput?.Reset();
                batchScheduler?.CancelForSnapshot();
                particleEffects?.ClearInactive();
                session.BeginSnapshot(responseSession);
                snapshotReplacement!.Begin(responseSession, snapshot);
                AdvanceSnapshotPreparation(configured);
            }
            catch (MasonrySnapshotReplacementException exception)
            {
                FailSession(configured, exception.Message);
            }
        }

        private void AdvanceSnapshotPreparation(MasonryRunnerOptions configured)
        {
            if (session.Phase != MasonrySessionPhase.ApplyingSnapshot)
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
                LogPendingConnection();
            }
            catch (MasonrySnapshotReplacementException exception)
            {
                FailSession(configured, exception.Message);
            }
        }

        private void FailSession(
            MasonryRunnerOptions configured,
            string message,
            MasonryTransportResult? result = null,
            TimeSpan? duration = null,
            int? payloadBytes = null
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
                if (!string.IsNullOrEmpty(result.Diagnostic))
                {
                    fields["diagnostic"] = result.Diagnostic!;
                }
            }
            else if (payloadBytes is not null)
            {
                fields["payload_bytes"] = payloadBytes.Value.ToString(CultureInfo.InvariantCulture);
            }

            Log(MasonryLogSeverity.Error, "masonry.session.failed", message, fields);
            StopSession(configured, false);
        }

        private void SubmitFailure(
            MasonryRunnerOptions configured,
            Func<byte[]> serialize,
            string eventName,
            string message,
            SessionId sessionId,
            BatchId batchId,
            CommandId? commandId,
            object errorCode
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

            Log(MasonryLogSeverity.Error, eventName, message, fields);

            TimeSpan started = configured.Clock.Elapsed;
            int payloadBytes = 0;
            try
            {
                MasonryTransportResult result;
                byte[] messagePack;
                using (MasonryProfiler.Serialization.Auto())
                {
                    messagePack = serialize();
                    payloadBytes = messagePack.Length;
                }

                using (MasonryProfiler.Transport.Auto())
                {
                    result = configured.Transport.Submit(messagePack);
                }

                ProcessTransportResult(
                    configured,
                    result,
                    "Failure submission failed.",
                    duration: configured.Clock.Elapsed - started
                );
            }
            catch (Exception exception)
            {
                FailSession(
                    configured,
                    $"Failure submission response failed: {exception.Message}",
                    duration: configured.Clock.Elapsed - started,
                    payloadBytes: payloadBytes
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

        private static void ThrowReportedFailureInEditor(string message)
        {
#if UNITY_EDITOR
            if (Application.isPlaying)
            {
                throw new InvalidOperationException(message);
            }
#endif
        }

        private void LogSlowFrame(
            MasonryRunnerOptions configured,
            TimeSpan frameDuration,
            TimeSpan masonryDuration,
            int payloadBytes
        )
        {
            var fields = new Dictionary<string, string>
            {
                ["duration_ms"] = Milliseconds(frameDuration),
                ["masonry_duration_ms"] = Milliseconds(masonryDuration),
                ["payload_bytes"] = payloadBytes.ToString(CultureInfo.InvariantCulture),
            };
            AddSessionField(fields);
            configured.Logger.Log(
                new MasonryLogRecord(
                    MasonryLogSeverity.Warning,
                    "masonry.frame.slow",
                    "Masonry did work during a slow Unity frame.",
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

        private void StopSession(MasonryRunnerOptions configured, bool log)
        {
            pointerInput?.Reset();
            keyboardInput?.Reset();
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
                Log(MasonryLogSeverity.Information, "masonry.host.stopped", "Host stopped.");
            }
        }

        private void SetInputEnabled(bool isEnabled)
        {
            session.SetInputEnabled(isEnabled);
            if (!isEnabled)
            {
                pointerInput?.CancelPresses();
                keyboardInput?.Reset();
            }
        }

        private bool EmitAction(ActionBody body)
        {
            if (!CanEmitInput || session.LastSession is not SessionId currentSession)
            {
                return false;
            }

            MasonryRunnerOptions configured = RequireOptions();
            byte[] message;
            using (MasonryProfiler.Serialization.Auto())
            {
                message = configured.ProtocolCodec.SerializeAction(
                    new Action(new ActionId(Guid.NewGuid()), currentSession, body)
                );
            }

            Submit(message);
            return CanEmitInput && session.LastSession == currentSession;
        }

        private bool CanEmitInput => session.IsInputAvailable && hasApplicationFocus;

        private void LogPendingConnection()
        {
            (string EventName, string Message)? connection = session.TakeConnectionLog();
            if (connection is null)
            {
                return;
            }

            Log(
                MasonryLogSeverity.Information,
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

        private MasonryRunnerOptions RequireOptions()
        {
            if (isDisposed)
            {
                throw new ObjectDisposedException(nameof(MasonryRunner));
            }

            return options
                ?? throw new InvalidOperationException(
                    "Configure the runner with public host dependencies before use."
                );
        }

        private MasonryRunnerOptions RequireRunningSession()
        {
            MasonryRunnerOptions configured = RequireOptions();
            if (session.Phase != MasonrySessionPhase.Running || session.LastSession is null)
            {
                throw new InvalidOperationException(
                    "Failures may only be reported while the runner is active."
                );
            }

            return configured;
        }

        private IMasonryExtensionProtocolCodec RequireExtensionCodec() =>
            RequireOptions().ProtocolCodec as IMasonryExtensionProtocolCodec
            ?? throw new InvalidOperationException(
                "Registered custom code requires an extension-capable protocol codec."
            );

        private void RequireConfiguredAndStopped()
        {
            RequireOptions();
            EnsureMainThread();
            if (session.Phase != MasonrySessionPhase.Stopped)
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
                    "Masonry custom code must run on Unity's main thread."
                );
            }
        }

        private MasonryCommandContext CreateCommandContext(TimeSpan now)
        {
            EnsureMainThread();
            MasonryRunnerOptions configured = RequireOptions();
            return new MasonryCommandContext(
                CancellationToken.None,
                configured.Logger,
                this,
                this,
                new MasonryTweenHelpers(
                    tweens ?? throw new InvalidOperationException("Tween helpers are unavailable."),
                    now
                )
            );
        }

        private void Log(
            MasonryLogSeverity severity,
            string eventName,
            string message,
            IReadOnlyDictionary<string, string>? fields = null
        ) =>
            RequireOptions().Logger.Log(new MasonryLogRecord(severity, eventName, message, fields));

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

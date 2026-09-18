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
            IBattlementUiAssetLookup,
            IBattlementGeometryWorldSource,
            IBattlementUiEventActivation
    {
        [SerializeField]
        private bool showLoadingSurface = true;

        [SerializeField]
        private bool runnerDiagnostics = true;

        [SerializeField]
        private List<BattlementModule> selectedModules = new();

        private BattlementConfiguredRuntime? configuredRuntime;
        private BattlementWorldFocusInput? worldFocus;
        private readonly BattlementGeometryFrames geometryFrames = new();
        private readonly BattlementResponseStream responses = new();
        private readonly BattlementSessionState session = new();
        private readonly BattlementBatchAdmission batchAdmission = new();
        private readonly ConcurrentQueue<BattlementCapturedUnityError> unityErrors = new();
        private readonly BattlementConnectRequestWriter connectRequests = new();
        private readonly BattlementCoreClientMessageWriter coreRequests = new();
        private bool isApplicationPaused;
        private bool hasApplicationFocus = true;
        private bool dittoInputActive;
        private BattlementControlledPointerLease? dittoPointerLease;
        private DittoActivationTransaction? dittoActivationTransaction;
        private ApplicationState? publishedApplicationState;
        private ReducedMotionPreference publishedReducedMotionPreference;
        private bool isDisposed;
        private bool isNativePanicRecovery;
        private bool isRuntimePoisoned;
        private bool completedInitialSnapshot;
        private int mainThreadId;
        private ulong dittoStateVersion;
        private ulong dittoResponseDecodeNs;
        private ulong dittoResponseApplyNs;
        private PendingUiFailure? pendingUiFailure;

        internal Action<string>? ConfigureConstructionProbe { get; set; }

        private const int MaximumDiagnosticBytes = 65_536;
        private static readonly TimeSpan SlowFrameThreshold = TimeSpan.FromMilliseconds(16.67);

        /// <summary>Whether Battlement renders its built-in loading and failure surface.</summary>
        public bool ShowLoadingSurface => showLoadingSurface;

        /// <summary>Whether builds containing this runner include automation diagnostics.</summary>
        public bool RunnerDiagnostics => runnerDiagnostics;

        /// <summary>
        /// Whether Battlement may currently emit pointer, keyboard, and controller input.
        /// </summary>
        public bool IsInputAvailable => pendingUiFailure is null && session.IsInputAvailable;

        /// <summary>Current nontechnical failure presentation, if the session failed.</summary>
        public BattlementPlayerFailure? CurrentFailure => configuredRuntime?.FailureSurface.Current;

        /// <summary>Whether an unknown runtime failure requires an application restart.</summary>
        public bool IsRestartRequired => isRuntimePoisoned;

        /// <summary>Returns diagnostics for the most recently presented Motion frame.</summary>
        public BattlementMotionPerformanceSnapshot MotionPerformance =>
            configuredRuntime?.UiDocuments.MotionPerformance ?? default;

        /// <summary>Recent causal records for synchronous Reactant UI events.</summary>
        public IReadOnlyList<BattlementUiEventInspection> UiEventInspections =>
            configuredRuntime?.UiEventDispatcher.Inspections
            ?? Array.Empty<BattlementUiEventInspection>();

        internal System.Action? SnapshotApplicationProbe
        {
            set => configuredRuntime!.SnapshotReplacement.ApplicationProbe = value;
        }

        internal BattlementUiDocuments UiDocumentsForTests => configuredRuntime!.UiDocuments;

        internal BattlementPanelInputCoordinator PanelInputForTests =>
            configuredRuntime!.PanelInput;

        internal BattlementIdentity? PickWorldPointer(UnityEngine.Vector2 position) =>
            configuredRuntime?.PointerInput.PickWorld(0, position);

        internal Camera? DittoInputCamera => configuredRuntime?.World.InputCamera;

        internal BattlementUiDocuments DittoUiDocuments => configuredRuntime!.UiDocuments;

        internal TimeSpan DittoElapsed => configuredRuntime!.DittoMotionClock.Elapsed;

        internal void SetDittoFrameRate(uint framesPerSecond) =>
            configuredRuntime!.DittoMotionClock.SetFramesPerSecond(framesPerSecond);

        internal bool IsDittoConfigured => configuredRuntime is not null;

        internal string? DittoInputDiagnostic =>
            configuredRuntime?.PointerInput.ControlledFailure is string controlledFailure
                ? controlledFailure
            : CanEmitInput ? null
            : $"Runner input is unavailable: focused={hasApplicationFocus}, "
                + $"paused={isApplicationPaused}, sessionInput={session.IsInputAvailable}, "
                + $"pendingUiFailure={pendingUiFailure is not null}.";

        internal BattlementControlledPointerLease BeginDittoInput(string? sessionId = null)
        {
            if (dittoInputActive)
                throw new InvalidOperationException(
                    "A Ditto executor already owns this runner's input."
                );
            BattlementPointerInput pointer =
                configuredRuntime?.PointerInput
                ?? throw new InvalidOperationException("Ditto input requires a configured runner.");
            dittoPointerLease = pointer.BeginDittoControl(sessionId ?? Guid.NewGuid().ToString());
            configuredRuntime?.KeyboardInput.Reset();
            configuredRuntime?.ControllerInput.Reset();
            dittoInputActive = true;
            return dittoPointerLease;
        }

        internal void EndDittoInput()
        {
            dittoActivationTransaction = null;
            try
            {
                if (dittoPointerLease is BattlementControlledPointerLease lease)
                    configuredRuntime?.PointerInput.EndDittoControl(lease);
            }
            finally
            {
                dittoPointerLease = null;
                dittoInputActive = false;
            }
        }

        internal void EnqueueDittoPointerSample(
            ulong sequence,
            int pointerId,
            UnityEngine.Vector2 position,
            IReadOnlyCollection<PointerButton> buttons,
            bool isPresent,
            bool isCancelled,
            ObjectId? expectedTarget,
            ulong presentationBoundary
        )
        {
            EnsureMainThread();
            BattlementControlledPointerLease lease =
                dittoPointerLease
                ?? throw new InvalidOperationException("Ditto input is not leased.");
            configuredRuntime!.PointerInput.EnqueueControlled(
                new BattlementControlledPointerSample(
                    lease,
                    sequence,
                    pointerId,
                    configuredRuntime.PointerInput.ControlledSpace,
                    position,
                    buttons,
                    isPresent,
                    isCancelled,
                    expectedTarget,
                    presentationBoundary
                )
            );
        }

        internal IReadOnlyList<BattlementControlledPointerReceipt> TakeDittoPointerReceipts()
        {
            EnsureMainThread();
            BattlementControlledPointerLease lease =
                dittoPointerLease
                ?? throw new InvalidOperationException("Ditto input is not leased.");
            return configuredRuntime!.PointerInput.TakeControlledReceipts(lease);
        }

        internal void BeginDittoActivationTransaction(
            string transactionId,
            ObjectId target,
            ulong committedFrame
        )
        {
            EnsureMainThread();
            if (!dittoInputActive || dittoActivationTransaction is not null)
                throw new InvalidOperationException(
                    "Ditto activation transaction ownership is unavailable."
                );
            dittoActivationTransaction = new DittoActivationTransaction(
                transactionId,
                target,
                committedFrame
            );
        }

        internal bool CompleteDittoActivationTransaction(
            string transactionId,
            ulong committedFrame,
            out DittoActivationReceipt? receipt,
            out string? diagnostic
        )
        {
            EnsureMainThread();
            DittoActivationTransaction transaction =
                dittoActivationTransaction
                ?? throw new InvalidOperationException(
                    "No Ditto activation transaction is active."
                );
            if (transaction.Id != transactionId)
                throw new InvalidOperationException(
                    "Ditto activation transaction identity changed."
                );
            dittoActivationTransaction = null;
            return transaction.Complete(committedFrame, out receipt, out diagnostic);
        }

        internal bool ValidateDittoActivationDelivery(string transactionId, out string? diagnostic)
        {
            EnsureMainThread();
            DittoActivationTransaction transaction =
                dittoActivationTransaction
                ?? throw new InvalidOperationException(
                    "No Ditto activation transaction is active."
                );
            if (transaction.Id != transactionId)
                throw new InvalidOperationException(
                    "Ditto activation transaction identity changed."
                );
            return transaction.ValidateDelivery(out diagnostic);
        }

        internal void RejectDittoActivationTransaction(string reason) =>
            dittoActivationTransaction?.Reject(reason);

        internal void CancelDittoActivationTransaction() => dittoActivationTransaction = null;

        bool IBattlementUiEventActivation.TryBegin(UiEventAction action, out string? route)
        {
            route = null;
            return dittoActivationTransaction?.TryBeginUiDispatch(
                    action.Id,
                    action.Event.TargetId,
                    action.Event.Body,
                    out route
                ) == true;
        }

        void IBattlementUiEventActivation.ObserveHandled(UiEventAction action, string route)
        {
            Debug.Log(
                $"[Battlement/Ditto-trace] activation-acknowledgement "
                    + $"route={route} object={action.Event.TargetId.Value} "
                    + $"active={dittoActivationTransaction is not null}"
            );
            dittoActivationTransaction?.ObserveHandled(action.Id, action.Event.TargetId, route);
        }

        void IBattlementUiEventActivation.Reject(string reason) =>
            dittoActivationTransaction?.Reject(reason);

        internal bool DispatchDittoActivation(ObjectId target, out string? diagnostic)
        {
            EnsureMainThread();
            DittoActivationTransaction transaction =
                dittoActivationTransaction
                ?? throw new InvalidOperationException(
                    "No Ditto activation transaction is active."
                );
            var actionId = new ActionId(Guid.NewGuid());
            transaction.BeginDispatch(actionId, "world-activate");
            if (EmitAction(new ActionBody.Activate(target), actionId))
            {
                diagnostic = null;
                return true;
            }
            diagnostic = $"Activation of {target.Value} was rejected before dispatch.";
            return false;
        }

        internal bool DispatchDittoNavigation(DittoNavigationAction action)
        {
            EnsureMainThread();
            return CanEmitInput && worldFocus?.Dispatch(action) == true;
        }

        internal BattlementNativeTransport DittoNativeTransport =>
            RequireOptions().Transport as BattlementNativeTransport
            ?? throw new InvalidOperationException("Ditto requires the native transport.");

        internal void BeginDittoMotion(DittoMotion motion)
        {
            EnsureMainThread();
            configuredRuntime!.DittoMotionClock.Begin(motion);
        }

        internal TimeSpan PrepareDittoFrame(bool advance = true)
        {
            EnsureMainThread();
            return configuredRuntime!.DittoMotionClock.PrepareFrame(advance);
        }

        internal void CompleteDittoPresentedFrame()
        {
            EnsureMainThread();
            int completed = configuredRuntime?.UiDocuments.CompleteDittoPresentedFrame() ?? 0;
            dittoStateVersion += checked((ulong)completed);
        }

        internal DittoWorkObservation ObserveDittoWork()
        {
            EnsureMainThread();
            int finiteMotion =
                (configuredRuntime?.UiDocuments.DittoActiveFiniteTimelineCount ?? 0)
                + (configuredRuntime?.BatchScheduler.FiniteOperationCount ?? 0);
            int infiniteMotion =
                (configuredRuntime?.UiDocuments.DittoActiveInfiniteTimelineCount ?? 0)
                + (configuredRuntime?.BatchScheduler.InfiniteOperationCount ?? 0);
            int heldMotion =
                (configuredRuntime?.UiDocuments.DittoActiveHeldTimelineCount ?? 0)
                + (configuredRuntime?.BatchScheduler.HeldOperationCount ?? 0);
            bool deferredUi = configuredRuntime?.UiDocuments.DittoHasPendingDeferredWork == true;
            return new DittoWorkObservation(
                dittoStateVersion + (configuredRuntime?.BatchScheduler.ActivityVersion ?? 0),
                configuredRuntime?.UiDocuments.DittoLayoutFingerprint() ?? 0,
                responses.HasPending
                    || configuredRuntime?.SnapshotReplacement.IsPending == true
                    || configuredRuntime?.BatchScheduler.HasPendingWork == true
                    || geometryFrames.HasPending
                    || finiteMotion != 0
                    || deferredUi,
                configuredRuntime?.BatchScheduler.HasInfiniteOperations == true
                    || infiniteMotion != 0,
                heldMotion != 0,
                finiteMotion,
                infiniteMotion,
                heldMotion,
                deferredUi,
                configuredRuntime?.UiDocuments.DittoActiveTimelineDiagnostic ?? ""
            );
        }

        internal void BeginDittoReset()
        {
            BattlementRunnerOptions configured = RequireOptions();
            EnsureMainThread();
            Exception? failure = null;
            Reset(() => configuredRuntime?.UiDocuments.SetInputEnabled(false), ref failure);
            Reset(() => configuredRuntime?.PointerInput.Reset(), ref failure);
            Reset(() => configuredRuntime?.KeyboardInput.Reset(), ref failure);
            Reset(() => configuredRuntime?.ControllerInput.Reset(), ref failure);
            Reset(() => configuredRuntime?.ControllerInput.StopHaptics(), ref failure);
            Reset(() => configuredRuntime?.BatchScheduler.BeginSession(), ref failure);
            Reset(() => configuredRuntime?.GeometrySampler.Reset(), ref failure);
            Reset(geometryFrames.Reset, ref failure);
            Reset(() => configuredRuntime?.SnapshotReplacement.Cancel(), ref failure);
            Reset(() => configuredRuntime?.Scenes.BeginSession(), ref failure);
            Reset(() => configuredRuntime?.World.BeginSession(), ref failure);
            Reset(() => configuredRuntime?.ParticleEffects.ClearInactive(), ref failure);
            Reset(
                () => configuredRuntime?.AudioSources.ClearInactive(clearSuppressed: true),
                ref failure
            );
            Reset(() => configuredRuntime?.PanelInput.Clear(), ref failure);
            Reset(() => configuredRuntime?.PreparedAssets.BeginSession(), ref failure);
            Reset(() => configuredRuntime?.Modules.Dispose(), ref failure);
            Reset(() => configuredRuntime?.DevelopmentDiagnostics?.Hide(), ref failure);
            Reset(
                () => configuredRuntime?.FailureSurface.Clear(configuredRuntime.Errors),
                ref failure
            );
            session.Stop();
            batchAdmission.BeginSession();
            responses.Clear();
            configuredRuntime?.UiEventDispatcher.Clear();
            while (unityErrors.TryDequeue(out _)) { }
            completedInitialSnapshot = false;
            isNativePanicRecovery = false;
            isRuntimePoisoned = false;
            configuredRuntime?.DittoMotionClock.Reset();
            dittoStateVersion++;
            if (failure is not null)
            {
                throw new InvalidOperationException(
                    "Battlement-owned state failed to reset.",
                    failure
                );
            }
        }

        internal bool TryCompleteDittoReset(out Exception? error)
        {
            EnsureMainThread();
            error = null;
            BattlementAssetException? sceneError = null;
            if (
                configuredRuntime is not null
                && !configuredRuntime.Scenes.TryCompleteSessionReset(out sceneError)
            )
            {
                return false;
            }
            if (sceneError is not null)
            {
                error = sceneError;
                return true;
            }
            if (configuredRuntime?.PreparedAssets.IsSessionEmpty == false)
            {
                error = new InvalidOperationException(
                    "Battlement-owned asset leases remained after scene reset."
                );
            }
            return true;
        }

        Camera? IBattlementGeometryWorldSource.InputCamera => configuredRuntime?.World.InputCamera;

        BattlementGeometryObjectKind IBattlementGeometryWorldSource.LookupObject(
            ObjectId id,
            out GameObject? gameObject
        )
        {
            if (configuredRuntime?.World.TryGetObject(id, out gameObject) == true)
                return BattlementGeometryObjectKind.World;
            gameObject = null;
            return configuredRuntime?.UiDocuments.TryGet(id, out _) == true
                ? BattlementGeometryObjectKind.Ui
                : BattlementGeometryObjectKind.Missing;
        }

        /// <summary>Returns whether a global physical key is selected for input dispatch.</summary>
        public bool IsGlobalKeyEnabled(PhysicalKey key) =>
            configuredRuntime?.World.IsGlobalKeyEnabled(key) == true;

        /// <summary>Injects the dependencies owned by this runner.</summary>
        public void Configure(BattlementRunnerOptions runnerOptions)
        {
            BattlementRunnerOptions checkedOptions = Preconditions.CheckNotNull(
                runnerOptions,
                nameof(runnerOptions)
            );

            if (configuredRuntime is not null)
            {
                throw new InvalidOperationException("The runner is already configured.");
            }

            if (isDisposed)
            {
                throw new ObjectDisposedException(nameof(BattlementRunner));
            }

            if (checkedOptions.Transport is BattlementNativeTransport nativeTransport)
            {
                nativeTransport.SetExpectedWireContractDigest(
                    checkedOptions.FlatBufferResponseSchema?.WireContractDigest
                        ?? BattlementNativeContract.WireContractDigest
                );
            }

            BattlementConfiguredRuntime runtime = new BattlementConfiguredRuntime(checkedOptions);
            configuredRuntime = runtime;
            try
            {
                BattlementDevelopmentDiagnostics? developmentDiagnostics = null;
                if (!checkedOptions.SuppressDevelopmentErrorDialogs)
                {
                    developmentDiagnostics = new BattlementDevelopmentDiagnostics(
                        transform,
                        ContinueAfterFailure
                    );
                }
                runtime.SetDevelopmentDiagnostics(developmentDiagnostics);
                System.Action<BattlementError>? showDevelopmentError = developmentDiagnostics
                    is null
                    ? null
                    : developmentDiagnostics.Show;
                BattlementErrorReporter errors = new BattlementErrorReporter(
                    checkedOptions.Logger,
                    checkedOptions.ErrorSink,
                    showDevelopmentError,
                    checkedOptions.CaughtFailureReporter
                );
                runtime.SetErrors(errors);
                BattlementFailureSurface failureSurface = new BattlementFailureSurface(
                    transform,
                    showLoadingSurface,
                    checkedOptions.FailurePresenter,
                    ContinueAfterFailure,
                    () => developmentDiagnostics?.IsVisible == true
                );
                runtime.SetFailureSurface(failureSurface);
                IDisposable unityErrorSubscription = BattlementUnityErrors.Subscribe(
                    unityErrors.Enqueue
                );
                runtime.SetUnityErrorSubscription(unityErrorSubscription);
                mainThreadId = Environment.CurrentManagedThreadId;
                BattlementPreparedAssets preparedAssets = new BattlementPreparedAssets(
                    checkedOptions.AssetStorage
                );
                runtime.SetPreparedAssets(preparedAssets);
                DittoMotionClock dittoMotionClock = new DittoMotionClock(checkedOptions.Clock);
                runtime.SetDittoMotionClock(dittoMotionClock);
                BattlementWorld world = new BattlementWorld(gameObject.scene, preparedAssets);
                runtime.SetWorld(world);
                BattlementPointerInput pointerInput = new BattlementPointerInput(
                    transform,
                    EmitAction
                );
                runtime.SetPointerInput(pointerInput);
                BattlementPanelInputCoordinator panelInput = new BattlementPanelInputCoordinator();
                runtime.SetPanelInput(panelInput);
                BattlementKeyboardInput keyboardInput = new BattlementKeyboardInput(
                    key => IsGlobalKeyEnabled(key) || worldFocus?.Enables(key) == true,
                    EmitAction
                );
                runtime.SetKeyboardInput(keyboardInput);
                BattlementControllerInput controllerInput = new BattlementControllerInput(
                    () => worldFocus?.Settings(world.ControllerInput) ?? world.ControllerInput,
                    () => pointerInput.NavigationTiming,
                    EmitAction
                );
                runtime.SetControllerInput(controllerInput);
                world.InputCameraChanged += pointerInput.SetCamera;
                BattlementParticleEffects particleEffects = new BattlementParticleEffects(
                    world,
                    preparedAssets,
                    dittoMotionClock
                );
                runtime.SetParticleEffects(particleEffects);
                BattlementAudioSources audioSources = new BattlementAudioSources(
                    world,
                    preparedAssets,
                    transform,
                    dittoMotionClock
                );
                runtime.SetAudioSources(audioSources);
                BattlementScenes scenes = new BattlementScenes(
                    checkedOptions.AssetStorage,
                    preparedAssets,
                    world
                );
                runtime.SetScenes(scenes);
                BattlementCustomCommands customCommands = new BattlementCustomCommands(now =>
                    CreateCommandContext(now)
                );
                runtime.SetCustomCommands(customCommands);
                BattlementUiEventDispatcher uiEventDispatcher = new BattlementUiEventDispatcher(
                    checkedOptions.Transport,
                    responses,
                    dittoMotionClock,
                    (payload, owner) => DecodeResponse(checkedOptions, payload, owner),
                    RecordUiFailure,
                    (severity, eventName, message) => Log(severity, eventName, message),
                    this
                );
                runtime.SetUiEventDispatcher(uiEventDispatcher);
                BattlementUiDocuments uiDocuments = new BattlementUiDocuments(
                    EmitUiEvent,
                    world.ContainsLiveObject,
                    world.ReserveUiIdentities,
                    world.ReleaseUiIdentities,
                    this,
                    () => dittoMotionClock.Elapsed,
                    audioSources.MotionTime,
                    uiEventDispatcher.RecordNativePrevention,
                    () =>
                        dittoMotionClock.IsControlled || dittoMotionClock.IsInstant
                            ? dittoMotionClock.Elapsed
                            : TimeSpan.FromSeconds(Time.timeAsDouble),
                    () => dittoMotionClock.IsInstant
                );
                runtime.SetUiDocuments(uiDocuments);
                worldFocus = new BattlementWorldFocusInput(
                    world,
                    uiDocuments.Navigation,
                    uiDocuments.HasPointerModal,
                    EmitUiEvent
                );
                pointerInput.ConfigureLogical(
                    EmitUiEvent,
                    (id, position) =>
                        uiDocuments.BlocksWorldPointer(
                            BattlementPointerDevices.UiPointerId(id),
                            position
                        ),
                    uiDocuments.HasPointerModal
                );
                pointerInput.ConfigureControlledUi(
                    uiDocuments.ProcessControlledPointer,
                    uiDocuments.ResetControlledPointer
                );
                uiDocuments.SetWorldCaptureResolver(pointerInput.IsWorldCaptured);
                BattlementGeometrySampler geometrySampler = new BattlementGeometrySampler(
                    uiDocuments,
                    world: this
                );
                runtime.SetGeometrySampler(geometrySampler);
                BattlementSnapshotReplacement snapshotReplacement =
                    new BattlementSnapshotReplacement(
                        preparedAssets,
                        scenes,
                        world,
                        uiDocuments,
                        panelInput,
                        BattlementReactantAssetCatalog.Load()
                    );
                runtime.SetSnapshotReplacement(snapshotReplacement);
                ConfigureConstructionProbe?.Invoke("after-snapshot-replacement");
                var operations = new BattlementOperationRegistry(
                    (failure, exception) => ReportOperationFailure(failure, exception),
                    ReportCustomOperationFailure
                );
                BattlementTweenAdapter tweens = new BattlementTweenAdapter(
                    checkedOptions.UseInstantAnimations,
                    checkedOptions.Clock is not UnityBattlementClock,
                    dittoMotionClock
                );
                runtime.SetTweens(tweens);
                BattlementModules modules = new BattlementModules(selectedModules);
                runtime.SetModules(modules);
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
                    dittoMotionClock,
                    SetInputEnabled,
                    uiDocuments,
                    ApplyGeometryObservations,
                    ApplyGeometryObservations,
                    modules,
                    checkedOptions.OpenExternalUrl
                );
                BattlementBatchScheduler batchScheduler = new BattlementBatchScheduler(
                    dittoMotionClock,
                    commandExecutor,
                    operations,
                    (failure, exception) => ReportBatchFailure(failure, exception),
                    ReportCustomBatchFailure
                );
                runtime.SetBatchScheduler(batchScheduler);
            }
            catch
            {
                try
                {
                    runtime.Dispose();
                }
                catch
                {
                    isDisposed = true;
                    throw;
                }
                finally
                {
                    configuredRuntime = null;
                }

                throw;
            }
        }

        /// <summary>Registers a generated, synchronously borrowed custom-command payload.</summary>
        public void RegisterFlatBufferCommand<TPayloadView, TError>(
            string type,
            IBattlementFlatBufferCommandHandler<TPayloadView> handler
        )
        {
            RequireConfiguredAndStopped();
            configuredRuntime!.CustomCommands.RegisterFlatBuffer<TPayloadView, TError>(
                type,
                handler
            );
        }

        /// <summary>Emits a typed game-owned action through the active transport.</summary>
        public ActionId EmitCustomAction<TPayload>(string type, TPayload payload)
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
            var action = new CustomAction<TPayload>(actionId, currentSession, type, payload);
            Submit(RequireFlatBufferClientSchema().SerializeCustomAction(action));
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
                configuredRuntime!.FailureSurface.Current?.Kind
                == BattlementPlayerFailureKind.ContinueAllowed;
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
            if (
                configuredRuntime?.FailureSurface.Current?.Kind
                != BattlementPlayerFailureKind.ContinueAllowed
            )
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
            if (session.Phase == BattlementSessionPhase.Stopped || configuredRuntime is null)
            {
                return;
            }

            StopSession(configuredRuntime.Options, true);
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
        public void Submit(ReadOnlyMemory<byte> message)
        {
            BattlementRunnerOptions configured = RequireOptions();
            if (session.Phase != BattlementSessionPhase.Running)
            {
                throw new InvalidOperationException(
                    "Client messages may only be submitted while the runner is active."
                );
            }
            if (message.Length > BattlementProtocolLimits.MaximumMessageBytes)
            {
                FailSession(
                    configured,
                    $"A client message cannot exceed "
                        + $"{BattlementProtocolLimits.MaximumMessageBytes} bytes.",
                    payloadBytes: message.Length
                );
                return;
            }

            TimeSpan started = configuredRuntime!.DittoMotionClock.Elapsed;
            try
            {
                BattlementTransportResult result;
                using (BattlementProfiler.Transport.Auto())
                {
                    result = configured.Transport.Submit(message);
                }

                ProcessTransportResult(
                    configured,
                    result,
                    "Submit failed.",
                    duration: configuredRuntime.DittoMotionClock.Elapsed - started
                );
            }
            catch (Exception exception)
            {
                FailSession(
                    configured,
                    $"Submit response failed: {exception.Message}",
                    duration: configuredRuntime.DittoMotionClock.Elapsed - started,
                    payloadBytes: message.Length,
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
                () =>
                {
                    RecordBatchFailure(configured, bounded);
                    return coreRequests.WriteBatchFailure(bounded);
                },
                native =>
                {
                    RecordBatchFailure(configured, bounded);
                    return native.Submit(bounded);
                },
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
                () =>
                {
                    RecordOperationFailure(configured, bounded);
                    return coreRequests.WriteOperationFailure(bounded);
                },
                native =>
                {
                    RecordOperationFailure(configured, bounded);
                    return native.Submit(bounded);
                },
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
                    exception.Registration.SerializeFlatBufferBatchFailure(
                        RequireFlatBufferClientSchema(),
                        sessionId,
                        batchId,
                        commandId,
                        exception.ErrorCode,
                        message
                    ),
                null,
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
                    exception.Registration.SerializeFlatBufferOperationFailure(
                        RequireFlatBufferClientSchema(),
                        sessionId,
                        batchId,
                        commandId,
                        exception.ErrorCode,
                        message
                    ),
                null,
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
            configuredRuntime?.PreparedAssets.TryGet(asset, out value) ?? ReturnMissing(out value);

        bool IBattlementPreparedAssetLookup.TryGet(PreparedAsset asset, out object? value) =>
            TryGetPreparedAsset(asset, out value);

        IBattlementUiAssetLease IBattlementUiAssetLookup.Acquire(PreparedAsset asset) =>
            AcquirePreparedAsset(asset);

        /// <summary>
        /// Acquires a usage lease that must be disposed when the asset is no longer referenced.
        /// </summary>
        public IBattlementAssetLease AcquirePreparedAsset(PreparedAsset asset) =>
            configuredRuntime?.PreparedAssets.Acquire(asset)
            ?? throw new InvalidOperationException("The runner is not configured.");

        /// <summary>Looks up a live Battlement-controlled Unity object.</summary>
        public bool TryGetObject(ObjectId id, out GameObject? gameObject) =>
            configuredRuntime?.World.TryGetObject(id, out gameObject)
            ?? ReturnMissingObject(out gameObject);

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
                configuredRuntime?.Dispose();
            }
            finally
            {
                configuredRuntime = null;
                isDisposed = true;
            }
        }

        /// <summary>Advances Battlement work for the current Unity frame.</summary>
        public void RunFrame()
        {
            if (configuredRuntime is null)
            {
                return;
            }

            BattlementRunnerOptions configured = configuredRuntime.Options;
            DrainUnityErrors();
            ApplyPendingUiFailure(configured);
            if (session.Phase == BattlementSessionPhase.Stopped)
            {
                return;
            }
            AdvanceSnapshotPreparation(configured);
            if (session.Phase == BattlementSessionPhase.Stopped)
            {
                return;
            }

            if (configuredRuntime!.UiEventDispatcher.IsDispatching != true)
                DrainResponses(configured);
            if (session.Phase == BattlementSessionPhase.Stopped)
            {
                return;
            }

            configuredRuntime.BatchScheduler.Advance();
            configuredRuntime.UiDocuments.Advance();
            worldFocus?.Refresh(CanEmitInput);
            PublishApplicationState();
            PublishReducedMotionPreference();
            bool physicalInputAvailable = CanEmitInput && !dittoInputActive;
            configuredRuntime.PointerInput.Update(CanEmitInput, !dittoInputActive);
            configuredRuntime.KeyboardInput.Update(physicalInputAvailable);
            configuredRuntime.ControllerInput.Update(
                physicalInputAvailable,
                configuredRuntime.DittoMotionClock.Elapsed
            );
            if (session.Phase == BattlementSessionPhase.Stopped)
            {
                return;
            }

            TimeSpan started = configuredRuntime.DittoMotionClock.Elapsed;
            TimeSpan previous = session.PreviousStepTime ?? started;
            if (started < previous)
            {
                throw new InvalidOperationException("The Battlement clock must be monotonic.");
            }

            if (dittoActivationTransaction?.HasDispatched == true)
            {
                session.PreviousStepTime = configuredRuntime.DittoMotionClock.Elapsed;
                if (!Application.isPlaying)
                    configuredRuntime.World.UpdateBillboards();
                return;
            }

            int payloadBytes = 0;
            using (BattlementProfiler.Frame.Auto())
            {
                try
                {
                    MotionEventBatch? motion = configuredRuntime.UiDocuments.TakeMotionEvents();
                    GeometryObservationBatch? geometry = motion is null
                        ? geometryFrames.Take()
                        : null;
                    BattlementTransportResult result;
                    TimeSpan exchangeStarted = configuredRuntime.DittoMotionClock.Elapsed;
                    if (motion is null && geometry is null)
                    {
                        using (BattlementProfiler.Poll.Auto())
                        using (BattlementProfiler.Transport.Auto())
                            result = configured.Transport.Poll();
                    }
                    else
                    {
                        var action = new Action(
                            new ActionId(Guid.NewGuid()),
                            session.LastSession
                                ?? throw new InvalidOperationException(
                                    "No Battlement session is active."
                                ),
                            motion is not null
                                ? new ActionBody.MotionEvents(motion)
                                : new ActionBody.GeometryObservations(geometry!)
                        );
                        int messageLength;
                        ReadOnlyMemory<byte> message = default;
                        using (BattlementProfiler.Serialization.Auto())
                        {
                            if (configured.Transport is BattlementNativeTransport native)
                            {
                                RecordCoreAction(configured, action);
                                message = native.WriteCore(action);
                                messageLength = message.Length;
                            }
                            else
                            {
                                message = coreRequests.WriteAction(action);
                                RecordCoreAction(configured, action);
                                messageLength = message.Length;
                            }
                        }
                        if (motion is not null)
                            configuredRuntime.UiDocuments.RecordMotionTraffic(messageLength);
                        if (messageLength > BattlementProtocolLimits.MaximumMessageBytes)
                        {
                            FailSession(
                                configured,
                                $"A client message cannot exceed "
                                    + $"{BattlementProtocolLimits.MaximumMessageBytes} bytes.",
                                payloadBytes: messageLength
                            );
                            return;
                        }
                        using (BattlementProfiler.Transport.Auto())
                        {
                            try
                            {
                                result = configured.Transport.Submit(message);
                            }
                            finally
                            {
                                if (configured.Transport is not BattlementNativeTransport)
                                    coreRequests.TrimOversized();
                            }
                        }
                    }

                    payloadBytes = result.PayloadLength;
                    bool emptyPoll =
                        geometry is null && result.Status == BattlementTransportStatus.NoMessage;
                    if (!emptyPoll)
                    {
                        ProcessTransportResult(
                            configured,
                            result,
                            geometry is null ? "Poll failed." : "Geometry submit failed.",
                            duration: configuredRuntime.DittoMotionClock.Elapsed - exchangeStarted
                        );
                    }
                }
                catch (Exception exception)
                {
                    FailSession(
                        configured,
                        $"Frame exchange failed: {exception.Message}",
                        duration: configuredRuntime.DittoMotionClock.Elapsed - started,
                        payloadBytes: payloadBytes,
                        exception: exception
                    );
                }
            }

            TimeSpan finished = configuredRuntime.DittoMotionClock.Elapsed;
            session.PreviousStepTime = finished;
            TimeSpan frameDuration = finished - previous;
            if (
                !configuredRuntime.DittoMotionClock.IsControlled
                && frameDuration > SlowFrameThreshold
            )
            {
                LogSlowFrame(configured, frameDuration, finished - started, payloadBytes);
            }

            if (!Application.isPlaying)
            {
                configuredRuntime.World.UpdateBillboards();
            }
        }

        private void Update()
        {
            if (!dittoInputActive)
                RunFrame();
        }

        private void LateUpdate()
        {
            configuredRuntime?.World.UpdateBillboards();
            if (!dittoInputActive)
                CompleteNativeFrame();
            configuredRuntime?.FailureSurface.Refresh(completedInitialSnapshot);
        }

        internal void CompleteNativeFrame()
        {
            if (configuredRuntime is not null)
            {
                BattlementRunnerOptions configured = configuredRuntime.Options;
                DrainResponses(configured);
                configuredRuntime.BatchScheduler.Advance();
                if (session.Phase != BattlementSessionPhase.Running)
                    return;
                try
                {
                    GeometryObservationBatch? sample = configuredRuntime.GeometrySampler.Sample();
                    if (sample is not null)
                        geometryFrames.Merge(sample);
                }
                catch (Exception exception)
                {
                    FailSession(
                        configured,
                        $"Geometry sampling failed: {exception.Message}",
                        exception: exception
                    );
                }
            }
        }

        internal void BeginDittoFrameObservation()
        {
            dittoResponseDecodeNs = 0;
            dittoResponseApplyNs = 0;
        }

        internal (ulong DecodeNs, ulong ApplyNs) DittoResponseObservation() =>
            (dittoResponseDecodeNs, dittoResponseApplyNs);

        internal DittoTransportFrameMetrics? DittoTransportObservation()
        {
            if (configuredRuntime?.Options.Transport is not BattlementNativeTransport native)
                return null;
            BattlementNativeTransportDiagnostics value = native.Diagnostics;
            return new DittoTransportFrameMetrics(
                value.LiveBufferCount,
                value.LiveAllocationBytes,
                value.PendingFinalizerReleases,
                value.BuildersCreated,
                value.BuildersReused,
                value.BuilderGrowths,
                value.BuilderCopiedBytes,
                value.IdleBuilderBytes,
                value.HandoffPayloadCopies,
                value.ClientBuilderGrowths,
                value.ClientBuilderCopiedBytes,
                value.ClientBuilderRetainedBytes
            );
        }

        private void OnApplicationPause(bool pauseStatus)
        {
            isApplicationPaused = pauseStatus;
            Debug.Log(
                $"[Battlement/Ditto-trace] application-pause paused={pauseStatus} "
                    + $"focus={hasApplicationFocus} ditto={dittoInputActive}"
            );
            if (pauseStatus)
            {
                RejectDittoActivationTransaction(
                    "Application suspension interrupted semantic activation."
                );
                configuredRuntime?.PointerInput.CancelPresses();
                if (dittoInputActive)
                    configuredRuntime?.PointerInput.FailControlled(
                        "Application suspension interrupted controlled pointer input."
                    );
                configuredRuntime?.KeyboardInput.Reset();
                configuredRuntime?.ControllerInput.Reset();
            }
            if (dittoInputActive)
            {
                return;
            }
            PublishApplicationState();
        }

        private void OnApplicationFocus(bool hasFocus)
        {
            hasApplicationFocus = hasFocus;
            Debug.Log(
                $"[Battlement/Ditto-trace] application-focus focus={hasFocus} "
                    + $"paused={isApplicationPaused} ditto={dittoInputActive}"
            );
            if (dittoInputActive)
            {
                return;
            }
            if (!hasFocus && session.Phase != BattlementSessionPhase.Stopped)
            {
                configuredRuntime?.PointerInput.CancelPresses();
                configuredRuntime?.KeyboardInput.Reset();
                configuredRuntime?.ControllerInput.Reset();
                Log(
                    BattlementLogSeverity.Information,
                    "battlement.input.pointer_presses_cancelled",
                    "Pointer presses were cancelled after application focus loss."
                );
            }
            PublishApplicationState();
        }

        private void PublishApplicationState()
        {
            var state = new ApplicationState(
                dittoInputActive || hasApplicationFocus,
                isApplicationPaused
            );
            if (
                session.Phase != BattlementSessionPhase.Running
                || state == publishedApplicationState
            )
                return;
            publishedApplicationState = state;
            SubmitCoreAction(new ActionBody.ApplicationStateChanged(state));
        }

        private void PublishReducedMotionPreference()
        {
            ReducedMotionPreference preference = BattlementReducedMotion.Preference();
            if (
                session.Phase != BattlementSessionPhase.Running
                || preference == publishedReducedMotionPreference
            )
                return;
            publishedReducedMotionPreference = preference;
            SubmitCoreAction(new ActionBody.ReducedMotionPreferenceChanged(preference));
        }

        private void OnApplicationQuit() => Stop();

        private void OnDestroy() => Dispose();

        private void StartSession(
            BattlementRunnerOptions configured,
            bool reconnecting,
            SessionId? previousSession = null
        )
        {
            configuredRuntime!.DevelopmentDiagnostics?.Hide();
            configuredRuntime.FailureSurface.Clear(configuredRuntime.Errors);
            batchAdmission.BeginSession();
            configuredRuntime.BatchScheduler.BeginSession();
            configuredRuntime.GeometrySampler.Reset();
            geometryFrames.Reset();
            configuredRuntime.Scenes.BeginSession();
            configuredRuntime.World.BeginSession();
            configuredRuntime.PanelInput.Clear();
            session.BeginConnection(configuredRuntime.DittoMotionClock.Elapsed, reconnecting);

            TimeSpan started = configuredRuntime.DittoMotionClock.Elapsed;
            try
            {
                configuredRuntime.Modules.Prepare();
                if (
                    configured.Transport is BattlementNativeTransport
                    && configuredRuntime.CustomCommands.Types.Count > 0
                    && (
                        configured.FlatBufferResponseSchema is null
                        || configured.FlatBufferClientSchema is null
                    )
                )
                {
                    throw new InvalidOperationException(
                        "Native custom commands require generated FlatBuffer client and "
                            + "random-access response schemas."
                    );
                }
                Connect connect = BuildConnect(configured);
                (configured.Transport as IBattlementClientMessageObserver)?.RecordConnect(connect);
                ReadOnlyMemory<byte> bytes;
                using (BattlementProfiler.Serialization.Auto())
                {
                    bytes = configured.Transport is BattlementNativeTransport native
                        ? native.WriteConnect(connect)
                        : connectRequests.Write(connect);
                }
                if (bytes.Length > BattlementProtocolLimits.MaximumMessageBytes)
                {
                    FailSession(
                        configured,
                        $"A connect request cannot exceed "
                            + $"{BattlementProtocolLimits.MaximumMessageBytes} bytes.",
                        duration: configuredRuntime.DittoMotionClock.Elapsed - started,
                        payloadBytes: bytes.Length
                    );
                    return;
                }

                BattlementTransportResult result;
                using (BattlementProfiler.Transport.Auto())
                {
                    try
                    {
                        result = configured.Transport.Connect(bytes);
                    }
                    finally
                    {
                        if (configured.Transport is not BattlementNativeTransport)
                            connectRequests.TrimOversized();
                    }
                }

                if (result.Status != BattlementTransportStatus.Success)
                {
                    FailSession(
                        configured,
                        "Connect failed.",
                        result,
                        configuredRuntime.DittoMotionClock.Elapsed - started
                    );
                    return;
                }

                ProcessTransportResult(
                    configured,
                    result,
                    "Connect failed.",
                    true,
                    previousSession,
                    configuredRuntime.DittoMotionClock.Elapsed - started
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
                    duration: configuredRuntime.DittoMotionClock.Elapsed - started,
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
            using (result)
            {
                if (result.Status != BattlementTransportStatus.Success)
                {
                    FailSession(configured, failureMessage, result, duration);
                    return;
                }

                responses.Enqueue(
                    (payload, owner) => DecodeResponse(configured, payload, owner),
                    result.BorrowedPayload,
                    isInitial,
                    previousSession,
                    result.DetachPayloadOwner()
                );
                BattlementConfiguredRuntime runtime = configuredRuntime!;
                if (runtime.UiEventDispatcher.IsDispatching != true)
                    DrainResponses(configured);
            }
        }

        private IBattlementResponseView DecodeResponse(
            BattlementRunnerOptions configured,
            ReadOnlyMemory<byte> payload,
            IDisposable? payloadOwner
        )
        {
            try
            {
                if (BattlementFlatBufferResponse.HasIdentifier(payload))
                {
                    if (configured.FlatBufferResponseSchema is null)
                    {
                        IDisposable? owner = payloadOwner;
                        payloadOwner = null;
                        return new BattlementFlatBufferResponse(payload, owner);
                    }
                    IDisposable? customOwner = payloadOwner;
                    payloadOwner = null;
                    return new BattlementCustomFlatBufferResponse(
                        payload,
                        customOwner,
                        configured.FlatBufferResponseSchema
                    );
                }

                throw new InvalidDataException(
                    "The engine returned a response outside the FlatBuffer schema."
                );
            }
            catch
            {
                payloadOwner?.Dispose();
                throw;
            }
        }

        private void DrainResponses(BattlementRunnerOptions configured)
        {
            try
            {
                responses.Drain(
                    (response, isInitial, previousSession) =>
                        ValidateResponse(configured, response, isInitial, previousSession),
                    (session, response, index) =>
                        ApplyMessage(configured, session, response, index),
                    () => session.Phase == BattlementSessionPhase.ApplyingSnapshot,
                    () => session.Phase == BattlementSessionPhase.Stopped,
                    dittoInputActive
                );
                dittoResponseDecodeNs = checked(
                    dittoResponseDecodeNs + StopwatchNanoseconds(responses.LastDecodeTicks)
                );
                dittoResponseApplyNs = checked(
                    dittoResponseApplyNs + StopwatchNanoseconds(responses.LastApplyTicks)
                );
            }
            catch (Exception exception)
            {
                if (session.Phase != BattlementSessionPhase.Stopped)
                {
                    FailSession(
                        configured,
                        $"Deferred response failed: {exception.Message}",
                        exception: exception
                    );
                }
            }
        }

        private static ulong StopwatchNanoseconds(long ticks) =>
            checked(
                (ulong)Math.Max(0, ticks)
                * 1_000_000_000UL
                / (ulong)System.Diagnostics.Stopwatch.Frequency
            );

        private bool ValidateResponse(
            BattlementRunnerOptions configured,
            IBattlementResponseView response,
            bool isInitial,
            SessionId? previousSession
        )
        {
            if (response.SessionId.Value == Guid.Empty)
            {
                FailSession(configured, "The response did not contain orderable identity fields.");
                return false;
            }

            if (response.MessageCount > 256)
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

            if (isInitial && (response.MessageCount == 0 || !response.IsSnapshot(0)))
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
            IBattlementResponseView response,
            int messageIndex
        )
        {
            dittoStateVersion++;
            if (response.IsSnapshot(messageIndex))
            {
                ApplySnapshot(configured, responseSession, response.ReadSnapshot(messageIndex));
            }
            else
            {
                ApplyBatch(configured, responseSession, response.ReadBatch(messageIndex));
            }
        }

        private void ApplyBatch(
            BattlementRunnerOptions configured,
            SessionId responseSession,
            IBattlementBatchView batch
        )
        {
            bool transferred = false;
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

                if (batch.CausedByActionId is ActionId actionId)
                    dittoActivationTransaction?.ObserveCausalBatch(actionId);

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
                configuredRuntime!.BatchScheduler.Schedule(responseSession, batch, result);
                transferred = true;
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
            finally
            {
                if (!transferred)
                    batch.Dispose();
            }
        }

        private Connect BuildConnect(BattlementRunnerOptions configured)
        {
            var commandTypes = new SortedSet<string>(
                configured.CustomCommandTypes,
                StringComparer.Ordinal
            );
            commandTypes.UnionWith(configuredRuntime!.CustomCommands.Types);

            var state = new ApplicationState(
                dittoInputActive || hasApplicationFocus,
                isApplicationPaused
            );
            publishedApplicationState = state;
            publishedReducedMotionPreference = BattlementReducedMotion.Preference();
            return new Connect(
                PlatformName(Application.platform),
                Application.unityVersion,
                BattlementLogicalPixels.ScreenSize,
                new List<string>(commandTypes),
                Path.GetFullPath(Application.persistentDataPath),
                Path.GetFullPath(Application.streamingAssetsPath),
                configuredRuntime.Modules.ModuleIds
            )
            {
                ApplicationState = state,
                ReducedMotionPreference = publishedReducedMotionPreference,
            };
        }

        private void ApplySnapshot(
            BattlementRunnerOptions configured,
            SessionId responseSession,
            IBattlementSnapshotView snapshot
        )
        {
            bool transferred = false;
            try
            {
                configuredRuntime!.PointerInput.Suspend();
                configuredRuntime.KeyboardInput.Reset();
                configuredRuntime.ControllerInput.Reset();
                configuredRuntime.BatchScheduler.CancelForSnapshot();
                configuredRuntime.GeometrySampler.Reset();
                geometryFrames.Reset();
                configuredRuntime.ParticleEffects.ClearInactive();
                session.BeginSnapshot(responseSession);
                configuredRuntime.SnapshotReplacement.Begin(
                    responseSession,
                    snapshot,
                    session.IsReconnecting
                );
                transferred = true;
                AdvanceSnapshotPreparation(configured);
            }
            catch (BattlementSnapshotReplacementException exception)
            {
                FailSession(configured, exception.Message, exception: exception);
            }
            finally
            {
                if (!transferred)
                    snapshot.Dispose();
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
                if (!configuredRuntime!.SnapshotReplacement.TryComplete(out bool inputDisabled))
                {
                    return;
                }

                session.CompleteSnapshot(inputDisabled);
                configuredRuntime.UiDocuments.SetInputEnabled(!inputDisabled);
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
                fields["payload_bytes"] = result.PayloadLength.ToString(
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
            BattlementError error = configuredRuntime!.Errors.Report(
                restartRequired
                    ? BattlementErrorType.RestartRequired
                    : BattlementErrorType.SessionFailed,
                Source(result, exception),
                "battlement.session.failed",
                message,
                exception,
                stackTrace: panicDiagnostic?.PlainText,
                ansiStackTrace: nativePanic ? result?.Diagnostic : null,
                fields: fields,
                reportingDisposition: nativePanic || exception is not null
                    ? BattlementErrorReportingDisposition.ReportCaughtFailure
                    : BattlementErrorReportingDisposition.Ignore
            );
            isRuntimePoisoned |= restartRequired;
            if (nativePanic)
            {
                configuredRuntime.FailureSurface.Show(
                    new BattlementPlayerFailure(
                        restartRequired
                            ? BattlementPlayerFailureKind.RestartRequired
                            : BattlementPlayerFailureKind.ContinueAllowed,
                        error.Id
                    ),
                    configuredRuntime.Errors
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
                configuredRuntime!.Errors.Report(
                    BattlementErrorType.Logged,
                    BattlementErrorSource.Unity,
                    "battlement.unhandled_unity_exception",
                    UnityErrorMessage(error),
                    error.Exception,
                    stackTrace: error.StackTrace,
                    fields: fields,
                    reportingDisposition: error.Type == LogType.Exception
                        ? BattlementErrorReportingDisposition.AlreadyLoggedByUnity
                        : BattlementErrorReportingDisposition.Ignore
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
            Func<ReadOnlyMemory<byte>> serialize,
            Func<BattlementNativeTransport, BattlementTransportResult>? nativeSubmit,
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
                configuredRuntime!.Errors.Report(
                    BattlementErrorType.CommandFailed,
                    BattlementErrorSource.Unity,
                    eventName,
                    message,
                    exception,
                    fields: fields,
                    reportingDisposition: BattlementErrorReportingDisposition.ReportCaughtFailure
                );
            }

            TimeSpan started = configuredRuntime!.DittoMotionClock.Elapsed;
            int payloadBytes = 0;
            try
            {
                BattlementTransportResult result;
                ReadOnlyMemory<byte>? encoded = null;
                using (BattlementProfiler.Serialization.Auto())
                {
                    if (
                        configured.Transport is BattlementNativeTransport native
                        && nativeSubmit is not null
                    )
                    {
                        payloadBytes = 0;
                    }
                    else
                    {
                        encoded = serialize();
                        payloadBytes = encoded.Value.Length;
                    }
                }
                if (payloadBytes > BattlementProtocolLimits.MaximumMessageBytes)
                {
                    FailSession(
                        configured,
                        $"A failure message cannot exceed "
                            + $"{BattlementProtocolLimits.MaximumMessageBytes} bytes.",
                        duration: configuredRuntime.DittoMotionClock.Elapsed - started,
                        payloadBytes: payloadBytes
                    );
                    return;
                }

                using (BattlementProfiler.Transport.Auto())
                {
                    result =
                        configured.Transport is BattlementNativeTransport native
                        && nativeSubmit is not null
                            ? nativeSubmit(native)
                            : configured.Transport.Submit(encoded!.Value);
                }

                ProcessTransportResult(
                    configured,
                    result,
                    "Failure submission failed.",
                    duration: configuredRuntime.DittoMotionClock.Elapsed - started
                );
            }
            catch (Exception submissionException)
            {
                FailSession(
                    configured,
                    $"Failure submission response failed: {submissionException.Message}",
                    duration: configuredRuntime.DittoMotionClock.Elapsed - started,
                    payloadBytes: payloadBytes,
                    exception: submissionException
                );
            }
            finally
            {
                if (configured.Transport is not BattlementNativeTransport)
                    coreRequests.TrimOversized();
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
            configuredRuntime!.Errors.Log(
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

        private static void Reset(System.Action action, ref Exception? failure)
        {
            try
            {
                action();
            }
            catch (Exception exception)
            {
                failure ??= exception;
            }
        }

        private void StopSession(BattlementRunnerOptions configured, bool log)
        {
            try
            {
                configuredRuntime!.UiDocuments.SetInputEnabled(false);
                configuredRuntime.PointerInput.Reset();
                configuredRuntime.KeyboardInput.Reset();
                configuredRuntime.ControllerInput.Reset();
                configuredRuntime.ControllerInput.StopHaptics();
                configuredRuntime.BatchScheduler.BeginSession();
                configuredRuntime.GeometrySampler.Reset();
                geometryFrames.Reset();
                configuredRuntime.ParticleEffects.ClearInactive();
                configuredRuntime.SnapshotReplacement.Cancel();
                configuredRuntime.Scenes.BeginSession();
                configuredRuntime.World.BeginSession();
                configuredRuntime.PanelInput.Clear();
                configuredRuntime.PreparedAssets.BeginSession();
                configuredRuntime.Modules.Dispose();
                configured.Transport.Stop();
            }
            finally
            {
                session.Stop();
                batchAdmission.BeginSession();
                responses.Clear();
                pendingUiFailure = null;
            }
            if (log)
            {
                Log(BattlementLogSeverity.Information, "battlement.host.stopped", "Host stopped.");
            }
        }

        private void SetInputEnabled(bool isEnabled)
        {
            session.SetInputEnabled(isEnabled);
            configuredRuntime?.UiDocuments.SetInputEnabled(isEnabled);
            worldFocus?.Refresh(isEnabled);
            if (!isEnabled)
            {
                configuredRuntime?.PointerInput.CancelPresses();
                configuredRuntime?.KeyboardInput.Reset();
                configuredRuntime?.ControllerInput.Reset();
            }
        }

        private bool EmitAction(ActionBody body) => EmitAction(body, new ActionId(Guid.NewGuid()));

        private bool EmitAction(ActionBody body, ActionId actionId)
        {
            Debug.Log(
                $"[Battlement/Ditto-trace] world-input-dispatch body={body.GetType().Name} "
                    + $"available={CanEmitInput}"
            );
            if (!CanEmitInput || session.LastSession is not SessionId currentSession)
            {
                return false;
            }

            if (worldFocus?.TryHandle(body) == true)
                return CanEmitInput;
            SubmitCoreAction(body, actionId, currentSession);
            return CanEmitInput && session.LastSession == currentSession;
        }

        private void SubmitCoreAction(
            ActionBody body,
            ActionId? actionId = null,
            SessionId? expectedSession = null
        )
        {
            BattlementRunnerOptions configured = RequireOptions();
            SessionId currentSession =
                session.LastSession
                ?? throw new InvalidOperationException("No Battlement session is active.");
            if (expectedSession is SessionId expected && expected != currentSession)
                return;
            var action = new Action(actionId ?? new ActionId(Guid.NewGuid()), currentSession, body);
            if (configured.Transport is BattlementNativeTransport native)
            {
                RecordCoreAction(configured, action);
                SubmitNative(native, action);
            }
            else
            {
                ReadOnlyMemory<byte> message;
                using (BattlementProfiler.Serialization.Auto())
                    message = coreRequests.WriteAction(action);
                RecordCoreAction(configured, action);
                try
                {
                    Submit(message);
                }
                finally
                {
                    coreRequests.TrimOversized();
                }
            }
        }

        private static void RecordCoreAction(BattlementRunnerOptions configured, Action action)
        {
            (configured.Transport as IBattlementCoreMessageObserver)?.RecordAction(action);
            configured.CoreMessageObserver?.RecordAction(action);
        }

        private static void RecordBatchFailure(
            BattlementRunnerOptions configured,
            BatchFailed<CoreErrorCode> failure
        )
        {
            (configured.Transport as IBattlementCoreMessageObserver)?.RecordBatchFailure(failure);
            configured.CoreMessageObserver?.RecordBatchFailure(failure);
        }

        private static void RecordOperationFailure(
            BattlementRunnerOptions configured,
            OperationFailed<CoreErrorCode> failure
        )
        {
            (configured.Transport as IBattlementCoreMessageObserver)?.RecordOperationFailure(
                failure
            );
            configured.CoreMessageObserver?.RecordOperationFailure(failure);
        }

        private void SubmitNative(BattlementNativeTransport transport, Action action)
        {
            BattlementRunnerOptions configured = RequireOptions();
            TimeSpan started = configuredRuntime!.DittoMotionClock.Elapsed;
            try
            {
                BattlementTransportResult result;
                using (BattlementProfiler.Transport.Auto())
                    result = transport.Submit(action);
                ProcessTransportResult(
                    configured,
                    result,
                    "Submit failed.",
                    duration: configuredRuntime.DittoMotionClock.Elapsed - started
                );
            }
            catch (Exception exception)
            {
                FailSession(
                    configured,
                    $"Submit response failed: {exception.Message}",
                    duration: configuredRuntime.DittoMotionClock.Elapsed - started,
                    exception: exception
                );
            }
        }

        private void ApplyGeometryObservations(GeometryObservationUpdate update)
        {
            configuredRuntime!.GeometrySampler.Apply(update);
            geometryFrames.Retire(update);
        }

        private void ApplyGeometryObservations(BattlementDirectGeometryCommand update)
        {
            configuredRuntime!.GeometrySampler.Apply(update);
            geometryFrames.Retire(update);
        }

        private UiEventDisposition? EmitUiEvent(UiEvent value)
        {
            Debug.Log(
                $"[Battlement/Ditto-trace] ui-input-dispatch object={value.TargetId.Value} "
                    + $"kind={value.Body.GetType().Name} available={CanEmitInput}"
            );
            configuredRuntime?.World.Motion.Handle(value);
            if (
                configuredRuntime?.World.TryGetObject(value.TargetId, out GameObject? target)
                    == true
                && target != null
            )
            {
                BattlementIdentity identity = target.GetComponent<BattlementIdentity>();
                if (identity != null && identity.WorldPointer?.ForwardsUiEvents == false)
                {
                    return UiEventDisposition.Continue;
                }
            }
            if (!CanEmitInput || session.LastSession is not SessionId currentSession)
            {
                return null;
            }
            return configuredRuntime!.UiEventDispatcher.Dispatch(value, currentSession);
        }

        private bool CanEmitInput =>
            pendingUiFailure is null && session.IsInputAvailable && ApplicationAcceptsInput;

        private bool ApplicationAcceptsInput => !isApplicationPaused && HasInputFocus;

        private bool HasInputFocus =>
            dittoInputActive || hasApplicationFocus || Application.isBatchMode;

        private void RecordUiFailure(
            string message,
            BattlementUiEventTransportResult? result,
            TimeSpan duration,
            Exception? exception = null
        )
        {
            pendingUiFailure ??= new PendingUiFailure(message, result, duration, exception);
        }

        private void ApplyPendingUiFailure(BattlementRunnerOptions configured)
        {
            if (pendingUiFailure is not PendingUiFailure failure)
            {
                return;
            }
            pendingUiFailure = null;
            BattlementTransportResult? result = failure.Result is null
                ? null
                : new BattlementTransportResult(
                    failure.Result.Status,
                    diagnostic: failure.Result.Diagnostic,
                    nativeStatus: failure.Result.NativeStatus
                );
            FailSession(
                configured,
                failure.Message,
                result,
                failure.Duration,
                exception: failure.Exception
            );
        }

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

            return configuredRuntime?.Options
                ?? throw new InvalidOperationException(
                    "Configure the runner with public host dependencies before use."
                );
        }

        private sealed record PendingUiFailure(
            string Message,
            BattlementUiEventTransportResult? Result,
            TimeSpan Duration,
            Exception? Exception
        );

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

        private IBattlementFlatBufferClientSchema RequireFlatBufferClientSchema() =>
            RequireOptions().FlatBufferClientSchema
            ?? throw new InvalidOperationException(
                "Native custom code requires a generated FlatBuffer client schema."
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
                new BattlementTweenHelpers(configuredRuntime!.Tweens, now)
            );
        }

        private void Log(
            BattlementLogSeverity severity,
            string eventName,
            string message,
            IReadOnlyDictionary<string, string>? fields = null
        ) =>
            configuredRuntime!.Errors.Log(
                new BattlementLogRecord(severity, eventName, message, fields)
            );

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

#nullable enable

using System;
using Battlement.Errors;
using Battlement.UI;

namespace Battlement
{
    /// <summary>Owns the services created for one configured runner.</summary>
    internal sealed class BattlementConfiguredRuntime : IDisposable
    {
        internal BattlementConfiguredRuntime(BattlementRunnerOptions options)
        {
            Options = options ?? throw new ArgumentNullException(nameof(options));
        }

        internal BattlementRunnerOptions Options { get; }

        private BattlementWorld? world;

        internal BattlementWorld World => Require(world, nameof(World));

        private BattlementPreparedAssets? preparedAssets;

        internal BattlementPreparedAssets PreparedAssets =>
            Require(preparedAssets, nameof(PreparedAssets));

        private BattlementScenes? scenes;

        internal BattlementScenes Scenes => Require(scenes, nameof(Scenes));

        private BattlementSnapshotReplacement? snapshotReplacement;

        internal BattlementSnapshotReplacement SnapshotReplacement =>
            Require(snapshotReplacement, nameof(SnapshotReplacement));

        private BattlementBatchScheduler? batchScheduler;

        internal BattlementBatchScheduler BatchScheduler =>
            Require(batchScheduler, nameof(BatchScheduler));

        private BattlementParticleEffects? particleEffects;

        internal BattlementParticleEffects ParticleEffects =>
            Require(particleEffects, nameof(ParticleEffects));

        private BattlementAudioSources? audioSources;

        internal BattlementAudioSources AudioSources => Require(audioSources, nameof(AudioSources));

        private BattlementPointerInput? pointerInput;

        internal BattlementPointerInput PointerInput => Require(pointerInput, nameof(PointerInput));

        private BattlementPanelInputCoordinator? panelInput;

        internal BattlementPanelInputCoordinator PanelInput =>
            Require(panelInput, nameof(PanelInput));

        private BattlementKeyboardInput? keyboardInput;

        internal BattlementKeyboardInput KeyboardInput =>
            Require(keyboardInput, nameof(KeyboardInput));

        private BattlementControllerInput? controllerInput;

        internal BattlementControllerInput ControllerInput =>
            Require(controllerInput, nameof(ControllerInput));

        private BattlementCustomCommands? customCommands;

        internal BattlementCustomCommands CustomCommands =>
            Require(customCommands, nameof(CustomCommands));

        private BattlementTweenAdapter? tweens;

        internal BattlementTweenAdapter Tweens => Require(tweens, nameof(Tweens));

        private DittoMotionClock? dittoMotionClock;

        internal DittoMotionClock DittoMotionClock =>
            Require(dittoMotionClock, nameof(DittoMotionClock));

        private BattlementUiDocuments? uiDocuments;

        internal BattlementUiDocuments UiDocuments => Require(uiDocuments, nameof(UiDocuments));

        private BattlementGeometrySampler? geometrySampler;

        internal BattlementGeometrySampler GeometrySampler =>
            Require(geometrySampler, nameof(GeometrySampler));

        private BattlementModules? modules;

        internal BattlementModules Modules => Require(modules, nameof(Modules));

        internal BattlementDevelopmentDiagnostics? DevelopmentDiagnostics { get; private set; }

        private BattlementFailureSurface? failureSurface;

        internal BattlementFailureSurface FailureSurface =>
            Require(failureSurface, nameof(FailureSurface));

        private BattlementErrorReporter? errors;

        internal BattlementErrorReporter Errors => Require(errors, nameof(Errors));

        private BattlementUiEventDispatcher? uiEventDispatcher;

        internal BattlementUiEventDispatcher UiEventDispatcher =>
            Require(uiEventDispatcher, nameof(UiEventDispatcher));

        private IDisposable? unityErrorSubscription;

        internal void SetWorld(BattlementWorld value) => world = value;

        internal void SetPreparedAssets(BattlementPreparedAssets value) => preparedAssets = value;

        internal void SetScenes(BattlementScenes value) => scenes = value;

        internal void SetSnapshotReplacement(BattlementSnapshotReplacement value) =>
            snapshotReplacement = value;

        internal void SetBatchScheduler(BattlementBatchScheduler value) => batchScheduler = value;

        internal void SetParticleEffects(BattlementParticleEffects value) =>
            particleEffects = value;

        internal void SetAudioSources(BattlementAudioSources value) => audioSources = value;

        internal void SetPointerInput(BattlementPointerInput value) => pointerInput = value;

        internal void SetPanelInput(BattlementPanelInputCoordinator value) => panelInput = value;

        internal void SetKeyboardInput(BattlementKeyboardInput value) => keyboardInput = value;

        internal void SetControllerInput(BattlementControllerInput value) =>
            controllerInput = value;

        internal void SetCustomCommands(BattlementCustomCommands value) => customCommands = value;

        internal void SetTweens(BattlementTweenAdapter value) => tweens = value;

        internal void SetDittoMotionClock(DittoMotionClock value) => dittoMotionClock = value;

        internal void SetUiDocuments(BattlementUiDocuments value) => uiDocuments = value;

        internal void SetGeometrySampler(BattlementGeometrySampler value) =>
            geometrySampler = value;

        internal void SetModules(BattlementModules value) => modules = value;

        internal void SetDevelopmentDiagnostics(BattlementDevelopmentDiagnostics? value) =>
            DevelopmentDiagnostics = value;

        internal void SetFailureSurface(BattlementFailureSurface value) => failureSurface = value;

        internal void SetErrors(BattlementErrorReporter value) => errors = value;

        internal void SetUiEventDispatcher(BattlementUiEventDispatcher value) =>
            uiEventDispatcher = value;

        internal void SetUnityErrorSubscription(IDisposable value) =>
            unityErrorSubscription = value;

        private bool isDisposed;

        public void Dispose()
        {
            if (isDisposed)
            {
                return;
            }

            try
            {
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
                            Options.AssetStorage.Dispose();
                        }
                        finally
                        {
                            try
                            {
                                Options.Transport.Dispose();
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
                                                    uiDocuments?.Dispose();
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
                                                                try
                                                                {
                                                                    modules?.Dispose();
                                                                }
                                                                finally
                                                                {
                                                                    DisposeDevelopmentDiagnostics();
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
                }
            }
            finally
            {
                isDisposed = true;
            }
        }

        private void DisposeDevelopmentDiagnostics() => DevelopmentDiagnostics?.Dispose();

        private static T Require<T>(T? value, string name)
            where T : class =>
            value ?? throw new InvalidOperationException($"{name} is unavailable.");
    }
}

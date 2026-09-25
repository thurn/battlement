#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;

namespace Battlement
{
    internal sealed class BattlementCommandExecutor
    {
        private readonly BattlementWorkOwnership workOwnership = new();
        private readonly BattlementWorld world;
        private readonly BattlementPreparedAssets preparedAssets;
        private readonly BattlementScenes scenes;
        private readonly BattlementOperationRegistry operations;
        private readonly BattlementTweenAdapter tweens;
        private readonly BattlementParticleEffects particleEffects;
        private readonly BattlementAudioSources audioSources;
        private readonly BattlementCustomCommands customCommands;
        private readonly BattlementControllerInput controllerInput;
        private readonly DittoMotionClock motionClock;
        private readonly Action<bool> setInputEnabled;
        private readonly BattlementUiDocuments uiDocuments;
        private readonly Action<GeometryObservationUpdate> updateGeometry;
        private readonly Action<BattlementDirectGeometryCommand> updateDirectGeometry;
        private readonly BattlementModules modules;
        private readonly Action<string> openExternalUrl;

        public BattlementCommandExecutor(
            BattlementWorld world,
            BattlementPreparedAssets preparedAssets,
            BattlementScenes scenes,
            BattlementOperationRegistry operations,
            BattlementTweenAdapter tweens,
            BattlementParticleEffects particleEffects,
            BattlementAudioSources audioSources,
            BattlementControllerInput controllerInput,
            BattlementCustomCommands customCommands,
            DittoMotionClock motionClock,
            Action<bool> setInputEnabled,
            BattlementUiDocuments uiDocuments,
            Action<GeometryObservationUpdate> updateGeometry,
            Action<BattlementDirectGeometryCommand> updateDirectGeometry,
            BattlementModules modules,
            Action<string> openExternalUrl
        )
        {
            this.world = world;
            this.preparedAssets = preparedAssets;
            this.scenes = scenes;
            this.operations = operations;
            this.tweens = tweens;
            this.particleEffects = particleEffects;
            this.audioSources = audioSources;
            this.controllerInput = controllerInput;
            this.customCommands = customCommands;
            this.motionClock = motionClock;
            this.setInputEnabled = setInputEnabled;
            this.uiDocuments = uiDocuments;
            world.Motion.Bind(uiDocuments);
            world.Motion.Bind(audioSources);
            uiDocuments.BindMotionEffects(
                new BattlementMotionEffects(
                    world,
                    preparedAssets,
                    audioSources,
                    particleEffects,
                    motionClock
                )
            );
            this.updateGeometry = updateGeometry;
            this.updateDirectGeometry = updateDirectGeometry;
            this.modules = modules;
            this.openExternalUrl = openExternalUrl;
        }

        public void ResetWorkOwnership() => workOwnership.Clear();

        public IEnumerable<ObjectId> LiveUiSubtree(ObjectId id) => uiDocuments.LogicalSubtree(id);

        public void CancelScope(ulong scope) =>
            workOwnership.Cancel(scope, world, uiDocuments, operations);

        public void PauseScope(ulong scope) =>
            workOwnership.Pause(scope, world, uiDocuments, particleEffects);

        public void ResumeScope(ulong scope) =>
            workOwnership.Resume(scope, uiDocuments, particleEffects);

        public IBattlementCommandOperation? Launch(
            BattlementCommandExecution command,
            TimeSpan now,
            ulong? scope = null
        )
        {
            var operation = LaunchCore(command, now, scope);
            workOwnership.Record(command, scope, world, uiDocuments);
            return operation;
        }

        private IBattlementCommandOperation? LaunchCore(
            BattlementCommandExecution command,
            TimeSpan now,
            ulong? scope
        )
        {
            if (command.DirectLocalPosition is BattlementDirectLocalPosition position)
            {
                return LaunchDirectLocalPosition(position);
            }
            if (command.DirectWorldPosition is BattlementDirectWorldPosition worldPosition)
            {
                return LaunchDirectWorldPosition(worldPosition);
            }
            if (command.DirectRotation is BattlementDirectRotation rotation)
            {
                return LaunchDirect(() => BattlementTransformCommands.SetRotation(rotation, world));
            }
            if (command.DirectScale is BattlementDirectScale scale)
            {
                return LaunchDirect(() => BattlementTransformCommands.SetLocalScale(scale, world));
            }
            if (
                command.DirectTweenLocalPosition is BattlementDirectTweenLocalPosition tweenPosition
            )
            {
                RequireNonblockingForever(command.IsBlocking, tweenPosition.Tween);
                return LaunchDirect(() =>
                    BattlementTransformCommands.TweenLocalPosition(
                        tweenPosition,
                        world,
                        tweens,
                        now
                    )
                );
            }
            if (command.DirectTweenRotation is BattlementDirectTweenRotation tweenRotation)
            {
                RequireNonblockingForever(command.IsBlocking, tweenRotation.Tween);
                return LaunchDirect(() =>
                    BattlementTransformCommands.TweenRotation(tweenRotation, world, tweens, now)
                );
            }
            if (command.DirectTweenScale is BattlementDirectTweenScale tweenScale)
            {
                RequireNonblockingForever(command.IsBlocking, tweenScale.Tween);
                return LaunchDirect(() =>
                    BattlementTransformCommands.TweenLocalScale(tweenScale, world, tweens, now)
                );
            }
            if (command.DirectLabelUpdate is BattlementDirectLabelUpdate label)
            {
                return LaunchDirectLabelUpdate(label);
            }
            if (command.DirectTextContent is BattlementDirectTextContent text)
            {
                return LaunchDirect(() => BattlementImageTextCommands.SetContent(text, world));
            }
            if (command.DirectComponent is BattlementDirectComponentCommand component)
            {
                if (component.Tween is BattlementDirectTweenSettings tween)
                    RequireNonblockingForever(command.IsBlocking, tween);
                return LaunchDirect(() =>
                    component.Kind <= BattlementDirectComponentCommandKind.LightSetShadows
                        ? BattlementCameraLightCommands.Launch(component, world, tweens, now)
                        : BattlementImageTextCommands.Launch(
                            component,
                            world,
                            preparedAssets,
                            tweens,
                            now
                        )
                );
            }
            if (command.DirectAnimator is BattlementDirectAnimatorCommand animator)
                return BattlementAnimatorCommands.Launch(
                    animator,
                    world,
                    now,
                    motionClock.IsInstant
                );
            if (command.DirectGeometry is BattlementDirectGeometryCommand geometry)
                return ExecuteUi(() => updateDirectGeometry(geometry));
            if (command.DirectAccessibility is BattlementDirectAccessibilityCommand accessibility)
                return ExecuteUi(() => uiDocuments.Apply(accessibility));
            if (command.DirectDiagnostics is BattlementDirectDiagnostics diagnostics)
            {
                if (!command.IsBlocking)
                    throw new BattlementCommandException(
                        CoreErrorCode.InvalidProperty,
                        "Diagnostics commands must be blocking."
                    );
                return ExecuteModule(() => modules.Execute(diagnostics.Command));
            }
            if (command.DirectAssets is BattlementDirectAssetSet assets)
                return LaunchDirect(() =>
                    BattlementCoreCommandOperations.ReplaceAssets(assets, preparedAssets)
                );
            if (command.DirectUiDestroy is ObjectId uiDestroy)
                return ExecuteUi(() => uiDocuments.Destroy(uiDestroy));
            if (command.DirectUiAction is BattlementDirectVisualElementAction uiAction)
                return ExecuteUi(() => uiDocuments.PerformAction(uiAction));
            if (command.DirectUiPlacement is BattlementDirectVisualElementPlacement uiPlacement)
                return ExecuteUi(() => uiDocuments.Update(uiPlacement));
            if (command.DirectUiScalar is BattlementDirectUiScalar uiScalar)
                return ExecuteUi(() => uiDocuments.UpdateScalar(uiScalar));
            if (command.DirectUiProperties is BattlementDirectUiProperties uiProperties)
                return RequireFiniteBlockingMotion(
                    uiDocuments.UpdateProperties(
                        uiProperties.ObjectId,
                        uiProperties.ReadElement(),
                        scope.HasValue
                    ),
                    command.IsBlocking && !scope.HasValue
                );
            if (command.DirectUiCreate is BattlementDirectUiCreate uiCreate)
                return ExecuteUi(() =>
                    uiDocuments.Create(
                        uiCreate.ParentId,
                        uiCreate.RootId,
                        uiCreate,
                        uiCreate.ChildIndex
                    )
                );
            if (command.DirectMotion is BattlementDirectMotionCommand motion)
                return ExecuteUi(() => ApplyDirectMotion(motion));
            if (command.DirectMotionValue is BattlementDirectMotionValueCommand motionValue)
                return ApplyDirectMotionValue(motionValue, command.IsBlocking);
            if (command.DirectMotionControl is BattlementDirectMotionControlCommand motionControl)
                return ApplyDirectMotionControl(motionControl, command.IsBlocking);
            if (command.DirectMotionScope is BattlementDirectMotionScopeCommand motionScope)
                return uiDocuments.ApplyScope(motionScope, command.IsBlocking);
            if (command.DirectSetMaterial is BattlementDirectSetMaterial material)
            {
                return LaunchDirect(() =>
                    BattlementObjectCommands.SetMaterial(material, world, preparedAssets)
                );
            }
            if (command.DirectDestroyObject is BattlementDirectDestroyObject destroy)
            {
                return LaunchDirect(() =>
                    BattlementObjectCommands.Destroy(destroy, world, operations)
                );
            }
            if (command.DirectBoxHitRegionCreate is BattlementDirectBoxHitRegionCreate boxCreate)
                return LaunchDirect(() =>
                {
                    world.CreateObject(boxCreate);
                    return null;
                });
            if (
                command.DirectBoxHitRegionGeometry
                is BattlementDirectBoxHitRegionGeometry boxGeometry
            )
                return LaunchDirect(() =>
                {
                    if (
                        !world
                            .RequireObject(boxGeometry.ObjectId)
                            .TryGetComponent(out BattlementBoxHitRegion region)
                    )
                        throw new BattlementWorldException(
                            CoreErrorCode.ComponentMissing,
                            "Target is not a box hit region."
                        );
                    region.SetGeometry(boxGeometry.State);
                    return null;
                });
            if (command.DirectMaterialInstances is BattlementDirectMaterialInstances instances)
            {
                return LaunchDirect(() =>
                {
                    BattlementMaterialInstances.Apply(
                        world.RequireObject(instances.ObjectId),
                        preparedAssets,
                        instances.Values
                    );
                    return null;
                });
            }
            if (command.DirectWorldMotion is BattlementDirectWorldMotion worldMotion)
                return LaunchDirect(() =>
                    RequireFiniteBlockingMotion(
                        world.Motion.Install(
                            worldMotion.ObjectId,
                            worldMotion.Motion,
                            command.IsBlocking || scope.HasValue
                        ),
                        command.IsBlocking && !scope.HasValue
                    )
                );
            if (command.DirectWorldPointer is BattlementDirectWorldPointer pointer)
            {
                return LaunchDirect(() =>
                {
                    world
                        .RequireObject(pointer.ObjectId)
                        .GetComponent<BattlementIdentity>()
                        .WorldPointer = pointer.Settings;
                    return null;
                });
            }
            if (command.DirectRenderOrder is BattlementDirectRenderOrder order)
            {
                return LaunchDirect(() =>
                {
                    BattlementRenderOrder.Apply(world.RequireObject(order.ObjectId), order.Order);
                    return null;
                });
            }
            if (command.DirectObjectActive is BattlementDirectObjectActive active)
            {
                return LaunchDirect(() => BattlementObjectCommands.SetActive(active, world));
            }
            if (command.DirectObjectReparent is BattlementDirectObjectReparent reparent)
            {
                return LaunchDirect(() =>
                    BattlementObjectCommands.Reparent(reparent, world, operations)
                );
            }
            if (command.DirectInputEnabled is BattlementDirectInputEnabled input)
            {
                return BattlementInputCommands.SetEnabled(input, setInputEnabled);
            }
            if (command.DirectImageObjectCreate is BattlementDirectImageObjectCreate createImage)
            {
                return LaunchDirect(() => BattlementObjectCommands.Create(createImage, world));
            }
            if (
                command.DirectPrimitiveObjectCreate
                is BattlementDirectPrimitiveObjectCreate createPrimitive
            )
            {
                return LaunchDirect(() => BattlementObjectCommands.Create(createPrimitive, world));
            }
            if (command.DirectMeshObjectCreate is BattlementDirectMeshObjectCreate createMesh)
                return LaunchDirect(() => BattlementObjectCommands.Create(createMesh, world));
            if (command.DirectPrefabObjectCreate is BattlementDirectPrefabObjectCreate createPrefab)
            {
                return LaunchDirect(() => BattlementObjectCommands.Create(createPrefab, world));
            }
            if (command.DirectEmptyObjectCreate is BattlementDirectEmptyObjectCreate createEmpty)
            {
                return LaunchDirect(() => BattlementObjectCommands.Create(createEmpty, world));
            }
            if (command.DirectTextObjectCreate is BattlementDirectTextObjectCreate createText)
            {
                return LaunchDirect(() => BattlementObjectCommands.Create(createText, world));
            }
            if (command.DirectCameraObjectCreate is BattlementDirectCameraObjectCreate createCamera)
            {
                return LaunchDirect(() => BattlementObjectCommands.Create(createCamera, world));
            }
            if (command.DirectLightObjectCreate is BattlementDirectLightObjectCreate createLight)
            {
                return LaunchDirect(() => BattlementObjectCommands.Create(createLight, world));
            }
            if (command.DirectParticleSpawn is BattlementDirectParticleSpawn particle)
            {
                return LaunchDirect(() => particleEffects.Spawn(command.Id, particle, now));
            }
            if (command.DirectAudioPlay is BattlementDirectAudioPlay audio)
            {
                if (command.IsBlocking && audio.Loop)
                {
                    throw new BattlementCommandException(
                        CoreErrorCode.InvalidProperty,
                        "Looping audio must be nonblocking."
                    );
                }
                return LaunchDirect(() => audioSources.Play(command.Id, audio, now));
            }
            if (command.DirectParticlePlay is BattlementDirectParticlePlay particlePlay)
            {
                if (command.IsBlocking)
                    throw new BattlementCommandException(
                        CoreErrorCode.InvalidProperty,
                        "Particle play has no inferred end and must be nonblocking."
                    );
                return LaunchDirect(() => particleEffects.Play(particlePlay));
            }
            if (command.DirectParticleStop is BattlementDirectParticleStop particleStop)
                return LaunchDirect(() => particleEffects.Stop(particleStop));
            if (command.DirectAudioMix is AudioMix audioMix)
                return LaunchDirect(() => audioSources.SetMix(audioMix));
            if (command.DirectAudioStop is BattlementDirectAudioStop audioStop)
                return LaunchDirect(() => audioSources.Stop(audioStop, now));
            if (command.DirectAudioVolume is BattlementDirectAudioVolume audioVolume)
                return LaunchDirect(() => audioSources.SetVolume(audioVolume));
            if (command.DirectAudioControl is BattlementDirectAudioControl audioControl)
                return LaunchDirect(() => LaunchDirectAudioControl(audioControl, now));
            if (command.DirectTweenAudioVolume is BattlementDirectTweenAudioVolume tweenAudioVolume)
            {
                RequireNonblockingForever(command.IsBlocking, tweenAudioVolume.Tween);
                return LaunchDirect(() => audioSources.TweenVolume(tweenAudioVolume, tweens, now));
            }
            if (command.DirectWait is BattlementDirectWait wait)
            {
                return BattlementTimeCommands.Wait(
                    wait,
                    now,
                    motionClock.IsInstant || motionClock.IsControlled
                );
            }
            if (command.DirectVibration is BattlementDirectVibration vibration)
                return controllerInput.Vibrate(vibration, now);
            if (command.DirectDebugUi is BattlementDirectDebugUi debugUi)
                return ExecuteUi(() => BattlementDebugUi.SetVisible(debugUi));
            if (command.DirectSceneCommand is BattlementDirectSceneCommand scene)
            {
                return LaunchDirect(() =>
                    scene.Kind switch
                    {
                        BattlementDirectSceneCommandKind.Load =>
                            BattlementCoreCommandOperations.LoadScene(scene, scenes),
                        BattlementDirectSceneCommandKind.Unload =>
                            BattlementCoreCommandOperations.UnloadScene(
                                scene,
                                scenes,
                                world,
                                operations
                            ),
                        BattlementDirectSceneCommandKind.SetPrimary => scenes.SetPrimary(
                            scene.SceneId
                        ),
                        _ => throw new BattlementCommandException(
                            CoreErrorCode.InvalidProperty,
                            "A scene command kind is unknown."
                        ),
                    }
                );
            }
            if (
                command.DirectInputConfiguration
                is BattlementDirectInputConfiguration inputConfiguration
            )
                return LaunchDirectInputConfiguration(inputConfiguration);
            if (command.DirectOpenUrl is BattlementDirectOpenUrl openUrl)
            {
                return ExecuteUi(() =>
                {
                    _ = new Uri(openUrl.Url, UriKind.Absolute);
                    openExternalUrl(openUrl.Url);
                });
            }
            if (command.DirectCustomCommand is IBattlementDirectCustomCommand directCustom)
            {
                return directCustom.Launch(customCommands, now);
            }
            throw new BattlementCommandException(
                CoreErrorCode.InvalidProperty,
                "The batch contained no recognized command payload."
            );
        }

        public void BeginBatch() => uiDocuments.BeginCommit();

        private void ApplyDirectMotion(BattlementDirectMotionCommand command)
        {
            switch (command.Kind)
            {
                case BattlementDirectMotionCommandKind.ValuePlayback:
                    uiDocuments.ApplyValuePlayback(
                        command.ObjectId,
                        command.Generation,
                        command.Playback,
                        command.Micros,
                        command.Number,
                        command.Direction
                    );
                    break;
                case BattlementDirectMotionCommandKind.Playback:
                    uiDocuments.ApplyPlayback(
                        command.ObjectId,
                        command.Slot,
                        command.Generation,
                        command.Playback,
                        command.Micros,
                        command.Number,
                        command.Direction
                    );
                    break;
                case BattlementDirectMotionCommandKind.ControlledClockSet:
                    uiDocuments.ApplyControlledClock(command.ObjectId, command.Micros, false);
                    break;
                case BattlementDirectMotionCommandKind.ControlledClockAdvance:
                    uiDocuments.ApplyControlledClock(command.ObjectId, command.Micros, true);
                    break;
                case BattlementDirectMotionCommandKind.DragControl:
                    uiDocuments.ApplyDragControl(
                        command.ObjectId,
                        command.PointerId,
                        command.Device,
                        command.X,
                        command.Y,
                        command.SnapToCursor
                    );
                    break;
                default:
                    throw new BattlementUiException(
                        CoreErrorCode.InvalidProperty,
                        "A direct motion command kind is unknown."
                    );
            }
        }

        private IBattlementCommandOperation? ApplyDirectMotionValue(
            BattlementDirectMotionValueCommand command,
            bool blocking
        )
        {
            MotionValueOperationKind kind = command.Kind;
            bool hasValue = kind != MotionValueOperationKind.Stop;
            bool animates = kind == MotionValueOperationKind.Animate;
            return uiDocuments.ApplyValue(
                command.ValueId,
                kind,
                hasValue ? command.ReadValue() : null,
                animates ? command.PlaybackId : default,
                animates ? command.Generation : 0,
                animates ? command.ReadTransition() : null,
                blocking
            );
        }

        private IBattlementCommandOperation? ApplyDirectMotionControl(
            BattlementDirectMotionControlCommand command,
            bool blocking
        )
        {
            MotionControlOperationKind kind = command.Kind;
            bool hasTarget =
                kind is MotionControlOperationKind.Start or MotionControlOperationKind.Set;
            return uiDocuments.ApplyControl(
                command.ControlId,
                kind,
                kind == MotionControlOperationKind.Start ? command.PlaybackId : default,
                kind == MotionControlOperationKind.Start ? command.Generation : 0,
                hasTarget ? command.ReadTarget() : null,
                blocking
            );
        }

        public void EndBatch() => uiDocuments.EndCommit();

        private static void RequireNonblockingForever(
            bool isBlocking,
            BattlementDirectTweenSettings tween
        )
        {
            if (isBlocking && tween.RepeatKind == 2)
            {
                throw new BattlementCommandException(
                    CoreErrorCode.InvalidProperty,
                    "A forever tween must be nonblocking."
                );
            }
        }

        private IBattlementCommandOperation? LaunchDirectLocalPosition(
            BattlementDirectLocalPosition position
        )
        {
            try
            {
                return BattlementTransformCommands.SetLocalPosition(position, world);
            }
            catch (BattlementWorldException exception)
            {
                throw new BattlementCommandException(
                    exception.ErrorCode,
                    exception.Message,
                    exception
                );
            }
        }

        private IBattlementCommandOperation? LaunchDirectWorldPosition(
            BattlementDirectWorldPosition position
        )
        {
            return LaunchDirect(() =>
                BattlementTransformCommands.SetWorldPosition(position, world)
            );
        }

        private static IBattlementCommandOperation? LaunchDirect(
            Func<IBattlementCommandOperation?> launch
        )
        {
            try
            {
                return launch();
            }
            catch (BattlementWorldException exception)
            {
                throw new BattlementCommandException(
                    exception.ErrorCode,
                    exception.Message,
                    exception
                );
            }
            catch (BattlementAssetException exception)
            {
                throw new BattlementCommandException(
                    exception.ErrorCode,
                    exception.Message,
                    exception
                );
            }
        }

        private IBattlementCommandOperation? LaunchDirectLabelUpdate(
            BattlementDirectLabelUpdate label
        )
        {
            try
            {
                string text = label.ReadText();
                uiDocuments.UpdateLabelText(label.ObjectId, text);
                return null;
            }
            catch (BattlementUiException exception)
            {
                throw new BattlementCommandException(
                    exception.ErrorCode,
                    exception.Message,
                    exception
                );
            }
        }

        private IBattlementCommandOperation? LaunchDirectAudioControl(
            BattlementDirectAudioControl command,
            TimeSpan now
        ) =>
            command.Kind switch
            {
                BattlementDirectAudioControlKind.Pause => audioSources.Pause(
                    command.AudioCommandId
                ),
                BattlementDirectAudioControlKind.Resume => audioSources.Resume(
                    command.AudioCommandId
                ),
                BattlementDirectAudioControlKind.Seek => audioSources.Seek(
                    command.AudioCommandId,
                    TimeSpan.FromMilliseconds(command.PositionMilliseconds),
                    now
                ),
                BattlementDirectAudioControlKind.SetBuffering => audioSources.SetBuffering(
                    command.AudioCommandId,
                    command.Buffering
                ),
                BattlementDirectAudioControlKind.Replace => audioSources.Replace(
                    command.AudioCommandId,
                    command.Address
                        ?? throw new BattlementCommandException(
                            CoreErrorCode.InvalidProperty,
                            "An audio replacement address is absent."
                        ),
                    now
                ),
                _ => throw new BattlementCommandException(
                    CoreErrorCode.InvalidProperty,
                    "An audio control kind is unknown."
                ),
            };

        private IBattlementCommandOperation? LaunchDirectInputConfiguration(
            BattlementDirectInputConfiguration command
        )
        {
            switch (command.Kind)
            {
                case BattlementDirectInputConfigurationKind.Camera:
                    world.ConfigureInputCamera(
                        command.ObjectId
                            ?? throw new BattlementCommandException(
                                CoreErrorCode.InvalidProperty,
                                "An input camera UUID is absent."
                            )
                    );
                    break;
                case BattlementDirectInputConfigurationKind.PointerEvents:
                    PointerEvent[] pointerEvents = command.ReadPointerEvents();
                    world.SetPointerEvents(
                        command.ObjectId
                            ?? throw new BattlementCommandException(
                                CoreErrorCode.InvalidProperty,
                                "An input object UUID is absent."
                            ),
                        pointerEvents
                    );
                    break;
                case BattlementDirectInputConfigurationKind.GlobalKeys:
                    PhysicalKey[] globalKeys = command.ReadGlobalKeys();
                    world.SetGlobalKeys(globalKeys);
                    break;
                case BattlementDirectInputConfigurationKind.Controller:
                    ControllerInputSettings controller = command.ReadController();
                    world.SetControllerInput(controller);
                    break;
                default:
                    throw new BattlementCommandException(
                        CoreErrorCode.InvalidProperty,
                        "An input configuration kind is unknown."
                    );
            }
            return null;
        }

        private static IBattlementCommandOperation? ExecuteUi(System.Action execute)
        {
            execute();
            return null;
        }

        private static IBattlementCommandOperation? RequireFiniteBlockingMotion(
            IBattlementCommandOperation? operation,
            bool blocking
        )
        {
            if (!blocking || operation?.IsInfinite != true)
                return operation;
            operation.Cancel();
            throw new BattlementCommandException(
                CoreErrorCode.InvalidProperty,
                "An infinite Motion playback must be nonblocking."
            );
        }

        private static IBattlementCommandOperation? ExecuteModule(System.Action execute)
        {
            execute();
            return null;
        }
    }

    internal sealed class BattlementCommandException : InvalidOperationException
    {
        public BattlementCommandException(
            CoreErrorCode errorCode,
            string message,
            Exception? innerException = null
        )
            : base(message, innerException) => ErrorCode = errorCode;

        public CoreErrorCode ErrorCode { get; }

        public Exception? DeveloperException =>
            ErrorCode is CoreErrorCode.HandlerFailed or CoreErrorCode.UnityException
                ? InnerException ?? this
                : null;
    }
}

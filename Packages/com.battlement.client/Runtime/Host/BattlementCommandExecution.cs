#nullable enable

using System;
using Battlement.UI;

namespace Battlement
{
    internal sealed class BattlementCommandExecutor
    {
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
            this.updateGeometry = updateGeometry;
            this.updateDirectGeometry = updateDirectGeometry;
            this.modules = modules;
            this.openExternalUrl = openExternalUrl;
        }

        public IBattlementCommandOperation? Launch(BattlementCommandExecution command, TimeSpan now)
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
                return ExecuteModule(() => modules.Execute(diagnostics.Key, diagnostics.Value));
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
                return ExecuteUi(() =>
                    uiDocuments.UpdateProperties(uiProperties.ObjectId, uiProperties.ReadElement())
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
                return ExecuteUi(() => ApplyDirectMotionValue(motionValue));
            if (command.DirectMotionControl is BattlementDirectMotionControlCommand motionControl)
                return ExecuteUi(() => ApplyDirectMotionControl(motionControl));
            if (command.DirectMotionScope is BattlementDirectMotionScopeCommand motionScope)
                return ExecuteUi(() => uiDocuments.ApplyScope(motionScope));
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
                if (!command.IsBlocking)
                    throw new BattlementCommandException(
                        CoreErrorCode.InvalidProperty,
                        "A wait command must be blocking."
                    );
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
                return scene.Kind switch
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
                    BattlementDirectSceneCommandKind.SetPrimary => scenes.SetPrimary(scene.SceneId),
                    _ => throw new BattlementCommandException(
                        CoreErrorCode.InvalidProperty,
                        "A scene command kind is unknown."
                    ),
                };
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
            if (command.CustomCommand is ICommand custom)
            {
                return customCommands.Launch(custom, now);
            }
            if (command.DirectCustomCommand is IBattlementDirectCustomCommand directCustom)
            {
                return directCustom.Launch(customCommands, now);
            }

            return LaunchCore(
                command.Id,
                command.CoreBody
                    ?? throw new BattlementCommandException(
                        CoreErrorCode.InvalidProperty,
                        "The batch contained neither a core nor a custom command payload."
                    ),
                command.IsBlocking,
                now
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

        private void ApplyDirectMotionValue(BattlementDirectMotionValueCommand command)
        {
            MotionValueOperationKind kind = command.Kind;
            bool hasValue = kind != MotionValueOperationKind.Stop;
            bool animates = kind == MotionValueOperationKind.Animate;
            uiDocuments.ApplyValue(
                command.ValueId,
                kind,
                hasValue ? command.ReadValue() : null,
                animates ? command.PlaybackId : default,
                animates ? command.Generation : 0,
                animates ? command.ReadTransition() : null
            );
        }

        private void ApplyDirectMotionControl(BattlementDirectMotionControlCommand command)
        {
            MotionControlOperationKind kind = command.Kind;
            bool hasTarget =
                kind is MotionControlOperationKind.Start or MotionControlOperationKind.Set;
            uiDocuments.ApplyControl(
                command.ControlId,
                kind,
                kind == MotionControlOperationKind.Start ? command.PlaybackId : default,
                kind == MotionControlOperationKind.Start ? command.Generation : 0,
                hasTarget ? command.ReadTarget() : null
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

        private IBattlementCommandOperation? LaunchCore(
            CommandId id,
            CommandBody body,
            bool isBlocking,
            TimeSpan now
        )
        {
            try
            {
                if (
                    isBlocking && BattlementTweenAdapter.IsForever(BattlementTweenAdapter.For(body))
                )
                {
                    throw new BattlementCommandException(
                        CoreErrorCode.InvalidProperty,
                        "A forever tween must be nonblocking."
                    );
                }

                if (isBlocking && body is CommandBody.Particle.Play)
                {
                    throw new BattlementCommandException(
                        CoreErrorCode.InvalidProperty,
                        "Particle play has no inferred end and must be nonblocking."
                    );
                }

                if (body is CommandBody.Diagnostics && !isBlocking)
                {
                    throw new BattlementCommandException(
                        CoreErrorCode.InvalidProperty,
                        "Diagnostics commands must be blocking."
                    );
                }

                if (body is CommandBody.Diagnostics validatedDiagnostics)
                {
                    CoreErrorCode? validation = DiagnosticsProtocol.Validate(
                        validatedDiagnostics.Command
                    );
                    if (validation is CoreErrorCode errorCode)
                    {
                        throw new BattlementCommandException(
                            errorCode,
                            "The Diagnostics command is invalid."
                        );
                    }
                }

                if (isBlocking && body is CommandBody.Audio.Play { Loop: true })
                {
                    throw new BattlementCommandException(
                        CoreErrorCode.InvalidProperty,
                        "Looping audio must be nonblocking."
                    );
                }

                if (body is CommandBody.Controller.Vibrate vibration)
                {
                    ValidateVibration(vibration);
                }

                return body switch
                {
                    CommandBody.Assets.ReplaceSet assets =>
                        BattlementCoreCommandOperations.ReplaceAssets(assets, preparedAssets),
                    CommandBody.Scene.Load scene => BattlementCoreCommandOperations.LoadScene(
                        scene,
                        scenes
                    ),
                    CommandBody.Scene.Unload scene => BattlementCoreCommandOperations.UnloadScene(
                        scene,
                        scenes,
                        world,
                        operations
                    ),
                    CommandBody.Scene.SetPrimary scene => scenes.SetPrimary(scene.SceneId),
                    CommandBody.Time.Wait wait => BattlementTimeCommands.Wait(
                        wait,
                        now,
                        motionClock.IsInstant
                    ),
                    CommandBody.Object.Create create => BattlementObjectCommands.Create(
                        create,
                        world
                    ),
                    CommandBody.Object.Destroy destroy => BattlementObjectCommands.Destroy(
                        destroy,
                        world,
                        operations
                    ),
                    CommandBody.Object.SetActive active => BattlementObjectCommands.SetActive(
                        active,
                        world
                    ),
                    CommandBody.Object.Reparent reparent => BattlementObjectCommands.Reparent(
                        reparent,
                        world,
                        operations
                    ),
                    CommandBody.Transform.SetLocalPosition position =>
                        BattlementTransformCommands.SetLocalPosition(position, world),
                    CommandBody.Transform.SetWorldPosition position =>
                        BattlementTransformCommands.SetWorldPosition(position, world),
                    CommandBody.Transform.TweenLocalPosition position =>
                        BattlementTransformCommands.TweenLocalPosition(
                            position,
                            world,
                            tweens,
                            now
                        ),
                    CommandBody.Transform.TweenWorldPosition position =>
                        BattlementTransformCommands.TweenWorldPosition(
                            position,
                            world,
                            tweens,
                            now
                        ),
                    CommandBody.Transform.SetLocalRotation rotation =>
                        BattlementTransformCommands.SetLocalRotation(rotation, world),
                    CommandBody.Transform.SetWorldRotation rotation =>
                        BattlementTransformCommands.SetWorldRotation(rotation, world),
                    CommandBody.Transform.TweenLocalRotation rotation =>
                        BattlementTransformCommands.TweenLocalRotation(
                            rotation,
                            world,
                            tweens,
                            now
                        ),
                    CommandBody.Transform.TweenWorldRotation rotation =>
                        BattlementTransformCommands.TweenWorldRotation(
                            rotation,
                            world,
                            tweens,
                            now
                        ),
                    CommandBody.Transform.SetLocalScale scale =>
                        BattlementTransformCommands.SetLocalScale(scale, world),
                    CommandBody.Transform.TweenLocalScale scale =>
                        BattlementTransformCommands.TweenLocalScale(scale, world, tweens, now),
                    CommandBody.Camera.SetEnabled camera =>
                        BattlementCameraLightCommands.SetCameraEnabled(camera, world),
                    CommandBody.Camera.SetPerspective camera =>
                        BattlementCameraLightCommands.SetPerspective(camera, world),
                    CommandBody.Camera.TweenFieldOfView camera =>
                        BattlementCameraLightCommands.TweenFieldOfView(camera, world, tweens, now),
                    CommandBody.Camera.SetOrthographic camera =>
                        BattlementCameraLightCommands.SetOrthographic(camera, world),
                    CommandBody.Camera.TweenOrthographicSize camera =>
                        BattlementCameraLightCommands.TweenOrthographicSize(
                            camera,
                            world,
                            tweens,
                            now
                        ),
                    CommandBody.Camera.SetClipping camera =>
                        BattlementCameraLightCommands.SetClipping(camera, world),
                    CommandBody.Camera.SetClear camera => BattlementCameraLightCommands.SetClear(
                        camera,
                        world
                    ),
                    CommandBody.Light.SetEnabled light =>
                        BattlementCameraLightCommands.SetLightEnabled(light, world),
                    CommandBody.Light.SetType light => BattlementCameraLightCommands.SetLightType(
                        light,
                        world
                    ),
                    CommandBody.Light.SetColor light => BattlementCameraLightCommands.SetLightColor(
                        light,
                        world
                    ),
                    CommandBody.Light.TweenColor light =>
                        BattlementCameraLightCommands.TweenLightColor(light, world, tweens, now),
                    CommandBody.Light.SetIntensity light =>
                        BattlementCameraLightCommands.SetLightIntensity(light, world),
                    CommandBody.Light.TweenIntensity light =>
                        BattlementCameraLightCommands.TweenLightIntensity(
                            light,
                            world,
                            tweens,
                            now
                        ),
                    CommandBody.Light.SetRange light => BattlementCameraLightCommands.SetLightRange(
                        light,
                        world
                    ),
                    CommandBody.Light.SetSpotAngle light =>
                        BattlementCameraLightCommands.SetSpotAngle(light, world),
                    CommandBody.Light.SetShadows light => BattlementCameraLightCommands.SetShadows(
                        light,
                        world
                    ),
                    CommandBody.Image.SetTexture image => BattlementImageTextCommands.SetTexture(
                        image,
                        world,
                        preparedAssets
                    ),
                    CommandBody.Image.SetSize image => BattlementImageTextCommands.SetSize(
                        image,
                        world
                    ),
                    CommandBody.Image.SetFit image => BattlementImageTextCommands.SetFit(
                        image,
                        world
                    ),
                    CommandBody.Image.SetTint image => BattlementImageTextCommands.SetTint(
                        image,
                        world
                    ),
                    CommandBody.Image.TweenTint image => BattlementImageTextCommands.TweenTint(
                        image,
                        world,
                        tweens,
                        now
                    ),
                    CommandBody.Image.SetOpacity image => BattlementImageTextCommands.SetOpacity(
                        image,
                        world
                    ),
                    CommandBody.Image.TweenOpacity image =>
                        BattlementImageTextCommands.TweenOpacity(image, world, tweens, now),
                    CommandBody.Image.SetFaceCamera image =>
                        BattlementImageTextCommands.SetImageFaceCamera(image, world),
                    CommandBody.Text.SetContent text => BattlementImageTextCommands.SetContent(
                        text,
                        world
                    ),
                    CommandBody.Text.SetFont text => BattlementImageTextCommands.SetFont(
                        text,
                        world,
                        preparedAssets
                    ),
                    CommandBody.Text.SetSize text => BattlementImageTextCommands.SetTextSize(
                        text,
                        world
                    ),
                    CommandBody.Text.TweenSize text => BattlementImageTextCommands.TweenTextSize(
                        text,
                        world,
                        tweens,
                        now
                    ),
                    CommandBody.Text.SetColor text => BattlementImageTextCommands.SetTextColor(
                        text,
                        world
                    ),
                    CommandBody.Text.TweenColor text => BattlementImageTextCommands.TweenTextColor(
                        text,
                        world,
                        tweens,
                        now
                    ),
                    CommandBody.Text.SetAlignment text => BattlementImageTextCommands.SetAlignment(
                        text,
                        world
                    ),
                    CommandBody.Text.SetWrapping text => BattlementImageTextCommands.SetWrapping(
                        text,
                        world
                    ),
                    CommandBody.Text.SetRichText text => BattlementImageTextCommands.SetRichText(
                        text,
                        world
                    ),
                    CommandBody.Text.SetFaceCamera text =>
                        BattlementImageTextCommands.SetTextFaceCamera(text, world),
                    CommandBody.Renderer.SetMaterial material =>
                        BattlementObjectCommands.SetMaterial(material, world, preparedAssets),
                    CommandBody.Animator.Play animator => BattlementAnimatorCommands.Play(
                        animator,
                        world,
                        now,
                        motionClock.IsInstant
                    ),
                    CommandBody.Animator.CrossFade animator => BattlementAnimatorCommands.CrossFade(
                        animator,
                        world,
                        now,
                        motionClock.IsInstant
                    ),
                    CommandBody.Animator.SetBool animator => BattlementAnimatorCommands.SetBool(
                        animator,
                        world
                    ),
                    CommandBody.Animator.SetInt animator => BattlementAnimatorCommands.SetInt(
                        animator,
                        world
                    ),
                    CommandBody.Animator.SetFloat animator => BattlementAnimatorCommands.SetFloat(
                        animator,
                        world
                    ),
                    CommandBody.Animator.SetTrigger animator =>
                        BattlementAnimatorCommands.SetTrigger(animator, world),
                    CommandBody.Animator.SetSpeed animator => BattlementAnimatorCommands.SetSpeed(
                        animator,
                        world
                    ),
                    CommandBody.Particle.Play particle => particleEffects.Play(particle),
                    CommandBody.Particle.Stop particle => particleEffects.Stop(particle),
                    CommandBody.Particle.Spawn particle => particleEffects.Spawn(id, particle, now),
                    CommandBody.Audio.Play audio => audioSources.Play(id, audio, now),
                    CommandBody.Audio.Stop audio => audioSources.Stop(audio, now),
                    CommandBody.Audio.Pause audio => audioSources.Pause(audio),
                    CommandBody.Audio.Resume audio => audioSources.Resume(audio),
                    CommandBody.Audio.Seek audio => audioSources.Seek(audio, now),
                    CommandBody.Audio.SetBuffering audio => audioSources.SetBuffering(audio),
                    CommandBody.Audio.Replace audio => audioSources.Replace(audio, now),
                    CommandBody.Audio.SetVolume audio => audioSources.SetVolume(audio),
                    CommandBody.Audio.TweenVolume audio => audioSources.TweenVolume(
                        audio,
                        tweens,
                        now
                    ),
                    CommandBody.Input.SetEnabled input => BattlementInputCommands.SetEnabled(
                        input,
                        setInputEnabled
                    ),
                    CommandBody.Input.SetCamera input => BattlementInputCommands.SetCamera(
                        input,
                        world
                    ),
                    CommandBody.Input.SetPointerEvents input =>
                        BattlementInputCommands.SetPointerEvents(input, world),
                    CommandBody.Input.SetGlobalKeys input => BattlementInputCommands.SetGlobalKeys(
                        input,
                        world
                    ),
                    CommandBody.Input.SetController input => BattlementInputCommands.SetController(
                        input,
                        world
                    ),
                    CommandBody.Controller.Vibrate input => controllerInput.Vibrate(input, now),
                    CommandBody.DebugUi debugUi => ExecuteUi(() =>
                        BattlementDebugUi.SetVisible(debugUi)
                    ),
                    CommandBody.VisualElement.Create ui => ExecuteUi(() => uiDocuments.Create(ui)),
                    CommandBody.VisualElement.Update ui => ExecuteUi(() => uiDocuments.Update(ui)),
                    CommandBody.VisualElement.Destroy ui => ExecuteUi(() =>
                        uiDocuments.Destroy(ui)
                    ),
                    CommandBody.VisualElement.PerformAction ui => ExecuteUi(() =>
                        uiDocuments.PerformAction(ui)
                    ),
                    CommandBody.Motion.ValueCommand motion => ExecuteUi(() =>
                        uiDocuments.Apply(motion.Payload)
                    ),
                    CommandBody.Motion.ValuePlayback motion => ExecuteUi(() =>
                        uiDocuments.Apply(motion.Payload)
                    ),
                    CommandBody.Motion.Playback motion => ExecuteUi(() =>
                        uiDocuments.Apply(motion.Payload)
                    ),
                    CommandBody.Motion.ControlledClock motion => ExecuteUi(() =>
                        uiDocuments.Apply(motion.Payload)
                    ),
                    CommandBody.Motion.Control motion => ExecuteUi(() =>
                        uiDocuments.Apply(motion.Payload)
                    ),
                    CommandBody.Motion.Scope motion => ExecuteUi(() =>
                        uiDocuments.Apply(motion.Payload)
                    ),
                    CommandBody.Motion.DragControl motion => ExecuteUi(() =>
                        uiDocuments.Apply(motion.Payload)
                    ),
                    CommandBody.GeometryObservation geometry => ExecuteUi(() =>
                        updateGeometry(geometry.Value)
                    ),
                    CommandBody.AccessibilityUpdate accessibility => ExecuteUi(() =>
                        uiDocuments.Apply(accessibility.Value)
                    ),
                    CommandBody.ApplicationOpenUrl request => ExecuteUi(() =>
                    {
                        _ = new Uri(request.Url, UriKind.Absolute);
                        openExternalUrl(request.Url);
                    }),
                    CommandBody.Diagnostics diagnostics => ExecuteModule(() =>
                        modules.Execute(diagnostics.Command)
                    ),
                    _ => throw new BattlementCommandException(
                        CoreErrorCode.InvalidProperty,
                        $"Command {body.GetType().Name} is not implemented yet."
                    ),
                };
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
            catch (BattlementUiException exception)
            {
                throw new BattlementCommandException(
                    exception.ErrorCode,
                    exception.Message,
                    exception
                );
            }
            catch (BattlementModuleException exception)
            {
                throw new BattlementCommandException(
                    exception.ErrorCode,
                    exception.Message,
                    exception.InnerException
                );
            }
        }

        private static IBattlementCommandOperation? ExecuteUi(System.Action execute)
        {
            execute();
            return null;
        }

        private static IBattlementCommandOperation? ExecuteModule(System.Action execute)
        {
            execute();
            return null;
        }

        private static void ValidateVibration(CommandBody.Controller.Vibrate command)
        {
            bool invalidLow = command.LowFrequency < 0 || command.LowFrequency > 1;
            bool invalidHigh = command.HighFrequency < 0 || command.HighFrequency > 1;
            if (invalidLow || invalidHigh)
            {
                throw new BattlementCommandException(
                    CoreErrorCode.InvalidProperty,
                    "Controller motor intensities must be between zero and one."
                );
            }
            if (command.Duration < TimeSpan.Zero)
            {
                throw new BattlementCommandException(
                    CoreErrorCode.InvalidProperty,
                    "Controller vibration duration cannot be negative."
                );
            }
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

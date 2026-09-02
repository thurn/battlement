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
            this.modules = modules;
            this.openExternalUrl = openExternalUrl;
        }

        public IBattlementCommandOperation? Launch(ICommand command, TimeSpan now)
        {
            if (command is ICustomCommand)
            {
                return customCommands.Launch(command, now);
            }

            return LaunchCore(
                command as Command
                    ?? throw new BattlementCommandException(
                        CoreErrorCode.InvalidProperty,
                        "The batch contained an unknown command implementation."
                    ),
                now
            );
        }

        public void BeginBatch() => uiDocuments.BeginCommit();

        public void EndBatch() => uiDocuments.EndCommit();

        private IBattlementCommandOperation? LaunchCore(Command command, TimeSpan now)
        {
            try
            {
                if (
                    command.IsBlocking
                    && BattlementTweenAdapter.IsForever(BattlementTweenAdapter.For(command.Body))
                )
                {
                    throw new BattlementCommandException(
                        CoreErrorCode.InvalidProperty,
                        "A forever tween must be nonblocking."
                    );
                }

                if (command.IsBlocking && command.Body is CommandBody.Particle.Play)
                {
                    throw new BattlementCommandException(
                        CoreErrorCode.InvalidProperty,
                        "Particle play has no inferred end and must be nonblocking."
                    );
                }

                if (command.Body is CommandBody.Diagnostics && !command.IsBlocking)
                {
                    throw new BattlementCommandException(
                        CoreErrorCode.InvalidProperty,
                        "Diagnostics commands must be blocking."
                    );
                }

                if (command.Body is CommandBody.Diagnostics validatedDiagnostics)
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

                if (command.IsBlocking && command.Body is CommandBody.Audio.Play { Loop: true })
                {
                    throw new BattlementCommandException(
                        CoreErrorCode.InvalidProperty,
                        "Looping audio must be nonblocking."
                    );
                }

                if (command.Body is CommandBody.Controller.Vibrate vibration)
                {
                    ValidateVibration(vibration);
                }

                return command.Body switch
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
                    CommandBody.Particle.Spawn particle => particleEffects.Spawn(
                        command.Id,
                        particle,
                        now
                    ),
                    CommandBody.Audio.Play audio => audioSources.Play(command.Id, audio, now),
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
                        $"Command {command.Body.GetType().Name} is not implemented yet."
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

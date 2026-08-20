#nullable enable

using System;

namespace Masonry
{
    internal sealed class MasonryCommandExecutor
    {
        private readonly MasonryWorld world;
        private readonly MasonryPreparedAssets preparedAssets;
        private readonly MasonryScenes scenes;
        private readonly MasonryOperationRegistry operations;
        private readonly MasonryTweenAdapter tweens;
        private readonly MasonryParticleEffects particleEffects;
        private readonly Action<bool> setInputEnabled;

        public MasonryCommandExecutor(
            MasonryWorld world,
            MasonryPreparedAssets preparedAssets,
            MasonryScenes scenes,
            MasonryOperationRegistry operations,
            MasonryTweenAdapter tweens,
            MasonryParticleEffects particleEffects,
            Action<bool> setInputEnabled
        )
        {
            this.world = world;
            this.preparedAssets = preparedAssets;
            this.scenes = scenes;
            this.operations = operations;
            this.tweens = tweens;
            this.particleEffects = particleEffects;
            this.setInputEnabled = setInputEnabled;
        }

        public IMasonryCommandOperation? Launch(Command command, TimeSpan now)
        {
            try
            {
                if (
                    command.IsBlocking
                    && MasonryTweenAdapter.IsForever(MasonryTweenAdapter.For(command.Body))
                )
                {
                    throw new MasonryCommandException(
                        CoreErrorCode.InvalidProperty,
                        "A forever tween must be nonblocking."
                    );
                }

                if (command.IsBlocking && command.Body is CommandBody.Particle.Play)
                {
                    throw new MasonryCommandException(
                        CoreErrorCode.InvalidProperty,
                        "Particle play has no inferred end and must be nonblocking."
                    );
                }

                return command.Body switch
                {
                    CommandBody.Assets.ReplaceSet assets =>
                        MasonryCoreCommandOperations.ReplaceAssets(assets, preparedAssets),
                    CommandBody.Scene.Load scene => MasonryCoreCommandOperations.LoadScene(
                        scene,
                        scenes
                    ),
                    CommandBody.Scene.Unload scene => MasonryCoreCommandOperations.UnloadScene(
                        scene,
                        scenes,
                        world,
                        operations
                    ),
                    CommandBody.Scene.SetPrimary scene => scenes.SetPrimary(scene.SceneId),
                    CommandBody.Time.Wait wait => MasonryTimeCommands.Wait(wait, now),
                    CommandBody.Object.Create create => MasonryObjectCommands.Create(create, world),
                    CommandBody.Object.Destroy destroy => MasonryObjectCommands.Destroy(
                        destroy,
                        world,
                        operations
                    ),
                    CommandBody.Object.SetActive active => MasonryObjectCommands.SetActive(
                        active,
                        world
                    ),
                    CommandBody.Object.Reparent reparent => MasonryObjectCommands.Reparent(
                        reparent,
                        world,
                        operations
                    ),
                    CommandBody.Transform.SetLocalPosition position =>
                        MasonryTransformCommands.SetLocalPosition(position, world),
                    CommandBody.Transform.SetWorldPosition position =>
                        MasonryTransformCommands.SetWorldPosition(position, world),
                    CommandBody.Transform.TweenLocalPosition position =>
                        MasonryTransformCommands.TweenLocalPosition(position, world, tweens, now),
                    CommandBody.Transform.TweenWorldPosition position =>
                        MasonryTransformCommands.TweenWorldPosition(position, world, tweens, now),
                    CommandBody.Transform.SetLocalRotation rotation =>
                        MasonryTransformCommands.SetLocalRotation(rotation, world),
                    CommandBody.Transform.SetWorldRotation rotation =>
                        MasonryTransformCommands.SetWorldRotation(rotation, world),
                    CommandBody.Transform.TweenLocalRotation rotation =>
                        MasonryTransformCommands.TweenLocalRotation(rotation, world, tweens, now),
                    CommandBody.Transform.TweenWorldRotation rotation =>
                        MasonryTransformCommands.TweenWorldRotation(rotation, world, tweens, now),
                    CommandBody.Transform.SetLocalScale scale =>
                        MasonryTransformCommands.SetLocalScale(scale, world),
                    CommandBody.Transform.TweenLocalScale scale =>
                        MasonryTransformCommands.TweenLocalScale(scale, world, tweens, now),
                    CommandBody.Camera.SetEnabled camera =>
                        MasonryCameraLightCommands.SetCameraEnabled(camera, world),
                    CommandBody.Camera.SetPerspective camera =>
                        MasonryCameraLightCommands.SetPerspective(camera, world),
                    CommandBody.Camera.TweenFieldOfView camera =>
                        MasonryCameraLightCommands.TweenFieldOfView(camera, world, tweens, now),
                    CommandBody.Camera.SetOrthographic camera =>
                        MasonryCameraLightCommands.SetOrthographic(camera, world),
                    CommandBody.Camera.TweenOrthographicSize camera =>
                        MasonryCameraLightCommands.TweenOrthographicSize(
                            camera,
                            world,
                            tweens,
                            now
                        ),
                    CommandBody.Camera.SetClipping camera => MasonryCameraLightCommands.SetClipping(
                        camera,
                        world
                    ),
                    CommandBody.Camera.SetClear camera => MasonryCameraLightCommands.SetClear(
                        camera,
                        world
                    ),
                    CommandBody.Light.SetEnabled light =>
                        MasonryCameraLightCommands.SetLightEnabled(light, world),
                    CommandBody.Light.SetType light => MasonryCameraLightCommands.SetLightType(
                        light,
                        world
                    ),
                    CommandBody.Light.SetColor light => MasonryCameraLightCommands.SetLightColor(
                        light,
                        world
                    ),
                    CommandBody.Light.TweenColor light =>
                        MasonryCameraLightCommands.TweenLightColor(light, world, tweens, now),
                    CommandBody.Light.SetIntensity light =>
                        MasonryCameraLightCommands.SetLightIntensity(light, world),
                    CommandBody.Light.TweenIntensity light =>
                        MasonryCameraLightCommands.TweenLightIntensity(light, world, tweens, now),
                    CommandBody.Light.SetRange light => MasonryCameraLightCommands.SetLightRange(
                        light,
                        world
                    ),
                    CommandBody.Light.SetSpotAngle light => MasonryCameraLightCommands.SetSpotAngle(
                        light,
                        world
                    ),
                    CommandBody.Light.SetShadows light => MasonryCameraLightCommands.SetShadows(
                        light,
                        world
                    ),
                    CommandBody.Image.SetTexture image => MasonryImageTextCommands.SetTexture(
                        image,
                        world,
                        preparedAssets
                    ),
                    CommandBody.Image.SetSize image => MasonryImageTextCommands.SetSize(
                        image,
                        world
                    ),
                    CommandBody.Image.SetFit image => MasonryImageTextCommands.SetFit(image, world),
                    CommandBody.Image.SetTint image => MasonryImageTextCommands.SetTint(
                        image,
                        world
                    ),
                    CommandBody.Image.TweenTint image => MasonryImageTextCommands.TweenTint(
                        image,
                        world,
                        tweens,
                        now
                    ),
                    CommandBody.Image.SetOpacity image => MasonryImageTextCommands.SetOpacity(
                        image,
                        world
                    ),
                    CommandBody.Image.TweenOpacity image => MasonryImageTextCommands.TweenOpacity(
                        image,
                        world,
                        tweens,
                        now
                    ),
                    CommandBody.Image.SetFaceCamera image =>
                        MasonryImageTextCommands.SetImageFaceCamera(image, world),
                    CommandBody.Text.SetContent text => MasonryImageTextCommands.SetContent(
                        text,
                        world
                    ),
                    CommandBody.Text.SetFont text => MasonryImageTextCommands.SetFont(
                        text,
                        world,
                        preparedAssets
                    ),
                    CommandBody.Text.SetSize text => MasonryImageTextCommands.SetTextSize(
                        text,
                        world
                    ),
                    CommandBody.Text.TweenSize text => MasonryImageTextCommands.TweenTextSize(
                        text,
                        world,
                        tweens,
                        now
                    ),
                    CommandBody.Text.SetColor text => MasonryImageTextCommands.SetTextColor(
                        text,
                        world
                    ),
                    CommandBody.Text.TweenColor text => MasonryImageTextCommands.TweenTextColor(
                        text,
                        world,
                        tweens,
                        now
                    ),
                    CommandBody.Text.SetAlignment text => MasonryImageTextCommands.SetAlignment(
                        text,
                        world
                    ),
                    CommandBody.Text.SetWrapping text => MasonryImageTextCommands.SetWrapping(
                        text,
                        world
                    ),
                    CommandBody.Text.SetRichText text => MasonryImageTextCommands.SetRichText(
                        text,
                        world
                    ),
                    CommandBody.Text.SetFaceCamera text =>
                        MasonryImageTextCommands.SetTextFaceCamera(text, world),
                    CommandBody.Renderer.SetMaterial material => MasonryObjectCommands.SetMaterial(
                        material,
                        world,
                        preparedAssets
                    ),
                    CommandBody.Animator.Play animator => MasonryAnimatorCommands.Play(
                        animator,
                        world,
                        now
                    ),
                    CommandBody.Animator.CrossFade animator => MasonryAnimatorCommands.CrossFade(
                        animator,
                        world,
                        now
                    ),
                    CommandBody.Animator.SetBool animator => MasonryAnimatorCommands.SetBool(
                        animator,
                        world
                    ),
                    CommandBody.Animator.SetInt animator => MasonryAnimatorCommands.SetInt(
                        animator,
                        world
                    ),
                    CommandBody.Animator.SetFloat animator => MasonryAnimatorCommands.SetFloat(
                        animator,
                        world
                    ),
                    CommandBody.Animator.SetTrigger animator => MasonryAnimatorCommands.SetTrigger(
                        animator,
                        world
                    ),
                    CommandBody.Animator.SetSpeed animator => MasonryAnimatorCommands.SetSpeed(
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
                    CommandBody.Input.SetEnabled input => MasonryInputCommands.SetEnabled(
                        input,
                        setInputEnabled
                    ),
                    CommandBody.Input.SetCamera input => MasonryInputCommands.SetCamera(
                        input,
                        world
                    ),
                    CommandBody.Input.SetPointerEvents input =>
                        MasonryInputCommands.SetPointerEvents(input, world),
                    CommandBody.Input.SetGlobalKeys input => MasonryInputCommands.SetGlobalKeys(
                        input,
                        world
                    ),
                    _ => throw new MasonryCommandException(
                        CoreErrorCode.InvalidProperty,
                        $"Command {command.Body.GetType().Name} is not implemented yet."
                    ),
                };
            }
            catch (MasonryWorldException exception)
            {
                throw new MasonryCommandException(
                    exception.ErrorCode,
                    exception.Message,
                    exception
                );
            }
            catch (MasonryAssetException exception)
            {
                throw new MasonryCommandException(
                    exception.ErrorCode,
                    exception.Message,
                    exception
                );
            }
        }
    }

    internal sealed class MasonryCommandException : InvalidOperationException
    {
        public MasonryCommandException(
            CoreErrorCode errorCode,
            string message,
            Exception? innerException = null
        )
            : base(message, innerException) => ErrorCode = errorCode;

        public CoreErrorCode ErrorCode { get; }
    }
}

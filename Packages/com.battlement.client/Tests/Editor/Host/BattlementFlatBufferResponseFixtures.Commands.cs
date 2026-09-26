#nullable enable

using System;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    internal static partial class BattlementFlatBufferResponseFixtures
    {
        private static Payload DisplayOperation(FlatBufferBuilder builder, DisplayCommand value)
        {
            Offset<Wire.DisplayConfiguration> configuration = default;
            if (value is DisplayCommand.Preview preview)
            {
                DisplayConfiguration target = preview.Configuration;
                Wire.DisplayConfiguration.StartDisplayConfiguration(builder);
                Wire.DisplayConfiguration.AddMode(builder, (Wire.DisplayMode)target.Mode);
                Wire.DisplayConfiguration.AddResolution(
                    builder,
                    Wire.DisplayResolution.CreateDisplayResolution(
                        builder,
                        target.Resolution.Width,
                        target.Resolution.Height,
                        target.Resolution.RefreshNumerator,
                        target.Resolution.RefreshDenominator
                    )
                );
                configuration = Wire.DisplayConfiguration.EndDisplayConfiguration(builder);
            }
            Wire.DisplayCommandPayload.StartDisplayCommandPayload(builder);
            Wire.DisplayCommandPayload.AddConfiguration(builder, configuration);
            if (value is DisplayCommand.Confirm confirm)
            {
                Wire.DisplayCommandPayload.AddOperation(builder, Wire.DisplayOperation.Confirm);
                Wire.DisplayCommandPayload.AddPreviewId(
                    builder,
                    Uuid(builder, confirm.PreviewId.Value)
                );
            }
            else if (value is DisplayCommand.Cancel cancel)
            {
                Wire.DisplayCommandPayload.AddOperation(builder, Wire.DisplayOperation.Cancel);
                Wire.DisplayCommandPayload.AddPreviewId(
                    builder,
                    Uuid(builder, cancel.PreviewId.Value)
                );
            }
            return new Payload(
                Wire.CoreCommandKind.ApplicationDisplay,
                Wire.CoreCommandPayload.DisplayCommandPayload,
                Wire.DisplayCommandPayload.EndDisplayCommandPayload(builder).Value
            );
        }

        internal static Offset<Wire.CoreCommand> WriteCommand(
            FlatBufferBuilder builder,
            Command command
        )
        {
            Payload payload = command.Body switch
            {
                CommandBody.Diagnostics value => Diagnostics(builder, value),
                CommandBody.Motion.SetWorldDescriptor value => WorldMotion(builder, value),
                CommandBody.ApplicationDisplay value => DisplayOperation(builder, value.Value),
                CommandBody.ApplicationOpenUrl value => ExternalUrl(builder, value),
                CommandBody.ApplicationSetFramePacing value => new Payload(
                    Wire.CoreCommandKind.ApplicationSetFramePacing,
                    Wire.CoreCommandPayload.FramePacingPayload,
                    Wire.FramePacingPayload.CreateFramePacingPayload(
                        builder,
                        value.Value.MaximumFrameRate,
                        value.Value.Vsync
                    ).Value
                ),
                CommandBody.DebugUi value => DebugUi(builder, value),
                CommandBody.Assets.ReplaceSet value => ReplaceAssets(builder, value),
                CommandBody.Scene.Load value => LoadScene(builder, value),
                CommandBody.Scene.Unload value => SceneId(
                    builder,
                    value.SceneId,
                    Wire.CoreCommandKind.SceneUnload
                ),
                CommandBody.Scene.SetPrimary value => SceneId(
                    builder,
                    value.SceneId,
                    Wire.CoreCommandKind.SceneSetPrimary
                ),
                CommandBody.Object.Create value => CreateObject(builder, value),
                CommandBody.Object.Destroy value => ObjectId(
                    builder,
                    value.ObjectId,
                    Wire.CoreCommandKind.ObjectDestroy
                ),
                CommandBody.Object.SetRenderOrder value => SetRenderOrder(builder, value),
                CommandBody.Object.SetActive value => ObjectEnabled(
                    builder,
                    value.ObjectId,
                    value.IsActive,
                    Wire.CoreCommandKind.ObjectSetActive,
                    Wire.CoreCommandPayload.ObjectSetActivePayload
                ),
                CommandBody.Object.Reparent value => ReparentObject(builder, value),
                CommandBody.Transform.SetLocalPosition value => Position(
                    builder,
                    value.ObjectId,
                    value.Position,
                    value.OnConflict,
                    Wire.CoreCommandKind.TransformSetLocalPosition
                ),
                CommandBody.Transform.SetWorldPosition value => Position(
                    builder,
                    value.ObjectId,
                    value.Position,
                    value.OnConflict,
                    Wire.CoreCommandKind.TransformSetWorldPosition
                ),
                CommandBody.Transform.TweenLocalPosition value => TweenPosition(
                    builder,
                    value.ObjectId,
                    value.Position,
                    value.Tween,
                    value.OnConflict,
                    Wire.CoreCommandKind.TransformTweenLocalPosition
                ),
                CommandBody.Transform.TweenWorldPosition value => TweenPosition(
                    builder,
                    value.ObjectId,
                    value.Position,
                    value.Tween,
                    value.OnConflict,
                    Wire.CoreCommandKind.TransformTweenWorldPosition
                ),
                CommandBody.Transform.SetLocalRotation value => Rotation(
                    builder,
                    value.ObjectId,
                    value.Rotation,
                    value.OnConflict,
                    Wire.CoreCommandKind.TransformSetLocalRotation
                ),
                CommandBody.Transform.SetWorldRotation value => Rotation(
                    builder,
                    value.ObjectId,
                    value.Rotation,
                    value.OnConflict,
                    Wire.CoreCommandKind.TransformSetWorldRotation
                ),
                CommandBody.Transform.TweenLocalRotation value => TweenRotation(
                    builder,
                    value.ObjectId,
                    value.Rotation,
                    value.Tween,
                    value.OnConflict,
                    Wire.CoreCommandKind.TransformTweenLocalRotation
                ),
                CommandBody.Transform.TweenWorldRotation value => TweenRotation(
                    builder,
                    value.ObjectId,
                    value.Rotation,
                    value.Tween,
                    value.OnConflict,
                    Wire.CoreCommandKind.TransformTweenWorldRotation
                ),
                CommandBody.Transform.SetLocalScale value => Scale(
                    builder,
                    value.ObjectId,
                    value.Scale,
                    value.OnConflict
                ),
                CommandBody.Transform.TweenLocalScale value => TweenScale(
                    builder,
                    value.ObjectId,
                    value.Scale,
                    value.Tween,
                    value.OnConflict
                ),
                CommandBody.Renderer.SetMaterial value => SetMaterial(builder, value),
                CommandBody.SetBoxHitRegion value => SetBoxHitRegion(builder, value),
                CommandBody.Renderer.SetInstances value => SetInstances(builder, value),
                CommandBody.Camera.SetEnabled value => ObjectEnabled(
                    builder,
                    value.ObjectId,
                    value.IsEnabled,
                    Wire.CoreCommandKind.CameraSetEnabled,
                    Wire.CoreCommandPayload.ObjectEnabledPayload
                ),
                CommandBody.Camera.SetPerspective value => Perspective(builder, value),
                CommandBody.Camera.TweenFieldOfView value => TweenFieldOfView(builder, value),
                CommandBody.Camera.SetOrthographic value => Orthographic(builder, value),
                CommandBody.Camera.TweenOrthographicSize value => TweenOrthographic(builder, value),
                CommandBody.Camera.SetClipping value => CameraClipping(builder, value),
                CommandBody.Camera.SetClear value => CameraClear(builder, value),
                CommandBody.Light.SetEnabled value => ObjectEnabled(
                    builder,
                    value.ObjectId,
                    value.IsEnabled,
                    Wire.CoreCommandKind.LightSetEnabled,
                    Wire.CoreCommandPayload.ObjectEnabledPayload
                ),
                CommandBody.Light.SetType value => LightType(builder, value),
                CommandBody.Light.SetColor value => Color(
                    builder,
                    value.ObjectId,
                    value.Color,
                    value.OnConflict,
                    Wire.CoreCommandKind.LightSetColor
                ),
                CommandBody.Light.TweenColor value => TweenColor(
                    builder,
                    value.ObjectId,
                    value.Color,
                    value.Tween,
                    value.OnConflict,
                    Wire.CoreCommandKind.LightTweenColor
                ),
                CommandBody.Light.SetIntensity value => Intensity(
                    builder,
                    value.ObjectId,
                    value.Intensity,
                    value.OnConflict,
                    Wire.CoreCommandKind.LightSetIntensity
                ),
                CommandBody.Light.TweenIntensity value => TweenIntensity(builder, value),
                CommandBody.Light.SetRange value => LightRange(builder, value),
                CommandBody.Light.SetSpotAngle value => SpotAngle(builder, value),
                CommandBody.Light.SetShadows value => LightShadows(builder, value),
                CommandBody.Image.SetTexture value => Address(
                    builder,
                    value.ObjectId,
                    value.Address.Value,
                    Wire.CoreCommandKind.ImageSetTexture,
                    Wire.CoreCommandPayload.SetTexturePayload,
                    true
                ),
                CommandBody.Image.SetSize value => ImageSize(builder, value),
                CommandBody.Image.SetFit value => ImageFit(builder, value),
                CommandBody.Image.SetTint value => Tint(
                    builder,
                    value.ObjectId,
                    value.Tint,
                    value.OnConflict,
                    Wire.CoreCommandKind.ImageSetTint
                ),
                CommandBody.Image.TweenTint value => TweenTint(builder, value),
                CommandBody.Image.SetOpacity value => Opacity(
                    builder,
                    value.ObjectId,
                    value.Opacity,
                    value.OnConflict,
                    Wire.CoreCommandKind.ImageSetOpacity
                ),
                CommandBody.Image.TweenOpacity value => TweenOpacity(builder, value),
                CommandBody.Image.SetFaceCamera value => ObjectEnabled(
                    builder,
                    value.ObjectId,
                    value.FacesCamera,
                    Wire.CoreCommandKind.ImageSetFaceCamera,
                    Wire.CoreCommandPayload.ObjectEnabledPayload
                ),
                CommandBody.Text.SetContent value => TextContent(builder, value),
                CommandBody.Text.SetFont value => Address(
                    builder,
                    value.ObjectId,
                    value.Address.Value,
                    Wire.CoreCommandKind.TextSetFont,
                    Wire.CoreCommandPayload.SetFontPayload,
                    false
                ),
                CommandBody.Text.SetSize value => TextSize(
                    builder,
                    value.ObjectId,
                    value.Size,
                    value.OnConflict,
                    Wire.CoreCommandKind.TextSetSize
                ),
                CommandBody.Text.TweenSize value => TweenTextSize(builder, value),
                CommandBody.Text.SetColor value => Color(
                    builder,
                    value.ObjectId,
                    value.Color,
                    value.OnConflict,
                    Wire.CoreCommandKind.TextSetColor
                ),
                CommandBody.Text.TweenColor value => TweenColor(
                    builder,
                    value.ObjectId,
                    value.Color,
                    value.Tween,
                    value.OnConflict,
                    Wire.CoreCommandKind.TextTweenColor
                ),
                CommandBody.Text.SetAlignment value => TextAlignment(builder, value),
                CommandBody.Text.SetWrapping value => TextWrapping(builder, value),
                CommandBody.Text.SetRichText value => ObjectEnabled(
                    builder,
                    value.ObjectId,
                    value.IsRichText,
                    Wire.CoreCommandKind.TextSetRichText,
                    Wire.CoreCommandPayload.ObjectEnabledPayload
                ),
                CommandBody.Text.SetFaceCamera value => ObjectEnabled(
                    builder,
                    value.ObjectId,
                    value.FacesCamera,
                    Wire.CoreCommandKind.TextSetFaceCamera,
                    Wire.CoreCommandPayload.ObjectEnabledPayload
                ),
                CommandBody.Animator.Play value => AnimatorPlay(builder, value),
                CommandBody.Animator.CrossFade value => AnimatorCrossFade(builder, value),
                CommandBody.Animator.SetBool value => AnimatorBool(builder, value),
                CommandBody.Animator.SetInt value => AnimatorInt(builder, value),
                CommandBody.Animator.SetFloat value => AnimatorFloat(builder, value),
                CommandBody.Animator.SetTrigger value => AnimatorParameter(
                    builder,
                    value.ObjectId,
                    value.Parameter,
                    Wire.CoreCommandKind.AnimatorSetTrigger
                ),
                CommandBody.Animator.SetSpeed value => AnimatorSpeed(builder, value),
                CommandBody.Particle.Play value => ParticlePlay(builder, value),
                CommandBody.Particle.Stop value => ParticleStop(builder, value),
                CommandBody.Particle.Spawn value => ParticleSpawn(builder, value),
                CommandBody.Audio.Play value => AudioPlay(builder, value),
                CommandBody.Audio.SetMix value => AudioMix(builder, value),
                CommandBody.Audio.Stop value => AudioStop(builder, value),
                CommandBody.Audio.Pause value => AudioPlayback(
                    builder,
                    value.AudioCommandId,
                    Wire.CoreCommandKind.AudioPause
                ),
                CommandBody.Audio.Resume value => AudioPlayback(
                    builder,
                    value.AudioCommandId,
                    Wire.CoreCommandKind.AudioResume
                ),
                CommandBody.Audio.Seek value => AudioSeek(builder, value),
                CommandBody.Audio.SetBuffering value => AudioBuffering(builder, value),
                CommandBody.Audio.Replace value => AudioReplace(builder, value),
                CommandBody.Audio.SetVolume value => AudioVolume(builder, value),
                CommandBody.Audio.TweenVolume value => TweenAudioVolume(builder, value),
                CommandBody.Time.Wait value => Wait(builder, value),
                CommandBody.Operation.Cancel value => Cancel(builder, value),
                CommandBody.Input.SetEnabled value => SetInputEnabled(builder, value),
                CommandBody.Input.SetCamera value => InputCamera(builder, value),
                CommandBody.Input.SetPointerEvents value => PointerEvents(builder, value),
                CommandBody.Input.SetGlobalKeys value => GlobalKeys(builder, value),
                CommandBody.Input.SetController value => ControllerInput(builder, value),
                CommandBody.Input.Capture value => InputCapture(builder, value.Value),
                CommandBody.Controller.Vibrate value => ControllerVibration(builder, value),
                CommandBody.VisualElement.Create value => VisualCreate(builder, value),
                CommandBody.VisualElement.Update value => VisualUpdate(builder, value),
                CommandBody.VisualElement.Destroy value => VisualDestroy(builder, value),
                CommandBody.VisualElement.PerformAction value => VisualAction(builder, value),
                CommandBody.Motion.ValueCommand value => MotionValue(builder, value),
                CommandBody.Motion.ValuePlayback value => MotionValuePlayback(builder, value),
                CommandBody.Motion.Playback value => MotionPlayback(builder, value),
                CommandBody.Motion.ControlledClock value => MotionControlledClock(builder, value),
                CommandBody.Motion.Control value => MotionControl(builder, value),
                CommandBody.Motion.Scope value => MotionScope(builder, value),
                CommandBody.Motion.DragControl value => MotionDragControl(builder, value),
                CommandBody.GeometryObservation value => GeometryObservation(builder, value),
                CommandBody.AccessibilityUpdate value => AccessibilityUpdate(builder, value),
                _ => throw new ArgumentException("Unknown core command body.", nameof(command)),
            };
            Wire.CoreCommand.StartCoreCommand(builder);
            Wire.CoreCommand.AddPayload(builder, payload.Offset);
            Wire.CoreCommand.AddPayloadType(builder, payload.Type);
            Wire.CoreCommand.AddKind(builder, payload.Kind);
            Wire.CoreCommand.AddBlocking(builder, command.IsBlocking);
            Wire.CoreCommand.AddCommandId(builder, Uuid(builder, command.Id.Value));
            return Wire.CoreCommand.EndCoreCommand(builder);
        }

        private static Payload Diagnostics(FlatBufferBuilder builder, CommandBody.Diagnostics value)
        {
            if (value.Command is not DiagnosticsCommand.SetReporting reporting)
                throw Unsupported(value);
            return new(
                Wire.CoreCommandKind.Diagnostics,
                Wire.CoreCommandPayload.DiagnosticsPayload,
                Wire.DiagnosticsPayload.CreateDiagnosticsPayload(
                    builder,
                    Wire.DiagnosticsOperation.SetReporting,
                    enabled: reporting.Enabled
                ).Value
            );
        }

        private static Payload WorldMotion(
            FlatBufferBuilder builder,
            CommandBody.Motion.SetWorldDescriptor value
        )
        {
            if (value.Descriptor is not null)
                throw Unsupported(value);
            Wire.WorldMotionPayload.StartWorldMotionPayload(builder);
            Wire.WorldMotionPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.MotionSetWorldDescriptor,
                Wire.CoreCommandPayload.WorldMotionPayload,
                Wire.WorldMotionPayload.EndWorldMotionPayload(builder).Value
            );
        }

        private readonly struct Payload
        {
            internal Payload(Wire.CoreCommandKind kind, Wire.CoreCommandPayload type, int offset) =>
                (Kind, Type, Offset) = (kind, type, offset);

            internal Wire.CoreCommandKind Kind { get; }
            internal Wire.CoreCommandPayload Type { get; }
            internal int Offset { get; }
        }

        private static Wire.ConflictPolicy Conflict(ConflictPolicy value) =>
            (Wire.ConflictPolicy)value;

        private static Offset<Wire.Tween> Tween(FlatBufferBuilder builder, Tween value)
        {
            (Wire.TweenRepeatKind kind, uint count, Wire.RepeatMode mode) = value.Repeat switch
            {
                TweenRepeat.Once => (Wire.TweenRepeatKind.Once, 0u, Wire.RepeatMode.Restart),
                TweenRepeat.Count repeated => (
                    Wire.TweenRepeatKind.Count,
                    repeated.AdditionalTraversals,
                    (Wire.RepeatMode)repeated.Mode
                ),
                TweenRepeat.Forever repeated => (
                    Wire.TweenRepeatKind.Forever,
                    0u,
                    (Wire.RepeatMode)repeated.Mode
                ),
                _ => throw new ArgumentException("Unknown tween repetition.", nameof(value)),
            };
            return Wire.Tween.CreateTween(
                builder,
                Milliseconds(value.Delay),
                Milliseconds(value.Duration),
                (Wire.Easing)value.Easing,
                kind,
                count,
                mode
            );
        }
    }
}

#nullable enable

using MessagePack;
using MessagePack.Formatters;

namespace Masonry
{
    internal static partial class ProtocolFormat
    {
        private static void WriteCommand(ref MessagePackWriter writer, Command value)
        {
            WriteArrayHeader(ref writer, 3);
            WriteCommandId(ref writer, value.Id);
            writer.Write(value.IsBlocking);
            WriteCommandBody(ref writer, value.Body);
        }

        private static void WriteAnyCommand<TPayload>(
            ref MessagePackWriter writer,
            ICommand value,
            IMessagePackFormatter<TPayload> payloadFormatter,
            MessagePackSerializerOptions options
        )
        {
            switch (value)
            {
                case Command command:
                    WriteVariantHeader(ref writer, "Core");
                    WriteCommand(ref writer, command);
                    break;
                case CustomCommand<TPayload> custom:
                    WriteVariantHeader(ref writer, "Custom");
                    WriteArrayHeader(ref writer, 4);
                    WriteCommandId(ref writer, custom.Id);
                    WriteString(ref writer, custom.Type);
                    writer.Write(custom.IsBlocking);
                    payloadFormatter.Serialize(ref writer, custom.Payload, options);
                    break;
                default:
                    throw new MessagePackSerializationException(
                        $"Expected Command or CustomCommand<{typeof(TPayload).Name}>."
                    );
            }
        }

        private static void WriteCommandBody(ref MessagePackWriter writer, CommandBody value)
        {
            switch (value)
            {
                case CommandBody.Assets.ReplaceSet command:
                    BeginBody(ref writer, "AssetsReplaceSet", 1);
                    WritePreparedAssets(ref writer, command.PreparedAssets);
                    break;
                case CommandBody.Scene.Load command:
                    BeginBody(ref writer, "SceneLoad", 3);
                    WriteSceneId(ref writer, command.SceneId);
                    WriteSceneAddress(ref writer, command.Address);
                    writer.Write(command.MakePrimary);
                    break;
                case CommandBody.Scene.Unload command:
                    WriteSceneBody(ref writer, "SceneUnload", command.SceneId);
                    break;
                case CommandBody.Scene.SetPrimary command:
                    WriteSceneBody(ref writer, "SceneSetPrimary", command.SceneId);
                    break;
                case CommandBody.Object.Create command:
                    BeginBody(ref writer, "ObjectCreate", 1);
                    WriteGameObject(ref writer, command.GameObject);
                    break;
                case CommandBody.Object.Destroy command:
                    WriteObjectBody(ref writer, "ObjectDestroy", command.ObjectId);
                    break;
                case CommandBody.Object.SetActive command:
                    WriteObjectEnabledBody(
                        ref writer,
                        "ObjectSetActive",
                        command.ObjectId,
                        command.IsActive
                    );
                    break;
                case CommandBody.Object.Reparent command:
                    BeginBody(ref writer, "ObjectReparent", 3);
                    WriteObjectId(ref writer, command.ObjectId);
                    WriteOptionalObjectId(ref writer, command.ParentId);
                    writer.Write(command.WorldPositionStays);
                    break;
                case CommandBody.Transform.SetLocalPosition command:
                    WritePositionBody(
                        ref writer,
                        "TransformSetLocalPosition",
                        command.ObjectId,
                        command.Position,
                        command.OnConflict
                    );
                    break;
                case CommandBody.Transform.SetWorldPosition command:
                    WritePositionBody(
                        ref writer,
                        "TransformSetWorldPosition",
                        command.ObjectId,
                        command.Position,
                        command.OnConflict
                    );
                    break;
                case CommandBody.Transform.TweenLocalPosition command:
                    WriteTweenPositionBody(
                        ref writer,
                        "TransformTweenLocalPosition",
                        command.ObjectId,
                        command.Position,
                        command.Tween,
                        command.OnConflict
                    );
                    break;
                case CommandBody.Transform.TweenWorldPosition command:
                    WriteTweenPositionBody(
                        ref writer,
                        "TransformTweenWorldPosition",
                        command.ObjectId,
                        command.Position,
                        command.Tween,
                        command.OnConflict
                    );
                    break;
                case CommandBody.Transform.SetLocalRotation command:
                    WriteRotationBody(
                        ref writer,
                        "TransformSetLocalRotation",
                        command.ObjectId,
                        command.Rotation,
                        command.OnConflict
                    );
                    break;
                case CommandBody.Transform.SetWorldRotation command:
                    WriteRotationBody(
                        ref writer,
                        "TransformSetWorldRotation",
                        command.ObjectId,
                        command.Rotation,
                        command.OnConflict
                    );
                    break;
                case CommandBody.Transform.TweenLocalRotation command:
                    WriteTweenRotationBody(
                        ref writer,
                        "TransformTweenLocalRotation",
                        command.ObjectId,
                        command.Rotation,
                        command.Tween,
                        command.OnConflict
                    );
                    break;
                case CommandBody.Transform.TweenWorldRotation command:
                    WriteTweenRotationBody(
                        ref writer,
                        "TransformTweenWorldRotation",
                        command.ObjectId,
                        command.Rotation,
                        command.Tween,
                        command.OnConflict
                    );
                    break;
                case CommandBody.Transform.SetLocalScale command:
                    WriteScaleBody(
                        ref writer,
                        "TransformSetLocalScale",
                        command.ObjectId,
                        command.Scale,
                        command.OnConflict
                    );
                    break;
                case CommandBody.Transform.TweenLocalScale command:
                    WriteTweenScaleBody(
                        ref writer,
                        "TransformTweenLocalScale",
                        command.ObjectId,
                        command.Scale,
                        command.Tween,
                        command.OnConflict
                    );
                    break;
                case CommandBody.Renderer.SetMaterial command:
                    BeginPropertyBody(ref writer, "RendererSetMaterial", command.OnConflict, 3);
                    WriteObjectId(ref writer, command.ObjectId);
                    WriteMaterialAddress(ref writer, command.Address);
                    WriteOptionalUInt32(ref writer, command.Slot);
                    break;
                case CommandBody.Camera.SetEnabled command:
                    WriteObjectEnabledBody(
                        ref writer,
                        "CameraSetEnabled",
                        command.ObjectId,
                        command.IsEnabled
                    );
                    break;
                case CommandBody.Camera.SetPerspective command:
                    BeginPropertyBody(ref writer, "CameraSetPerspective", command.OnConflict, 2);
                    WriteObjectId(ref writer, command.ObjectId);
                    writer.Write(command.FieldOfView);
                    break;
                case CommandBody.Camera.TweenFieldOfView command:
                    BeginPropertyBody(ref writer, "CameraTweenFieldOfView", command.OnConflict, 3);
                    WriteObjectId(ref writer, command.ObjectId);
                    writer.Write(command.FieldOfView);
                    WriteTween(ref writer, command.Tween);
                    break;
                case CommandBody.Camera.SetOrthographic command:
                    BeginPropertyBody(ref writer, "CameraSetOrthographic", command.OnConflict, 2);
                    WriteObjectId(ref writer, command.ObjectId);
                    writer.Write(command.Size);
                    break;
                case CommandBody.Camera.TweenOrthographicSize command:
                    BeginPropertyBody(
                        ref writer,
                        "CameraTweenOrthographicSize",
                        command.OnConflict,
                        3
                    );
                    WriteObjectId(ref writer, command.ObjectId);
                    writer.Write(command.Size);
                    WriteTween(ref writer, command.Tween);
                    break;
                case CommandBody.Camera.SetClipping command:
                    BeginBody(ref writer, "CameraSetClipping", 3);
                    WriteObjectId(ref writer, command.ObjectId);
                    writer.Write(command.Near);
                    writer.Write(command.Far);
                    break;
                case CommandBody.Camera.SetClear command:
                    BeginBody(ref writer, "CameraSetClear", 3);
                    WriteObjectId(ref writer, command.ObjectId);
                    WriteCameraClearMode(ref writer, command.ClearMode);
                    WriteOptionalColor(ref writer, command.ClearColor);
                    break;
                case CommandBody.Light.SetEnabled command:
                    WriteObjectEnabledBody(
                        ref writer,
                        "LightSetEnabled",
                        command.ObjectId,
                        command.IsEnabled
                    );
                    break;
                case CommandBody.Light.SetType command:
                    BeginBody(ref writer, "LightSetType", 2);
                    WriteObjectId(ref writer, command.ObjectId);
                    WriteLightType(ref writer, command.Type);
                    break;
                case CommandBody.Light.SetColor command:
                    WriteColorBody(
                        ref writer,
                        "LightSetColor",
                        command.ObjectId,
                        command.Color,
                        command.OnConflict
                    );
                    break;
                case CommandBody.Light.TweenColor command:
                    WriteTweenColorBody(
                        ref writer,
                        "LightTweenColor",
                        command.ObjectId,
                        command.Color,
                        command.Tween,
                        command.OnConflict
                    );
                    break;
                case CommandBody.Light.SetIntensity command:
                    WriteDoublePropertyBody(
                        ref writer,
                        "LightSetIntensity",
                        command.ObjectId,
                        command.Intensity,
                        command.OnConflict
                    );
                    break;
                case CommandBody.Light.TweenIntensity command:
                    BeginPropertyBody(ref writer, "LightTweenIntensity", command.OnConflict, 3);
                    WriteObjectId(ref writer, command.ObjectId);
                    writer.Write(command.Intensity);
                    WriteTween(ref writer, command.Tween);
                    break;
                case CommandBody.Light.SetRange command:
                    BeginBody(ref writer, "LightSetRange", 2);
                    WriteObjectId(ref writer, command.ObjectId);
                    writer.Write(command.Range);
                    break;
                case CommandBody.Light.SetSpotAngle command:
                    BeginBody(ref writer, "LightSetSpotAngle", 3);
                    WriteObjectId(ref writer, command.ObjectId);
                    writer.Write(command.OuterSpotAngle);
                    writer.Write(command.InnerSpotAngle);
                    break;
                case CommandBody.Light.SetShadows command:
                    BeginBody(ref writer, "LightSetShadows", 2);
                    WriteObjectId(ref writer, command.ObjectId);
                    WriteShadowMode(ref writer, command.Shadows);
                    break;
                case CommandBody.Image.SetTexture command:
                    BeginBody(ref writer, "ImageSetTexture", 2);
                    WriteObjectId(ref writer, command.ObjectId);
                    WriteTextureAddress(ref writer, command.Address);
                    break;
                case CommandBody.Image.SetSize command:
                    BeginBody(ref writer, "ImageSetSize", 3);
                    WriteObjectId(ref writer, command.ObjectId);
                    writer.Write(command.Width);
                    writer.Write(command.Height);
                    break;
                case CommandBody.Image.SetFit command:
                    BeginBody(ref writer, "ImageSetFit", 2);
                    WriteObjectId(ref writer, command.ObjectId);
                    WriteImageFit(ref writer, command.Fit);
                    break;
                case CommandBody.Image.SetTint command:
                    WriteTintBody(ref writer, "ImageSetTint", command);
                    break;
                case CommandBody.Image.TweenTint command:
                    BeginPropertyBody(ref writer, "ImageTweenTint", command.OnConflict, 3);
                    WriteObjectId(ref writer, command.ObjectId);
                    WriteRgbColor(ref writer, command.Tint);
                    WriteTween(ref writer, command.Tween);
                    break;
                case CommandBody.Image.SetOpacity command:
                    WriteDoublePropertyBody(
                        ref writer,
                        "ImageSetOpacity",
                        command.ObjectId,
                        command.Opacity,
                        command.OnConflict
                    );
                    break;
                case CommandBody.Image.TweenOpacity command:
                    BeginPropertyBody(ref writer, "ImageTweenOpacity", command.OnConflict, 3);
                    WriteObjectId(ref writer, command.ObjectId);
                    writer.Write(command.Opacity);
                    WriteTween(ref writer, command.Tween);
                    break;
                case CommandBody.Image.SetFaceCamera command:
                    WriteObjectEnabledBody(
                        ref writer,
                        "ImageSetFaceCamera",
                        command.ObjectId,
                        command.FacesCamera
                    );
                    break;
                case CommandBody.Text.SetContent command:
                    BeginBody(ref writer, "TextSetContent", 2);
                    WriteObjectId(ref writer, command.ObjectId);
                    WriteString(ref writer, command.Content);
                    break;
                case CommandBody.Text.SetFont command:
                    BeginBody(ref writer, "TextSetFont", 2);
                    WriteObjectId(ref writer, command.ObjectId);
                    WriteFontAddress(ref writer, command.Address);
                    break;
                case CommandBody.Text.SetSize command:
                    WriteDoublePropertyBody(
                        ref writer,
                        "TextSetSize",
                        command.ObjectId,
                        command.Size,
                        command.OnConflict
                    );
                    break;
                case CommandBody.Text.TweenSize command:
                    BeginPropertyBody(ref writer, "TextTweenSize", command.OnConflict, 3);
                    WriteObjectId(ref writer, command.ObjectId);
                    writer.Write(command.Size);
                    WriteTween(ref writer, command.Tween);
                    break;
                case CommandBody.Text.SetColor command:
                    WriteColorBody(
                        ref writer,
                        "TextSetColor",
                        command.ObjectId,
                        command.Color,
                        command.OnConflict
                    );
                    break;
                case CommandBody.Text.TweenColor command:
                    WriteTweenColorBody(
                        ref writer,
                        "TextTweenColor",
                        command.ObjectId,
                        command.Color,
                        command.Tween,
                        command.OnConflict
                    );
                    break;
                case CommandBody.Text.SetAlignment command:
                    BeginBody(ref writer, "TextSetAlignment", 3);
                    WriteObjectId(ref writer, command.ObjectId);
                    WriteHorizontalAlignment(ref writer, command.Horizontal);
                    WriteVerticalAlignment(ref writer, command.Vertical);
                    break;
                case CommandBody.Text.SetWrapping command:
                    BeginBody(ref writer, "TextSetWrapping", 2);
                    WriteObjectId(ref writer, command.ObjectId);
                    WriteOptionalDouble(ref writer, command.WrapWidth);
                    break;
                case CommandBody.Text.SetRichText command:
                    WriteObjectEnabledBody(
                        ref writer,
                        "TextSetRichText",
                        command.ObjectId,
                        command.IsRichText
                    );
                    break;
                case CommandBody.Text.SetFaceCamera command:
                    WriteObjectEnabledBody(
                        ref writer,
                        "TextSetFaceCamera",
                        command.ObjectId,
                        command.FacesCamera
                    );
                    break;
                case CommandBody.Animator.Play command:
                    BeginBody(ref writer, "AnimatorPlay", 5);
                    WriteObjectId(ref writer, command.ObjectId);
                    WriteString(ref writer, command.State);
                    writer.Write(command.Layer);
                    writer.Write(command.NormalizedStartTime);
                    writer.Write(Milliseconds(command.Wait));
                    break;
                case CommandBody.Animator.CrossFade command:
                    BeginBody(ref writer, "AnimatorCrossFade", 6);
                    WriteObjectId(ref writer, command.ObjectId);
                    WriteString(ref writer, command.State);
                    writer.Write(command.Layer);
                    writer.Write(command.NormalizedStartTime);
                    writer.Write(Milliseconds(command.Wait));
                    writer.Write(Milliseconds(command.CrossFadeDuration));
                    break;
                case CommandBody.Animator.SetBool command:
                    WriteAnimatorParameterBody(
                        ref writer,
                        "AnimatorSetBool",
                        command.ObjectId,
                        command.Parameter,
                        command.Value
                    );
                    break;
                case CommandBody.Animator.SetInt command:
                    WriteAnimatorParameterBody(
                        ref writer,
                        "AnimatorSetInt",
                        command.ObjectId,
                        command.Parameter,
                        command.Value
                    );
                    break;
                case CommandBody.Animator.SetFloat command:
                    WriteAnimatorParameterBody(
                        ref writer,
                        "AnimatorSetFloat",
                        command.ObjectId,
                        command.Parameter,
                        command.Value
                    );
                    break;
                case CommandBody.Animator.SetTrigger command:
                    BeginBody(ref writer, "AnimatorSetTrigger", 2);
                    WriteObjectId(ref writer, command.ObjectId);
                    WriteString(ref writer, command.Parameter);
                    break;
                case CommandBody.Animator.SetSpeed command:
                    BeginBody(ref writer, "AnimatorSetSpeed", 2);
                    WriteObjectId(ref writer, command.ObjectId);
                    writer.Write(command.Speed);
                    break;
                case CommandBody.Particle.Play command:
                    BeginBody(ref writer, "ParticlePlay", 2);
                    WriteObjectId(ref writer, command.ObjectId);
                    writer.Write(command.Restart);
                    break;
                case CommandBody.Particle.Stop command:
                    BeginBody(ref writer, "ParticleStop", 2);
                    WriteObjectId(ref writer, command.ObjectId);
                    writer.Write(command.Clear);
                    break;
                case CommandBody.Particle.Spawn command:
                    BeginBody(ref writer, "ParticleSpawn", 3);
                    WriteParticleEffectAddress(ref writer, command.Address);
                    WriteParticleSpawnLocation(ref writer, command.Location);
                    writer.Write(Milliseconds(command.Lifetime));
                    break;
                case CommandBody.Audio.Play command:
                    BeginBody(ref writer, "AudioPlay", 5);
                    WriteAudioClipAddress(ref writer, command.Address);
                    writer.Write(command.Volume);
                    writer.Write(command.Pitch);
                    writer.Write(command.Loop);
                    writer.Write(Milliseconds(command.FadeIn));
                    break;
                case CommandBody.Audio.Stop command:
                    BeginBody(ref writer, "AudioStop", 2);
                    WriteCommandId(ref writer, command.AudioCommandId);
                    writer.Write(Milliseconds(command.FadeOut));
                    break;
                case CommandBody.Audio.SetVolume command:
                    BeginPropertyBody(ref writer, "AudioSetVolume", command.OnConflict, 2);
                    WriteCommandId(ref writer, command.AudioCommandId);
                    writer.Write(command.Volume);
                    break;
                case CommandBody.Audio.TweenVolume command:
                    BeginPropertyBody(ref writer, "AudioTweenVolume", command.OnConflict, 3);
                    WriteCommandId(ref writer, command.AudioCommandId);
                    writer.Write(command.Volume);
                    WriteTween(ref writer, command.Tween);
                    break;
                case CommandBody.Time.Wait command:
                    BeginBody(ref writer, "TimeWait", 1);
                    writer.Write(Milliseconds(command.Duration));
                    break;
                case CommandBody.Operation.Cancel command:
                    BeginBody(ref writer, "OperationCancel", 1);
                    WriteCommandId(ref writer, command.CommandId);
                    break;
                case CommandBody.Input.SetEnabled command:
                    BeginBody(ref writer, "InputSetEnabled", 1);
                    writer.Write(command.IsEnabled);
                    break;
                case CommandBody.Input.SetCamera command:
                    WriteObjectBody(ref writer, "InputSetCamera", command.ObjectId);
                    break;
                case CommandBody.Input.SetPointerEvents command:
                    BeginBody(ref writer, "InputSetPointerEvents", 2);
                    WriteObjectId(ref writer, command.ObjectId);
                    WritePointerEvents(ref writer, command.Events);
                    break;
                case CommandBody.Input.SetGlobalKeys command:
                    BeginBody(ref writer, "InputSetGlobalKeys", 1);
                    WriteKeyCodes(ref writer, command.Keys);
                    break;
                default:
                    throw new MessagePackSerializationException("Unknown command body value.");
            }
        }

        private static void BeginBody(ref MessagePackWriter writer, string variant, int payloadSize)
        {
            WriteVariantHeader(ref writer, variant);
            WriteArrayHeader(ref writer, payloadSize);
        }

        private static void BeginPropertyBody(
            ref MessagePackWriter writer,
            string variant,
            ConflictPolicy conflict,
            int payloadSize
        )
        {
            WriteVariantHeader(ref writer, variant);
            WriteArrayHeader(ref writer, 2);
            WriteConflictPolicy(ref writer, conflict);
            WriteArrayHeader(ref writer, payloadSize);
        }

        private static void WriteSceneBody(
            ref MessagePackWriter writer,
            string variant,
            SceneId sceneId
        )
        {
            BeginBody(ref writer, variant, 1);
            WriteSceneId(ref writer, sceneId);
        }

        private static void WriteObjectBody(
            ref MessagePackWriter writer,
            string variant,
            ObjectId objectId
        )
        {
            BeginBody(ref writer, variant, 1);
            WriteObjectId(ref writer, objectId);
        }

        private static void WriteObjectEnabledBody(
            ref MessagePackWriter writer,
            string variant,
            ObjectId objectId,
            bool enabled
        )
        {
            BeginBody(ref writer, variant, 2);
            WriteObjectId(ref writer, objectId);
            writer.Write(enabled);
        }

        private static void WritePositionBody(
            ref MessagePackWriter writer,
            string variant,
            ObjectId objectId,
            Vector3 position,
            ConflictPolicy conflict
        )
        {
            BeginPropertyBody(ref writer, variant, conflict, 2);
            WriteObjectId(ref writer, objectId);
            WriteVector3(ref writer, position);
        }

        private static void WriteTweenPositionBody(
            ref MessagePackWriter writer,
            string variant,
            ObjectId objectId,
            Vector3 position,
            Tween tween,
            ConflictPolicy conflict
        )
        {
            BeginPropertyBody(ref writer, variant, conflict, 3);
            WriteObjectId(ref writer, objectId);
            WriteVector3(ref writer, position);
            WriteTween(ref writer, tween);
        }

        private static void WriteRotationBody(
            ref MessagePackWriter writer,
            string variant,
            ObjectId objectId,
            Quaternion rotation,
            ConflictPolicy conflict
        )
        {
            BeginPropertyBody(ref writer, variant, conflict, 2);
            WriteObjectId(ref writer, objectId);
            WriteQuaternion(ref writer, rotation);
        }

        private static void WriteTweenRotationBody(
            ref MessagePackWriter writer,
            string variant,
            ObjectId objectId,
            Quaternion rotation,
            Tween tween,
            ConflictPolicy conflict
        )
        {
            BeginPropertyBody(ref writer, variant, conflict, 3);
            WriteObjectId(ref writer, objectId);
            WriteQuaternion(ref writer, rotation);
            WriteTween(ref writer, tween);
        }

        private static void WriteScaleBody(
            ref MessagePackWriter writer,
            string variant,
            ObjectId objectId,
            Vector3 scale,
            ConflictPolicy conflict
        )
        {
            BeginPropertyBody(ref writer, variant, conflict, 2);
            WriteObjectId(ref writer, objectId);
            WriteVector3(ref writer, scale);
        }

        private static void WriteTweenScaleBody(
            ref MessagePackWriter writer,
            string variant,
            ObjectId objectId,
            Vector3 scale,
            Tween tween,
            ConflictPolicy conflict
        )
        {
            BeginPropertyBody(ref writer, variant, conflict, 3);
            WriteObjectId(ref writer, objectId);
            WriteVector3(ref writer, scale);
            WriteTween(ref writer, tween);
        }

        private static void WriteColorBody(
            ref MessagePackWriter writer,
            string variant,
            ObjectId objectId,
            Color color,
            ConflictPolicy conflict
        )
        {
            BeginPropertyBody(ref writer, variant, conflict, 2);
            WriteObjectId(ref writer, objectId);
            WriteColor(ref writer, color);
        }

        private static void WriteTweenColorBody(
            ref MessagePackWriter writer,
            string variant,
            ObjectId objectId,
            Color color,
            Tween tween,
            ConflictPolicy conflict
        )
        {
            BeginPropertyBody(ref writer, variant, conflict, 3);
            WriteObjectId(ref writer, objectId);
            WriteColor(ref writer, color);
            WriteTween(ref writer, tween);
        }

        private static void WriteTintBody(
            ref MessagePackWriter writer,
            string variant,
            CommandBody.Image.SetTint command
        )
        {
            BeginPropertyBody(ref writer, variant, command.OnConflict, 2);
            WriteObjectId(ref writer, command.ObjectId);
            WriteRgbColor(ref writer, command.Tint);
        }

        private static void WriteDoublePropertyBody(
            ref MessagePackWriter writer,
            string variant,
            ObjectId objectId,
            double value,
            ConflictPolicy conflict
        )
        {
            BeginPropertyBody(ref writer, variant, conflict, 2);
            WriteObjectId(ref writer, objectId);
            writer.Write(value);
        }

        private static void WriteAnimatorParameterBody<T>(
            ref MessagePackWriter writer,
            string variant,
            ObjectId objectId,
            string parameter,
            T value
        )
        {
            BeginBody(ref writer, variant, 3);
            WriteObjectId(ref writer, objectId);
            WriteString(ref writer, parameter);
            switch (value)
            {
                case bool boolean:
                    writer.Write(boolean);
                    break;
                case int integer:
                    writer.Write(integer);
                    break;
                case double number:
                    writer.Write(number);
                    break;
                default:
                    throw new MessagePackSerializationException("Unsupported Animator value.");
            }
        }

        private static void WriteParticleSpawnLocation(
            ref MessagePackWriter writer,
            ParticleSpawnLocation value
        )
        {
            switch (value)
            {
                case ParticleSpawnLocation.AtGameObject gameObject:
                    WriteVariantHeader(ref writer, "GameObject");
                    WriteObjectId(ref writer, gameObject.ObjectId);
                    break;
                case ParticleSpawnLocation.AtWorldPosition position:
                    WriteVariantHeader(ref writer, "WorldPosition");
                    WriteVector3(ref writer, position.Position);
                    break;
                default:
                    throw new MessagePackSerializationException("Unknown particle spawn location.");
            }
        }

        private static void WriteOptionalUInt32(ref MessagePackWriter writer, uint? value)
        {
            if (value is null)
            {
                writer.WriteNil();
            }
            else
            {
                writer.Write(value.Value);
            }
        }

        private static void WriteOptionalDouble(ref MessagePackWriter writer, double? value)
        {
            if (value is null)
            {
                writer.WriteNil();
            }
            else
            {
                writer.Write(value.Value);
            }
        }

        private static void WriteOptionalColor(ref MessagePackWriter writer, Color? value)
        {
            if (value is null)
            {
                writer.WriteNil();
            }
            else
            {
                WriteColor(ref writer, value.Value);
            }
        }
    }
}

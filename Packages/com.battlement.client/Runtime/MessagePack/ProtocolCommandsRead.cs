#nullable enable

using System;
using MessagePack;
using MessagePack.Formatters;

namespace Battlement
{
    internal static partial class ProtocolFormat
    {
        private static Command ReadCommand(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 3);
            CommandId id = ReadCommandId(ref reader);
            bool blocking = reader.ReadBoolean();
            CommandBody body = ReadCommandBody(ref reader);
            return new Command(id, body, blocking);
        }

        private static ICommand ReadAnyCommand<TPayload>(
            ref MessagePackReader reader,
            IMessagePackFormatter<TPayload> payloadFormatter,
            MessagePackSerializerOptions options
        )
        {
            string variant = ReadVariantHeader(ref reader);
            switch (variant)
            {
                case "Core":
                    return ReadCommand(ref reader);
                case "Custom":
                    ReadArrayHeader(ref reader, 4);
                    CommandId id = ReadCommandId(ref reader);
                    string type = ReadString(ref reader);
                    bool blocking = reader.ReadBoolean();
                    TPayload payload = payloadFormatter.Deserialize(ref reader, options);
                    return new CustomCommand<TPayload>(id, type, payload, blocking);
                default:
                    throw UnknownVariant("command", variant);
            }
        }

        private static CommandBody ReadCommandBody(ref MessagePackReader reader)
        {
            string variant = ReadVariantHeader(ref reader);
            switch (variant)
            {
                case "AssetsReplaceSet":
                    ReadArrayHeader(ref reader, 1);
                    return new CommandBody.Assets.ReplaceSet(ReadPreparedAssets(ref reader));
                case "SceneLoad":
                    ReadArrayHeader(ref reader, 3);
                    return new CommandBody.Scene.Load(
                        ReadSceneId(ref reader),
                        ReadSceneAddress(ref reader),
                        reader.ReadBoolean()
                    );
                case "SceneUnload":
                    return new CommandBody.Scene.Unload(ReadScenePayload(ref reader));
                case "SceneSetPrimary":
                    return new CommandBody.Scene.SetPrimary(ReadScenePayload(ref reader));
                case "ObjectCreate":
                    ReadArrayHeader(ref reader, 1);
                    return new CommandBody.Object.Create(ReadGameObject(ref reader));
                case "ObjectDestroy":
                    return new CommandBody.Object.Destroy(ReadObjectPayload(ref reader));
                case "ObjectSetActive":
                    ReadArrayHeader(ref reader, 2);
                    return new CommandBody.Object.SetActive(
                        ReadObjectId(ref reader),
                        reader.ReadBoolean()
                    );
                case "ObjectReparent":
                    ReadArrayHeader(ref reader, 3);
                    return new CommandBody.Object.Reparent(
                        ReadObjectId(ref reader),
                        ReadOptionalObjectId(ref reader),
                        reader.ReadBoolean()
                    );
                case "TransformSetLocalPosition":
                    return ReadSetPosition(ref reader, local: true);
                case "TransformSetWorldPosition":
                    return ReadSetPosition(ref reader, local: false);
                case "TransformTweenLocalPosition":
                    return ReadTweenPosition(ref reader, local: true);
                case "TransformTweenWorldPosition":
                    return ReadTweenPosition(ref reader, local: false);
                case "TransformSetLocalRotation":
                    return ReadSetRotation(ref reader, local: true);
                case "TransformSetWorldRotation":
                    return ReadSetRotation(ref reader, local: false);
                case "TransformTweenLocalRotation":
                    return ReadTweenRotation(ref reader, local: true);
                case "TransformTweenWorldRotation":
                    return ReadTweenRotation(ref reader, local: false);
                case "TransformSetLocalScale":
                    return ReadSetScale(ref reader);
                case "TransformTweenLocalScale":
                    return ReadTweenScale(ref reader);
                case "RendererSetMaterial":
                {
                    ConflictPolicy conflict = ReadPropertyHeader(ref reader, 3);
                    ObjectId objectId = ReadObjectId(ref reader);
                    MaterialAddress address = ReadMaterialAddress(ref reader);
                    uint? slot = ReadOptionalUInt32(ref reader);
                    return new CommandBody.Renderer.SetMaterial(objectId, address, slot, conflict);
                }
                case "CameraSetEnabled":
                {
                    (ObjectId objectId, bool enabled) = ReadObjectEnabled(ref reader);
                    return new CommandBody.Camera.SetEnabled(objectId, enabled);
                }
                case "CameraSetPerspective":
                {
                    ConflictPolicy conflict = ReadPropertyHeader(ref reader, 2);
                    return new CommandBody.Camera.SetPerspective(
                        ReadObjectId(ref reader),
                        reader.ReadDouble(),
                        conflict
                    );
                }
                case "CameraTweenFieldOfView":
                {
                    ConflictPolicy conflict = ReadPropertyHeader(ref reader, 3);
                    return new CommandBody.Camera.TweenFieldOfView(
                        ReadObjectId(ref reader),
                        reader.ReadDouble(),
                        ReadTween(ref reader),
                        conflict
                    );
                }
                case "CameraSetOrthographic":
                {
                    ConflictPolicy conflict = ReadPropertyHeader(ref reader, 2);
                    return new CommandBody.Camera.SetOrthographic(
                        ReadObjectId(ref reader),
                        reader.ReadDouble(),
                        conflict
                    );
                }
                case "CameraTweenOrthographicSize":
                {
                    ConflictPolicy conflict = ReadPropertyHeader(ref reader, 3);
                    return new CommandBody.Camera.TweenOrthographicSize(
                        ReadObjectId(ref reader),
                        reader.ReadDouble(),
                        ReadTween(ref reader),
                        conflict
                    );
                }
                case "CameraSetClipping":
                    ReadArrayHeader(ref reader, 3);
                    return new CommandBody.Camera.SetClipping(
                        ReadObjectId(ref reader),
                        reader.ReadDouble(),
                        reader.ReadDouble()
                    );
                case "CameraSetClear":
                    ReadArrayHeader(ref reader, 3);
                    return new CommandBody.Camera.SetClear(
                        ReadObjectId(ref reader),
                        ReadCameraClearMode(ref reader),
                        ReadOptionalColor(ref reader)
                    );
                case "LightSetEnabled":
                {
                    (ObjectId objectId, bool enabled) = ReadObjectEnabled(ref reader);
                    return new CommandBody.Light.SetEnabled(objectId, enabled);
                }
                case "LightSetType":
                    ReadArrayHeader(ref reader, 2);
                    return new CommandBody.Light.SetType(
                        ReadObjectId(ref reader),
                        ReadLightType(ref reader)
                    );
                case "LightSetColor":
                    return ReadSetColor(ref reader, light: true);
                case "LightTweenColor":
                    return ReadTweenColor(ref reader, light: true);
                case "LightSetIntensity":
                {
                    ConflictPolicy conflict = ReadPropertyHeader(ref reader, 2);
                    return new CommandBody.Light.SetIntensity(
                        ReadObjectId(ref reader),
                        reader.ReadDouble(),
                        conflict
                    );
                }
                case "LightTweenIntensity":
                {
                    ConflictPolicy conflict = ReadPropertyHeader(ref reader, 3);
                    return new CommandBody.Light.TweenIntensity(
                        ReadObjectId(ref reader),
                        reader.ReadDouble(),
                        ReadTween(ref reader),
                        conflict
                    );
                }
                case "LightSetRange":
                    ReadArrayHeader(ref reader, 2);
                    return new CommandBody.Light.SetRange(
                        ReadObjectId(ref reader),
                        reader.ReadDouble()
                    );
                case "LightSetSpotAngle":
                    ReadArrayHeader(ref reader, 3);
                    return new CommandBody.Light.SetSpotAngle(
                        ReadObjectId(ref reader),
                        reader.ReadDouble(),
                        reader.ReadDouble()
                    );
                case "LightSetShadows":
                    ReadArrayHeader(ref reader, 2);
                    return new CommandBody.Light.SetShadows(
                        ReadObjectId(ref reader),
                        ReadShadowMode(ref reader)
                    );
                case "ImageSetTexture":
                    ReadArrayHeader(ref reader, 2);
                    return new CommandBody.Image.SetTexture(
                        ReadObjectId(ref reader),
                        ReadTextureAddress(ref reader)
                    );
                case "ImageSetSize":
                    ReadArrayHeader(ref reader, 3);
                    return new CommandBody.Image.SetSize(
                        ReadObjectId(ref reader),
                        reader.ReadDouble(),
                        reader.ReadDouble()
                    );
                case "ImageSetFit":
                    ReadArrayHeader(ref reader, 2);
                    return new CommandBody.Image.SetFit(
                        ReadObjectId(ref reader),
                        ReadImageFit(ref reader)
                    );
                case "ImageSetTint":
                {
                    ConflictPolicy conflict = ReadPropertyHeader(ref reader, 2);
                    return new CommandBody.Image.SetTint(
                        ReadObjectId(ref reader),
                        ReadRgbColor(ref reader),
                        conflict
                    );
                }
                case "ImageTweenTint":
                {
                    ConflictPolicy conflict = ReadPropertyHeader(ref reader, 3);
                    return new CommandBody.Image.TweenTint(
                        ReadObjectId(ref reader),
                        ReadRgbColor(ref reader),
                        ReadTween(ref reader),
                        conflict
                    );
                }
                case "ImageSetOpacity":
                {
                    ConflictPolicy conflict = ReadPropertyHeader(ref reader, 2);
                    return new CommandBody.Image.SetOpacity(
                        ReadObjectId(ref reader),
                        reader.ReadDouble(),
                        conflict
                    );
                }
                case "ImageTweenOpacity":
                {
                    ConflictPolicy conflict = ReadPropertyHeader(ref reader, 3);
                    return new CommandBody.Image.TweenOpacity(
                        ReadObjectId(ref reader),
                        reader.ReadDouble(),
                        ReadTween(ref reader),
                        conflict
                    );
                }
                case "ImageSetFaceCamera":
                {
                    (ObjectId objectId, bool enabled) = ReadObjectEnabled(ref reader);
                    return new CommandBody.Image.SetFaceCamera(objectId, enabled);
                }
                case "TextSetContent":
                    ReadArrayHeader(ref reader, 2);
                    return new CommandBody.Text.SetContent(
                        ReadObjectId(ref reader),
                        ReadString(ref reader)
                    );
                case "TextSetFont":
                    ReadArrayHeader(ref reader, 2);
                    return new CommandBody.Text.SetFont(
                        ReadObjectId(ref reader),
                        ReadFontAddress(ref reader)
                    );
                case "TextSetSize":
                {
                    ConflictPolicy conflict = ReadPropertyHeader(ref reader, 2);
                    return new CommandBody.Text.SetSize(
                        ReadObjectId(ref reader),
                        reader.ReadDouble(),
                        conflict
                    );
                }
                case "TextTweenSize":
                {
                    ConflictPolicy conflict = ReadPropertyHeader(ref reader, 3);
                    return new CommandBody.Text.TweenSize(
                        ReadObjectId(ref reader),
                        reader.ReadDouble(),
                        ReadTween(ref reader),
                        conflict
                    );
                }
                case "TextSetColor":
                    return ReadSetColor(ref reader, light: false);
                case "TextTweenColor":
                    return ReadTweenColor(ref reader, light: false);
                case "TextSetAlignment":
                    ReadArrayHeader(ref reader, 3);
                    return new CommandBody.Text.SetAlignment(
                        ReadObjectId(ref reader),
                        ReadHorizontalAlignment(ref reader),
                        ReadVerticalAlignment(ref reader)
                    );
                case "TextSetWrapping":
                    ReadArrayHeader(ref reader, 2);
                    return new CommandBody.Text.SetWrapping(
                        ReadObjectId(ref reader),
                        ReadOptionalDouble(ref reader)
                    );
                case "TextSetRichText":
                {
                    (ObjectId objectId, bool enabled) = ReadObjectEnabled(ref reader);
                    return new CommandBody.Text.SetRichText(objectId, enabled);
                }
                case "TextSetFaceCamera":
                {
                    (ObjectId objectId, bool enabled) = ReadObjectEnabled(ref reader);
                    return new CommandBody.Text.SetFaceCamera(objectId, enabled);
                }
                case "AnimatorPlay":
                    ReadArrayHeader(ref reader, 5);
                    return new CommandBody.Animator.Play(
                        ReadObjectId(ref reader),
                        ReadString(ref reader),
                        reader.ReadUInt32(),
                        reader.ReadDouble(),
                        ReadDuration(ref reader)
                    );
                case "AnimatorCrossFade":
                {
                    ReadArrayHeader(ref reader, 6);
                    ObjectId objectId = ReadObjectId(ref reader);
                    string state = ReadString(ref reader);
                    uint layer = reader.ReadUInt32();
                    double startTime = reader.ReadDouble();
                    TimeSpan wait = ReadDuration(ref reader);
                    TimeSpan crossFade = ReadDuration(ref reader);
                    return new CommandBody.Animator.CrossFade(
                        objectId,
                        state,
                        crossFade,
                        layer,
                        startTime,
                        wait
                    );
                }
                case "AnimatorSetBool":
                    ReadArrayHeader(ref reader, 3);
                    return new CommandBody.Animator.SetBool(
                        ReadObjectId(ref reader),
                        ReadString(ref reader),
                        reader.ReadBoolean()
                    );
                case "AnimatorSetInt":
                    ReadArrayHeader(ref reader, 3);
                    return new CommandBody.Animator.SetInt(
                        ReadObjectId(ref reader),
                        ReadString(ref reader),
                        reader.ReadInt32()
                    );
                case "AnimatorSetFloat":
                    ReadArrayHeader(ref reader, 3);
                    return new CommandBody.Animator.SetFloat(
                        ReadObjectId(ref reader),
                        ReadString(ref reader),
                        reader.ReadDouble()
                    );
                case "AnimatorSetTrigger":
                    ReadArrayHeader(ref reader, 2);
                    return new CommandBody.Animator.SetTrigger(
                        ReadObjectId(ref reader),
                        ReadString(ref reader)
                    );
                case "AnimatorSetSpeed":
                    ReadArrayHeader(ref reader, 2);
                    return new CommandBody.Animator.SetSpeed(
                        ReadObjectId(ref reader),
                        reader.ReadDouble()
                    );
                case "ParticlePlay":
                    ReadArrayHeader(ref reader, 2);
                    return new CommandBody.Particle.Play(
                        ReadObjectId(ref reader),
                        reader.ReadBoolean()
                    );
                case "ParticleStop":
                    ReadArrayHeader(ref reader, 2);
                    return new CommandBody.Particle.Stop(
                        ReadObjectId(ref reader),
                        reader.ReadBoolean()
                    );
                case "ParticleSpawn":
                    ReadArrayHeader(ref reader, 3);
                    return new CommandBody.Particle.Spawn(
                        ReadParticleEffectAddress(ref reader),
                        ReadParticleSpawnLocation(ref reader),
                        ReadDuration(ref reader)
                    );
                case "AudioPlay":
                    ReadArrayHeader(ref reader, 5);
                    return new CommandBody.Audio.Play(
                        ReadAudioClipAddress(ref reader),
                        reader.ReadDouble(),
                        reader.ReadDouble(),
                        reader.ReadBoolean(),
                        ReadDuration(ref reader)
                    );
                case "AudioStop":
                    ReadArrayHeader(ref reader, 2);
                    return new CommandBody.Audio.Stop(
                        ReadCommandId(ref reader),
                        ReadDuration(ref reader)
                    );
                case "AudioSetVolume":
                {
                    ConflictPolicy conflict = ReadPropertyHeader(ref reader, 2);
                    return new CommandBody.Audio.SetVolume(
                        ReadCommandId(ref reader),
                        reader.ReadDouble(),
                        conflict
                    );
                }
                case "AudioTweenVolume":
                {
                    ConflictPolicy conflict = ReadPropertyHeader(ref reader, 3);
                    return new CommandBody.Audio.TweenVolume(
                        ReadCommandId(ref reader),
                        reader.ReadDouble(),
                        ReadTween(ref reader),
                        conflict
                    );
                }
                case "TimeWait":
                    ReadArrayHeader(ref reader, 1);
                    return new CommandBody.Time.Wait(ReadDuration(ref reader));
                case "OperationCancel":
                    ReadArrayHeader(ref reader, 1);
                    return new CommandBody.Operation.Cancel(ReadCommandId(ref reader));
                case "InputSetEnabled":
                    ReadArrayHeader(ref reader, 1);
                    return new CommandBody.Input.SetEnabled(reader.ReadBoolean());
                case "InputSetCamera":
                    return new CommandBody.Input.SetCamera(ReadObjectPayload(ref reader));
                case "InputSetPointerEvents":
                    ReadArrayHeader(ref reader, 2);
                    return new CommandBody.Input.SetPointerEvents(
                        ReadObjectId(ref reader),
                        ReadPointerEvents(ref reader)
                    );
                case "InputSetGlobalKeys":
                    ReadArrayHeader(ref reader, 1);
                    return new CommandBody.Input.SetGlobalKeys(ReadKeyCodes(ref reader));
                default:
                    throw UnknownVariant(nameof(CommandBody), variant);
            }
        }

        private static SceneId ReadScenePayload(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 1);
            return ReadSceneId(ref reader);
        }

        private static ObjectId ReadObjectPayload(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 1);
            return ReadObjectId(ref reader);
        }

        private static ConflictPolicy ReadPropertyHeader(
            ref MessagePackReader reader,
            int payloadSize
        )
        {
            ReadArrayHeader(ref reader, 2);
            ConflictPolicy conflict = ReadConflictPolicy(ref reader);
            ReadArrayHeader(ref reader, payloadSize);
            return conflict;
        }

        private static CommandBody ReadSetPosition(ref MessagePackReader reader, bool local)
        {
            ConflictPolicy conflict = ReadPropertyHeader(ref reader, 2);
            ObjectId objectId = ReadObjectId(ref reader);
            Vector3 position = ReadVector3(ref reader);
            return local
                ? new CommandBody.Transform.SetLocalPosition(objectId, position, conflict)
                : new CommandBody.Transform.SetWorldPosition(objectId, position, conflict);
        }

        private static CommandBody ReadTweenPosition(ref MessagePackReader reader, bool local)
        {
            ConflictPolicy conflict = ReadPropertyHeader(ref reader, 3);
            ObjectId objectId = ReadObjectId(ref reader);
            Vector3 position = ReadVector3(ref reader);
            Tween tween = ReadTween(ref reader);
            return local
                ? new CommandBody.Transform.TweenLocalPosition(objectId, position, tween, conflict)
                : new CommandBody.Transform.TweenWorldPosition(objectId, position, tween, conflict);
        }

        private static CommandBody ReadSetRotation(ref MessagePackReader reader, bool local)
        {
            ConflictPolicy conflict = ReadPropertyHeader(ref reader, 2);
            ObjectId objectId = ReadObjectId(ref reader);
            Quaternion rotation = ReadQuaternion(ref reader);
            return local
                ? new CommandBody.Transform.SetLocalRotation(objectId, rotation, conflict)
                : new CommandBody.Transform.SetWorldRotation(objectId, rotation, conflict);
        }

        private static CommandBody ReadTweenRotation(ref MessagePackReader reader, bool local)
        {
            ConflictPolicy conflict = ReadPropertyHeader(ref reader, 3);
            ObjectId objectId = ReadObjectId(ref reader);
            Quaternion rotation = ReadQuaternion(ref reader);
            Tween tween = ReadTween(ref reader);
            return local
                ? new CommandBody.Transform.TweenLocalRotation(objectId, rotation, tween, conflict)
                : new CommandBody.Transform.TweenWorldRotation(objectId, rotation, tween, conflict);
        }

        private static CommandBody ReadSetScale(ref MessagePackReader reader)
        {
            ConflictPolicy conflict = ReadPropertyHeader(ref reader, 2);
            return new CommandBody.Transform.SetLocalScale(
                ReadObjectId(ref reader),
                ReadVector3(ref reader),
                conflict
            );
        }

        private static CommandBody ReadTweenScale(ref MessagePackReader reader)
        {
            ConflictPolicy conflict = ReadPropertyHeader(ref reader, 3);
            return new CommandBody.Transform.TweenLocalScale(
                ReadObjectId(ref reader),
                ReadVector3(ref reader),
                ReadTween(ref reader),
                conflict
            );
        }

        private static CommandBody ReadSetColor(ref MessagePackReader reader, bool light)
        {
            ConflictPolicy conflict = ReadPropertyHeader(ref reader, 2);
            ObjectId objectId = ReadObjectId(ref reader);
            Color color = ReadColor(ref reader);
            return light
                ? new CommandBody.Light.SetColor(objectId, color, conflict)
                : new CommandBody.Text.SetColor(objectId, color, conflict);
        }

        private static CommandBody ReadTweenColor(ref MessagePackReader reader, bool light)
        {
            ConflictPolicy conflict = ReadPropertyHeader(ref reader, 3);
            ObjectId objectId = ReadObjectId(ref reader);
            Color color = ReadColor(ref reader);
            Tween tween = ReadTween(ref reader);
            return light
                ? new CommandBody.Light.TweenColor(objectId, color, tween, conflict)
                : new CommandBody.Text.TweenColor(objectId, color, tween, conflict);
        }

        private static (ObjectId ObjectId, bool Enabled) ReadObjectEnabled(
            ref MessagePackReader reader
        )
        {
            ReadArrayHeader(ref reader, 2);
            return (ReadObjectId(ref reader), reader.ReadBoolean());
        }

        private static ParticleSpawnLocation ReadParticleSpawnLocation(ref MessagePackReader reader)
        {
            string variant = ReadVariantHeader(ref reader);
            return variant switch
            {
                "GameObject" => new ParticleSpawnLocation.AtGameObject(ReadObjectId(ref reader)),
                "WorldPosition" => new ParticleSpawnLocation.AtWorldPosition(
                    ReadVector3(ref reader)
                ),
                _ => throw UnknownVariant(nameof(ParticleSpawnLocation), variant),
            };
        }

        private static uint? ReadOptionalUInt32(ref MessagePackReader reader) =>
            reader.TryReadNil() ? null : reader.ReadUInt32();

        private static double? ReadOptionalDouble(ref MessagePackReader reader) =>
            reader.TryReadNil() ? null : reader.ReadDouble();

        private static Color? ReadOptionalColor(ref MessagePackReader reader) =>
            reader.TryReadNil() ? null : ReadColor(ref reader);
    }
}

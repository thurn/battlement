#nullable enable

using System;
using System.Collections.Generic;
using MessagePack;

namespace Masonry
{
    internal static partial class ProtocolFormat
    {
        private static void WriteSessionId(ref MessagePackWriter writer, SessionId value) =>
            WriteGuid(ref writer, value.Value);

        private static SessionId ReadSessionId(ref MessagePackReader reader) =>
            new(ReadGuid(ref reader));

        private static void WriteActionId(ref MessagePackWriter writer, ActionId value) =>
            WriteGuid(ref writer, value.Value);

        private static ActionId ReadActionId(ref MessagePackReader reader) =>
            new(ReadGuid(ref reader));

        private static void WriteBatchId(ref MessagePackWriter writer, BatchId value) =>
            WriteGuid(ref writer, value.Value);

        private static BatchId ReadBatchId(ref MessagePackReader reader) =>
            new(ReadGuid(ref reader));

        private static void WriteCommandId(ref MessagePackWriter writer, CommandId value) =>
            WriteGuid(ref writer, value.Value);

        private static CommandId ReadCommandId(ref MessagePackReader reader) =>
            new(ReadGuid(ref reader));

        private static void WriteObjectId(ref MessagePackWriter writer, ObjectId value) =>
            WriteGuid(ref writer, value.Value);

        private static ObjectId ReadObjectId(ref MessagePackReader reader) =>
            new(ReadGuid(ref reader));

        private static void WriteSceneId(ref MessagePackWriter writer, SceneId value) =>
            WriteGuid(ref writer, value.Value);

        private static SceneId ReadSceneId(ref MessagePackReader reader) =>
            new(ReadGuid(ref reader));

        private static void WriteOptionalActionId(ref MessagePackWriter writer, ActionId? value)
        {
            if (value is null)
            {
                writer.WriteNil();
            }
            else
            {
                WriteActionId(ref writer, value.Value);
            }
        }

        private static ActionId? ReadOptionalActionId(ref MessagePackReader reader) =>
            reader.TryReadNil() ? null : ReadActionId(ref reader);

        private static void WriteOptionalCommandId(ref MessagePackWriter writer, CommandId? value)
        {
            if (value is null)
            {
                writer.WriteNil();
            }
            else
            {
                WriteCommandId(ref writer, value.Value);
            }
        }

        private static CommandId? ReadOptionalCommandId(ref MessagePackReader reader) =>
            reader.TryReadNil() ? null : ReadCommandId(ref reader);

        private static void WriteOptionalObjectId(ref MessagePackWriter writer, ObjectId? value)
        {
            if (value is null)
            {
                writer.WriteNil();
            }
            else
            {
                WriteObjectId(ref writer, value.Value);
            }
        }

        private static ObjectId? ReadOptionalObjectId(ref MessagePackReader reader) =>
            reader.TryReadNil() ? null : ReadObjectId(ref reader);

        private static void WriteOptionalSceneId(ref MessagePackWriter writer, SceneId? value)
        {
            if (value is null)
            {
                writer.WriteNil();
            }
            else
            {
                WriteSceneId(ref writer, value.Value);
            }
        }

        private static SceneId? ReadOptionalSceneId(ref MessagePackReader reader) =>
            reader.TryReadNil() ? null : ReadSceneId(ref reader);

        private static void WriteSceneAddress(ref MessagePackWriter writer, SceneAddress value) =>
            WriteString(ref writer, value.Value);

        private static SceneAddress ReadSceneAddress(ref MessagePackReader reader) =>
            new(ReadString(ref reader));

        private static void WritePrefabAddress(ref MessagePackWriter writer, PrefabAddress value) =>
            WriteString(ref writer, value.Value);

        private static PrefabAddress ReadPrefabAddress(ref MessagePackReader reader) =>
            new(ReadString(ref reader));

        private static void WriteParticleEffectAddress(
            ref MessagePackWriter writer,
            ParticleEffectAddress value
        ) => WriteString(ref writer, value.Value);

        private static ParticleEffectAddress ReadParticleEffectAddress(
            ref MessagePackReader reader
        ) => new(ReadString(ref reader));

        private static void WriteMaterialAddress(
            ref MessagePackWriter writer,
            MaterialAddress value
        ) => WriteString(ref writer, value.Value);

        private static MaterialAddress ReadMaterialAddress(ref MessagePackReader reader) =>
            new(ReadString(ref reader));

        private static void WriteTextureAddress(
            ref MessagePackWriter writer,
            TextureAddress value
        ) => WriteString(ref writer, value.Value);

        private static TextureAddress ReadTextureAddress(ref MessagePackReader reader) =>
            new(ReadString(ref reader));

        private static void WriteAudioClipAddress(
            ref MessagePackWriter writer,
            AudioClipAddress value
        ) => WriteString(ref writer, value.Value);

        private static AudioClipAddress ReadAudioClipAddress(ref MessagePackReader reader) =>
            new(ReadString(ref reader));

        private static void WriteFontAddress(ref MessagePackWriter writer, FontAddress value) =>
            WriteString(ref writer, value.Value);

        private static FontAddress ReadFontAddress(ref MessagePackReader reader) =>
            new(ReadString(ref reader));

        private static void WritePreparedAsset(ref MessagePackWriter writer, PreparedAsset value)
        {
            switch (value)
            {
                case PreparedAsset.Scene scene:
                    WriteVariantHeader(ref writer, "Scene");
                    WriteSceneAddress(ref writer, scene.Address);
                    break;
                case PreparedAsset.Prefab prefab:
                    WriteVariantHeader(ref writer, "Prefab");
                    WritePrefabAddress(ref writer, prefab.Address);
                    break;
                case PreparedAsset.ParticleEffect effect:
                    WriteVariantHeader(ref writer, "ParticleEffect");
                    WriteParticleEffectAddress(ref writer, effect.Address);
                    break;
                case PreparedAsset.Material material:
                    WriteVariantHeader(ref writer, "Material");
                    WriteMaterialAddress(ref writer, material.Address);
                    break;
                case PreparedAsset.Texture texture:
                    WriteVariantHeader(ref writer, "Texture");
                    WriteTextureAddress(ref writer, texture.Address);
                    break;
                case PreparedAsset.AudioClip clip:
                    WriteVariantHeader(ref writer, "AudioClip");
                    WriteAudioClipAddress(ref writer, clip.Address);
                    break;
                case PreparedAsset.Font font:
                    WriteVariantHeader(ref writer, "Font");
                    WriteFontAddress(ref writer, font.Address);
                    break;
                default:
                    throw new MessagePackSerializationException("Unknown prepared asset value.");
            }
        }

        private static PreparedAsset ReadPreparedAsset(ref MessagePackReader reader)
        {
            string variant = ReadVariantHeader(ref reader);
            return variant switch
            {
                "Scene" => new PreparedAsset.Scene(ReadSceneAddress(ref reader)),
                "Prefab" => new PreparedAsset.Prefab(ReadPrefabAddress(ref reader)),
                "ParticleEffect" => new PreparedAsset.ParticleEffect(
                    ReadParticleEffectAddress(ref reader)
                ),
                "Material" => new PreparedAsset.Material(ReadMaterialAddress(ref reader)),
                "Texture" => new PreparedAsset.Texture(ReadTextureAddress(ref reader)),
                "AudioClip" => new PreparedAsset.AudioClip(ReadAudioClipAddress(ref reader)),
                "Font" => new PreparedAsset.Font(ReadFontAddress(ref reader)),
                _ => throw UnknownVariant(nameof(PreparedAsset), variant),
            };
        }

        private static void WritePreparedAssets(
            ref MessagePackWriter writer,
            IReadOnlyList<PreparedAsset> values
        )
        {
            writer.WriteArrayHeader(values.Count);
            foreach (PreparedAsset value in values)
            {
                WritePreparedAsset(ref writer, value);
            }
        }

        private static IReadOnlyList<PreparedAsset> ReadPreparedAssets(ref MessagePackReader reader)
        {
            int count = reader.ReadArrayHeader();
            var values = new PreparedAsset[count];
            for (int index = 0; index < count; index++)
            {
                values[index] = ReadPreparedAsset(ref reader);
            }

            return values;
        }

        private static void WriteScene(ref MessagePackWriter writer, Scene value)
        {
            WriteArrayHeader(ref writer, 2);
            WriteSceneId(ref writer, value.Id);
            WriteSceneAddress(ref writer, value.Address);
        }

        private static Scene ReadScene(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 2);
            return new Scene(ReadSceneId(ref reader), ReadSceneAddress(ref reader));
        }

        private static void WriteScenes(ref MessagePackWriter writer, IReadOnlyList<Scene> values)
        {
            writer.WriteArrayHeader(values.Count);
            foreach (Scene value in values)
            {
                WriteScene(ref writer, value);
            }
        }

        private static IReadOnlyList<Scene> ReadScenes(ref MessagePackReader reader)
        {
            int count = reader.ReadArrayHeader();
            var values = new Scene[count];
            for (int index = 0; index < count; index++)
            {
                values[index] = ReadScene(ref reader);
            }

            return values;
        }

        private static void WriteGameObject(ref MessagePackWriter writer, GameObject value)
        {
            WriteArrayHeader(ref writer, 7);
            WriteObjectId(ref writer, value.Id);
            WriteParentScene(ref writer, value.ParentScene);
            WriteOptionalObjectId(ref writer, value.ParentId);
            writer.Write(value.IsActive);
            WriteLocalTransform(ref writer, value.LocalTransform);
            WritePointerEvents(ref writer, value.PointerEvents);
            WriteGameObjectKind(ref writer, value.Kind);
        }

        private static GameObject ReadGameObject(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 7);
            ObjectId id = ReadObjectId(ref reader);
            ParentScene parentScene = ReadParentScene(ref reader);
            ObjectId? parentId = ReadOptionalObjectId(ref reader);
            bool isActive = reader.ReadBoolean();
            LocalTransform transform = ReadLocalTransform(ref reader);
            IReadOnlyList<PointerEvent> events = ReadPointerEvents(ref reader);
            GameObjectKind kind = ReadGameObjectKind(ref reader);
            return new GameObject(id, kind, parentScene, parentId, isActive, transform, events);
        }

        private static void WriteGameObjects(
            ref MessagePackWriter writer,
            IReadOnlyList<GameObject> values
        )
        {
            writer.WriteArrayHeader(values.Count);
            foreach (GameObject value in values)
            {
                WriteGameObject(ref writer, value);
            }
        }

        private static IReadOnlyList<GameObject> ReadGameObjects(ref MessagePackReader reader)
        {
            int count = reader.ReadArrayHeader();
            var values = new GameObject[count];
            for (int index = 0; index < count; index++)
            {
                values[index] = ReadGameObject(ref reader);
            }

            return values;
        }

        private static void WritePointerEvents(
            ref MessagePackWriter writer,
            IReadOnlyList<PointerEvent> values
        )
        {
            writer.WriteArrayHeader(values.Count);
            foreach (PointerEvent value in values)
            {
                WritePointerEvent(ref writer, value);
            }
        }

        private static IReadOnlyList<PointerEvent> ReadPointerEvents(ref MessagePackReader reader)
        {
            int count = reader.ReadArrayHeader();
            var values = new PointerEvent[count];
            for (int index = 0; index < count; index++)
            {
                values[index] = ReadPointerEvent(ref reader);
            }

            return values;
        }

        private static void WriteParentScene(ref MessagePackWriter writer, ParentScene value)
        {
            switch (value)
            {
                case ParentScene.Primary:
                    WriteVariant(ref writer, "PrimaryScene");
                    break;
                case ParentScene.Specific scene:
                    WriteVariantHeader(ref writer, "Scene");
                    WriteSceneId(ref writer, scene.SceneId);
                    break;
                case ParentScene.Persistent:
                    WriteVariant(ref writer, "Persistent");
                    break;
                default:
                    throw new MessagePackSerializationException("Unknown parent scene value.");
            }
        }

        private static ParentScene ReadParentScene(ref MessagePackReader reader)
        {
            if (reader.NextMessagePackType == MessagePackType.String)
            {
                string unit = ReadVariant(ref reader);
                return unit switch
                {
                    "PrimaryScene" => new ParentScene.Primary(),
                    "Persistent" => new ParentScene.Persistent(),
                    _ => throw UnknownVariant(nameof(ParentScene), unit),
                };
            }

            string variant = ReadVariantHeader(ref reader);
            return variant == "Scene"
                ? new ParentScene.Specific(ReadSceneId(ref reader))
                : throw UnknownVariant(nameof(ParentScene), variant);
        }

        private static void WriteGameObjectKind(ref MessagePackWriter writer, GameObjectKind value)
        {
            switch (value)
            {
                case GameObjectKind.Empty:
                    WriteVariant(ref writer, "Empty");
                    break;
                case GameObjectKind.Cube cube:
                    WriteMaterialsKind(ref writer, "Cube", cube.Materials);
                    break;
                case GameObjectKind.Sphere sphere:
                    WriteMaterialsKind(ref writer, "Sphere", sphere.Materials);
                    break;
                case GameObjectKind.Capsule capsule:
                    WriteMaterialsKind(ref writer, "Capsule", capsule.Materials);
                    break;
                case GameObjectKind.Cylinder cylinder:
                    WriteMaterialsKind(ref writer, "Cylinder", cylinder.Materials);
                    break;
                case GameObjectKind.Plane plane:
                    WriteMaterialsKind(ref writer, "Plane", plane.Materials);
                    break;
                case GameObjectKind.Quad quad:
                    WriteMaterialsKind(ref writer, "Quad", quad.Materials);
                    break;
                case GameObjectKind.Image image:
                    WriteVariantHeader(ref writer, "Image");
                    WriteArrayHeader(ref writer, 1);
                    WriteImageState(ref writer, image.State);
                    break;
                case GameObjectKind.Text text:
                    WriteVariantHeader(ref writer, "Text");
                    WriteArrayHeader(ref writer, 1);
                    WriteTextState(ref writer, text.State);
                    break;
                case GameObjectKind.Camera camera:
                    WriteVariantHeader(ref writer, "Camera");
                    WriteArrayHeader(ref writer, 1);
                    WriteCameraState(ref writer, camera.State);
                    break;
                case GameObjectKind.Light light:
                    WriteVariantHeader(ref writer, "Light");
                    WriteArrayHeader(ref writer, 1);
                    WriteLightState(ref writer, light.State);
                    break;
                case GameObjectKind.Prefab prefab:
                    WriteVariantHeader(ref writer, "Prefab");
                    WriteArrayHeader(ref writer, 3);
                    WritePrefabAddress(ref writer, prefab.Address);
                    WriteMaterialAssignments(ref writer, prefab.Materials);
                    WriteOptionalAnimatorState(ref writer, prefab.Animator);
                    break;
                default:
                    throw new MessagePackSerializationException("Unknown game object kind.");
            }
        }

        private static GameObjectKind ReadGameObjectKind(ref MessagePackReader reader)
        {
            if (reader.NextMessagePackType == MessagePackType.String)
            {
                string unit = ReadVariant(ref reader);
                return unit == "Empty"
                    ? new GameObjectKind.Empty()
                    : throw UnknownVariant(nameof(GameObjectKind), unit);
            }

            string variant = ReadVariantHeader(ref reader);
            switch (variant)
            {
                case "Cube":
                    return new GameObjectKind.Cube(ReadMaterialsKind(ref reader));
                case "Sphere":
                    return new GameObjectKind.Sphere(ReadMaterialsKind(ref reader));
                case "Capsule":
                    return new GameObjectKind.Capsule(ReadMaterialsKind(ref reader));
                case "Cylinder":
                    return new GameObjectKind.Cylinder(ReadMaterialsKind(ref reader));
                case "Plane":
                    return new GameObjectKind.Plane(ReadMaterialsKind(ref reader));
                case "Quad":
                    return new GameObjectKind.Quad(ReadMaterialsKind(ref reader));
                case "Image":
                    ReadArrayHeader(ref reader, 1);
                    return new GameObjectKind.Image(ReadImageState(ref reader));
                case "Text":
                    ReadArrayHeader(ref reader, 1);
                    return new GameObjectKind.Text(ReadTextState(ref reader));
                case "Camera":
                    ReadArrayHeader(ref reader, 1);
                    return new GameObjectKind.Camera(ReadCameraState(ref reader));
                case "Light":
                    ReadArrayHeader(ref reader, 1);
                    return new GameObjectKind.Light(ReadLightState(ref reader));
                case "Prefab":
                    ReadArrayHeader(ref reader, 3);
                    return new GameObjectKind.Prefab(
                        ReadPrefabAddress(ref reader),
                        ReadMaterialAssignments(ref reader),
                        ReadOptionalAnimatorState(ref reader)
                    );
                default:
                    throw UnknownVariant(nameof(GameObjectKind), variant);
            }
        }

        private static void WriteMaterialsKind(
            ref MessagePackWriter writer,
            string variant,
            IReadOnlyList<MaterialAssignment> materials
        )
        {
            WriteVariantHeader(ref writer, variant);
            WriteArrayHeader(ref writer, 1);
            WriteMaterialAssignments(ref writer, materials);
        }

        private static IReadOnlyList<MaterialAssignment> ReadMaterialsKind(
            ref MessagePackReader reader
        )
        {
            ReadArrayHeader(ref reader, 1);
            return ReadMaterialAssignments(ref reader);
        }

        private static void WriteMaterialAssignments(
            ref MessagePackWriter writer,
            IReadOnlyList<MaterialAssignment> values
        )
        {
            writer.WriteArrayHeader(values.Count);
            foreach (MaterialAssignment value in values)
            {
                WriteArrayHeader(ref writer, 2);
                writer.Write(value.Slot);
                WriteMaterialAddress(ref writer, value.Address);
            }
        }

        private static IReadOnlyList<MaterialAssignment> ReadMaterialAssignments(
            ref MessagePackReader reader
        )
        {
            int count = reader.ReadArrayHeader();
            var values = new MaterialAssignment[count];
            for (int index = 0; index < count; index++)
            {
                ReadArrayHeader(ref reader, 2);
                values[index] = new MaterialAssignment(
                    reader.ReadUInt32(),
                    ReadMaterialAddress(ref reader)
                );
            }

            return values;
        }

        private static void WriteImageState(ref MessagePackWriter writer, ImageState value)
        {
            WriteArrayHeader(ref writer, 7);
            WriteTextureAddress(ref writer, value.Texture);
            writer.Write(value.Width);
            writer.Write(value.Height);
            WriteImageFit(ref writer, value.Fit);
            WriteRgbColor(ref writer, value.Tint);
            writer.Write(value.Opacity);
            writer.Write(value.FacesCamera);
        }

        private static ImageState ReadImageState(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 7);
            return new ImageState(
                ReadTextureAddress(ref reader),
                reader.ReadDouble(),
                reader.ReadDouble(),
                ReadImageFit(ref reader),
                ReadRgbColor(ref reader),
                reader.ReadDouble(),
                reader.ReadBoolean()
            );
        }

        private static void WriteTextState(ref MessagePackWriter writer, TextState value)
        {
            WriteArrayHeader(ref writer, 9);
            WriteString(ref writer, value.Text);
            WriteFontAddress(ref writer, value.Font);
            writer.Write(value.Size);
            WriteColor(ref writer, value.Color);
            WriteHorizontalAlignment(ref writer, value.HorizontalAlignment);
            WriteVerticalAlignment(ref writer, value.VerticalAlignment);
            if (value.WrapWidth is null)
            {
                writer.WriteNil();
            }
            else
            {
                writer.Write(value.WrapWidth.Value);
            }

            writer.Write(value.IsRichText);
            writer.Write(value.FacesCamera);
        }

        private static TextState ReadTextState(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 9);
            string text = ReadString(ref reader);
            FontAddress font = ReadFontAddress(ref reader);
            double size = reader.ReadDouble();
            Color color = ReadColor(ref reader);
            HorizontalAlignment horizontal = ReadHorizontalAlignment(ref reader);
            VerticalAlignment vertical = ReadVerticalAlignment(ref reader);
            double? wrapWidth = reader.TryReadNil() ? null : reader.ReadDouble();
            bool richText = reader.ReadBoolean();
            bool faceCamera = reader.ReadBoolean();
            return new TextState(
                text,
                font,
                size,
                color,
                horizontal,
                vertical,
                wrapWidth,
                richText,
                faceCamera
            );
        }

        private static void WriteCameraState(ref MessagePackWriter writer, CameraState value)
        {
            WriteArrayHeader(ref writer, 8);
            writer.Write(value.IsEnabled);
            WriteCameraProjection(ref writer, value.Projection);
            writer.Write(value.FieldOfView);
            writer.Write(value.OrthographicSize);
            writer.Write(value.NearClip);
            writer.Write(value.FarClip);
            WriteCameraClearMode(ref writer, value.ClearMode);
            WriteColor(ref writer, value.ClearColor);
        }

        private static CameraState ReadCameraState(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 8);
            return new CameraState(
                reader.ReadBoolean(),
                ReadCameraProjection(ref reader),
                reader.ReadDouble(),
                reader.ReadDouble(),
                reader.ReadDouble(),
                reader.ReadDouble(),
                ReadCameraClearMode(ref reader),
                ReadColor(ref reader)
            );
        }

        private static void WriteLightState(ref MessagePackWriter writer, LightState value)
        {
            WriteArrayHeader(ref writer, 8);
            writer.Write(value.IsEnabled);
            WriteLightType(ref writer, value.Type);
            WriteColor(ref writer, value.Color);
            writer.Write(value.Intensity);
            writer.Write(value.Range);
            writer.Write(value.OuterSpotAngle);
            writer.Write(value.InnerSpotAngle);
            WriteShadowMode(ref writer, value.Shadows);
        }

        private static LightState ReadLightState(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 8);
            return new LightState(
                reader.ReadBoolean(),
                ReadLightType(ref reader),
                ReadColor(ref reader),
                reader.ReadDouble(),
                reader.ReadDouble(),
                reader.ReadDouble(),
                reader.ReadDouble(),
                ReadShadowMode(ref reader)
            );
        }

        private static void WriteOptionalAnimatorState(
            ref MessagePackWriter writer,
            AnimatorState? value
        )
        {
            if (value is null)
            {
                writer.WriteNil();
            }
            else
            {
                WriteAnimatorState(ref writer, value);
            }
        }

        private static AnimatorState? ReadOptionalAnimatorState(ref MessagePackReader reader) =>
            reader.TryReadNil() ? null : ReadAnimatorState(ref reader);

        private static void WriteAnimatorState(ref MessagePackWriter writer, AnimatorState value)
        {
            WriteArrayHeader(ref writer, 7);
            WriteString(ref writer, value.State);
            writer.Write(value.Layer);
            writer.Write(value.NormalizedStartTime);
            WriteBoolMap(ref writer, value.BoolParameters);
            WriteIntMap(ref writer, value.IntParameters);
            WriteDoubleMap(ref writer, value.FloatParameters);
            writer.Write(value.Speed);
        }

        private static AnimatorState ReadAnimatorState(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 7);
            return new AnimatorState(
                ReadString(ref reader),
                reader.ReadUInt32(),
                reader.ReadDouble(),
                ReadBoolMap(ref reader),
                ReadIntMap(ref reader),
                ReadDoubleMap(ref reader),
                reader.ReadDouble()
            );
        }

        private static void WriteBoolMap(
            ref MessagePackWriter writer,
            IReadOnlyDictionary<string, bool> values
        )
        {
            var sorted = new SortedDictionary<string, bool>(StringComparer.Ordinal);
            foreach (KeyValuePair<string, bool> entry in values)
            {
                sorted.Add(entry.Key, entry.Value);
            }

            writer.WriteMapHeader(sorted.Count);
            foreach (KeyValuePair<string, bool> entry in sorted)
            {
                writer.Write(entry.Key);
                writer.Write(entry.Value);
            }
        }

        private static void WriteIntMap(
            ref MessagePackWriter writer,
            IReadOnlyDictionary<string, int> values
        )
        {
            var sorted = new SortedDictionary<string, int>(StringComparer.Ordinal);
            foreach (KeyValuePair<string, int> entry in values)
            {
                sorted.Add(entry.Key, entry.Value);
            }

            writer.WriteMapHeader(sorted.Count);
            foreach (KeyValuePair<string, int> entry in sorted)
            {
                writer.Write(entry.Key);
                writer.Write(entry.Value);
            }
        }

        private static void WriteDoubleMap(
            ref MessagePackWriter writer,
            IReadOnlyDictionary<string, double> values
        )
        {
            var sorted = new SortedDictionary<string, double>(StringComparer.Ordinal);
            foreach (KeyValuePair<string, double> entry in values)
            {
                sorted.Add(entry.Key, entry.Value);
            }

            writer.WriteMapHeader(sorted.Count);
            foreach (KeyValuePair<string, double> entry in sorted)
            {
                writer.Write(entry.Key);
                writer.Write(entry.Value);
            }
        }

        private static IReadOnlyDictionary<string, bool> ReadBoolMap(ref MessagePackReader reader)
        {
            int count = reader.ReadMapHeader();
            var values = new SortedDictionary<string, bool>(StringComparer.Ordinal);
            for (int index = 0; index < count; index++)
            {
                values.Add(ReadString(ref reader), reader.ReadBoolean());
            }

            return values;
        }

        private static IReadOnlyDictionary<string, int> ReadIntMap(ref MessagePackReader reader)
        {
            int count = reader.ReadMapHeader();
            var values = new SortedDictionary<string, int>(StringComparer.Ordinal);
            for (int index = 0; index < count; index++)
            {
                values.Add(ReadString(ref reader), reader.ReadInt32());
            }

            return values;
        }

        private static IReadOnlyDictionary<string, double> ReadDoubleMap(
            ref MessagePackReader reader
        )
        {
            int count = reader.ReadMapHeader();
            var values = new SortedDictionary<string, double>(StringComparer.Ordinal);
            for (int index = 0; index < count; index++)
            {
                values.Add(ReadString(ref reader), reader.ReadDouble());
            }

            return values;
        }
    }
}

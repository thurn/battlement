#nullable enable

using System;
using System.Collections.Generic;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    internal static partial class BattlementFlatBufferResponseFixtures
    {
        private static Offset<Wire.GameObject> WriteObject(
            FlatBufferBuilder builder,
            BattlementGameObject value
        )
        {
            Offset<Wire.ParentScene> parentScene = WriteParentScene(builder, value.ParentScene);
            VectorOffset pointerEvents = PointerEventVector(builder, value.PointerEvents);
            (Wire.GameObjectKind kind, Wire.GameObjectContent contentType, int content) =
                WriteObjectContent(builder, value.Kind);
            Offset<Wire.RenderOrder> renderOrder = WriteRenderOrder(builder, value.RenderOrder);
            VectorOffset instances = WriteInstances(
                builder,
                value.MaterialInstances ?? Array.Empty<MaterialInstance>()
            );
            Wire.GameObject.StartGameObject(builder);
            Wire.GameObject.AddMaterialInstances(builder, instances);
            Wire.GameObject.AddRenderOrder(builder, renderOrder);
            Wire.GameObject.AddContent(builder, content);
            Wire.GameObject.AddContentType(builder, contentType);
            Wire.GameObject.AddKind(builder, kind);
            Wire.GameObject.AddDragMode(
                builder,
                value.DragMode is null
                    ? Wire.DragMode.None
                    : (Wire.DragMode)((byte)value.DragMode + 1)
            );
            Wire.GameObject.AddPointerEvents(builder, pointerEvents);
            LocalTransform transform = value.LocalTransform;
            Wire.GameObject.AddLocalTransform(
                builder,
                Wire.LocalTransform.CreateLocalTransform(
                    builder,
                    transform.Position.X,
                    transform.Position.Y,
                    transform.Position.Z,
                    transform.Rotation.X,
                    transform.Rotation.Y,
                    transform.Rotation.Z,
                    transform.Rotation.W,
                    transform.Scale.X,
                    transform.Scale.Y,
                    transform.Scale.Z
                )
            );
            Wire.GameObject.AddActive(builder, value.IsActive);
            if (value.ParentId is ObjectId parentId)
                Wire.GameObject.AddParentId(builder, Uuid(builder, parentId.Value));
            Wire.GameObject.AddParentScene(builder, parentScene);
            Wire.GameObject.AddObjectId(builder, Uuid(builder, value.Id.Value));
            return Wire.GameObject.EndGameObject(builder);
        }

        private static Offset<Wire.RenderOrder> WriteRenderOrder(
            FlatBufferBuilder builder,
            RenderOrder? value
        ) =>
            value is RenderOrder order
                ? Wire.RenderOrder.CreateRenderOrder(
                    builder,
                    (Wire.RenderOrderKind)order.Kind,
                    order.Order
                )
                : default;

        private static Payload SetRenderOrder(
            FlatBufferBuilder builder,
            CommandBody.Object.SetRenderOrder value
        )
        {
            Offset<Wire.RenderOrder> order = WriteRenderOrder(builder, value.Order);
            Wire.ObjectRenderOrderPayload.StartObjectRenderOrderPayload(builder);
            Wire.ObjectRenderOrderPayload.AddRenderOrder(builder, order);
            Wire.ObjectRenderOrderPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new Payload(
                Wire.CoreCommandKind.ObjectSetRenderOrder,
                Wire.CoreCommandPayload.ObjectRenderOrderPayload,
                Wire.ObjectRenderOrderPayload.EndObjectRenderOrderPayload(builder).Value
            );
        }

        private static Offset<Wire.ParentScene> WriteParentScene(
            FlatBufferBuilder builder,
            ParentScene value
        )
        {
            Wire.ParentScene.StartParentScene(builder);
            switch (value)
            {
                case ParentScene.Primary:
                    Wire.ParentScene.AddKind(builder, Wire.ParentSceneKind.PrimaryScene);
                    break;
                case ParentScene.Persistent:
                    Wire.ParentScene.AddKind(builder, Wire.ParentSceneKind.Persistent);
                    break;
                case ParentScene.Specific specific:
                    Wire.ParentScene.AddSceneId(builder, Uuid(builder, specific.SceneId.Value));
                    Wire.ParentScene.AddKind(builder, Wire.ParentSceneKind.Scene);
                    break;
                default:
                    throw new ArgumentException("Unknown parent scene.", nameof(value));
            }
            return Wire.ParentScene.EndParentScene(builder);
        }

        private static (Wire.GameObjectKind, Wire.GameObjectContent, int) WriteObjectContent(
            FlatBufferBuilder builder,
            GameObjectKind value
        )
        {
            switch (value)
            {
                case GameObjectKind.Empty:
                    Wire.EmptyObject.StartEmptyObject(builder);
                    return (
                        Wire.GameObjectKind.Empty,
                        Wire.GameObjectContent.EmptyObject,
                        Wire.EmptyObject.EndEmptyObject(builder).Value
                    );
                case GameObjectKind.Cube cube:
                    return Primitive(builder, Wire.GameObjectKind.Cube, cube.Materials);
                case GameObjectKind.Sphere sphere:
                    return Primitive(builder, Wire.GameObjectKind.Sphere, sphere.Materials);
                case GameObjectKind.Capsule capsule:
                    return Primitive(builder, Wire.GameObjectKind.Capsule, capsule.Materials);
                case GameObjectKind.Cylinder cylinder:
                    return Primitive(builder, Wire.GameObjectKind.Cylinder, cylinder.Materials);
                case GameObjectKind.Plane plane:
                    return Primitive(builder, Wire.GameObjectKind.Plane, plane.Materials);
                case GameObjectKind.Quad quad:
                    return Primitive(builder, Wire.GameObjectKind.Quad, quad.Materials);
                case GameObjectKind.Image image:
                    return Image(builder, image.State);
                case GameObjectKind.Text text:
                    return Text(builder, text.State);
                case GameObjectKind.Camera camera:
                    return Camera(builder, camera.State);
                case GameObjectKind.Light light:
                    return Light(builder, light.State);
                case GameObjectKind.Mesh mesh:
                {
                    StringOffset address = builder.CreateString(mesh.Address.Value);
                    VectorOffset materials = WriteMaterials(builder, mesh.Materials);
                    return (
                        Wire.GameObjectKind.Mesh,
                        Wire.GameObjectContent.MeshObject,
                        Wire.MeshObject.CreateMeshObject(builder, address, materials).Value
                    );
                }
                case GameObjectKind.Prefab prefab:
                    return Prefab(builder, prefab);
                case GameObjectKind.UiDocumentState document:
                    return UiDocumentObject(builder, document);
                default:
                    throw new ArgumentException("Unknown game object kind.", nameof(value));
            }
        }

        private static (Wire.GameObjectKind, Wire.GameObjectContent, int) Primitive(
            FlatBufferBuilder builder,
            Wire.GameObjectKind kind,
            IReadOnlyList<MaterialAssignment> materials
        )
        {
            Offset<Wire.PrimitiveObject> value = Wire.PrimitiveObject.CreatePrimitiveObject(
                builder,
                WriteMaterials(builder, materials)
            );
            return (kind, Wire.GameObjectContent.PrimitiveObject, value.Value);
        }

        private static (Wire.GameObjectKind, Wire.GameObjectContent, int) Image(
            FlatBufferBuilder builder,
            ImageState value
        )
        {
            StringOffset texture = builder.CreateString(value.Texture.Value);
            Wire.ImageObject.StartImageObject(builder);
            Wire.ImageObject.AddFaceCamera(builder, value.FacesCamera);
            Wire.ImageObject.AddOpacity(builder, value.Opacity);
            Wire.ImageObject.AddTint(
                builder,
                Wire.RgbColor.CreateRgbColor(
                    builder,
                    value.Tint.Red,
                    value.Tint.Green,
                    value.Tint.Blue
                )
            );
            Wire.ImageObject.AddFit(builder, (Wire.ImageFit)value.Fit);
            Wire.ImageObject.AddHeight(builder, value.Height);
            Wire.ImageObject.AddWidth(builder, value.Width);
            Wire.ImageObject.AddTexture(builder, texture);
            return (
                Wire.GameObjectKind.Image,
                Wire.GameObjectContent.ImageObject,
                Wire.ImageObject.EndImageObject(builder).Value
            );
        }

        private static (Wire.GameObjectKind, Wire.GameObjectContent, int) Text(
            FlatBufferBuilder builder,
            TextState value
        )
        {
            StringOffset text = builder.CreateString(value.Text);
            StringOffset font = builder.CreateString(value.Font.Value);
            Wire.TextObject.StartTextObject(builder);
            Wire.TextObject.AddFaceCamera(builder, value.FacesCamera);
            Wire.TextObject.AddRichText(builder, value.IsRichText);
            Wire.TextObject.AddWrapWidth(builder, value.WrapWidth);
            Wire.TextObject.AddVertical(builder, (Wire.VerticalAlignment)value.VerticalAlignment);
            Wire.TextObject.AddHorizontal(
                builder,
                (Wire.HorizontalAlignment)value.HorizontalAlignment
            );
            Wire.TextObject.AddColor(builder, Rgba(builder, value.Color));
            Wire.TextObject.AddSize(builder, value.Size);
            Wire.TextObject.AddFont(builder, font);
            Wire.TextObject.AddText(builder, text);
            return (
                Wire.GameObjectKind.Text,
                Wire.GameObjectContent.TextObject,
                Wire.TextObject.EndTextObject(builder).Value
            );
        }

        private static (Wire.GameObjectKind, Wire.GameObjectContent, int) Camera(
            FlatBufferBuilder builder,
            CameraState value
        )
        {
            Wire.CameraObject.StartCameraObject(builder);
            Wire.CameraObject.AddClearColor(builder, Rgba(builder, value.ClearColor));
            Wire.CameraObject.AddClearMode(builder, (Wire.CameraClearMode)value.ClearMode);
            Wire.CameraObject.AddFar(builder, value.FarClip);
            Wire.CameraObject.AddNear(builder, value.NearClip);
            Wire.CameraObject.AddOrthographicSize(builder, value.OrthographicSize);
            Wire.CameraObject.AddFieldOfView(builder, value.FieldOfView);
            Wire.CameraObject.AddProjection(builder, (Wire.CameraProjection)value.Projection);
            Wire.CameraObject.AddEnabled(builder, value.IsEnabled);
            return (
                Wire.GameObjectKind.Camera,
                Wire.GameObjectContent.CameraObject,
                Wire.CameraObject.EndCameraObject(builder).Value
            );
        }

        private static (Wire.GameObjectKind, Wire.GameObjectContent, int) Light(
            FlatBufferBuilder builder,
            LightState value
        )
        {
            Wire.LightObject.StartLightObject(builder);
            Wire.LightObject.AddShadows(builder, (Wire.ShadowMode)value.Shadows);
            Wire.LightObject.AddInnerSpotAngle(builder, value.InnerSpotAngle);
            Wire.LightObject.AddOuterSpotAngle(builder, value.OuterSpotAngle);
            Wire.LightObject.AddRange(builder, value.Range);
            Wire.LightObject.AddIntensity(builder, value.Intensity);
            Wire.LightObject.AddColor(builder, Rgba(builder, value.Color));
            Wire.LightObject.AddLightType(builder, (Wire.LightType)value.Type);
            Wire.LightObject.AddEnabled(builder, value.IsEnabled);
            return (
                Wire.GameObjectKind.Light,
                Wire.GameObjectContent.LightObject,
                Wire.LightObject.EndLightObject(builder).Value
            );
        }

        private static (Wire.GameObjectKind, Wire.GameObjectContent, int) Prefab(
            FlatBufferBuilder builder,
            GameObjectKind.Prefab value
        )
        {
            StringOffset address = builder.CreateString(value.Address.Value);
            VectorOffset materials = WriteMaterials(builder, value.Materials);
            Offset<Wire.AnimatorState>? animator = value.Animator is null
                ? null
                : WriteAnimator(builder, value.Animator);
            Wire.PrefabObject.StartPrefabObject(builder);
            if (animator.HasValue)
                Wire.PrefabObject.AddAnimator(builder, animator.Value);
            Wire.PrefabObject.AddMaterials(builder, materials);
            Wire.PrefabObject.AddAddress(builder, address);
            return (
                Wire.GameObjectKind.Prefab,
                Wire.GameObjectContent.PrefabObject,
                Wire.PrefabObject.EndPrefabObject(builder).Value
            );
        }

        private static (Wire.GameObjectKind, Wire.GameObjectContent, int) UiDocumentObject(
            FlatBufferBuilder builder,
            GameObjectKind.UiDocumentState value
        )
        {
            Offset<Wire.PanelSettings> settings = WritePanelSettings(
                builder,
                value.PanelSettings ?? new PanelSettingsValue()
            );
            ScreenSize size = value.WorldSpaceSize ?? new ScreenSize(1920, 1080);
            Wire.UiDocumentObject.StartUiDocumentObject(builder);
            Wire.UiDocumentObject.AddSortingOrder(builder, value.SortingOrder);
            Wire.UiDocumentObject.AddPivot(builder, (Wire.DocumentPivot)value.Pivot);
            Wire.UiDocumentObject.AddPivotReferenceSize(
                builder,
                (Wire.PivotReferenceSize)value.PivotReferenceSize
            );
            Wire.UiDocumentObject.AddWorldSpaceSize(
                builder,
                Wire.ScreenSize.CreateScreenSize(builder, size.Width, size.Height)
            );
            Wire.UiDocumentObject.AddWorldSpaceSizeMode(
                builder,
                (Wire.WorldSpaceSizeMode)value.WorldSpaceSizeMode
            );
            Wire.UiDocumentObject.AddPosition(builder, (Wire.DocumentPosition)value.Position);
            Wire.UiDocumentObject.AddPanelSettings(builder, settings);
            Wire.UiDocumentObject.AddRootId(builder, Uuid(builder, value.RootId.Value));
            return (
                Wire.GameObjectKind.UiDocument,
                Wire.GameObjectContent.UiDocumentObject,
                Wire.UiDocumentObject.EndUiDocumentObject(builder).Value
            );
        }

        private static VectorOffset WriteMaterials(
            FlatBufferBuilder builder,
            IReadOnlyList<MaterialAssignment> values
        )
        {
            var offsets = new int[values.Count];
            for (int index = 0; index < offsets.Length; index++)
                offsets[index] = Wire
                    .MaterialAssignment.CreateMaterialAssignment(
                        builder,
                        values[index].Slot,
                        builder.CreateString(values[index].Address.Value)
                    )
                    .Value;
            return OffsetVector(builder, offsets);
        }

        private static Offset<Wire.AnimatorState> WriteAnimator(
            FlatBufferBuilder builder,
            AnimatorState value
        )
        {
            VectorOffset bools = AnimatorBoolVector(builder, value.BoolParameters);
            VectorOffset ints = AnimatorIntVector(builder, value.IntParameters);
            VectorOffset floats = AnimatorFloatVector(builder, value.FloatParameters);
            return Wire.AnimatorState.CreateAnimatorState(
                builder,
                builder.CreateString(value.State),
                value.Layer,
                value.NormalizedStartTime,
                bools,
                ints,
                floats,
                value.Speed
            );
        }

        private static VectorOffset AnimatorBoolVector(
            FlatBufferBuilder builder,
            IReadOnlyDictionary<string, bool> values
        ) =>
            AnimatorVector(
                builder,
                values,
                (name, value) =>
                    Wire
                        .AnimatorBoolParameter.CreateAnimatorBoolParameter(
                            builder,
                            builder.CreateString(name),
                            value
                        )
                        .Value
            );

        private static VectorOffset AnimatorIntVector(
            FlatBufferBuilder builder,
            IReadOnlyDictionary<string, int> values
        ) =>
            AnimatorVector(
                builder,
                values,
                (name, value) =>
                    Wire
                        .AnimatorIntParameter.CreateAnimatorIntParameter(
                            builder,
                            builder.CreateString(name),
                            value
                        )
                        .Value
            );

        private static VectorOffset AnimatorFloatVector(
            FlatBufferBuilder builder,
            IReadOnlyDictionary<string, double> values
        ) =>
            AnimatorVector(
                builder,
                values,
                (name, value) =>
                    Wire
                        .AnimatorFloatParameter.CreateAnimatorFloatParameter(
                            builder,
                            builder.CreateString(name),
                            value
                        )
                        .Value
            );

        private static VectorOffset AnimatorVector<T>(
            FlatBufferBuilder builder,
            IReadOnlyDictionary<string, T> values,
            Func<string, T, int> write
        )
        {
            var offsets = new int[values.Count];
            int index = 0;
            foreach ((string name, T value) in values)
                offsets[index++] = write(name, value);
            return OffsetVector(builder, offsets);
        }

        private static VectorOffset PointerEventVector(
            FlatBufferBuilder builder,
            IReadOnlyList<PointerEvent> values
        )
        {
            var encoded = new byte[values.Count];
            for (int index = 0; index < encoded.Length; index++)
                encoded[index] = (byte)values[index];
            return ByteVector(builder, encoded);
        }

        private static Offset<Wire.RgbaColor> Rgba(FlatBufferBuilder builder, Color value) =>
            Wire.RgbaColor.CreateRgbaColor(
                builder,
                value.Red,
                value.Green,
                value.Blue,
                value.Alpha
            );
    }
}

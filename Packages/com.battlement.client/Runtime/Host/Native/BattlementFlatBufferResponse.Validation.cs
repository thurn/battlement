#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    internal sealed partial class BattlementFlatBufferResponse
    {
        private void ValidateBatch(Wire.Batch value)
        {
            _ = ReadUuid(value.BatchId, "batch");
            if (ReadUuid(value.SessionId, "batch session") != SessionId.Value)
                throw new InvalidDataException("The batch session does not match its response.");
            if (value.CausedByActionId.HasValue)
                _ = ReadUuid(value.CausedByActionId, "causing action");
            if (!Known(value.Start, Wire.BatchStart.AfterEarlierAssetPreparation))
                throw new InvalidDataException("The batch start value is unknown.");
            if (value.WorkScope == 0 || value.CancelScope == 0)
                throw new InvalidDataException("A work scope must be nonzero.");
            if (
                value.CancelScope.HasValue
                && (value.WorkScope.HasValue || value.Start != Wire.BatchStart.Now)
            )
                throw new InvalidDataException("Cancellation must be independent unowned work.");
            if (
                value.GroupsLength > 256
                || (value.GroupsLength == 0 && !value.CancelScope.HasValue)
            )
                throw new InvalidDataException(
                    "A batch must contain between one and 256 parallel groups."
                );
            var commandIds = new HashSet<Guid>();
            int commandCount = 0;
            for (int groupIndex = 0; groupIndex < value.GroupsLength; groupIndex++)
            {
                Wire.ParallelCommandGroup group =
                    value.Groups(groupIndex)
                    ?? throw new InvalidDataException("A parallel command group is absent.");
                if (group.CommandsLength == 0)
                    throw new InvalidDataException("A parallel group must contain a command.");
                commandCount = checked(commandCount + group.CommandsLength);
                if (commandCount > 4_096)
                    throw new InvalidDataException("A batch cannot exceed 4096 commands.");
                for (int commandIndex = 0; commandIndex < group.CommandsLength; commandIndex++)
                {
                    Wire.CommandEntry entry =
                        group.Commands(commandIndex)
                        ?? throw new InvalidDataException("A command entry is absent.");
                    if (entry.CommandType != Wire.CommandEntryPayload.CoreCommand)
                        throw new InvalidDataException("A core command entry has an unknown tag.");
                    Wire.CoreCommand command = entry.CommandAsCoreCommand();
                    if (!commandIds.Add(ReadUuid(command.CommandId, "command")))
                        throw new InvalidDataException("A batch repeats a command UUID.");
                    if (
                        value.CancelScope.HasValue
                        && command.Kind != Wire.CoreCommandKind.VisualElementDestroy
                        && command.Kind != Wire.CoreCommandKind.ObjectDestroy
                    )
                        throw new InvalidDataException(
                            "Cancellation cleanup may only destroy objects."
                        );
                    ValidateCommand(command);
                }
            }
        }

        internal static void ValidateSnapshot(Wire.Snapshot value, Guid expectedSession)
        {
            const int MaximumObjects = 100_000;
            if (ReadUuid(value.SessionId, "snapshot session") != expectedSession)
                throw new InvalidDataException("The snapshot session does not match its response.");
            if (value.ObjectsLength > MaximumObjects)
                throw new InvalidDataException("The snapshot object limit of 100000 was exceeded.");
            var scenes = new HashSet<Guid>();
            for (int index = 0; index < value.ScenesLength; index++)
                if (!scenes.Add(ReadUuid(value.Scenes(index)!.Value.SceneId, "snapshot scene")))
                    throw new InvalidDataException("The snapshot repeats a scene UUID.");
            if (
                value.PrimarySceneId.HasValue
                && !scenes.Contains(ReadUuid(value.PrimarySceneId, "primary scene"))
            )
                throw new InvalidDataException("The snapshot primary scene is unavailable.");
            var objects = new HashSet<Guid>();
            for (int index = 0; index < value.ObjectsLength; index++)
            {
                Wire.GameObject item = value.Objects(index)!.Value;
                if (!objects.Add(ReadUuid(item.ObjectId, "snapshot object")))
                    throw new InvalidDataException("The snapshot repeats an object UUID.");
                ValidateGameObject(item, scenes);
            }
            for (int index = 0; index < value.ObjectsLength; index++)
            {
                Wire.GameObject item = value.Objects(index)!.Value;
                if (item.ParentId.HasValue)
                {
                    Guid parent = ReadUuid(item.ParentId, "object parent");
                    if (parent == ReadUuid(item.ObjectId, "snapshot object"))
                        throw new InvalidDataException("A snapshot object cannot parent itself.");
                    if (!objects.Contains(parent))
                        throw new InvalidDataException("A snapshot object parent is unavailable.");
                }
            }
            if (
                value.InputCameraId.HasValue
                && !objects.Contains(ReadUuid(value.InputCameraId, "input camera"))
            )
                throw new InvalidDataException("The snapshot input camera is unavailable.");
            var uiIds = new HashSet<Guid>();
            var documentIds = new HashSet<Guid>();
            for (int index = 0; index < value.UiLength; index++)
                ValidateDocument(
                    value.Ui(index) ?? throw new InvalidDataException("A UI document is absent."),
                    uiIds,
                    documentIds
                );
        }

        private static void ValidateDirectSnapshot(Wire.Snapshot value)
        {
            const int MaximumAssets = 16_384;
            const int MaximumScenes = 32;
            const int MaximumObjects = 100_000;
            if (value.PreparedAssetsLength > MaximumAssets)
                throw new InvalidDataException("The prepared-asset limit was exceeded.");
            var assets = new Dictionary<string, Wire.PreparedAssetKind>(
                value.PreparedAssetsLength,
                StringComparer.Ordinal
            );
            for (int index = 0; index < value.PreparedAssetsLength; index++)
            {
                Wire.PreparedAsset asset = value.PreparedAssets(index)!.Value;
                RequireSnapshotString(asset.Address, "prepared asset address", allowEmpty: false);
                if (!Enum.IsDefined(typeof(Wire.PreparedAssetKind), asset.Kind))
                    throw new InvalidDataException("A prepared asset kind is unknown.");
                if (!assets.TryAdd(asset.Address, asset.Kind))
                    throw new InvalidDataException("A prepared asset address is repeated.");
            }

            if (value.ScenesLength == 0 || value.ScenesLength > MaximumScenes)
                throw new InvalidDataException(
                    "A snapshot must contain between one and 32 scenes."
                );
            var scenes = new Dictionary<Guid, string>(value.ScenesLength);
            var sceneAddresses = new HashSet<string>(StringComparer.Ordinal);
            for (int index = 0; index < value.ScenesLength; index++)
            {
                Wire.Scene scene = value.Scenes(index)!.Value;
                Guid id = ReadUuid(scene.SceneId, "snapshot scene");
                RequireSnapshotString(scene.Address, "scene address", allowEmpty: false);
                if (!scenes.TryAdd(id, scene.Address))
                    throw new InvalidDataException("A scene UUID is repeated.");
                if (!sceneAddresses.Add(scene.Address))
                    throw new InvalidDataException("A scene address is repeated.");
                RequirePrepared(assets, scene.Address, Wire.PreparedAssetKind.Scene, "scene");
            }
            Guid? primary = value.PrimarySceneId.HasValue
                ? ReadUuid(value.PrimarySceneId, "primary scene")
                : null;
            if (value.ScenesLength > 1 && primary is null)
                throw new InvalidDataException("A multi-scene snapshot has no primary scene.");
            if (primary is Guid primaryId && !scenes.ContainsKey(primaryId))
                throw new InvalidDataException("The primary scene is unavailable.");
            if (primary is null && value.ScenesLength == 1)
                primary = ReadUuid(value.Scenes(0)!.Value.SceneId, "snapshot scene");

            if (value.ObjectsLength > MaximumObjects)
                throw new InvalidDataException("The snapshot object limit was exceeded.");
            var objects = new Dictionary<Guid, Wire.GameObject>(value.ObjectsLength);
            for (int index = 0; index < value.ObjectsLength; index++)
            {
                Wire.GameObject item = value.Objects(index)!.Value;
                Guid id = ReadUuid(item.ObjectId, "snapshot object");
                if (scenes.ContainsKey(id) || !objects.TryAdd(id, item))
                    throw new InvalidDataException("A snapshot UUID is repeated.");
                ValidateDirectSnapshotObject(item, assets);
            }

            var depths = new Dictionary<Guid, int>(objects.Count);
            var visiting = new HashSet<Guid>();
            int Depth(Guid id)
            {
                if (depths.TryGetValue(id, out int known))
                    return known;
                if (!visiting.Add(id))
                    throw new InvalidDataException("The object hierarchy is cyclic.");
                Wire.GameObject item = objects[id];
                int depth = 0;
                if (item.ParentId.HasValue)
                {
                    Guid parentId = ReadUuid(item.ParentId, "object parent");
                    if (!objects.TryGetValue(parentId, out Wire.GameObject parent))
                        throw new InvalidDataException("An object parent is unavailable.");
                    if (Placement(item, primary) != Placement(parent, primary))
                        throw new InvalidDataException(
                            "An object and its parent belong to different scenes."
                        );
                    depth = checked(Depth(parentId) + 1);
                    if (depth > 256)
                        throw new InvalidDataException("The object hierarchy exceeds 256 levels.");
                }
                visiting.Remove(id);
                depths.Add(id, depth);
                return depth;
            }
            foreach (Guid id in objects.Keys)
                _ = Depth(id);

            ValidateDirectInputCamera(value.InputCameraId, objects);
            var keys = new HashSet<Wire.PhysicalKey>();
            for (int index = 0; index < value.GlobalKeysLength; index++)
            {
                Wire.PhysicalKey key = value.GlobalKeys(index);
                if (!Enum.IsDefined(typeof(Wire.PhysicalKey), key) || !keys.Add(key))
                    throw new InvalidDataException("Global keys are unknown or repeated.");
            }
            if (value.ControllerInput.HasValue)
                ValidateDirectController(value.ControllerInput.Value);
            ValidateDirectPanel(value.PanelInputConfiguration!.Value);
        }

        private static void ValidateDirectSnapshotObject(
            Wire.GameObject value,
            IReadOnlyDictionary<string, Wire.PreparedAssetKind> assets
        )
        {
            ValidateDirectCreatedPlacement(value);
            switch (value.Kind)
            {
                case Wire.GameObjectKind.UiDocument:
                {
                    if (value.ContentType != Wire.GameObjectContent.UiDocumentObject)
                        throw new InvalidDataException(
                            "A UI document object has the wrong payload."
                        );
                    GameObjectKind.UiDocumentState state =
                        BattlementFlatBufferRetainedCopy.UiDocumentState(
                            value.ContentAsUiDocumentObject()
                        );
                    BattlementUiDocumentValidator.Validate(
                        state,
                        address =>
                            assets.TryGetValue(address.Value, out Wire.PreparedAssetKind kind)
                            && kind == Wire.PreparedAssetKind.RenderTexture
                    );
                    break;
                }
                case Wire.GameObjectKind.Empty:
                    if (value.ContentType != Wire.GameObjectContent.EmptyObject)
                        throw new InvalidDataException("An empty object has the wrong payload.");
                    break;
                case Wire.GameObjectKind.Image:
                {
                    ValidateDirectCreatedImage(value);
                    Wire.ImageObject image = value.ContentAsImageObject();
                    RequirePrepared(
                        assets,
                        image.Texture,
                        Wire.PreparedAssetKind.Texture,
                        "texture"
                    );
                    break;
                }
                case Wire.GameObjectKind.Text:
                {
                    ValidateDirectCreatedText(value);
                    Wire.TextObject text = value.ContentAsTextObject();
                    RequirePrepared(
                        assets,
                        text.Font,
                        Wire.PreparedAssetKind.TextMeshProFont,
                        "font"
                    );
                    break;
                }
                case Wire.GameObjectKind.Camera:
                    ValidateDirectCreatedCamera(value);
                    break;
                case Wire.GameObjectKind.Light:
                    ValidateDirectCreatedLight(value);
                    break;
                case Wire.GameObjectKind.Mesh:
                {
                    Wire.MeshObject mesh = value.ContentAsMeshObject();
                    RequirePrepared(assets, mesh.Address, Wire.PreparedAssetKind.Mesh, "mesh");
                    ValidateDirectMaterials(
                        mesh.MaterialsLength,
                        index => mesh.Materials(index)!.Value
                    );
                    ValidatePreparedMaterials(mesh.MaterialsLength, mesh.Materials, assets);
                    break;
                }
                case Wire.GameObjectKind.Prefab:
                {
                    Wire.PrefabObject prefab = value.ContentAsPrefabObject();
                    RequirePrepared(
                        assets,
                        prefab.Address,
                        Wire.PreparedAssetKind.Prefab,
                        "prefab"
                    );
                    ValidateDirectMaterials(prefab);
                    ValidatePreparedMaterials(prefab.MaterialsLength, prefab.Materials, assets);
                    if (prefab.Animator.HasValue)
                        ValidateDirectAnimator(prefab.Animator.Value);
                    break;
                }

                case Wire.GameObjectKind.Cube:
                case Wire.GameObjectKind.Sphere:
                case Wire.GameObjectKind.Capsule:
                case Wire.GameObjectKind.Cylinder:
                case Wire.GameObjectKind.Plane:
                case Wire.GameObjectKind.Quad:
                {
                    if (value.ContentType != Wire.GameObjectContent.PrimitiveObject)
                        throw new InvalidDataException(
                            "A primitive object has the wrong content payload."
                        );
                    Wire.PrimitiveObject primitive = value.ContentAsPrimitiveObject();
                    ValidateDirectMaterials(primitive);
                    ValidatePreparedMaterials(
                        primitive.MaterialsLength,
                        primitive.Materials,
                        assets
                    );
                    break;
                }
                default:
                    throw new InvalidDataException("A direct snapshot object kind is unknown.");
            }
        }

        private static void ValidatePreparedMaterials(
            int count,
            Func<int, Wire.MaterialAssignment?> item,
            IReadOnlyDictionary<string, Wire.PreparedAssetKind> assets
        )
        {
            for (int index = 0; index < count; index++)
            {
                Wire.MaterialAssignment material = item(index)!.Value;
                RequirePrepared(
                    assets,
                    material.Address,
                    Wire.PreparedAssetKind.Material,
                    "material"
                );
            }
        }

        private static (bool Persistent, Guid? Scene) Placement(
            Wire.GameObject value,
            Guid? primary
        )
        {
            Wire.ParentScene placement = value.ParentScene!.Value;
            return placement.Kind switch
            {
                Wire.ParentSceneKind.Persistent => (true, null),
                Wire.ParentSceneKind.PrimaryScene => (false, primary),
                Wire.ParentSceneKind.Scene => (
                    false,
                    ReadUuid(placement.SceneId, "object parent scene")
                ),
                _ => throw new InvalidDataException("An object parent scene kind is unknown."),
            };
        }

        private static void ValidateDirectInputCamera(
            Wire.Uuid? cameraId,
            IReadOnlyDictionary<Guid, Wire.GameObject> objects
        )
        {
            if (!cameraId.HasValue)
                return;
            Guid id = ReadUuid(cameraId, "input camera");
            if (!objects.TryGetValue(id, out Wire.GameObject camera))
                throw new InvalidDataException("The input camera is unavailable.");
            bool validKind = camera.Kind == Wire.GameObjectKind.Prefab;
            if (camera.Kind == Wire.GameObjectKind.Camera)
                validKind = camera.ContentAsCameraObject().Enabled;
            if (!validKind)
                throw new InvalidDataException("The input camera kind is invalid or disabled.");
            Wire.GameObject current = camera;
            while (true)
            {
                if (!current.Active)
                    throw new InvalidDataException("The input camera hierarchy is inactive.");
                if (!current.ParentId.HasValue)
                    return;
                current = objects[ReadUuid(current.ParentId, "input camera parent")];
            }
        }

        private static void ValidateDirectController(Wire.ControllerInputSettings value)
        {
            var buttons = new HashSet<Wire.ControllerButton>();
            for (int index = 0; index < value.ButtonsLength; index++)
            {
                Wire.ControllerButton button = value.Buttons(index);
                if (!Enum.IsDefined(typeof(Wire.ControllerButton), button) || !buttons.Add(button))
                    throw new InvalidDataException("Controller buttons are unknown or repeated.");
            }
            if (value.StickDeadZone is double deadZone)
            {
                RequireFinite(deadZone);
                if (deadZone < 0 || deadZone >= 1)
                    throw new InvalidDataException("The controller stick dead zone is invalid.");
            }
            if (value.RepeatDelayMs is ulong delay)
                RequirePositiveSnapshotDuration(delay, "controller repeat delay");
            if (value.RepeatIntervalMs is ulong interval)
                RequirePositiveSnapshotDuration(interval, "controller repeat interval");
        }

        private static void ValidateDirectPanel(Wire.PanelInputConfiguration value)
        {
            if (!Known(value.InputRedirection, Wire.PanelInputRedirection.Always))
                throw new InvalidDataException("The panel input redirection is unknown.");
            switch (value.DistanceKind)
            {
                case Wire.InteractionDistanceKind.Unbounded:
                    break;
                case Wire.InteractionDistanceKind.Inclusive:
                    if (!float.IsFinite(value.MaximumInteractionDistance))
                        throw new InvalidDataException(
                            "The panel interaction distance is invalid."
                        );
                    if (value.MaximumInteractionDistance < 0)
                        throw new InvalidDataException(
                            "The panel interaction distance is negative."
                        );
                    break;
                default:
                    throw new InvalidDataException(
                        "The panel interaction distance kind is unknown."
                    );
            }
        }

        private static void RequirePrepared(
            IReadOnlyDictionary<string, Wire.PreparedAssetKind> assets,
            string address,
            Wire.PreparedAssetKind expected,
            string kind
        )
        {
            RequireSnapshotString(address, $"{kind} address", allowEmpty: false);
            if (
                !assets.TryGetValue(address, out Wire.PreparedAssetKind actual)
                || actual != expected
            )
                throw new InvalidDataException(
                    $"The {kind} address '{address}' is not prepared with the required type."
                );
        }

        private static void RequireSnapshotString(string value, string field, bool allowEmpty)
        {
            if (!allowEmpty && value.Length == 0)
                throw new InvalidDataException($"The {field} is empty.");
            if (Encoding.UTF8.GetByteCount(value) > 65_536)
                throw new InvalidDataException($"The {field} exceeds 65536 UTF-8 bytes.");
        }

        private static void RequirePositiveSnapshotDuration(ulong value, string field)
        {
            if (value == 0 || value > 86_400_000)
                throw new InvalidDataException(
                    $"The {field} must be positive and at most one day."
                );
        }

        internal static void ValidateCommand(Wire.CoreCommand value)
        {
            _ = ReadUuid(value.CommandId, "command");
            Wire.CoreCommandPayload expected = ExpectedPayload(value.Kind);
            if (value.PayloadType != expected)
            {
                throw new InvalidDataException(
                    $"Core command {value.Kind} requires {expected}, not {value.PayloadType}."
                );
            }
            switch (value.Kind)
            {
                case Wire.CoreCommandKind.VisualElementCreate:
                    ValidateCreate(value.PayloadAsVisualElementCreatePayload());
                    break;
                case Wire.CoreCommandKind.VisualElementUpdate:
                    ValidateUpdate(value.PayloadAsVisualElementUpdatePayload());
                    break;
                case Wire.CoreCommandKind.VisualElementDestroy:
                    _ = ReadUuid(
                        value.PayloadAsVisualElementDestroyPayload().ObjectId,
                        "visual destroy object"
                    );
                    break;
                case Wire.CoreCommandKind.VisualElementPerformAction:
                    ValidateAction(value.PayloadAsVisualElementActionPayload());
                    break;
                case Wire.CoreCommandKind.ApplicationOpenUrl:
                    break;
                case Wire.CoreCommandKind.Diagnostics:
                    break;
                case Wire.CoreCommandKind.AssetsReplaceSet:
                    break;
                case Wire.CoreCommandKind.SceneLoad:
                    break;
                case Wire.CoreCommandKind.SceneUnload:
                    break;
                case Wire.CoreCommandKind.SceneSetPrimary:
                    break;
                case Wire.CoreCommandKind.ObjectCreate:
                    break;
                case Wire.CoreCommandKind.ObjectDestroy:
                    break;
                case Wire.CoreCommandKind.ObjectSetActive:
                    break;
                case Wire.CoreCommandKind.ObjectReparent:
                    break;
                case Wire.CoreCommandKind.TransformSetLocalPosition:
                    break;
                case Wire.CoreCommandKind.TransformSetWorldPosition:
                    break;
                case Wire.CoreCommandKind.TransformTweenLocalPosition:
                    break;
                case Wire.CoreCommandKind.TransformTweenWorldPosition:
                    break;
                case Wire.CoreCommandKind.TransformSetLocalRotation:
                    break;
                case Wire.CoreCommandKind.TransformSetWorldRotation:
                    break;
                case Wire.CoreCommandKind.TransformTweenLocalRotation:
                    break;
                case Wire.CoreCommandKind.TransformTweenWorldRotation:
                    break;
                case Wire.CoreCommandKind.TransformSetLocalScale:
                    break;
                case Wire.CoreCommandKind.TransformTweenLocalScale:
                    break;
                case Wire.CoreCommandKind.RendererSetMaterial:
                    break;
                case Wire.CoreCommandKind.CameraSetEnabled:
                    break;
                case Wire.CoreCommandKind.CameraSetPerspective:
                    break;
                case Wire.CoreCommandKind.CameraTweenFieldOfView:
                    break;
                case Wire.CoreCommandKind.CameraSetOrthographic:
                    break;
                case Wire.CoreCommandKind.CameraTweenOrthographicSize:
                    break;
                case Wire.CoreCommandKind.CameraSetClipping:
                    break;
                case Wire.CoreCommandKind.CameraSetClear:
                    break;
                case Wire.CoreCommandKind.LightSetEnabled:
                    break;
                case Wire.CoreCommandKind.LightSetType:
                    break;
                case Wire.CoreCommandKind.LightSetColor:
                    break;
                case Wire.CoreCommandKind.LightTweenColor:
                    break;
                case Wire.CoreCommandKind.LightSetIntensity:
                    break;
                case Wire.CoreCommandKind.LightTweenIntensity:
                    break;
                case Wire.CoreCommandKind.LightSetRange:
                    break;
                case Wire.CoreCommandKind.LightSetSpotAngle:
                    break;
                case Wire.CoreCommandKind.LightSetShadows:
                    break;
                case Wire.CoreCommandKind.ImageSetTexture:
                    break;
                case Wire.CoreCommandKind.ImageSetSize:
                    break;
                case Wire.CoreCommandKind.ImageSetFit:
                    break;
                case Wire.CoreCommandKind.ImageSetTint:
                    break;
                case Wire.CoreCommandKind.ImageTweenTint:
                    break;
                case Wire.CoreCommandKind.ImageSetOpacity:
                    break;
                case Wire.CoreCommandKind.ImageTweenOpacity:
                    break;
                case Wire.CoreCommandKind.ImageSetFaceCamera:
                    break;
                case Wire.CoreCommandKind.TextSetContent:
                    break;
                case Wire.CoreCommandKind.TextSetFont:
                    break;
                case Wire.CoreCommandKind.TextSetSize:
                    break;
                case Wire.CoreCommandKind.TextTweenSize:
                    break;
                case Wire.CoreCommandKind.TextSetColor:
                    break;
                case Wire.CoreCommandKind.TextTweenColor:
                    break;
                case Wire.CoreCommandKind.TextSetAlignment:
                    break;
                case Wire.CoreCommandKind.TextSetWrapping:
                    break;
                case Wire.CoreCommandKind.TextSetRichText:
                    break;
                case Wire.CoreCommandKind.TextSetFaceCamera:
                    break;
                case Wire.CoreCommandKind.AnimatorPlay:
                    break;
                case Wire.CoreCommandKind.AnimatorCrossFade:
                    break;
                case Wire.CoreCommandKind.AnimatorSetBool:
                    break;
                case Wire.CoreCommandKind.AnimatorSetInt:
                    break;
                case Wire.CoreCommandKind.AnimatorSetFloat:
                    break;
                case Wire.CoreCommandKind.AnimatorSetTrigger:
                    break;
                case Wire.CoreCommandKind.AnimatorSetSpeed:
                    break;
                case Wire.CoreCommandKind.ParticlePlay:
                    break;
                case Wire.CoreCommandKind.ParticleStop:
                    break;
                case Wire.CoreCommandKind.ParticleSpawn:
                    break;
                case Wire.CoreCommandKind.AudioPlay:
                    break;
                case Wire.CoreCommandKind.AudioStop:
                    break;
                case Wire.CoreCommandKind.AudioPause:
                    break;
                case Wire.CoreCommandKind.AudioResume:
                    break;
                case Wire.CoreCommandKind.AudioSeek:
                    break;
                case Wire.CoreCommandKind.AudioSetBuffering:
                    break;
                case Wire.CoreCommandKind.AudioReplace:
                    break;
                case Wire.CoreCommandKind.AudioSetVolume:
                    break;
                case Wire.CoreCommandKind.AudioTweenVolume:
                    break;
                case Wire.CoreCommandKind.TimeWait:
                    break;
                case Wire.CoreCommandKind.OperationCancel:
                    break;
                case Wire.CoreCommandKind.InputSetEnabled:
                    break;
                case Wire.CoreCommandKind.InputSetCamera:
                    break;
                case Wire.CoreCommandKind.InputSetPointerEvents:
                    break;
                case Wire.CoreCommandKind.InputSetGlobalKeys:
                    break;
                case Wire.CoreCommandKind.InputSetController:
                    break;
                case Wire.CoreCommandKind.ControllerVibrate:
                    break;
                case Wire.CoreCommandKind.DebugUi:
                    break;
                case Wire.CoreCommandKind.MotionValue:
                    break;
                case Wire.CoreCommandKind.MotionValuePlayback:
                    break;
                case Wire.CoreCommandKind.MotionPlayback:
                    break;
                case Wire.CoreCommandKind.MotionControlledClock:
                    break;
                case Wire.CoreCommandKind.MotionControl:
                    break;
                case Wire.CoreCommandKind.MotionScope:
                    break;
                case Wire.CoreCommandKind.MotionDragControl:
                    break;
                case Wire.CoreCommandKind.GeometryObservationUpdate:
                    break;
                case Wire.CoreCommandKind.AccessibilityUpdate:
                    break;
                default:
                    break;
            }
            if (ValidateDirectCommand(value))
                return;
            throw new InvalidDataException(
                $"Core FlatBuffer command {value.Kind} has no direct semantic validator."
            );
        }

        private static bool ValidateDirectCommand(Wire.CoreCommand value)
        {
            if (ValidateDirectAnimatorCommand(value))
                return true;
            if (ValidateDirectComponentCommand(value))
                return true;
            switch (value.Kind)
            {
                case Wire.CoreCommandKind.VisualElementDestroy:
                case Wire.CoreCommandKind.VisualElementPerformAction:
                case Wire.CoreCommandKind.VisualElementCreate:
                    return true;
                case Wire.CoreCommandKind.AssetsReplaceSet:
                    ValidateDirectAssetSet(value.PayloadAsReplaceAssetSetPayload());
                    return true;
                case Wire.CoreCommandKind.Diagnostics:
                {
                    Wire.DiagnosticsPayload payload = value.PayloadAsDiagnosticsPayload();
                    if (!value.Blocking)
                        throw new InvalidDataException("Diagnostics commands must be blocking.");
                    if (DiagnosticsProtocol.Validate(payload.Key, payload.Value) is not null)
                        throw new InvalidDataException("Diagnostics metadata is invalid.");
                    return true;
                }
                case Wire.CoreCommandKind.GeometryObservationUpdate:
                    ValidateDirectGeometry(value.PayloadAsGeometryObservationUpdate());
                    return true;
                case Wire.CoreCommandKind.AccessibilityUpdate:
                    ValidateDirectAccessibility(value.PayloadAsAccessibilityUpdate());
                    return true;
                case Wire.CoreCommandKind.ApplicationOpenUrl:
                {
                    string url = value.PayloadAsExternalUrlPayload().Url;
                    if (!Uri.TryCreate(url, UriKind.Absolute, out _))
                        throw new InvalidDataException("An external URL is invalid.");
                    return true;
                }
                case Wire.CoreCommandKind.SceneLoad:
                {
                    Wire.SceneLoadPayload payload = value.PayloadAsSceneLoadPayload();
                    _ = ReadUuid(payload.SceneId, "scene");
                    RequireSnapshotString(payload.Address, "scene address", allowEmpty: false);
                    return true;
                }
                case Wire.CoreCommandKind.SceneUnload:
                case Wire.CoreCommandKind.SceneSetPrimary:
                    _ = ReadUuid(value.PayloadAsSceneIdPayload().SceneId, "scene");
                    return true;
                case Wire.CoreCommandKind.TransformSetLocalPosition:
                case Wire.CoreCommandKind.TransformSetWorldPosition:
                {
                    Wire.PositionPayload payload = value.PayloadAsPositionPayload();
                    _ = ReadUuid(payload.ObjectId, "position object");
                    RequireConflict(payload.OnConflict);
                    RequireFinite(payload.Position!.Value);
                    return true;
                }
                case Wire.CoreCommandKind.TransformTweenLocalPosition:
                case Wire.CoreCommandKind.TransformTweenWorldPosition:
                {
                    Wire.TweenPositionPayload payload = value.PayloadAsTweenPositionPayload();
                    _ = ReadUuid(payload.ObjectId, "position object");
                    RequireConflict(payload.OnConflict);
                    RequireFinite(payload.Position!.Value);
                    ValidateDirectTween(payload.Tween!.Value, value.Blocking);
                    return true;
                }
                case Wire.CoreCommandKind.TransformSetLocalRotation:
                case Wire.CoreCommandKind.TransformSetWorldRotation:
                {
                    Wire.RotationPayload payload = value.PayloadAsRotationPayload();
                    _ = ReadUuid(payload.ObjectId, "rotation object");
                    RequireConflict(payload.OnConflict);
                    RequireQuaternion(payload.Rotation!.Value);
                    return true;
                }
                case Wire.CoreCommandKind.TransformTweenLocalRotation:
                case Wire.CoreCommandKind.TransformTweenWorldRotation:
                {
                    Wire.TweenRotationPayload payload = value.PayloadAsTweenRotationPayload();
                    _ = ReadUuid(payload.ObjectId, "rotation object");
                    RequireConflict(payload.OnConflict);
                    RequireQuaternion(payload.Rotation!.Value);
                    ValidateDirectTween(payload.Tween!.Value, value.Blocking);
                    return true;
                }
                case Wire.CoreCommandKind.TransformSetLocalScale:
                {
                    Wire.ScalePayload payload = value.PayloadAsScalePayload();
                    _ = ReadUuid(payload.ObjectId, "scale object");
                    RequireConflict(payload.OnConflict);
                    RequireFinite(payload.Scale!.Value);
                    return true;
                }
                case Wire.CoreCommandKind.TransformTweenLocalScale:
                {
                    Wire.TweenScalePayload payload = value.PayloadAsTweenScalePayload();
                    _ = ReadUuid(payload.ObjectId, "scale object");
                    RequireConflict(payload.OnConflict);
                    RequireFinite(payload.Scale!.Value);
                    ValidateDirectTween(payload.Tween!.Value, value.Blocking);
                    return true;
                }
                case Wire.CoreCommandKind.TextSetContent:
                {
                    Wire.TextContentPayload payload = value.PayloadAsTextContentPayload();
                    _ = ReadUuid(payload.ObjectId, "text object");
                    _ = payload.Content;
                    return true;
                }
                case Wire.CoreCommandKind.RendererSetMaterial:
                {
                    Wire.SetMaterialPayload payload = value.PayloadAsSetMaterialPayload();
                    _ = ReadUuid(payload.ObjectId, "renderer object");
                    RequireConflict(payload.OnConflict);
                    _ = payload.Address;
                    return true;
                }
                case Wire.CoreCommandKind.ObjectDestroy:
                    _ = ReadUuid(value.PayloadAsObjectIdPayload().ObjectId, "destroyed object");
                    return true;
                case Wire.CoreCommandKind.InputSetEnabled:
                    return true;
                case Wire.CoreCommandKind.InputSetCamera:
                    _ = ReadUuid(value.PayloadAsObjectIdPayload().ObjectId, "input camera");
                    return true;
                case Wire.CoreCommandKind.InputSetPointerEvents:
                {
                    Wire.PointerEventsPayload payload = value.PayloadAsPointerEventsPayload();
                    _ = ReadUuid(payload.ObjectId, "input object");
                    var events = new HashSet<Wire.PointerEventKind>();
                    for (int index = 0; index < payload.EventsLength; index++)
                    {
                        Wire.PointerEventKind item = payload.Events(index);
                        if (!Known(item, Wire.PointerEventKind.Click) || !events.Add(item))
                            throw new InvalidDataException(
                                "Pointer events are unknown or repeated."
                            );
                    }
                    return true;
                }
                case Wire.CoreCommandKind.InputSetGlobalKeys:
                {
                    Wire.GlobalKeysPayload payload = value.PayloadAsGlobalKeysPayload();
                    var keys = new HashSet<Wire.PhysicalKey>();
                    for (int index = 0; index < payload.KeysLength; index++)
                    {
                        Wire.PhysicalKey key = payload.Keys(index);
                        if (!Enum.IsDefined(typeof(Wire.PhysicalKey), key) || !keys.Add(key))
                            throw new InvalidDataException("Global keys are unknown or repeated.");
                    }
                    return true;
                }
                case Wire.CoreCommandKind.InputSetController:
                    ValidateDirectController(value.PayloadAsControllerInputSettings());
                    return true;
                case Wire.CoreCommandKind.ObjectCreate:
                {
                    Wire.GameObject created = value.PayloadAsObjectCreatePayload().Object!.Value;
                    ValidateDirectCreatedPlacement(created);
                    if (created.Kind == Wire.GameObjectKind.Image)
                    {
                        ValidateDirectCreatedImage(created);
                        return true;
                    }
                    if (created.Kind == Wire.GameObjectKind.Empty)
                    {
                        if (created.ContentType != Wire.GameObjectContent.EmptyObject)
                            throw new InvalidDataException(
                                "An empty object has the wrong content payload."
                            );
                        return true;
                    }
                    if (created.Kind == Wire.GameObjectKind.Text)
                    {
                        ValidateDirectCreatedText(created);
                        return true;
                    }
                    if (created.Kind == Wire.GameObjectKind.Camera)
                    {
                        ValidateDirectCreatedCamera(created);
                        return true;
                    }
                    if (created.Kind == Wire.GameObjectKind.Light)
                    {
                        ValidateDirectCreatedLight(created);
                        return true;
                    }
                    if (IsDirectPrimitive(created.Kind))
                    {
                        if (created.ContentType != Wire.GameObjectContent.PrimitiveObject)
                            throw new InvalidDataException(
                                "A primitive object has the wrong content payload."
                            );
                        ValidateDirectMaterials(created.ContentAsPrimitiveObject());
                        return true;
                    }
                    if (created.Kind == Wire.GameObjectKind.Mesh)
                    {
                        if (created.ContentType != Wire.GameObjectContent.MeshObject)
                            throw new InvalidDataException(
                                "A mesh object has the wrong content payload."
                            );
                        Wire.MeshObject mesh = created.ContentAsMeshObject();
                        _ = mesh.Address;
                        ValidateDirectMaterials(
                            mesh.MaterialsLength,
                            index => mesh.Materials(index)!.Value
                        );
                        return true;
                    }
                    if (created.Kind == Wire.GameObjectKind.Prefab)
                    {
                        if (created.ContentType != Wire.GameObjectContent.PrefabObject)
                            throw new InvalidDataException(
                                "A prefab object has the wrong content payload."
                            );
                        Wire.PrefabObject prefab = created.ContentAsPrefabObject();
                        _ = prefab.Address;
                        ValidateDirectMaterials(prefab);
                        if (prefab.Animator.HasValue)
                            ValidateDirectAnimator(prefab.Animator.Value);
                        return true;
                    }
                    return false;
                }
                case Wire.CoreCommandKind.ObjectSetActive:
                {
                    _ = ReadUuid(value.PayloadAsObjectSetActivePayload().ObjectId, "active object");
                    return true;
                }
                case Wire.CoreCommandKind.ObjectReparent:
                {
                    Wire.ObjectReparentPayload payload = value.PayloadAsObjectReparentPayload();
                    Guid objectId = ReadUuid(payload.ObjectId, "reparented object");
                    if (payload.ParentId.HasValue)
                    {
                        Guid parentId = ReadUuid(payload.ParentId, "new object parent");
                        if (parentId == objectId)
                            throw new InvalidDataException(
                                "An object cannot be parented to itself."
                            );
                    }
                    return true;
                }
                case Wire.CoreCommandKind.ParticleSpawn:
                {
                    Wire.ParticleSpawnPayload payload = value.PayloadAsParticleSpawnPayload();
                    _ = payload.Address;
                    switch (payload.LocationKind)
                    {
                        case Wire.ParticleSpawnLocationKind.GameObject:
                            _ = ReadUuid(payload.ObjectId, "particle target");
                            if (payload.WorldPosition.HasValue)
                                throw new InvalidDataException(
                                    "A game-object particle location carries a world position."
                                );
                            break;
                        case Wire.ParticleSpawnLocationKind.WorldPosition:
                            if (payload.ObjectId.HasValue)
                                throw new InvalidDataException(
                                    "A world particle location carries an object UUID."
                                );
                            RequireFinite(
                                payload.WorldPosition
                                    ?? throw new InvalidDataException(
                                        "A world particle location is absent."
                                    )
                            );
                            break;
                        default:
                            throw new InvalidDataException("A particle location kind is unknown.");
                    }
                    if (payload.LifetimeMs == 0 || payload.LifetimeMs > 86_400_000)
                        throw new InvalidDataException(
                            "A particle lifetime must be positive and at most one day."
                        );
                    return true;
                }
                case Wire.CoreCommandKind.ParticlePlay:
                    _ = ReadUuid(value.PayloadAsParticlePlayPayload().ObjectId, "particle object");
                    if (value.Blocking)
                        throw new InvalidDataException("Particle play must be nonblocking.");
                    return true;
                case Wire.CoreCommandKind.ParticleStop:
                    _ = ReadUuid(value.PayloadAsParticleStopPayload().ObjectId, "particle object");
                    return true;
                case Wire.CoreCommandKind.AudioPlay:
                {
                    Wire.AudioPlayPayload payload = value.PayloadAsAudioPlayPayload();
                    _ = payload.Address;
                    RequireUnit(payload.Volume, "Audio volume");
                    RequireFinite(payload.Pitch);
                    if (payload.Pitch <= 0 || payload.Pitch > 3)
                        throw new InvalidDataException(
                            "Audio pitch must be greater than zero and at most three."
                        );
                    if (payload.FadeInMs > 86_400_000)
                        throw new InvalidDataException("Audio fade-in cannot exceed one day.");
                    if (value.Blocking && payload.Loop)
                        throw new InvalidDataException("Looping audio must be nonblocking.");
                    return true;
                }
                case Wire.CoreCommandKind.AudioStop:
                {
                    Wire.AudioStopPayload payload = value.PayloadAsAudioStopPayload();
                    _ = ReadUuid(payload.AudioCommandId, "audio command");
                    if (payload.FadeOutMs > 86_400_000)
                        throw new InvalidDataException("Audio fade-out cannot exceed one day.");
                    return true;
                }
                case Wire.CoreCommandKind.AudioSetVolume:
                {
                    Wire.AudioVolumePayload payload = value.PayloadAsAudioVolumePayload();
                    _ = ReadUuid(payload.AudioCommandId, "audio command");
                    RequireConflict(payload.OnConflict);
                    RequireUnit(payload.Volume, "Audio volume");
                    return true;
                }
                case Wire.CoreCommandKind.AudioPause:
                case Wire.CoreCommandKind.AudioResume:
                    _ = ReadUuid(
                        value.PayloadAsAudioPlaybackPayload().AudioCommandId,
                        "audio command"
                    );
                    return true;
                case Wire.CoreCommandKind.AudioSeek:
                {
                    Wire.AudioSeekPayload payload = value.PayloadAsAudioSeekPayload();
                    _ = ReadUuid(payload.AudioCommandId, "audio command");
                    if (payload.PositionMs > 922_337_203_685)
                        throw new InvalidDataException(
                            "Audio seek position exceeds the supported range."
                        );
                    return true;
                }
                case Wire.CoreCommandKind.AudioSetBuffering:
                    _ = ReadUuid(
                        value.PayloadAsAudioBufferingPayload().AudioCommandId,
                        "audio command"
                    );
                    return true;
                case Wire.CoreCommandKind.AudioReplace:
                {
                    Wire.AudioReplacePayload payload = value.PayloadAsAudioReplacePayload();
                    _ = ReadUuid(payload.AudioCommandId, "audio command");
                    _ = payload.Address;
                    return true;
                }
                case Wire.CoreCommandKind.AudioTweenVolume:
                {
                    Wire.TweenAudioVolumePayload payload = value.PayloadAsTweenAudioVolumePayload();
                    _ = ReadUuid(payload.AudioCommandId, "audio command");
                    RequireConflict(payload.OnConflict);
                    RequireUnit(payload.Volume, "Audio volume");
                    ValidateDirectTween(payload.Tween!.Value, value.Blocking);
                    return true;
                }
                case Wire.CoreCommandKind.TimeWait:
                {
                    ulong duration = value.PayloadAsWaitPayload().DurationMs;
                    if (duration == 0 || duration > 86_400_000)
                        throw new InvalidDataException(
                            "A wait duration must be positive and at most one day."
                        );
                    return true;
                }
                case Wire.CoreCommandKind.OperationCancel:
                    _ = ReadUuid(
                        value.PayloadAsCancelOperationPayload().CommandId,
                        "canceled command"
                    );
                    return true;
                case Wire.CoreCommandKind.ControllerVibrate:
                {
                    Wire.ControllerVibrationPayload payload =
                        value.PayloadAsControllerVibrationPayload();
                    RequireUnit(payload.LowFrequency, "Low-frequency motor intensity");
                    RequireUnit(payload.HighFrequency, "High-frequency motor intensity");
                    if (payload.DurationMs > 922_337_203_685)
                        throw new InvalidDataException(
                            "Controller vibration duration exceeds the supported range."
                        );
                    return true;
                }
                case Wire.CoreCommandKind.DebugUi:
                    if (
                        !Known(
                            value.PayloadAsDebugUiPayload().Surface,
                            Wire.DebugUiSurface.FpsViewer
                        )
                    )
                        throw new InvalidDataException("A debug UI surface is unknown.");
                    return true;
                case Wire.CoreCommandKind.VisualElementUpdate:
                    return true;
                case Wire.CoreCommandKind.MotionValue:
                    ValidateDirectMotionValue(value.PayloadAsMotionValueOperation());
                    return true;
                case Wire.CoreCommandKind.MotionValuePlayback:
                {
                    Wire.MotionValuePlaybackOperation operation =
                        value.PayloadAsMotionValuePlaybackOperation();
                    _ = ReadUuid(operation.PlaybackId, "motion value playback");
                    ValidateDirectPlayback(operation.Command);
                    return true;
                }
                case Wire.CoreCommandKind.MotionPlayback:
                {
                    Wire.MotionPlaybackOperation operation =
                        value.PayloadAsMotionPlaybackOperation();
                    _ = ReadUuid(operation.DescriptorId, "motion descriptor");
                    ValidateDirectPlayback(operation.Command);
                    return true;
                }
                case Wire.CoreCommandKind.MotionControlledClock:
                {
                    Wire.MotionControlledClockOperation operation =
                        value.PayloadAsMotionControlledClockOperation();
                    _ = ReadUuid(operation.ClockId, "motion clock");
                    if (!Known(operation.Command, Wire.MotionControlledClockCommandKind.Advance))
                        throw new InvalidDataException(
                            "A controlled motion clock command is unknown."
                        );
                    return true;
                }
                case Wire.CoreCommandKind.MotionControl:
                    ValidateDirectMotionControl(value.PayloadAsMotionControlOperation());
                    return true;
                case Wire.CoreCommandKind.MotionScope:
                    ValidateDirectMotionScope(value.PayloadAsMotionScopeOperation());
                    return true;
                case Wire.CoreCommandKind.MotionDragControl:
                {
                    Wire.MotionDragControlOperation operation =
                        value.PayloadAsMotionDragControlOperation();
                    _ = ReadUuid(operation.ControlId, "motion drag control");
                    if (
                        operation.PointerId < 0
                        || !Known(operation.Device, Wire.MotionPointerDevice.Gamepad)
                    )
                        throw new InvalidDataException("A motion drag input is invalid.");
                    Wire.MotionVector2 point = operation.Point!.Value;
                    if (!Finite(point.X) || !Finite(point.Y))
                        throw new InvalidDataException("A motion drag point must be finite.");
                    return true;
                }

                case Wire.CoreCommandKind.CameraSetEnabled:
                    break;
                case Wire.CoreCommandKind.CameraSetPerspective:
                    break;
                case Wire.CoreCommandKind.CameraTweenFieldOfView:
                    break;
                case Wire.CoreCommandKind.CameraSetOrthographic:
                    break;
                case Wire.CoreCommandKind.CameraTweenOrthographicSize:
                    break;
                case Wire.CoreCommandKind.CameraSetClipping:
                    break;
                case Wire.CoreCommandKind.CameraSetClear:
                    break;
                case Wire.CoreCommandKind.LightSetEnabled:
                    break;
                case Wire.CoreCommandKind.LightSetType:
                    break;
                case Wire.CoreCommandKind.LightSetColor:
                    break;
                case Wire.CoreCommandKind.LightTweenColor:
                    break;
                case Wire.CoreCommandKind.LightSetIntensity:
                    break;
                case Wire.CoreCommandKind.LightTweenIntensity:
                    break;
                case Wire.CoreCommandKind.LightSetRange:
                    break;
                case Wire.CoreCommandKind.LightSetSpotAngle:
                    break;
                case Wire.CoreCommandKind.LightSetShadows:
                    break;
                case Wire.CoreCommandKind.ImageSetTexture:
                    break;
                case Wire.CoreCommandKind.ImageSetSize:
                    break;
                case Wire.CoreCommandKind.ImageSetFit:
                    break;
                case Wire.CoreCommandKind.ImageSetTint:
                    break;
                case Wire.CoreCommandKind.ImageTweenTint:
                    break;
                case Wire.CoreCommandKind.ImageSetOpacity:
                    break;
                case Wire.CoreCommandKind.ImageTweenOpacity:
                    break;
                case Wire.CoreCommandKind.ImageSetFaceCamera:
                    break;
                case Wire.CoreCommandKind.TextSetFont:
                    break;
                case Wire.CoreCommandKind.TextSetSize:
                    break;
                case Wire.CoreCommandKind.TextTweenSize:
                    break;
                case Wire.CoreCommandKind.TextSetColor:
                    break;
                case Wire.CoreCommandKind.TextTweenColor:
                    break;
                case Wire.CoreCommandKind.TextSetAlignment:
                    break;
                case Wire.CoreCommandKind.TextSetWrapping:
                    break;
                case Wire.CoreCommandKind.TextSetRichText:
                    break;
                case Wire.CoreCommandKind.TextSetFaceCamera:
                    break;
                case Wire.CoreCommandKind.AnimatorPlay:
                    break;
                case Wire.CoreCommandKind.AnimatorCrossFade:
                    break;
                case Wire.CoreCommandKind.AnimatorSetBool:
                    break;
                case Wire.CoreCommandKind.AnimatorSetInt:
                    break;
                case Wire.CoreCommandKind.AnimatorSetFloat:
                    break;
                case Wire.CoreCommandKind.AnimatorSetTrigger:
                    break;
                case Wire.CoreCommandKind.AnimatorSetSpeed:
                    break;
                default:
                    return false;
            }
            return false;
        }

        private static void ValidateDirectMotionValue(Wire.MotionValueOperation value)
        {
            _ = ReadUuid(value.ValueId, "motion value");
            switch (value.Command)
            {
                case Wire.MotionValueCommandKind.Set:
                case Wire.MotionValueCommandKind.Jump:
                    if (value.ValueType == Wire.MotionValue.NONE)
                        throw new InvalidDataException("A motion value assignment is absent.");
                    break;
                case Wire.MotionValueCommandKind.Stop:
                    if (value.ValueType != Wire.MotionValue.NONE)
                        throw new InvalidDataException("A stopped motion value carries a value.");
                    break;
                case Wire.MotionValueCommandKind.Animate:
                    _ = ReadUuid(value.PlaybackId, "motion value playback");
                    if (value.ValueType == Wire.MotionValue.NONE || !value.Transition.HasValue)
                        throw new InvalidDataException("An animated motion value is incomplete.");
                    break;
                default:
                    throw new InvalidDataException("A motion value command is unknown.");
            }
        }

        private static void ValidateDirectPlayback(Wire.MotionPlaybackCommand? optional)
        {
            Wire.MotionPlaybackCommand value =
                optional ?? throw new InvalidDataException("A motion playback command is absent.");
            if (!Known(value.Kind, Wire.MotionPlaybackCommandKind.SetDirection))
                throw new InvalidDataException("A motion playback command is unknown.");
            if (value.Kind == Wire.MotionPlaybackCommandKind.SetSpeed && !Finite(value.Speed))
                throw new InvalidDataException("A motion playback speed must be finite.");
            if (
                value.Kind == Wire.MotionPlaybackCommandKind.SetDirection
                && !Known(value.Direction, Wire.MotionPlaybackDirection.AlternateReverse)
            )
                throw new InvalidDataException("A motion playback direction is unknown.");
        }

        private static void ValidateDirectMotionControl(Wire.MotionControlOperation value)
        {
            _ = ReadUuid(value.ControlId, "motion control");
            bool needsTarget =
                value.Command
                is Wire.MotionControlCommandKind.Start
                    or Wire.MotionControlCommandKind.Set;
            if (!Known(value.Command, Wire.MotionControlCommandKind.Clear))
                throw new InvalidDataException("A motion control command is unknown.");
            if (needsTarget != value.Target.HasValue)
                throw new InvalidDataException("A motion control target is noncanonical.");
            if (value.Command == Wire.MotionControlCommandKind.Start)
                _ = ReadUuid(value.PlaybackId, "motion control playback");
            if (!needsTarget)
                return;
            Wire.MotionControlTarget target = value.Target!.Value;
            bool canonical = target.Kind switch
            {
                Wire.MotionControlTargetKind.Target => target.Target.HasValue
                    && target.Variant is null,
                Wire.MotionControlTargetKind.Variant => !target.Target.HasValue
                    && !string.IsNullOrEmpty(target.Variant),
                _ => false,
            };
            if (!canonical)
                throw new InvalidDataException("A motion control target is invalid.");
        }

        private static void ValidateDirectMotionScope(Wire.MotionScopeOperation value)
        {
            _ = ReadUuid(value.ScopeId, "motion scope");
            switch (value.Command)
            {
                case Wire.MotionScopeCommandKind.Start:
                    _ = ReadUuid(value.PlaybackId, "motion scope playback");
                    if (value.Selector.HasValue || value.Target.HasValue)
                        throw new InvalidDataException(
                            "A motion scope start carries scalar fields."
                        );
                    for (int index = 0; index < value.StepsLength; index++)
                    {
                        Wire.MotionSequenceStep step = value.Steps(index)!.Value;
                        ValidateDirectMotionSelector(step.Selector);
                        if (!step.Target.HasValue)
                            throw new InvalidDataException("A motion scope step target is absent.");
                    }
                    break;
                case Wire.MotionScopeCommandKind.Set:
                    ValidateDirectMotionSelector(value.Selector);
                    if (!value.Target.HasValue || value.StepsLength != 0)
                        throw new InvalidDataException("A motion scope set is incomplete.");
                    break;
                case Wire.MotionScopeCommandKind.Stop:
                    ValidateDirectMotionSelector(value.Selector);
                    if (value.Target.HasValue || value.StepsLength != 0)
                        throw new InvalidDataException("A motion scope stop is noncanonical.");
                    break;
                default:
                    throw new InvalidDataException("A motion scope command is unknown.");
            }
        }

        private static void ValidateDirectMotionSelector(Wire.MotionSelector? optional)
        {
            Wire.MotionSelector value =
                optional ?? throw new InvalidDataException("A motion selector is absent.");
            if (!Known(value.Kind, Wire.MotionSelectorKind.Descendants))
                throw new InvalidDataException("A motion selector kind is unknown.");
            if (value.Kind == Wire.MotionSelectorKind.Element)
                _ = ReadUuid(value.ObjectId, "motion selector element");
            if (value.Kind == Wire.MotionSelectorKind.Name && string.IsNullOrEmpty(value.Name))
                throw new InvalidDataException("A named motion selector is empty.");
        }

        private static void ValidateDirectAssetSet(Wire.ReplaceAssetSetPayload value)
        {
            const int MaximumAssets = 16_384;
            if (value.AssetsLength > MaximumAssets)
                throw new InvalidDataException("The prepared-asset limit was exceeded.");
            var addresses = new HashSet<string>(StringComparer.Ordinal);
            for (int index = 0; index < value.AssetsLength; index++)
            {
                Wire.PreparedAsset asset =
                    value.Assets(index)
                    ?? throw new InvalidDataException("A prepared asset is absent.");
                if (!Enum.IsDefined(typeof(Wire.PreparedAssetKind), asset.Kind))
                    throw new InvalidDataException("A prepared asset kind is unknown.");
                RequireSnapshotString(asset.Address, "prepared asset address", allowEmpty: false);
                if (!addresses.Add(asset.Address))
                    throw new InvalidDataException("A prepared asset address is repeated.");
            }
        }

        private static void ValidateDirectGeometry(Wire.GeometryObservationUpdate value)
        {
            var ids = new HashSet<Guid>();
            for (int index = 0; index < value.RemovedLength; index++)
                if (!ids.Add(ReadUuid(value.Removed(index), "removed geometry observation")))
                    throw new InvalidDataException("A removed geometry observation is repeated.");
            for (int index = 0; index < value.AddedLength; index++)
            {
                Wire.GeometryObservation observation =
                    value.Added(index)
                    ?? throw new InvalidDataException("A geometry observation is absent.");
                if (!ids.Add(ReadUuid(observation.ObservationId, "geometry observation")))
                    throw new InvalidDataException("A geometry observation UUID is repeated.");
                Wire.GeometryObservationTarget target =
                    observation.Target
                    ?? throw new InvalidDataException("A geometry target is absent.");
                switch (target.Kind)
                {
                    case Wire.GeometryTargetKind.UiElement:
                        _ = ReadUuid(target.ObjectId, "geometry UI element");
                        break;
                    case Wire.GeometryTargetKind.Viewport:
                        break;
                    case Wire.GeometryTargetKind.WorldOrigin:
                    case Wire.GeometryTargetKind.WorldRenderedBounds:
                        _ = ReadUuid(target.ObjectId, "geometry world object");
                        ValidateDirectGeometryCamera(target);
                        break;
                    case Wire.GeometryTargetKind.WorldAnchor:
                        _ = ReadUuid(target.ObjectId, "geometry world object");
                        RequireSnapshotString(target.Anchor, "geometry anchor", allowEmpty: false);
                        ValidateDirectGeometryCamera(target);
                        break;
                    default:
                        throw new InvalidDataException("A geometry target kind is unknown.");
                }
            }
        }

        private static void ValidateDirectGeometryCamera(Wire.GeometryObservationTarget value)
        {
            if (value.CameraKind == Wire.CameraTargetKind.Input && !value.CameraObjectId.HasValue)
                return;
            if (value.CameraKind == Wire.CameraTargetKind.Object && value.CameraObjectId.HasValue)
            {
                _ = ReadUuid(value.CameraObjectId, "geometry camera");
                return;
            }
            throw new InvalidDataException("Geometry camera kind and UUID do not match.");
        }

        private static void ValidateDirectAccessibility(Wire.AccessibilityUpdate value)
        {
            for (int index = 0; index < value.AnnouncementsLength; index++)
                _ =
                    value.Announcements(index)
                    ?? throw new InvalidDataException("An accessibility announcement is absent.");
            if (!value.Snapshot.HasValue)
                return;
            Wire.AccessibilitySnapshot snapshot = value.Snapshot.Value;
            if (snapshot.CommitSequence == 0)
                throw new InvalidDataException("An accessibility commit sequence must be nonzero.");
            var nodes = new HashSet<Guid>();
            for (int index = 0; index < snapshot.NodesLength; index++)
            {
                Wire.AccessibilityNodeSnapshot node =
                    snapshot.Nodes(index)
                    ?? throw new InvalidDataException("An accessibility node is absent.");
                if (!nodes.Add(ReadUuid(node.ObjectId, "accessibility node")))
                    throw new InvalidDataException("An accessibility node UUID is repeated.");
                if (node.ParentId.HasValue)
                    _ = ReadUuid(node.ParentId, "accessibility parent");
                for (int child = 0; child < node.ChildrenLength; child++)
                    _ = ReadUuid(node.Children(child), "accessibility child");
                if (!Known(node.Role, Wire.SemanticRole.Region))
                    throw new InvalidDataException("An accessibility semantic role is unknown.");
                Wire.SemanticState state =
                    node.State
                    ?? throw new InvalidDataException("An accessibility semantic state is absent.");
                if (state.Checked.HasValue && !Known(state.Checked.Value, Wire.CheckedState.Mixed))
                    throw new InvalidDataException("An accessibility checked state is unknown.");
                if (state.Popup.HasValue && !Known(state.Popup.Value, Wire.PopupKind.ListBox))
                    throw new InvalidDataException("An accessibility popup kind is unknown.");
                if (state.Current.HasValue && !Known(state.Current.Value, Wire.CurrentPage.Page))
                    throw new InvalidDataException(
                        "An accessibility current-page kind is unknown."
                    );
                Wire.AccessibilityActionSet actions =
                    node.Actions
                    ?? throw new InvalidDataException("Accessibility actions are absent.");
                for (int action = 0; action < actions.ScrollLength; action++)
                    if (!Known(actions.Scroll(action), Wire.AccessibilityScrollDirection.Backward))
                        throw new InvalidDataException(
                            "An accessibility scroll action is unknown."
                        );
                if (
                    node.ScrollAxis.HasValue
                    && !Known(node.ScrollAxis.Value, Wire.AccessibilityScrollAxis.Vertical)
                )
                    throw new InvalidDataException("An accessibility scroll axis is unknown.");
                if (node.Value is Wire.AccessibilityRangeValue range)
                {
                    RequireFinite(range.Current);
                    RequireFinite(range.Minimum);
                    RequireFinite(range.Maximum);
                }
            }
            for (int index = 0; index < snapshot.RootsLength; index++)
                _ = ReadUuid(snapshot.Roots(index), "accessibility root");
        }

        private static bool ValidateDirectAnimatorCommand(Wire.CoreCommand value)
        {
            const ulong MaximumMilliseconds = 922_337_203_685;
            switch (value.Kind)
            {
                case Wire.CoreCommandKind.AnimatorPlay:
                {
                    Wire.AnimatorPlayPayload payload = value.PayloadAsAnimatorPlayPayload();
                    ValidateDirectAnimatorState(
                        payload.ObjectId,
                        payload.State,
                        payload.NormalizedStartTime
                    );
                    if (payload.WaitMs > MaximumMilliseconds)
                        throw new InvalidDataException(
                            "Animator wait exceeds the supported range."
                        );
                    return true;
                }
                case Wire.CoreCommandKind.AnimatorCrossFade:
                {
                    Wire.AnimatorCrossFadePayload payload =
                        value.PayloadAsAnimatorCrossFadePayload();
                    ValidateDirectAnimatorState(
                        payload.ObjectId,
                        payload.State,
                        payload.NormalizedStartTime
                    );
                    if (payload.WaitMs > MaximumMilliseconds || payload.CrossFadeMs == 0)
                        throw new InvalidDataException(
                            "Animator timing is outside the supported range."
                        );
                    if (payload.CrossFadeMs > MaximumMilliseconds)
                        throw new InvalidDataException(
                            "Animator cross-fade exceeds the supported range."
                        );
                    return true;
                }
                case Wire.CoreCommandKind.AnimatorSetBool:
                {
                    Wire.AnimatorBoolPayload payload = value.PayloadAsAnimatorBoolPayload();
                    ValidateDirectAnimatorParameter(payload.ObjectId, payload.Parameter);
                    return true;
                }
                case Wire.CoreCommandKind.AnimatorSetInt:
                {
                    Wire.AnimatorIntPayload payload = value.PayloadAsAnimatorIntPayload();
                    ValidateDirectAnimatorParameter(payload.ObjectId, payload.Parameter);
                    return true;
                }
                case Wire.CoreCommandKind.AnimatorSetFloat:
                {
                    Wire.AnimatorFloatPayload payload = value.PayloadAsAnimatorFloatPayload();
                    ValidateDirectAnimatorParameter(payload.ObjectId, payload.Parameter);
                    RequireFinite(payload.Value);
                    return true;
                }
                case Wire.CoreCommandKind.AnimatorSetTrigger:
                {
                    Wire.AnimatorParameterPayload payload =
                        value.PayloadAsAnimatorParameterPayload();
                    ValidateDirectAnimatorParameter(payload.ObjectId, payload.Parameter);
                    return true;
                }
                case Wire.CoreCommandKind.AnimatorSetSpeed:
                {
                    Wire.AnimatorSpeedPayload payload = value.PayloadAsAnimatorSpeedPayload();
                    _ = ReadUuid(payload.ObjectId, "animator object");
                    RequireNonnegative(payload.Speed, "Animator speed");
                    return true;
                }

                case Wire.CoreCommandKind.ApplicationOpenUrl:
                    break;
                case Wire.CoreCommandKind.Diagnostics:
                    break;
                case Wire.CoreCommandKind.AssetsReplaceSet:
                    break;
                case Wire.CoreCommandKind.SceneLoad:
                    break;
                case Wire.CoreCommandKind.SceneUnload:
                    break;
                case Wire.CoreCommandKind.SceneSetPrimary:
                    break;
                case Wire.CoreCommandKind.ObjectCreate:
                    break;
                case Wire.CoreCommandKind.ObjectDestroy:
                    break;
                case Wire.CoreCommandKind.ObjectSetActive:
                    break;
                case Wire.CoreCommandKind.ObjectReparent:
                    break;
                case Wire.CoreCommandKind.TransformSetLocalPosition:
                    break;
                case Wire.CoreCommandKind.TransformSetWorldPosition:
                    break;
                case Wire.CoreCommandKind.TransformTweenLocalPosition:
                    break;
                case Wire.CoreCommandKind.TransformTweenWorldPosition:
                    break;
                case Wire.CoreCommandKind.TransformSetLocalRotation:
                    break;
                case Wire.CoreCommandKind.TransformSetWorldRotation:
                    break;
                case Wire.CoreCommandKind.TransformTweenLocalRotation:
                    break;
                case Wire.CoreCommandKind.TransformTweenWorldRotation:
                    break;
                case Wire.CoreCommandKind.TransformSetLocalScale:
                    break;
                case Wire.CoreCommandKind.TransformTweenLocalScale:
                    break;
                case Wire.CoreCommandKind.RendererSetMaterial:
                    break;
                case Wire.CoreCommandKind.CameraSetEnabled:
                    break;
                case Wire.CoreCommandKind.CameraSetPerspective:
                    break;
                case Wire.CoreCommandKind.CameraTweenFieldOfView:
                    break;
                case Wire.CoreCommandKind.CameraSetOrthographic:
                    break;
                case Wire.CoreCommandKind.CameraTweenOrthographicSize:
                    break;
                case Wire.CoreCommandKind.CameraSetClipping:
                    break;
                case Wire.CoreCommandKind.CameraSetClear:
                    break;
                case Wire.CoreCommandKind.LightSetEnabled:
                    break;
                case Wire.CoreCommandKind.LightSetType:
                    break;
                case Wire.CoreCommandKind.LightSetColor:
                    break;
                case Wire.CoreCommandKind.LightTweenColor:
                    break;
                case Wire.CoreCommandKind.LightSetIntensity:
                    break;
                case Wire.CoreCommandKind.LightTweenIntensity:
                    break;
                case Wire.CoreCommandKind.LightSetRange:
                    break;
                case Wire.CoreCommandKind.LightSetSpotAngle:
                    break;
                case Wire.CoreCommandKind.LightSetShadows:
                    break;
                case Wire.CoreCommandKind.ImageSetTexture:
                    break;
                case Wire.CoreCommandKind.ImageSetSize:
                    break;
                case Wire.CoreCommandKind.ImageSetFit:
                    break;
                case Wire.CoreCommandKind.ImageSetTint:
                    break;
                case Wire.CoreCommandKind.ImageTweenTint:
                    break;
                case Wire.CoreCommandKind.ImageSetOpacity:
                    break;
                case Wire.CoreCommandKind.ImageTweenOpacity:
                    break;
                case Wire.CoreCommandKind.ImageSetFaceCamera:
                    break;
                case Wire.CoreCommandKind.TextSetContent:
                    break;
                case Wire.CoreCommandKind.TextSetFont:
                    break;
                case Wire.CoreCommandKind.TextSetSize:
                    break;
                case Wire.CoreCommandKind.TextTweenSize:
                    break;
                case Wire.CoreCommandKind.TextSetColor:
                    break;
                case Wire.CoreCommandKind.TextTweenColor:
                    break;
                case Wire.CoreCommandKind.TextSetAlignment:
                    break;
                case Wire.CoreCommandKind.TextSetWrapping:
                    break;
                case Wire.CoreCommandKind.TextSetRichText:
                    break;
                case Wire.CoreCommandKind.TextSetFaceCamera:
                    break;
                case Wire.CoreCommandKind.ParticlePlay:
                    break;
                case Wire.CoreCommandKind.ParticleStop:
                    break;
                case Wire.CoreCommandKind.ParticleSpawn:
                    break;
                case Wire.CoreCommandKind.AudioPlay:
                    break;
                case Wire.CoreCommandKind.AudioStop:
                    break;
                case Wire.CoreCommandKind.AudioPause:
                    break;
                case Wire.CoreCommandKind.AudioResume:
                    break;
                case Wire.CoreCommandKind.AudioSeek:
                    break;
                case Wire.CoreCommandKind.AudioSetBuffering:
                    break;
                case Wire.CoreCommandKind.AudioReplace:
                    break;
                case Wire.CoreCommandKind.AudioSetVolume:
                    break;
                case Wire.CoreCommandKind.AudioTweenVolume:
                    break;
                case Wire.CoreCommandKind.TimeWait:
                    break;
                case Wire.CoreCommandKind.OperationCancel:
                    break;
                case Wire.CoreCommandKind.InputSetEnabled:
                    break;
                case Wire.CoreCommandKind.InputSetCamera:
                    break;
                case Wire.CoreCommandKind.InputSetPointerEvents:
                    break;
                case Wire.CoreCommandKind.InputSetGlobalKeys:
                    break;
                case Wire.CoreCommandKind.InputSetController:
                    break;
                case Wire.CoreCommandKind.ControllerVibrate:
                    break;
                case Wire.CoreCommandKind.DebugUi:
                    break;
                case Wire.CoreCommandKind.VisualElementCreate:
                    break;
                case Wire.CoreCommandKind.VisualElementUpdate:
                    break;
                case Wire.CoreCommandKind.VisualElementDestroy:
                    break;
                case Wire.CoreCommandKind.VisualElementPerformAction:
                    break;
                case Wire.CoreCommandKind.MotionValue:
                    break;
                case Wire.CoreCommandKind.MotionValuePlayback:
                    break;
                case Wire.CoreCommandKind.MotionPlayback:
                    break;
                case Wire.CoreCommandKind.MotionControlledClock:
                    break;
                case Wire.CoreCommandKind.MotionControl:
                    break;
                case Wire.CoreCommandKind.MotionScope:
                    break;
                case Wire.CoreCommandKind.MotionDragControl:
                    break;
                case Wire.CoreCommandKind.GeometryObservationUpdate:
                    break;
                case Wire.CoreCommandKind.AccessibilityUpdate:
                    break;
                default:
                    return false;
            }
            return false;
        }

        private static void ValidateDirectAnimatorState(
            Wire.Uuid? objectId,
            string state,
            double normalizedStartTime
        )
        {
            _ = ReadUuid(objectId, "animator object");
            RequireSnapshotString(state, "animator state", allowEmpty: false);
            RequireUnit(normalizedStartTime, "Animator normalized start time");
        }

        private static void ValidateDirectAnimatorParameter(Wire.Uuid? objectId, string parameter)
        {
            _ = ReadUuid(objectId, "animator object");
            RequireSnapshotString(parameter, "animator parameter", allowEmpty: false);
        }

        private static bool ValidateDirectComponentCommand(Wire.CoreCommand value)
        {
            switch (value.Kind)
            {
                case Wire.CoreCommandKind.CameraSetEnabled:
                case Wire.CoreCommandKind.LightSetEnabled:
                case Wire.CoreCommandKind.ImageSetFaceCamera:
                case Wire.CoreCommandKind.TextSetRichText:
                case Wire.CoreCommandKind.TextSetFaceCamera:
                    _ = ReadUuid(
                        value.PayloadAsObjectEnabledPayload().ObjectId,
                        "component object"
                    );
                    return true;
                case Wire.CoreCommandKind.CameraSetPerspective:
                {
                    Wire.PerspectivePayload payload = value.PayloadAsPerspectivePayload();
                    _ = ReadUuid(payload.ObjectId, "camera object");
                    RequireConflict(payload.OnConflict);
                    RequireFinite(payload.FieldOfView);
                    return true;
                }
                case Wire.CoreCommandKind.CameraTweenFieldOfView:
                {
                    Wire.TweenFieldOfViewPayload payload = value.PayloadAsTweenFieldOfViewPayload();
                    _ = ReadUuid(payload.ObjectId, "camera object");
                    RequireConflict(payload.OnConflict);
                    RequireFinite(payload.FieldOfView);
                    ValidateDirectTween(payload.Tween!.Value, value.Blocking);
                    return true;
                }
                case Wire.CoreCommandKind.CameraSetOrthographic:
                {
                    Wire.OrthographicPayload payload = value.PayloadAsOrthographicPayload();
                    _ = ReadUuid(payload.ObjectId, "camera object");
                    RequireConflict(payload.OnConflict);
                    RequireFinite(payload.Size);
                    return true;
                }
                case Wire.CoreCommandKind.CameraTweenOrthographicSize:
                {
                    Wire.TweenOrthographicSizePayload payload =
                        value.PayloadAsTweenOrthographicSizePayload();
                    _ = ReadUuid(payload.ObjectId, "camera object");
                    RequireConflict(payload.OnConflict);
                    RequireFinite(payload.Size);
                    ValidateDirectTween(payload.Tween!.Value, value.Blocking);
                    return true;
                }
                case Wire.CoreCommandKind.CameraSetClipping:
                {
                    Wire.CameraClippingPayload payload = value.PayloadAsCameraClippingPayload();
                    _ = ReadUuid(payload.ObjectId, "camera object");
                    RequireFinite(payload.Near);
                    RequireFinite(payload.Far);
                    if (payload.Far <= payload.Near)
                        throw new InvalidDataException(
                            "Camera far clipping must be greater than near clipping."
                        );
                    return true;
                }
                case Wire.CoreCommandKind.CameraSetClear:
                {
                    Wire.CameraClearPayload payload = value.PayloadAsCameraClearPayload();
                    _ = ReadUuid(payload.ObjectId, "camera object");
                    if (!Known(payload.ClearMode, Wire.CameraClearMode.Nothing))
                        throw new InvalidDataException("A camera clear mode is unknown.");
                    bool needsColor = payload.ClearMode == Wire.CameraClearMode.SolidColor;
                    if (needsColor != payload.ClearColor.HasValue)
                        throw new InvalidDataException(
                            "Camera clear color presence does not match its clear mode."
                        );
                    if (payload.ClearColor is Wire.RgbaColor color)
                        ValidateDirectColor(color, "Camera clear");
                    return true;
                }
                case Wire.CoreCommandKind.LightSetType:
                {
                    Wire.LightTypePayload payload = value.PayloadAsLightTypePayload();
                    _ = ReadUuid(payload.ObjectId, "light object");
                    if (!Known(payload.LightType, Wire.LightType.Spot))
                        throw new InvalidDataException("A light type is unknown.");
                    return true;
                }
                case Wire.CoreCommandKind.LightSetColor:
                case Wire.CoreCommandKind.TextSetColor:
                    ValidateDirectColorPayload(value.PayloadAsColorPayload());
                    return true;
                case Wire.CoreCommandKind.LightTweenColor:
                case Wire.CoreCommandKind.TextTweenColor:
                {
                    Wire.TweenColorPayload payload = value.PayloadAsTweenColorPayload();
                    _ = ReadUuid(payload.ObjectId, "color object");
                    RequireConflict(payload.OnConflict);
                    ValidateDirectColor(payload.Color!.Value, "Component color");
                    ValidateDirectTween(payload.Tween!.Value, value.Blocking);
                    return true;
                }
                case Wire.CoreCommandKind.LightSetIntensity:
                {
                    Wire.IntensityPayload payload = value.PayloadAsIntensityPayload();
                    _ = ReadUuid(payload.ObjectId, "light object");
                    RequireConflict(payload.OnConflict);
                    RequireNonnegative(payload.Intensity, "Light intensity");
                    return true;
                }
                case Wire.CoreCommandKind.LightTweenIntensity:
                {
                    Wire.TweenIntensityPayload payload = value.PayloadAsTweenIntensityPayload();
                    _ = ReadUuid(payload.ObjectId, "light object");
                    RequireConflict(payload.OnConflict);
                    RequireNonnegative(payload.Intensity, "Light intensity");
                    ValidateDirectTween(payload.Tween!.Value, value.Blocking);
                    return true;
                }
                case Wire.CoreCommandKind.LightSetRange:
                {
                    Wire.LightRangePayload payload = value.PayloadAsLightRangePayload();
                    _ = ReadUuid(payload.ObjectId, "light object");
                    RequirePositive(payload.Range, "Light range");
                    return true;
                }
                case Wire.CoreCommandKind.LightSetSpotAngle:
                {
                    Wire.SpotAnglePayload payload = value.PayloadAsSpotAnglePayload();
                    _ = ReadUuid(payload.ObjectId, "light object");
                    RequireNonnegative(payload.InnerSpotAngle, "Light inner spot angle");
                    RequireFinite(payload.OuterSpotAngle);
                    if (payload.InnerSpotAngle > payload.OuterSpotAngle)
                        throw new InvalidDataException(
                            "A spot light's inner angle cannot exceed its outer angle."
                        );
                    return true;
                }
                case Wire.CoreCommandKind.LightSetShadows:
                {
                    Wire.LightShadowsPayload payload = value.PayloadAsLightShadowsPayload();
                    _ = ReadUuid(payload.ObjectId, "light object");
                    if (!Known(payload.Shadows, Wire.ShadowMode.Soft))
                        throw new InvalidDataException("A light shadow mode is unknown.");
                    return true;
                }
                case Wire.CoreCommandKind.ImageSetTexture:
                    ValidateDirectAsset(
                        value.PayloadAsSetTexturePayload().ObjectId,
                        value.PayloadAsSetTexturePayload().Address,
                        "image"
                    );
                    return true;
                case Wire.CoreCommandKind.TextSetFont:
                    ValidateDirectAsset(
                        value.PayloadAsSetFontPayload().ObjectId,
                        value.PayloadAsSetFontPayload().Address,
                        "text"
                    );
                    return true;
                case Wire.CoreCommandKind.ImageSetSize:
                {
                    Wire.ImageSizePayload payload = value.PayloadAsImageSizePayload();
                    _ = ReadUuid(payload.ObjectId, "image object");
                    RequirePositive(payload.Width, "Image width");
                    RequirePositive(payload.Height, "Image height");
                    return true;
                }
                case Wire.CoreCommandKind.ImageSetFit:
                {
                    Wire.ImageFitPayload payload = value.PayloadAsImageFitPayload();
                    _ = ReadUuid(payload.ObjectId, "image object");
                    if (!Known(payload.Fit, Wire.ImageFit.Cover))
                        throw new InvalidDataException("An image fit value is unknown.");
                    return true;
                }
                case Wire.CoreCommandKind.ImageSetTint:
                    ValidateDirectTint(value.PayloadAsTintPayload());
                    return true;
                case Wire.CoreCommandKind.ImageTweenTint:
                {
                    Wire.TweenTintPayload payload = value.PayloadAsTweenTintPayload();
                    ValidateDirectTint(payload.ObjectId, payload.Tint!.Value, payload.OnConflict);
                    ValidateDirectTween(payload.Tween!.Value, value.Blocking);
                    return true;
                }
                case Wire.CoreCommandKind.ImageSetOpacity:
                    ValidateDirectOpacity(value.PayloadAsOpacityPayload());
                    return true;
                case Wire.CoreCommandKind.ImageTweenOpacity:
                {
                    Wire.TweenOpacityPayload payload = value.PayloadAsTweenOpacityPayload();
                    ValidateDirectOpacity(payload.ObjectId, payload.Opacity, payload.OnConflict);
                    ValidateDirectTween(payload.Tween!.Value, value.Blocking);
                    return true;
                }
                case Wire.CoreCommandKind.TextSetSize:
                    ValidateDirectTextSize(value.PayloadAsTextSizePayload());
                    return true;
                case Wire.CoreCommandKind.TextTweenSize:
                {
                    Wire.TweenTextSizePayload payload = value.PayloadAsTweenTextSizePayload();
                    ValidateDirectTextSize(payload.ObjectId, payload.Size, payload.OnConflict);
                    ValidateDirectTween(payload.Tween!.Value, value.Blocking);
                    return true;
                }
                case Wire.CoreCommandKind.TextSetAlignment:
                {
                    Wire.TextAlignmentPayload payload = value.PayloadAsTextAlignmentPayload();
                    _ = ReadUuid(payload.ObjectId, "text object");
                    if (!Known(payload.Horizontal, Wire.HorizontalAlignment.Justified))
                        throw new InvalidDataException("A text horizontal alignment is unknown.");
                    if (!Known(payload.Vertical, Wire.VerticalAlignment.Bottom))
                        throw new InvalidDataException("A text vertical alignment is unknown.");
                    return true;
                }
                case Wire.CoreCommandKind.TextSetWrapping:
                {
                    Wire.TextWrappingPayload payload = value.PayloadAsTextWrappingPayload();
                    _ = ReadUuid(payload.ObjectId, "text object");
                    if (payload.WrapWidth is double width)
                        RequirePositive(width, "Text wrap width");
                    return true;
                }

                case Wire.CoreCommandKind.ApplicationOpenUrl:
                    break;
                case Wire.CoreCommandKind.Diagnostics:
                    break;
                case Wire.CoreCommandKind.AssetsReplaceSet:
                    break;
                case Wire.CoreCommandKind.SceneLoad:
                    break;
                case Wire.CoreCommandKind.SceneUnload:
                    break;
                case Wire.CoreCommandKind.SceneSetPrimary:
                    break;
                case Wire.CoreCommandKind.ObjectCreate:
                    break;
                case Wire.CoreCommandKind.ObjectDestroy:
                    break;
                case Wire.CoreCommandKind.ObjectSetActive:
                    break;
                case Wire.CoreCommandKind.ObjectReparent:
                    break;
                case Wire.CoreCommandKind.TransformSetLocalPosition:
                    break;
                case Wire.CoreCommandKind.TransformSetWorldPosition:
                    break;
                case Wire.CoreCommandKind.TransformTweenLocalPosition:
                    break;
                case Wire.CoreCommandKind.TransformTweenWorldPosition:
                    break;
                case Wire.CoreCommandKind.TransformSetLocalRotation:
                    break;
                case Wire.CoreCommandKind.TransformSetWorldRotation:
                    break;
                case Wire.CoreCommandKind.TransformTweenLocalRotation:
                    break;
                case Wire.CoreCommandKind.TransformTweenWorldRotation:
                    break;
                case Wire.CoreCommandKind.TransformSetLocalScale:
                    break;
                case Wire.CoreCommandKind.TransformTweenLocalScale:
                    break;
                case Wire.CoreCommandKind.RendererSetMaterial:
                    break;
                case Wire.CoreCommandKind.TextSetContent:
                    break;
                case Wire.CoreCommandKind.AnimatorPlay:
                    break;
                case Wire.CoreCommandKind.AnimatorCrossFade:
                    break;
                case Wire.CoreCommandKind.AnimatorSetBool:
                    break;
                case Wire.CoreCommandKind.AnimatorSetInt:
                    break;
                case Wire.CoreCommandKind.AnimatorSetFloat:
                    break;
                case Wire.CoreCommandKind.AnimatorSetTrigger:
                    break;
                case Wire.CoreCommandKind.AnimatorSetSpeed:
                    break;
                case Wire.CoreCommandKind.ParticlePlay:
                    break;
                case Wire.CoreCommandKind.ParticleStop:
                    break;
                case Wire.CoreCommandKind.ParticleSpawn:
                    break;
                case Wire.CoreCommandKind.AudioPlay:
                    break;
                case Wire.CoreCommandKind.AudioStop:
                    break;
                case Wire.CoreCommandKind.AudioPause:
                    break;
                case Wire.CoreCommandKind.AudioResume:
                    break;
                case Wire.CoreCommandKind.AudioSeek:
                    break;
                case Wire.CoreCommandKind.AudioSetBuffering:
                    break;
                case Wire.CoreCommandKind.AudioReplace:
                    break;
                case Wire.CoreCommandKind.AudioSetVolume:
                    break;
                case Wire.CoreCommandKind.AudioTweenVolume:
                    break;
                case Wire.CoreCommandKind.TimeWait:
                    break;
                case Wire.CoreCommandKind.OperationCancel:
                    break;
                case Wire.CoreCommandKind.InputSetEnabled:
                    break;
                case Wire.CoreCommandKind.InputSetCamera:
                    break;
                case Wire.CoreCommandKind.InputSetPointerEvents:
                    break;
                case Wire.CoreCommandKind.InputSetGlobalKeys:
                    break;
                case Wire.CoreCommandKind.InputSetController:
                    break;
                case Wire.CoreCommandKind.ControllerVibrate:
                    break;
                case Wire.CoreCommandKind.DebugUi:
                    break;
                case Wire.CoreCommandKind.VisualElementCreate:
                    break;
                case Wire.CoreCommandKind.VisualElementUpdate:
                    break;
                case Wire.CoreCommandKind.VisualElementDestroy:
                    break;
                case Wire.CoreCommandKind.VisualElementPerformAction:
                    break;
                case Wire.CoreCommandKind.MotionValue:
                    break;
                case Wire.CoreCommandKind.MotionValuePlayback:
                    break;
                case Wire.CoreCommandKind.MotionPlayback:
                    break;
                case Wire.CoreCommandKind.MotionControlledClock:
                    break;
                case Wire.CoreCommandKind.MotionControl:
                    break;
                case Wire.CoreCommandKind.MotionScope:
                    break;
                case Wire.CoreCommandKind.MotionDragControl:
                    break;
                case Wire.CoreCommandKind.GeometryObservationUpdate:
                    break;
                case Wire.CoreCommandKind.AccessibilityUpdate:
                    break;
                default:
                    return false;
            }
            return false;
        }

        private static void ValidateDirectColorPayload(Wire.ColorPayload payload)
        {
            _ = ReadUuid(payload.ObjectId, "color object");
            RequireConflict(payload.OnConflict);
            ValidateDirectColor(payload.Color!.Value, "Component color");
        }

        private static void ValidateDirectColor(Wire.RgbaColor value, string name)
        {
            RequireUnit(value.R, $"{name} red");
            RequireUnit(value.G, $"{name} green");
            RequireUnit(value.B, $"{name} blue");
            RequireUnit(value.A, $"{name} alpha");
        }

        private static void ValidateDirectAsset(Wire.Uuid? objectId, string address, string kind)
        {
            _ = ReadUuid(objectId, $"{kind} object");
            RequireSnapshotString(address, $"{kind} asset address", allowEmpty: false);
        }

        private static void ValidateDirectTint(Wire.TintPayload payload) =>
            ValidateDirectTint(payload.ObjectId, payload.Tint!.Value, payload.OnConflict);

        private static void ValidateDirectTint(
            Wire.Uuid? objectId,
            Wire.RgbColor value,
            Wire.ConflictPolicy conflict
        )
        {
            _ = ReadUuid(objectId, "image object");
            RequireConflict(conflict);
            RequireUnit(value.R, "Image tint red");
            RequireUnit(value.G, "Image tint green");
            RequireUnit(value.B, "Image tint blue");
        }

        private static void ValidateDirectOpacity(Wire.OpacityPayload payload) =>
            ValidateDirectOpacity(payload.ObjectId, payload.Opacity, payload.OnConflict);

        private static void ValidateDirectOpacity(
            Wire.Uuid? objectId,
            double value,
            Wire.ConflictPolicy conflict
        )
        {
            _ = ReadUuid(objectId, "image object");
            RequireConflict(conflict);
            RequireUnit(value, "Image opacity");
        }

        private static void ValidateDirectTextSize(Wire.TextSizePayload payload) =>
            ValidateDirectTextSize(payload.ObjectId, payload.Size, payload.OnConflict);

        private static void ValidateDirectTextSize(
            Wire.Uuid? objectId,
            double value,
            Wire.ConflictPolicy conflict
        )
        {
            _ = ReadUuid(objectId, "text object");
            RequireConflict(conflict);
            RequirePositive(value, "Text size");
        }

        private static bool IsDirectLabelUpdate(Wire.VisualElementUpdatePayload payload)
        {
            if (
                payload.Kind != Wire.VisualElementUpdateKind.Properties
                || !payload.Element.HasValue
            )
                return false;
            Wire.UiElement element = payload.Element.Value;
            if (
                element.Kind != Wire.UiElementKind.Label
                || element.UsageHints != 0
                || element.PropertiesLength != 1
                || element.EventSubscriptionsLength != 0
                || element.PartStylesLength != 0
            )
                return false;
            Wire.UiProperty property = element.Properties(0)!.Value;
            return property.Key == Wire.UiPropertyKey.Text
                && property.State == Wire.PropState.Set
                && property.ValueType == Wire.UiPropertyValue.TextPropertyValue;
        }

        private static void RequireConflict(Wire.ConflictPolicy value)
        {
            if (!Known(value, Wire.ConflictPolicy.Wait))
                throw new InvalidDataException("A command conflict policy is unknown.");
        }

        private static void ValidateDirectTween(Wire.Tween value, bool blocking)
        {
            if (!Known(value.Easing, Wire.Easing.InOutBounce))
                throw new InvalidDataException("A tween easing value is unknown.");
            if (!Known(value.RepeatKind, Wire.TweenRepeatKind.Forever))
                throw new InvalidDataException("A tween repeat kind is unknown.");
            if (!Known(value.RepeatMode, Wire.RepeatMode.PingPong))
                throw new InvalidDataException("A tween repeat mode is unknown.");
            if (value.RepeatKind == Wire.TweenRepeatKind.Once && value.RepeatCount != 0)
                throw new InvalidDataException("A non-repeating tween has a repeat count.");
            if (value.RepeatKind == Wire.TweenRepeatKind.Count && value.RepeatCount > 10_000)
                throw new InvalidDataException("A tween exceeds the repeat-count limit.");
            if (value.RepeatKind == Wire.TweenRepeatKind.Forever && blocking)
                throw new InvalidDataException("A blocking command cannot tween forever.");
            if (value.DurationMs == 0 && value.RepeatKind != Wire.TweenRepeatKind.Once)
                throw new InvalidDataException("A zero-duration tween cannot repeat.");
            const ulong MaximumMilliseconds = 922_337_203_685;
            if (value.DurationMs > MaximumMilliseconds || value.DelayMs > MaximumMilliseconds)
                throw new InvalidDataException("A tween duration exceeds the supported range.");
        }

        private static void ValidateDirectCreatedImage(Wire.GameObject value)
        {
            if (value.ContentType != Wire.GameObjectContent.ImageObject)
                throw new InvalidDataException("An image object has the wrong content payload.");
            Wire.ImageObject image = value.ContentAsImageObject();
            RequirePositive(image.Width, "Image width");
            RequirePositive(image.Height, "Image height");
            if (!Known(image.Fit, Wire.ImageFit.Cover))
                throw new InvalidDataException("An image fit value is unknown.");
            Wire.RgbColor tint = image.Tint!.Value;
            RequireUnit(tint.R, "Image tint red");
            RequireUnit(tint.G, "Image tint green");
            RequireUnit(tint.B, "Image tint blue");
            RequireUnit(image.Opacity, "Image opacity");
            _ = image.Texture;
        }

        private static void ValidateDirectCreatedText(Wire.GameObject value)
        {
            if (value.ContentType != Wire.GameObjectContent.TextObject)
                throw new InvalidDataException("A text object has the wrong content payload.");
            Wire.TextObject text = value.ContentAsTextObject();
            _ = text.Text;
            _ = text.Font;
            RequirePositive(text.Size, "Text size");
            Wire.RgbaColor color = text.Color!.Value;
            RequireUnit(color.R, "Text color red");
            RequireUnit(color.G, "Text color green");
            RequireUnit(color.B, "Text color blue");
            RequireUnit(color.A, "Text color alpha");
            if (!Known(text.Horizontal, Wire.HorizontalAlignment.Justified))
                throw new InvalidDataException("A text horizontal alignment is unknown.");
            if (!Known(text.Vertical, Wire.VerticalAlignment.Bottom))
                throw new InvalidDataException("A text vertical alignment is unknown.");
            if (text.WrapWidth is double width)
                RequirePositive(width, "Text wrap width");
        }

        private static void ValidateDirectCreatedCamera(Wire.GameObject value)
        {
            if (value.ContentType != Wire.GameObjectContent.CameraObject)
                throw new InvalidDataException("A camera object has the wrong content payload.");
            Wire.CameraObject camera = value.ContentAsCameraObject();
            if (!Known(camera.Projection, Wire.CameraProjection.Orthographic))
                throw new InvalidDataException("A camera projection is unknown.");
            RequireOpenRange(camera.FieldOfView, 1, 179, "Camera field of view");
            RequirePositive(camera.OrthographicSize, "Camera orthographic size");
            RequirePositive(camera.Near, "Camera near clip");
            RequireFinite(camera.Far);
            if (camera.Far <= camera.Near)
                throw new InvalidDataException("Camera clipping is invalid.");
            if (!Known(camera.ClearMode, Wire.CameraClearMode.Nothing))
                throw new InvalidDataException("A camera clear mode is unknown.");
            Wire.RgbaColor color = camera.ClearColor!.Value;
            RequireUnit(color.R, "Camera clear red");
            RequireUnit(color.G, "Camera clear green");
            RequireUnit(color.B, "Camera clear blue");
            RequireUnit(color.A, "Camera clear alpha");
        }

        private static void ValidateDirectCreatedLight(Wire.GameObject value)
        {
            if (value.ContentType != Wire.GameObjectContent.LightObject)
                throw new InvalidDataException("A light object has the wrong content payload.");
            Wire.LightObject light = value.ContentAsLightObject();
            if (!Known(light.LightType, Wire.LightType.Spot))
                throw new InvalidDataException("A light type is unknown.");
            Wire.RgbaColor color = light.Color!.Value;
            RequireUnit(color.R, "Light color red");
            RequireUnit(color.G, "Light color green");
            RequireUnit(color.B, "Light color blue");
            RequireUnit(color.A, "Light color alpha");
            RequireNonnegative(light.Intensity, "Light intensity");
            RequirePositive(light.Range, "Light range");
            RequireOpenRange(light.OuterSpotAngle, 0, 179, "Light outer spot angle");
            RequireNonnegative(light.InnerSpotAngle, "Light inner spot angle");
            if (light.InnerSpotAngle > light.OuterSpotAngle)
                throw new InvalidDataException("Spot-light angles are invalid.");
            if (!Known(light.Shadows, Wire.ShadowMode.Soft))
                throw new InvalidDataException("A light shadow mode is unknown.");
        }

        private static void ValidateDirectCreatedPlacement(Wire.GameObject value)
        {
            _ = ReadUuid(value.ObjectId, "created object");
            Wire.ParentScene parentScene = value.ParentScene!.Value;
            switch (parentScene.Kind)
            {
                case Wire.ParentSceneKind.PrimaryScene:
                case Wire.ParentSceneKind.Persistent:
                    if (parentScene.SceneId.HasValue)
                        throw new InvalidDataException(
                            "An object parent scene carries a noncanonical UUID."
                        );
                    break;
                case Wire.ParentSceneKind.Scene:
                    _ = ReadUuid(parentScene.SceneId, "object parent scene");
                    break;
                default:
                    throw new InvalidDataException("An object parent scene kind is unknown.");
            }
            if (value.ParentId.HasValue)
                _ = ReadUuid(value.ParentId, "object parent");
            Wire.LocalTransform transform = value.LocalTransform!.Value;
            RequireFinite(transform.Position);
            RequireQuaternion(transform.Rotation);
            RequireFinite(transform.Scale);
            if (!Known(value.DragMode, Wire.DragMode.PreserveOffset))
                throw new InvalidDataException("An object drag mode is unknown.");
            var pointerEvents = new HashSet<Wire.PointerEventKind>();
            for (int index = 0; index < value.PointerEventsLength; index++)
            {
                Wire.PointerEventKind pointer = value.PointerEvents(index);
                if (!Known(pointer, Wire.PointerEventKind.Click) || !pointerEvents.Add(pointer))
                    throw new InvalidDataException(
                        "Object pointer events are unknown or repeated."
                    );
            }
        }

        private static void ValidateDirectMaterials(Wire.PrimitiveObject value) =>
            ValidateDirectMaterials(value.MaterialsLength, index => value.Materials(index)!.Value);

        private static bool IsDirectPrimitive(Wire.GameObjectKind kind) =>
            kind
                is Wire.GameObjectKind.Cube
                    or Wire.GameObjectKind.Sphere
                    or Wire.GameObjectKind.Capsule
                    or Wire.GameObjectKind.Cylinder
                    or Wire.GameObjectKind.Plane
                    or Wire.GameObjectKind.Quad;

        private static void ValidateDirectMaterials(Wire.PrefabObject value) =>
            ValidateDirectMaterials(value.MaterialsLength, index => value.Materials(index)!.Value);

        private static void ValidateDirectMaterials(
            int count,
            Func<int, Wire.MaterialAssignment> item
        )
        {
            var slots = new HashSet<uint>();
            for (int index = 0; index < count; index++)
            {
                Wire.MaterialAssignment assignment = item(index);
                if (!slots.Add(assignment.Slot))
                    throw new InvalidDataException("A renderer material slot is repeated.");
                _ = assignment.Address;
            }
        }

        private static void ValidateDirectAnimator(Wire.AnimatorState value)
        {
            _ = value.State;
            RequireUnit(value.NormalizedStartTime, "Animator normalized start time");
            RequireNonnegative(value.Speed, "Animator speed");
            var names = new HashSet<string>(StringComparer.Ordinal);
            for (int index = 0; index < value.BoolParametersLength; index++)
            {
                Wire.AnimatorBoolParameter parameter = value.BoolParameters(index)!.Value;
                if (!names.Add(parameter.Name))
                    throw new InvalidDataException("An Animator parameter name is repeated.");
            }
            for (int index = 0; index < value.IntParametersLength; index++)
            {
                Wire.AnimatorIntParameter parameter = value.IntParameters(index)!.Value;
                if (!names.Add(parameter.Name))
                    throw new InvalidDataException("An Animator parameter name is repeated.");
            }
            for (int index = 0; index < value.FloatParametersLength; index++)
            {
                Wire.AnimatorFloatParameter parameter = value.FloatParameters(index)!.Value;
                if (!names.Add(parameter.Name))
                    throw new InvalidDataException("An Animator parameter name is repeated.");
                RequireFinite(parameter.Value);
            }
        }

        private static void RequireQuaternion(Wire.Quaterniond value)
        {
            RequireFinite(value.X);
            RequireFinite(value.Y);
            RequireFinite(value.Z);
            RequireFinite(value.W);
            double squared =
                value.X * value.X + value.Y * value.Y + value.Z * value.Z + value.W * value.W;
            if (!(squared > 0) || double.IsInfinity(squared))
                throw new InvalidDataException("A quaternion must have nonzero finite length.");
        }

        private static void RequirePositive(double value, string name)
        {
            if (!double.IsFinite(value) || value <= 0 || !float.IsFinite((float)value))
                throw new InvalidDataException($"{name} must be finite and positive.");
        }

        private static void RequireUnit(double value, string name)
        {
            if (!double.IsFinite(value) || value < 0 || value > 1)
                throw new InvalidDataException($"{name} must be in the inclusive range [0, 1].");
        }

        private static void RequireNonnegative(double value, string name)
        {
            if (!double.IsFinite(value) || value < 0 || !float.IsFinite((float)value))
                throw new InvalidDataException($"{name} must be finite and nonnegative.");
        }

        private static void RequireOpenRange(
            double value,
            double minimum,
            double maximum,
            string name
        )
        {
            bool outsideRange = value <= minimum || value >= maximum;
            bool invalidNumber = !double.IsFinite(value) || !float.IsFinite((float)value);
            if (outsideRange || invalidNumber)
                throw new InvalidDataException(
                    $"{name} must be strictly between {minimum} and {maximum}."
                );
        }

        private static void RequireQuaternion(Quaternion value)
        {
            RequireFinite(value.X);
            RequireFinite(value.Y);
            RequireFinite(value.Z);
            RequireFinite(value.W);
            double squared =
                value.X * value.X + value.Y * value.Y + value.Z * value.Z + value.W * value.W;
            if (!(squared > 0) || double.IsInfinity(squared))
                throw new InvalidDataException("A quaternion must have nonzero finite length.");
        }

        private static void RequireFinite(Vector3 value)
        {
            RequireFinite(value.X);
            RequireFinite(value.Y);
            RequireFinite(value.Z);
        }

        private static void RequireFinite(Wire.Vector3d value)
        {
            RequireFinite(value.X);
            RequireFinite(value.Y);
            RequireFinite(value.Z);
        }

        private static void RequireFinite(RgbColor value)
        {
            RequireFinite(value.Red);
            RequireFinite(value.Green);
            RequireFinite(value.Blue);
        }

        private static void RequireFinite(Color value)
        {
            RequireFinite(value.Red);
            RequireFinite(value.Green);
            RequireFinite(value.Blue);
            RequireFinite(value.Alpha);
        }

        private static void RequireFinite(double value)
        {
            if (double.IsNaN(value) || double.IsInfinity(value))
                throw new InvalidDataException("A command contains a non-finite number.");
        }

        private static void ValidateGameObject(Wire.GameObject value, ISet<Guid> scenes)
        {
            Wire.ParentScene placement =
                value.ParentScene
                ?? throw new InvalidDataException("An object parent scene is absent.");
            switch (placement.Kind)
            {
                case Wire.ParentSceneKind.PrimaryScene:
                case Wire.ParentSceneKind.Persistent:
                    if (placement.SceneId.HasValue)
                        throw new InvalidDataException(
                            "An object parent scene carries a noncanonical UUID."
                        );
                    break;
                case Wire.ParentSceneKind.Scene:
                    if (!scenes.Contains(ReadUuid(placement.SceneId, "object parent scene")))
                        throw new InvalidDataException("An object parent scene is unavailable.");
                    break;
                default:
                    throw new InvalidDataException("An object parent scene kind is unknown.");
            }
            Wire.GameObjectContent expected = value.Kind switch
            {
                Wire.GameObjectKind.UiDocument => Wire.GameObjectContent.UiDocumentObject,
                Wire.GameObjectKind.Empty => Wire.GameObjectContent.EmptyObject,
                Wire.GameObjectKind.Cube
                or Wire.GameObjectKind.Sphere
                or Wire.GameObjectKind.Capsule
                or Wire.GameObjectKind.Cylinder
                or Wire.GameObjectKind.Plane
                or Wire.GameObjectKind.Quad => Wire.GameObjectContent.PrimitiveObject,
                Wire.GameObjectKind.Image => Wire.GameObjectContent.ImageObject,
                Wire.GameObjectKind.Text => Wire.GameObjectContent.TextObject,
                Wire.GameObjectKind.Camera => Wire.GameObjectContent.CameraObject,
                Wire.GameObjectKind.Light => Wire.GameObjectContent.LightObject,
                Wire.GameObjectKind.Mesh => Wire.GameObjectContent.MeshObject,
                Wire.GameObjectKind.Prefab => Wire.GameObjectContent.PrefabObject,
                _ => throw new InvalidDataException("A game object kind is unknown."),
            };
            if (value.ContentType != expected)
                throw new InvalidDataException(
                    "A game object kind and content payload do not match."
                );
        }

        private static void ValidateDocument(
            Wire.UiDocument value,
            ISet<Guid> allIds,
            ISet<Guid> documentIds
        )
        {
            if (!documentIds.Add(ReadUuid(value.DocumentId, "UI document")))
                throw new InvalidDataException("The snapshot repeats a UI document UUID.");
            Guid root = ReadUuid(value.RootId, "UI document root");
            if (!allIds.Add(root))
                throw new InvalidDataException("The snapshot repeats a UI UUID across documents.");
            Wire.UiElement rootElement =
                value.RootElement ?? throw new InvalidDataException("A UI root is absent.");
            if (
                rootElement.Kind != Wire.UiElementKind.VisualElement
                || rootElement.UsageHints != 0
                || rootElement.PartStylesLength != 0
            )
                throw new InvalidDataException("A UI document root has non-document state.");
            for (int index = 0; index < rootElement.PropertiesLength; index++)
            {
                Wire.UiPropertyKey key = rootElement.Properties(index)!.Value.Key;
                if (!IsDocumentRootProperty(key))
                    throw new InvalidDataException(
                        "A UI document root contains a non-document property."
                    );
            }
            ValidateElement(rootElement);
            var roots = new Guid[value.RootChildIdsLength];
            for (int index = 0; index < roots.Length; index++)
                roots[index] = ReadUuid(value.RootChildIds(index), "UI root child");
            ValidateForest(root, value.NodesLength, value.Nodes, roots, allIds);
        }

        private static bool IsDocumentRootProperty(Wire.UiPropertyKey key) =>
            key
                is Wire.UiPropertyKey.Name
                    or Wire.UiPropertyKey.Enabled
                    or Wire.UiPropertyKey.PickingMode
                    or Wire.UiPropertyKey.LanguageDirection
                    or Wire.UiPropertyKey.Focusable
                    or Wire.UiPropertyKey.TabIndex
                    or Wire.UiPropertyKey.DelegatesFocus
                    or Wire.UiPropertyKey.AutoFocus
                    or Wire.UiPropertyKey.Inert
                    or Wire.UiPropertyKey.Classes
                    or Wire.UiPropertyKey.Events
                    or Wire.UiPropertyKey.EventSubscriptions
            || (
                key >= Wire.UiPropertyKey.StyleAlignContent
                && key <= Wire.UiPropertyKey.StyleWordSpacing
            );

        private static void ValidateCreate(Wire.VisualElementCreatePayload value)
        {
            _ = ReadUuid(value.ParentId, "visual create parent");
            Guid root = ReadUuid(value.RootId, "visual create root");
            if (value.NodesLength == 0)
                throw new InvalidDataException("A visual create subtree is empty.");
            ValidateForest(null, value.NodesLength, value.Nodes, new[] { root }, null);
        }

        private static void ValidateForest(
            Guid? excludedRoot,
            int nodeCount,
            Func<int, Wire.UiNode?> readNode,
            IReadOnlyList<Guid> roots,
            ISet<Guid>? allIds
        )
        {
            var nodes = new Dictionary<Guid, Wire.UiNode>(nodeCount);
            for (int index = 0; index < nodeCount; index++)
            {
                Wire.UiNode node =
                    readNode(index) ?? throw new InvalidDataException("A UI node is absent.");
                Guid id = ReadUuid(node.ObjectId, "UI node");
                if (id == excludedRoot || !nodes.TryAdd(id, node))
                    throw new InvalidDataException("A UI forest repeats a node UUID.");
                if (allIds is not null && !allIds.Add(id))
                    throw new InvalidDataException(
                        "The snapshot repeats a UI UUID across documents."
                    );
                ValidateElement(
                    node.Element ?? throw new InvalidDataException("A UI element is absent.")
                );
            }
            var visited = new HashSet<Guid>();
            var pending = new Stack<(Guid Id, int Depth)>();
            for (int index = roots.Count - 1; index >= 0; index--)
                pending.Push((roots[index], 0));
            while (pending.Count != 0)
            {
                (Guid id, int depth) = pending.Pop();
                if (depth > 1_024)
                    throw new InvalidDataException("A UI forest exceeds logical depth 1024.");
                if (!visited.Add(id))
                    throw new InvalidDataException("A UI forest gives a node multiple parents.");
                if (!nodes.TryGetValue(id, out Wire.UiNode node))
                    throw new InvalidDataException("A UI forest contains a dangling child UUID.");
                for (int index = node.ChildIdsLength - 1; index >= 0; index--)
                    pending.Push((ReadUuid(node.ChildIds(index), "UI child"), depth + 1));
            }
            if (visited.Count != nodes.Count)
                throw new InvalidDataException("A UI forest contains unreachable or cyclic nodes.");
        }

        private static void ValidateElement(Wire.UiElement value)
        {
            if (!Enum.IsDefined(typeof(Wire.UiElementKind), value.Kind))
                throw new InvalidDataException("A UI element kind is unknown.");
            if ((value.UsageHints & ~0x0fU) != 0)
                throw new InvalidDataException("A UI usage-hint mask contains unknown bits.");
            var keys = new HashSet<Wire.UiPropertyKey>();
            var shorthandEvents = new HashSet<uint>();
            for (int index = 0; index < value.PropertiesLength; index++)
            {
                Wire.UiProperty property =
                    value.Properties(index)
                    ?? throw new InvalidDataException("A UI property is absent.");
                ValidateProperty(property, keys);
                if (
                    property.Key == Wire.UiPropertyKey.Classes
                    && property.State == Wire.PropState.Set
                )
                    ValidateUniqueTextList(property.ValueAsTextListPropertyValue());
                if (
                    property.Key == Wire.UiPropertyKey.Events
                    && property.State == Wire.PropState.Set
                )
                    ValidateShorthandEvents(
                        property.ValueAsUIntListPropertyValue(),
                        shorthandEvents
                    );
            }
            var subscriptions = new HashSet<(Wire.UiSubscriptionKind, byte)>();
            for (int index = 0; index < value.EventSubscriptionsLength; index++)
            {
                Wire.UiEventSubscriptionValue subscription =
                    value.EventSubscriptions(index)
                    ?? throw new InvalidDataException("A UI subscription is absent.");
                if (
                    !Enum.IsDefined(typeof(Wire.UiSubscriptionKind), subscription.Kind)
                    || subscription.Phases == 0
                    || (subscription.Phases & ~0x07) != 0
                )
                    throw new InvalidDataException("A UI event subscription is noncanonical.");
                if (!subscriptions.Add((subscription.Kind, subscription.Phases)))
                    throw new InvalidDataException("UI event subscriptions must be unique.");
                if (
                    (subscription.Phases & 0x02) != 0
                    && shorthandEvents.Contains((uint)subscription.Kind)
                )
                    throw new InvalidDataException(
                        "UI event subscriptions must be unique across shorthand and routed values."
                    );
            }
            var parts = new HashSet<(Wire.UiPart, uint?)>();
            for (int index = 0; index < value.PartStylesLength; index++)
            {
                Wire.PartStyle part =
                    value.PartStyles(index)
                    ?? throw new InvalidDataException("A UI part style is absent.");
                if (
                    !Enum.IsDefined(typeof(Wire.UiPart), part.Part)
                    || !parts.Add((part.Part, part.Index))
                )
                    throw new InvalidDataException("UI part styles repeat or use an unknown part.");
                var partKeys = new HashSet<Wire.UiPropertyKey>();
                for (int item = 0; item < part.PropertiesLength; item++)
                    ValidateProperty(
                        part.Properties(item)
                            ?? throw new InvalidDataException("A UI part property is absent."),
                        partKeys
                    );
            }
        }

        private static void ValidateUniqueTextList(Wire.TextListPropertyValue value)
        {
            var items = new HashSet<string>(StringComparer.Ordinal);
            for (int index = 0; index < value.ValuesLength; index++)
            {
                string item = value.Values(index);
                RequireSnapshotString(item, "UI class", allowEmpty: false);
                if (!items.Add(item))
                    throw new InvalidDataException("UI classes must be unique.");
            }
        }

        private static void ValidateShorthandEvents(
            Wire.UIntListPropertyValue value,
            ISet<uint> events
        )
        {
            for (int index = 0; index < value.ValuesLength; index++)
            {
                uint item = value.Values(index);
                if (!Enum.IsDefined(typeof(Wire.UiSubscriptionKind), (byte)item))
                    throw new InvalidDataException("A UI event subscription is unknown.");
                if (!events.Add(item))
                    throw new InvalidDataException("UI event subscriptions must be unique.");
            }
        }

        private static void ValidateProperty(Wire.UiProperty value, ISet<Wire.UiPropertyKey> keys)
        {
            if (!Enum.IsDefined(typeof(Wire.UiPropertyKey), value.Key) || !keys.Add(value.Key))
                throw new InvalidDataException("UI properties repeat or use an unknown key.");
            if (!Enum.IsDefined(typeof(Wire.StyleValueKind), value.StyleValueKind))
                throw new InvalidDataException("A UI style-value kind is unknown.");
            bool isStyle =
                value.Key >= Wire.UiPropertyKey.StyleAlignContent
                && value.Key <= Wire.UiPropertyKey.StyleWordSpacing;
            if (!isStyle && value.StyleValueKind != Wire.StyleValueKind.Value)
                throw new InvalidDataException("A non-style UI property uses a style keyword.");
            switch (value.State)
            {
                case Wire.PropState.Set:
                    bool isSubscriptionMarker =
                        value.Key == Wire.UiPropertyKey.EventSubscriptions
                        && value.StyleValueKind == Wire.StyleValueKind.Value
                        && value.ValueType == Wire.UiPropertyValue.NONE;
                    if (isSubscriptionMarker)
                        break;
                    if (value.StyleValueKind == Wire.StyleValueKind.Initial)
                    {
                        if (value.ValueType != Wire.UiPropertyValue.NONE)
                            throw new InvalidDataException(
                                "An initial UI style keyword carries a value."
                            );
                    }
                    else if (
                        value.ValueType == Wire.UiPropertyValue.NONE
                        || !Enum.IsDefined(typeof(Wire.UiPropertyValue), value.ValueType)
                    )
                    {
                        throw new InvalidDataException(
                            "A set UI property has no known typed value."
                        );
                    }
                    else
                    {
                        ValidatePropertyPayload(value);
                    }
                    break;
                case Wire.PropState.Reset:
                    if (
                        value.StyleValueKind != Wire.StyleValueKind.Value
                        || value.ValueType != Wire.UiPropertyValue.NONE
                    )
                        throw new InvalidDataException("A reset UI property carries a value.");
                    break;
                case Wire.PropState.Unset:
                    break;
                default:
                    throw new InvalidDataException("A present UI property has an invalid state.");
            }
        }

        private static void ValidatePropertyPayload(Wire.UiProperty value)
        {
            switch (value.ValueType)
            {
                case Wire.UiPropertyValue.BoolPropertyValue:
                case Wire.UiPropertyValue.IntPropertyValue:
                case Wire.UiPropertyValue.UIntPropertyValue:
                case Wire.UiPropertyValue.TextPropertyValue:
                case Wire.UiPropertyValue.TextListPropertyValue:
                case Wire.UiPropertyValue.UIntListPropertyValue:
                    return;
                case Wire.UiPropertyValue.FloatPropertyValue:
                    RequireFinite(value.ValueAsFloatPropertyValue().Value);
                    return;
                case Wire.UiPropertyValue.EnumPropertyValue:
                    if (
                        !Enum.IsDefined(
                            typeof(Wire.UiEnumCatalog),
                            value.ValueAsEnumPropertyValue().Catalog
                        )
                    )
                        throw new InvalidDataException("A UI enum property catalog is unknown.");
                    return;
                case Wire.UiPropertyValue.LengthPropertyValue:
                    ValidatePropertyLength(value.ValueAsLengthPropertyValue());
                    return;
                case Wire.UiPropertyValue.AspectRatioPropertyValue:
                {
                    Wire.AspectRatioPropertyValue item = value.ValueAsAspectRatioPropertyValue();
                    RequireFinite(item.Width);
                    RequireFinite(item.Height);
                    if (
                        item.Kind == Wire.AspectRatioKind.Auto
                        && (item.Width != 0 || item.Height != 0)
                    )
                        throw new InvalidDataException(
                            "An automatic UI aspect ratio carries dimensions."
                        );
                    if (
                        item.Kind == Wire.AspectRatioKind.Ratio
                        && (item.Width <= 0 || item.Height <= 0)
                    )
                        throw new InvalidDataException(
                            "UI aspect-ratio dimensions must be positive."
                        );
                    if (!Enum.IsDefined(typeof(Wire.AspectRatioKind), item.Kind))
                        throw new InvalidDataException("A UI aspect-ratio kind is unknown.");
                    return;
                }
                case Wire.UiPropertyValue.RotatePropertyValue:
                {
                    Wire.RotatePropertyValue item = value.ValueAsRotatePropertyValue();
                    RequireFinite(item.X);
                    RequireFinite(item.Y);
                    RequireFinite(item.Z);
                    RequireFinite(item.Degrees);
                    return;
                }
                case Wire.UiPropertyValue.ScalePropertyValue:
                {
                    Wire.ScalePropertyValue item = value.ValueAsScalePropertyValue();
                    RequireFinite(item.X);
                    RequireFinite(item.Y);
                    return;
                }
                case Wire.UiPropertyValue.FloatListPropertyValue:
                {
                    Wire.FloatListPropertyValue item = value.ValueAsFloatListPropertyValue();
                    for (int index = 0; index < item.ValuesLength; index++)
                        RequireFinite(item.Values(index));
                    return;
                }
                case Wire.UiPropertyValue.EnumListPropertyValue:
                    if (
                        !Enum.IsDefined(
                            typeof(Wire.UiEnumCatalog),
                            value.ValueAsEnumListPropertyValue().Catalog
                        )
                    )
                        throw new InvalidDataException(
                            "A UI enum-list property catalog is unknown."
                        );
                    return;
                case Wire.UiPropertyValue.TextAutoSizePropertyValue:
                {
                    Wire.TextAutoSizePropertyValue item = value.ValueAsTextAutoSizePropertyValue();
                    RequireFinite(item.MinSize);
                    RequireFinite(item.MaxSize);
                    if (
                        item.Kind == Wire.TextAutoSizeKind.None
                        && (item.MinSize != 0 || item.MaxSize != 0)
                    )
                        throw new InvalidDataException(
                            "Disabled UI text auto-size carries bounds."
                        );
                    if (
                        item.Kind == Wire.TextAutoSizeKind.BestFit
                        && (item.MinSize < 0 || item.MinSize > item.MaxSize)
                    )
                        throw new InvalidDataException("UI text auto-size bounds are invalid.");
                    if (!Enum.IsDefined(typeof(Wire.TextAutoSizeKind), item.Kind))
                        throw new InvalidDataException("A UI text-auto-size kind is unknown.");
                    return;
                }
                case Wire.UiPropertyValue.ColorPropertyValue:
                {
                    Wire.RgbaColor item = value.ValueAsColorPropertyValue().Value!.Value;
                    RequireFinite(item.R);
                    RequireFinite(item.G);
                    RequireFinite(item.B);
                    RequireFinite(item.A);
                    return;
                }
                case Wire.UiPropertyValue.RectPropertyValue:
                {
                    Wire.Rectd item = value.ValueAsRectPropertyValue().Value!.Value;
                    RequireFinite(item.X);
                    RequireFinite(item.Y);
                    RequireFinite(item.Width);
                    RequireFinite(item.Height);
                    return;
                }
                case Wire.UiPropertyValue.Vector2PropertyValue:
                {
                    Wire.Vector2d item = value.ValueAsVector2PropertyValue().Value!.Value;
                    RequireFinite(item.X);
                    RequireFinite(item.Y);
                    return;
                }
                case Wire.UiPropertyValue.AssetPropertyValue:
                    if (
                        !Enum.IsDefined(
                            typeof(Wire.AssetSourceKind),
                            value.ValueAsAssetPropertyValue().Kind
                        )
                    )
                        throw new InvalidDataException("A UI asset kind is unknown.");
                    return;
                case Wire.UiPropertyValue.GridTracksPropertyValue:
                {
                    Wire.GridTracksPropertyValue item = value.ValueAsGridTracksPropertyValue();
                    for (int index = 0; index < item.ValuesLength; index++)
                    {
                        Wire.GridTrackValue track = item.Values(index)!.Value;
                        if (!Enum.IsDefined(typeof(Wire.GridTrackKind), track.Kind))
                            throw new InvalidDataException("A UI grid track is invalid.");
                        RequireFinite(track.Value);
                        if (track.Kind != Wire.GridTrackKind.Auto && track.Value < 0)
                            throw new InvalidDataException("A UI grid-track value is negative.");
                        if (track.Kind == Wire.GridTrackKind.Auto && track.Value != 0)
                            throw new InvalidDataException(
                                "An automatic UI grid track carries a value."
                            );
                    }
                    return;
                }
                case Wire.UiPropertyValue.ChoicePropertyValue:
                {
                    Wire.ChoicePropertyValue item = value.ValueAsChoicePropertyValue();
                    if (
                        item.Kind == Wire.ChoiceKind.None
                        && (item.Index != 0 || item.Value is not null)
                    )
                        throw new InvalidDataException(
                            "An absent UI choice carries selection data."
                        );
                    if (item.Kind == Wire.ChoiceKind.Index && item.Value is null)
                        throw new InvalidDataException(
                            "A selected UI choice has no display value."
                        );
                    if (!Enum.IsDefined(typeof(Wire.ChoiceKind), item.Kind))
                        throw new InvalidDataException("A UI choice kind is unknown.");
                    return;
                }
                case Wire.UiPropertyValue.LimitPropertyValue:
                {
                    Wire.LimitPropertyValue item = value.ValueAsLimitPropertyValue();
                    RequireFinite(item.Value);
                    if (item.Kind == Wire.LimitKind.Unbounded && item.Value != 0)
                        throw new InvalidDataException("An unbounded UI limit carries a value.");
                    if (!Enum.IsDefined(typeof(Wire.LimitKind), item.Kind))
                        throw new InvalidDataException("A UI limit kind is unknown.");
                    return;
                }
                case Wire.UiPropertyValue.BackgroundPositionPropertyValue:
                    ValidatePropertyLength(
                        value.ValueAsBackgroundPositionPropertyValue().Offset
                            ?? throw new InvalidDataException(
                                "A UI background position has no offset."
                            )
                    );
                    return;
                case Wire.UiPropertyValue.BackgroundRepeatPropertyValue:
                {
                    Wire.BackgroundRepeatPropertyValue item =
                        value.ValueAsBackgroundRepeatPropertyValue();
                    if (item.X > 3 || item.Y > 3)
                        throw new InvalidDataException("A UI background-repeat mode is unknown.");
                    return;
                }
                case Wire.UiPropertyValue.BackgroundSizePropertyValue:
                {
                    Wire.BackgroundSizePropertyValue item =
                        value.ValueAsBackgroundSizePropertyValue();
                    if (item.Kind == Wire.BackgroundSizeKind.Axes)
                    {
                        ValidatePropertyLength(
                            item.X
                                ?? throw new InvalidDataException(
                                    "An axis UI background size has no x value."
                                )
                        );
                        ValidatePropertyLength(
                            item.Y
                                ?? throw new InvalidDataException(
                                    "An axis UI background size has no y value."
                                )
                        );
                        return;
                    }
                    if (!Enum.IsDefined(typeof(Wire.BackgroundSizeKind), item.Kind))
                        throw new InvalidDataException("A UI background-size kind is unknown.");
                    if (item.X.HasValue || item.Y.HasValue)
                        throw new InvalidDataException(
                            "A keyword UI background size carries axis values."
                        );
                    return;
                }
                case Wire.UiPropertyValue.CursorPropertyValue:
                {
                    Wire.CursorPropertyValue item = value.ValueAsCursorPropertyValue();
                    if (item.Kind == Wire.CursorKind.Default)
                    {
                        if (item.Address is not null || item.Hotspot.HasValue)
                            throw new InvalidDataException(
                                "A default UI cursor carries texture data."
                            );
                        return;
                    }
                    if (item.Kind != Wire.CursorKind.Texture)
                        throw new InvalidDataException("A UI cursor kind is unknown.");
                    _ =
                        item.Address
                        ?? throw new InvalidDataException("A texture UI cursor has no address.");
                    Wire.Vector2d hotspot =
                        item.Hotspot
                        ?? throw new InvalidDataException("A texture UI cursor has no hotspot.");
                    RequireFinite(hotspot.X);
                    RequireFinite(hotspot.Y);
                    return;
                }
                case Wire.UiPropertyValue.TextShadowPropertyValue:
                {
                    Wire.TextShadowPropertyValue item = value.ValueAsTextShadowPropertyValue();
                    RequireFinite(item.X);
                    RequireFinite(item.Y);
                    RequireFinite(item.BlurRadius);
                    Wire.RgbaColor color =
                        item.Color
                        ?? throw new InvalidDataException("A UI text shadow has no color.");
                    RequireFinite(color.R);
                    RequireFinite(color.G);
                    RequireFinite(color.B);
                    RequireFinite(color.A);
                    return;
                }
                case Wire.UiPropertyValue.TransformOriginPropertyValue:
                {
                    Wire.TransformOriginPropertyValue item =
                        value.ValueAsTransformOriginPropertyValue();
                    ValidatePropertyLength(
                        item.X
                            ?? throw new InvalidDataException(
                                "A UI transform origin has no x value."
                            )
                    );
                    ValidatePropertyLength(
                        item.Y
                            ?? throw new InvalidDataException(
                                "A UI transform origin has no y value."
                            )
                    );
                    RequireFinite(item.Z);
                    return;
                }
                case Wire.UiPropertyValue.TranslatePropertyValue:
                {
                    Wire.TranslatePropertyValue item = value.ValueAsTranslatePropertyValue();
                    ValidatePropertyLength(
                        item.X ?? throw new InvalidDataException("A UI translation has no x value.")
                    );
                    ValidatePropertyLength(
                        item.Y ?? throw new InvalidDataException("A UI translation has no y value.")
                    );
                    RequireFinite(item.Z);
                    return;
                }
                case Wire.UiPropertyValue.OverlayPlacementPropertyValue:
                    ValidateOverlayPlacement(value.ValueAsOverlayPlacementPropertyValue());
                    return;
                case Wire.UiPropertyValue.GridItemPropertyValue:
                {
                    Wire.GridItemPropertyValue item = value.ValueAsGridItemPropertyValue();
                    if (item.RowSpan == 0 || item.ColumnSpan == 0)
                        throw new InvalidDataException("A UI grid-item span must be positive.");
                    return;
                }
                case Wire.UiPropertyValue.StackItemPropertyValue:
                {
                    Wire.StackItemPropertyValue item = value.ValueAsStackItemPropertyValue();
                    RequireOptionalFinite(item.Top);
                    RequireOptionalFinite(item.Right);
                    RequireOptionalFinite(item.Bottom);
                    RequireOptionalFinite(item.Left);
                    return;
                }
                case Wire.UiPropertyValue.StickyPropertyValue:
                {
                    Wire.StickyPropertyValue item = value.ValueAsStickyPropertyValue();
                    RequireOptionalFinite(item.Top);
                    RequireOptionalFinite(item.Right);
                    RequireOptionalFinite(item.Bottom);
                    RequireOptionalFinite(item.Left);
                    return;
                }

                case Wire.UiPropertyValue.NONE:
                    break;
                case Wire.UiPropertyValue.PaintStylePropertyValue:
                    break;
                case Wire.UiPropertyValue.MotionDescriptorPropertyValue:
                    break;
                default:
                    return;
            }
        }

        private static void ValidateOverlayPlacement(Wire.OverlayPlacementPropertyValue value)
        {
            RequireFinite(value.MainOffset);
            RequireFinite(value.CrossOffset);
            RequireFinite(value.CollisionPadding);
            if (
                !Enum.IsDefined(typeof(Wire.PlacementSide), value.Side)
                || !Enum.IsDefined(typeof(Wire.PlacementAlign), value.Align)
            )
                throw new InvalidDataException("A UI overlay placement enum is unknown.");
            bool defaults =
                value.Side == Wire.PlacementSide.Bottom
                && value.Align == Wire.PlacementAlign.Start
                && value.MainOffset == 0
                && value.CrossOffset == 0
                && value.CollisionPadding == 0
                && !value.Flip
                && !value.Shift;
            switch (value.Kind)
            {
                case Wire.OverlayPlacementKind.PopoverLayer:
                case Wire.OverlayPlacementKind.ModalLayer:
                    if (
                        value.Anchor.HasValue
                        || value.InitialFocus.HasValue
                        || value.RestoreFocus.HasValue
                        || !defaults
                    )
                        throw new InvalidDataException(
                            "A UI overlay layer carries placement data."
                        );
                    return;
                case Wire.OverlayPlacementKind.Popover:
                    _ = ReadUuid(value.Anchor, "UI popover anchor");
                    if (value.InitialFocus.HasValue || value.RestoreFocus.HasValue)
                        throw new InvalidDataException("A UI popover carries modal focus UUIDs.");
                    return;
                case Wire.OverlayPlacementKind.Modal:
                    if (value.Anchor.HasValue || !defaults)
                        throw new InvalidDataException(
                            "A UI modal carries popover placement data."
                        );
                    if (value.InitialFocus.HasValue)
                        _ = ReadUuid(value.InitialFocus, "UI modal initial focus");
                    if (value.RestoreFocus.HasValue)
                        _ = ReadUuid(value.RestoreFocus, "UI modal restore focus");
                    return;
                default:
                    throw new InvalidDataException("A UI overlay-placement kind is unknown.");
            }
        }

        private static void RequireOptionalFinite(float? value)
        {
            if (value.HasValue)
                RequireFinite(value.Value);
        }

        private static void ValidatePropertyLength(Wire.LengthPropertyValue value)
        {
            RequireFinite(value.Pixels);
            RequireFinite(value.Percentage);
            switch (value.Kind)
            {
                case Wire.LengthKind.Pixels when value.Percentage == 0:
                case Wire.LengthKind.Percent when value.Pixels == 0:
                case Wire.LengthKind.Calc:
                case Wire.LengthKind.Auto when value.Pixels == 0 && value.Percentage == 0:
                    return;
                case Wire.LengthKind.Pixels:
                case Wire.LengthKind.Percent:
                case Wire.LengthKind.Auto:
                    throw new InvalidDataException("A UI length carries inactive scalar data.");
                default:
                    throw new InvalidDataException("A UI length kind is unknown.");
            }
        }

        private static void ValidateUpdate(Wire.VisualElementUpdatePayload value)
        {
            _ = ReadUuid(value.ObjectId, "visual update object");
            switch (value.Kind)
            {
                case Wire.VisualElementUpdateKind.Properties
                    when value.Element.HasValue
                        && !value.ParentId.HasValue
                        && !value.ChildIndex.HasValue:
                    ValidateElement(value.Element.Value);
                    break;
                case Wire.VisualElementUpdateKind.Parent
                    when value.ParentId.HasValue && !value.Element.HasValue:
                    _ = ReadUuid(value.ParentId, "visual update parent");
                    break;
                case Wire.VisualElementUpdateKind.Index
                    when value.ChildIndex.HasValue
                        && !value.Element.HasValue
                        && !value.ParentId.HasValue:
                    break;
                case Wire.VisualElementUpdateKind.Properties:
                    break;
                case Wire.VisualElementUpdateKind.Parent:
                    break;
                case Wire.VisualElementUpdateKind.Index:
                    break;
                default:
                    throw new InvalidDataException("A visual update carries noncanonical fields.");
            }
        }

        private static void ValidateAction(Wire.VisualElementActionPayload value)
        {
            _ = ReadUuid(value.ObjectId, "visual action object");
            if (!Enum.IsDefined(typeof(Wire.VisualElementActionKind), value.Kind))
                throw new InvalidDataException("A visual action kind is unknown.");
            if (value.Kind == Wire.VisualElementActionKind.ScrollTo)
                _ = ReadUuid(value.DescendantId, "scroll descendant");
            if (value.Kind == Wire.VisualElementActionKind.ParticleStreaks)
            {
                if (value.StreaksLength > 128)
                    throw new InvalidDataException("A particle action exceeds 128 streaks.");
                for (int index = 0; index < value.StreaksLength; index++)
                {
                    Wire.UiParticleStreak streak =
                        value.Streaks(index)
                        ?? throw new InvalidDataException("A particle streak is absent.");
                    Wire.F32Vector2 origin =
                        streak.Origin
                        ?? throw new InvalidDataException("A particle origin is absent.");
                    Wire.F32Vector2 travel =
                        streak.Travel
                        ?? throw new InvalidDataException("A particle travel vector is absent.");
                    Wire.F32Vector2 size =
                        streak.Size ?? throw new InvalidDataException("A particle size is absent.");
                    if (
                        !Finite(origin.X)
                        || !Finite(origin.Y)
                        || !Finite(travel.X)
                        || !Finite(travel.Y)
                        || !Finite(size.X)
                        || !Finite(size.Y)
                        || !Finite(streak.Rotation)
                        || origin.X < 0
                        || origin.X > 1
                        || origin.Y < 0
                        || origin.Y > 1
                        || size.X <= 0
                        || size.X > 1024
                        || size.Y <= 0
                        || size.Y > 1024
                        || Math.Abs(travel.X) > 1024
                        || Math.Abs(travel.Y) > 1024
                        || streak.LifetimeMs < 1
                        || streak.LifetimeMs > 1000
                        || streak.DelayMs > 1000
                    )
                        throw new InvalidDataException(
                            "A particle action contains an invalid streak."
                        );
                }
            }
        }

        private static Wire.CoreCommandPayload ExpectedPayload(Wire.CoreCommandKind kind) =>
            kind switch
            {
                Wire.CoreCommandKind.ApplicationOpenUrl =>
                    Wire.CoreCommandPayload.ExternalUrlPayload,
                Wire.CoreCommandKind.Diagnostics => Wire.CoreCommandPayload.DiagnosticsPayload,
                Wire.CoreCommandKind.AssetsReplaceSet =>
                    Wire.CoreCommandPayload.ReplaceAssetSetPayload,
                Wire.CoreCommandKind.SceneLoad => Wire.CoreCommandPayload.SceneLoadPayload,
                Wire.CoreCommandKind.SceneUnload or Wire.CoreCommandKind.SceneSetPrimary =>
                    Wire.CoreCommandPayload.SceneIdPayload,
                Wire.CoreCommandKind.ObjectCreate => Wire.CoreCommandPayload.ObjectCreatePayload,
                Wire.CoreCommandKind.ObjectDestroy or Wire.CoreCommandKind.InputSetCamera =>
                    Wire.CoreCommandPayload.ObjectIdPayload,
                Wire.CoreCommandKind.ObjectSetActive =>
                    Wire.CoreCommandPayload.ObjectSetActivePayload,
                Wire.CoreCommandKind.ObjectReparent =>
                    Wire.CoreCommandPayload.ObjectReparentPayload,
                Wire.CoreCommandKind.TransformSetLocalPosition
                or Wire.CoreCommandKind.TransformSetWorldPosition =>
                    Wire.CoreCommandPayload.PositionPayload,
                Wire.CoreCommandKind.TransformTweenLocalPosition
                or Wire.CoreCommandKind.TransformTweenWorldPosition =>
                    Wire.CoreCommandPayload.TweenPositionPayload,
                Wire.CoreCommandKind.TransformSetLocalRotation
                or Wire.CoreCommandKind.TransformSetWorldRotation =>
                    Wire.CoreCommandPayload.RotationPayload,
                Wire.CoreCommandKind.TransformTweenLocalRotation
                or Wire.CoreCommandKind.TransformTweenWorldRotation =>
                    Wire.CoreCommandPayload.TweenRotationPayload,
                Wire.CoreCommandKind.TransformSetLocalScale => Wire.CoreCommandPayload.ScalePayload,
                Wire.CoreCommandKind.TransformTweenLocalScale =>
                    Wire.CoreCommandPayload.TweenScalePayload,
                Wire.CoreCommandKind.RendererSetMaterial =>
                    Wire.CoreCommandPayload.SetMaterialPayload,
                Wire.CoreCommandKind.CameraSetEnabled
                or Wire.CoreCommandKind.LightSetEnabled
                or Wire.CoreCommandKind.ImageSetFaceCamera
                or Wire.CoreCommandKind.TextSetRichText
                or Wire.CoreCommandKind.TextSetFaceCamera =>
                    Wire.CoreCommandPayload.ObjectEnabledPayload,
                Wire.CoreCommandKind.CameraSetPerspective =>
                    Wire.CoreCommandPayload.PerspectivePayload,
                Wire.CoreCommandKind.CameraTweenFieldOfView =>
                    Wire.CoreCommandPayload.TweenFieldOfViewPayload,
                Wire.CoreCommandKind.CameraSetOrthographic =>
                    Wire.CoreCommandPayload.OrthographicPayload,
                Wire.CoreCommandKind.CameraTweenOrthographicSize =>
                    Wire.CoreCommandPayload.TweenOrthographicSizePayload,
                Wire.CoreCommandKind.CameraSetClipping =>
                    Wire.CoreCommandPayload.CameraClippingPayload,
                Wire.CoreCommandKind.CameraSetClear => Wire.CoreCommandPayload.CameraClearPayload,
                Wire.CoreCommandKind.LightSetType => Wire.CoreCommandPayload.LightTypePayload,
                Wire.CoreCommandKind.LightSetColor or Wire.CoreCommandKind.TextSetColor =>
                    Wire.CoreCommandPayload.ColorPayload,
                Wire.CoreCommandKind.LightTweenColor or Wire.CoreCommandKind.TextTweenColor =>
                    Wire.CoreCommandPayload.TweenColorPayload,
                Wire.CoreCommandKind.LightSetIntensity => Wire.CoreCommandPayload.IntensityPayload,
                Wire.CoreCommandKind.LightTweenIntensity =>
                    Wire.CoreCommandPayload.TweenIntensityPayload,
                Wire.CoreCommandKind.LightSetRange => Wire.CoreCommandPayload.LightRangePayload,
                Wire.CoreCommandKind.LightSetSpotAngle => Wire.CoreCommandPayload.SpotAnglePayload,
                Wire.CoreCommandKind.LightSetShadows => Wire.CoreCommandPayload.LightShadowsPayload,
                Wire.CoreCommandKind.ImageSetTexture => Wire.CoreCommandPayload.SetTexturePayload,
                Wire.CoreCommandKind.TextSetFont => Wire.CoreCommandPayload.SetFontPayload,
                Wire.CoreCommandKind.ImageSetSize => Wire.CoreCommandPayload.ImageSizePayload,
                Wire.CoreCommandKind.ImageSetFit => Wire.CoreCommandPayload.ImageFitPayload,
                Wire.CoreCommandKind.ImageSetTint => Wire.CoreCommandPayload.TintPayload,
                Wire.CoreCommandKind.ImageTweenTint => Wire.CoreCommandPayload.TweenTintPayload,
                Wire.CoreCommandKind.ImageSetOpacity => Wire.CoreCommandPayload.OpacityPayload,
                Wire.CoreCommandKind.ImageTweenOpacity =>
                    Wire.CoreCommandPayload.TweenOpacityPayload,
                Wire.CoreCommandKind.TextSetContent => Wire.CoreCommandPayload.TextContentPayload,
                Wire.CoreCommandKind.TextSetSize => Wire.CoreCommandPayload.TextSizePayload,
                Wire.CoreCommandKind.TextTweenSize => Wire.CoreCommandPayload.TweenTextSizePayload,
                Wire.CoreCommandKind.TextSetAlignment =>
                    Wire.CoreCommandPayload.TextAlignmentPayload,
                Wire.CoreCommandKind.TextSetWrapping => Wire.CoreCommandPayload.TextWrappingPayload,
                Wire.CoreCommandKind.AnimatorPlay => Wire.CoreCommandPayload.AnimatorPlayPayload,
                Wire.CoreCommandKind.AnimatorCrossFade =>
                    Wire.CoreCommandPayload.AnimatorCrossFadePayload,
                Wire.CoreCommandKind.AnimatorSetBool => Wire.CoreCommandPayload.AnimatorBoolPayload,
                Wire.CoreCommandKind.AnimatorSetInt => Wire.CoreCommandPayload.AnimatorIntPayload,
                Wire.CoreCommandKind.AnimatorSetFloat =>
                    Wire.CoreCommandPayload.AnimatorFloatPayload,
                Wire.CoreCommandKind.AnimatorSetTrigger =>
                    Wire.CoreCommandPayload.AnimatorParameterPayload,
                Wire.CoreCommandKind.AnimatorSetSpeed =>
                    Wire.CoreCommandPayload.AnimatorSpeedPayload,
                Wire.CoreCommandKind.ParticlePlay => Wire.CoreCommandPayload.ParticlePlayPayload,
                Wire.CoreCommandKind.ParticleStop => Wire.CoreCommandPayload.ParticleStopPayload,
                Wire.CoreCommandKind.ParticleSpawn => Wire.CoreCommandPayload.ParticleSpawnPayload,
                Wire.CoreCommandKind.AudioPlay => Wire.CoreCommandPayload.AudioPlayPayload,
                Wire.CoreCommandKind.AudioStop => Wire.CoreCommandPayload.AudioStopPayload,
                Wire.CoreCommandKind.AudioPause or Wire.CoreCommandKind.AudioResume =>
                    Wire.CoreCommandPayload.AudioPlaybackPayload,
                Wire.CoreCommandKind.AudioSeek => Wire.CoreCommandPayload.AudioSeekPayload,
                Wire.CoreCommandKind.AudioSetBuffering =>
                    Wire.CoreCommandPayload.AudioBufferingPayload,
                Wire.CoreCommandKind.AudioReplace => Wire.CoreCommandPayload.AudioReplacePayload,
                Wire.CoreCommandKind.AudioSetVolume => Wire.CoreCommandPayload.AudioVolumePayload,
                Wire.CoreCommandKind.AudioTweenVolume =>
                    Wire.CoreCommandPayload.TweenAudioVolumePayload,
                Wire.CoreCommandKind.TimeWait => Wire.CoreCommandPayload.WaitPayload,
                Wire.CoreCommandKind.OperationCancel =>
                    Wire.CoreCommandPayload.CancelOperationPayload,
                Wire.CoreCommandKind.InputSetEnabled =>
                    Wire.CoreCommandPayload.SetInputEnabledPayload,
                Wire.CoreCommandKind.InputSetPointerEvents =>
                    Wire.CoreCommandPayload.PointerEventsPayload,
                Wire.CoreCommandKind.InputSetGlobalKeys =>
                    Wire.CoreCommandPayload.GlobalKeysPayload,
                Wire.CoreCommandKind.InputSetController =>
                    Wire.CoreCommandPayload.ControllerInputSettings,
                Wire.CoreCommandKind.ControllerVibrate =>
                    Wire.CoreCommandPayload.ControllerVibrationPayload,
                Wire.CoreCommandKind.DebugUi => Wire.CoreCommandPayload.DebugUiPayload,
                Wire.CoreCommandKind.VisualElementCreate =>
                    Wire.CoreCommandPayload.VisualElementCreatePayload,
                Wire.CoreCommandKind.VisualElementUpdate =>
                    Wire.CoreCommandPayload.VisualElementUpdatePayload,
                Wire.CoreCommandKind.VisualElementDestroy =>
                    Wire.CoreCommandPayload.VisualElementDestroyPayload,
                Wire.CoreCommandKind.VisualElementPerformAction =>
                    Wire.CoreCommandPayload.VisualElementActionPayload,
                Wire.CoreCommandKind.MotionValue => Wire.CoreCommandPayload.MotionValueOperation,
                Wire.CoreCommandKind.MotionValuePlayback =>
                    Wire.CoreCommandPayload.MotionValuePlaybackOperation,
                Wire.CoreCommandKind.MotionPlayback =>
                    Wire.CoreCommandPayload.MotionPlaybackOperation,
                Wire.CoreCommandKind.MotionControlledClock =>
                    Wire.CoreCommandPayload.MotionControlledClockOperation,
                Wire.CoreCommandKind.MotionControl =>
                    Wire.CoreCommandPayload.MotionControlOperation,
                Wire.CoreCommandKind.MotionScope => Wire.CoreCommandPayload.MotionScopeOperation,
                Wire.CoreCommandKind.MotionDragControl =>
                    Wire.CoreCommandPayload.MotionDragControlOperation,
                Wire.CoreCommandKind.GeometryObservationUpdate =>
                    Wire.CoreCommandPayload.GeometryObservationUpdate,
                Wire.CoreCommandKind.AccessibilityUpdate =>
                    Wire.CoreCommandPayload.AccessibilityUpdate,
                _ => throw new InvalidDataException($"Unknown core command kind {kind}."),
            };

        private static bool Known<T>(T value, T maximum)
            where T : struct, System.Enum =>
            System.Convert.ToUInt64(value) <= System.Convert.ToUInt64(maximum);

        private static bool Finite(float value) => !float.IsNaN(value) && !float.IsInfinity(value);

        private static bool Finite(double value) =>
            !double.IsNaN(value) && !double.IsInfinity(value);
    }
}

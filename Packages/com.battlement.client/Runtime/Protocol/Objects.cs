#nullable enable

using System;
using System.Collections.Generic;
using Newtonsoft.Json;

namespace Battlement
{
    /// <summary>One additively loaded Addressable content-scene instance.</summary>
    /// <param name="Id">Identity of this scene instance within the session.</param>
    /// <param name="Address">Prepared Addressables scene address to load.</param>
    public sealed record BattlementScene(
        [property: JsonProperty("scene_id")] SceneId Id,
        SceneAddress Address
    );

    /// <summary>A complete game object from a snapshot or object-create command.</summary>
    /// <param name="Id">Session-unique identity of the game object.</param>
    /// <param name="Kind">Kind-specific object content and component state.</param>
    /// <param name="ParentScene">Scene that owns the game object.</param>
    /// <param name="ParentId">Optional parent game object in the same scene.</param>
    /// <param name="IsActive">The object's activation value.</param>
    /// <param name="LocalTransform">Local transform relative to the parent or placement.</param>
    /// <param name="PointerEvents">Unique pointer events enabled for this object.</param>
    /// <param name="DragMode">Local pointer-following behavior, or null when not draggable.</param>
    public sealed record BattlementGameObject(
        [property: JsonProperty("object_id")] ObjectId Id,
        GameObjectKind Kind,
        ParentScene ParentScene,
        ObjectId? ParentId,
        [property: JsonProperty("active")] bool IsActive,
        LocalTransform LocalTransform,
        IReadOnlyList<PointerEvent> PointerEvents,
        DragMode? DragMode = null
    )
    {
        public BattlementGameObject(ObjectId id, GameObjectKind kind)
            : this(
                id,
                kind,
                new ParentScene.Primary(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>(),
                null
            ) { }
    }

    /// <summary>The scene container that owns a game object.</summary>
    public abstract record ParentScene
    {
        private ParentScene() { }

        /// <summary>The primary content scene at the time the object is created.</summary>
        public sealed record Primary : ParentScene;

        /// <summary>A specific loaded content-scene instance.</summary>
        public sealed record Specific(SceneId SceneId) : ParentScene;

        /// <summary>The bootstrap-scene container for objects that survive scene unloads.</summary>
        public sealed record Persistent : ParentScene;
    }

    /// <summary>The concrete content created for a game object.</summary>
    public abstract record GameObjectKind
    {
        private GameObjectKind() { }

        /// <summary>
        /// Configures the native host of a Battlement-owned UI Toolkit document,
        /// independently from the logical element hierarchy.
        /// </summary>
        /// <param name="RootId">Identity linking the host to its logical document root.</param>
        /// <param name="PanelSettings">
        /// Private runtime panel rendering and scaling settings.
        /// </param>
        /// <param name="Position">Whether the document uses layout or absolute positioning.</param>
        /// <param name="WorldSpaceSizeMode">
        /// Whether world-space dimensions are fixed or derived dynamically.
        /// </param>
        /// <param name="WorldSpaceSize">Dimensions used for a fixed world-space document.</param>
        /// <param name="PivotReferenceSize">
        /// Geometry used as the reference frame for the world-space pivot.
        /// </param>
        /// <param name="Pivot">Point on the document aligned with the host transform.</param>
        /// <param name="SortingOrder">
        /// Draw priority among panels in the same context; larger values render above smaller ones.
        /// </param>
        public sealed record UiDocumentState(
            ObjectId RootId,
            PanelSettingsValue? PanelSettings = null,
            DocumentPosition Position = DocumentPosition.Relative,
            WorldSpaceSizeMode WorldSpaceSizeMode = WorldSpaceSizeMode.Fixed,
            ScreenSize? WorldSpaceSize = null,
            PivotReferenceSize PivotReferenceSize = PivotReferenceSize.BoundingBox,
            DocumentPivot Pivot = DocumentPivot.Center,
            int SortingOrder = 0
        ) : GameObjectKind;

        /// <summary>An empty game object.</summary>
        public sealed record Empty : GameObjectKind;

        /// <summary>A standard cube primitive.</summary>
        public sealed record Cube(IReadOnlyList<MaterialAssignment> Materials) : GameObjectKind
        {
            public Cube()
                : this(Array.Empty<MaterialAssignment>()) { }
        }

        /// <summary>A standard sphere primitive.</summary>
        public sealed record Sphere(IReadOnlyList<MaterialAssignment> Materials) : GameObjectKind
        {
            public Sphere()
                : this(Array.Empty<MaterialAssignment>()) { }
        }

        /// <summary>A standard capsule primitive.</summary>
        public sealed record Capsule(IReadOnlyList<MaterialAssignment> Materials) : GameObjectKind
        {
            public Capsule()
                : this(Array.Empty<MaterialAssignment>()) { }
        }

        /// <summary>A standard cylinder primitive.</summary>
        public sealed record Cylinder(IReadOnlyList<MaterialAssignment> Materials) : GameObjectKind
        {
            public Cylinder()
                : this(Array.Empty<MaterialAssignment>()) { }
        }

        /// <summary>A standard plane primitive.</summary>
        public sealed record Plane(IReadOnlyList<MaterialAssignment> Materials) : GameObjectKind
        {
            public Plane()
                : this(Array.Empty<MaterialAssignment>()) { }
        }

        /// <summary>A standard quad primitive.</summary>
        public sealed record Quad(IReadOnlyList<MaterialAssignment> Materials) : GameObjectKind
        {
            public Quad()
                : this(Array.Empty<MaterialAssignment>()) { }
        }

        /// <summary>A Battlement-owned image quad.</summary>
        public sealed record Image([property: JsonProperty("image")] ImageState State)
            : GameObjectKind;

        /// <summary>A world-space TextMesh Pro object.</summary>
        public sealed record Text([property: JsonProperty("text")] TextState State)
            : GameObjectKind;

        /// <summary>A standard camera.</summary>
        public sealed record Camera([property: JsonProperty("camera")] CameraState State)
            : GameObjectKind;

        /// <summary>A standard light.</summary>
        public sealed record Light([property: JsonProperty("light")] LightState State)
            : GameObjectKind;

        /// <summary>An instance of a prepared prefab.</summary>
        /// <param name="Address">Prepared prefab address.</param>
        /// <param name="Materials">Ordered material assignments with unique renderer slots.</param>
        /// <param name="Animator">Stable Animator state, when the prefab has an Animator.</param>
        public sealed record Prefab(
            PrefabAddress Address,
            IReadOnlyList<MaterialAssignment> Materials,
            AnimatorState? Animator = null
        ) : GameObjectKind
        {
            public Prefab(PrefabAddress address)
                : this(address, Array.Empty<MaterialAssignment>()) { }
        }
    }

    /// <summary>One prepared material assigned to a prefab renderer slot.</summary>
    /// <param name="Slot">Zero-based index in the renderer's material array.</param>
    /// <param name="Address">Prepared material address assigned to the slot.</param>
    public sealed record MaterialAssignment(uint Slot, MaterialAddress Address);

    /// <summary>Complete state for a Battlement-owned image quad.</summary>
    /// <param name="Texture">Prepared texture address.</param>
    /// <param name="Width">Positive world-space width around a centered pivot.</param>
    /// <param name="Height">Positive world-space height around a centered pivot.</param>
    /// <param name="Fit">How the texture fits the requested dimensions.</param>
    /// <param name="Tint">Linear RGB tint; opacity is controlled separately.</param>
    /// <param name="Opacity">Opacity in the inclusive range [0, 1].</param>
    /// <param name="FacesCamera">Whether the image rotates to face the input camera.</param>
    public sealed record ImageState(
        TextureAddress Texture,
        double Width,
        double Height,
        ImageFit Fit,
        RgbColor Tint,
        double Opacity,
        [property: JsonProperty("face_camera")] bool FacesCamera
    )
    {
        public ImageState(TextureAddress texture, double width, double height)
            : this(texture, width, height, ImageFit.Stretch, RgbColor.White, 1, false) { }
    }

    /// <summary>Complete state for a world-space TextMesh Pro object.</summary>
    /// <param name="Text">Displayed text content.</param>
    /// <param name="Font">Prepared TextMesh Pro font address.</param>
    /// <param name="Size">Positive world-space text size.</param>
    /// <param name="Color">Linear text color.</param>
    /// <param name="HorizontalAlignment">Horizontal text alignment.</param>
    /// <param name="VerticalAlignment">Vertical text alignment.</param>
    /// <param name="WrapWidth">Positive wrapping width; null disables wrapping.</param>
    /// <param name="IsRichText">Whether TextMesh Pro rich-text tags are interpreted.</param>
    /// <param name="FacesCamera">Whether the text rotates to face the input camera.</param>
    public sealed record TextState(
        string Text,
        TextMeshProFontAddress Font,
        double Size,
        Color Color,
        [property: JsonProperty("horizontal")] HorizontalAlignment HorizontalAlignment,
        [property: JsonProperty("vertical")] VerticalAlignment VerticalAlignment,
        double? WrapWidth,
        [property: JsonProperty("rich_text")] bool IsRichText,
        [property: JsonProperty("face_camera")] bool FacesCamera
    )
    {
        public TextState(string text, TextMeshProFontAddress font)
            : this(
                text,
                font,
                1,
                Battlement.Color.White,
                HorizontalAlignment.Center,
                VerticalAlignment.Middle,
                null,
                false,
                false
            ) { }
    }

    /// <summary>Complete state for a standard camera.</summary>
    /// <param name="IsEnabled">Whether the Camera component is enabled.</param>
    /// <param name="Projection">Perspective or orthographic projection.</param>
    /// <param name="FieldOfView">Perspective vertical field of view in degrees.</param>
    /// <param name="OrthographicSize">Positive orthographic half-height.</param>
    /// <param name="NearClip">Positive near clipping distance.</param>
    /// <param name="FarClip">Far clipping distance, greater than the near distance.</param>
    /// <param name="ClearMode">Camera clear behavior.</param>
    /// <param name="ClearColor">Linear color used by solid-color clearing.</param>
    public sealed record CameraState(
        [property: JsonProperty("enabled")] bool IsEnabled,
        CameraProjection Projection,
        double FieldOfView,
        double OrthographicSize,
        [property: JsonProperty("near")] double NearClip,
        [property: JsonProperty("far")] double FarClip,
        CameraClearMode ClearMode,
        Color ClearColor
    )
    {
        public CameraState()
            : this(
                true,
                CameraProjection.Perspective,
                60,
                5,
                0.3,
                1000,
                CameraClearMode.Skybox,
                Battlement.Color.Black
            ) { }
    }

    /// <summary>Complete state for a standard light.</summary>
    /// <param name="IsEnabled">Whether the Light component is enabled.</param>
    /// <param name="Type">Directional, point, or spot behavior.</param>
    /// <param name="Color">Linear light color.</param>
    /// <param name="Intensity">Nonnegative light intensity.</param>
    /// <param name="Range">Positive range for point and spot lights.</param>
    /// <param name="OuterSpotAngle">Outer spot angle in degrees.</param>
    /// <param name="InnerSpotAngle">Inner spot angle in degrees.</param>
    /// <param name="Shadows">Shadow rendering mode.</param>
    public sealed record LightState(
        [property: JsonProperty("enabled")] bool IsEnabled,
        [property: JsonProperty("light_type")] LightType Type,
        Color Color,
        double Intensity,
        double Range,
        double OuterSpotAngle,
        double InnerSpotAngle,
        ShadowMode Shadows
    )
    {
        public LightState()
            : this(true, LightType.Point, Battlement.Color.White, 1, 10, 30, 0, ShadowMode.None) { }
    }

    /// <summary>Stable Animator state reconstructed by a snapshot.</summary>
    /// <param name="State">Animator state name to play.</param>
    /// <param name="Layer">Nonnegative Animator layer index.</param>
    /// <param name="NormalizedStartTime">Normalized starting time in [0, 1].</param>
    /// <param name="BoolParameters">Persistent boolean parameters by name.</param>
    /// <param name="IntParameters">Persistent signed integer parameters by name.</param>
    /// <param name="FloatParameters">Persistent finite floating parameters by name.</param>
    /// <param name="Speed">Nonnegative Animator playback speed.</param>
    public sealed record AnimatorState(
        string State,
        uint Layer,
        double NormalizedStartTime,
        IReadOnlyDictionary<string, bool> BoolParameters,
        IReadOnlyDictionary<string, int> IntParameters,
        IReadOnlyDictionary<string, double> FloatParameters,
        double Speed
    )
    {
        public AnimatorState(string state)
            : this(
                state,
                0,
                0,
                new Dictionary<string, bool>(),
                new Dictionary<string, int>(),
                new Dictionary<string, double>(),
                1
            ) { }
    }
}

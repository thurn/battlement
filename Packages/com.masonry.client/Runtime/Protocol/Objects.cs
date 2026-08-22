#nullable enable

using System;
using System.Collections.Generic;

namespace Masonry
{
    /// <summary>One additively loaded Addressable content-scene instance.</summary>
    /// <param name="Id">Identity of this scene instance within the session.</param>
    /// <param name="Address">Prepared Addressables scene address to load.</param>
    public sealed record MasonryScene(SceneId Id, SceneAddress Address);

    /// <summary>A complete game object from a snapshot or object-create command.</summary>
    /// <param name="Id">Session-unique identity of the game object.</param>
    /// <param name="Kind">Kind-specific object content and component state.</param>
    /// <param name="ParentScene">Scene that owns the game object.</param>
    /// <param name="ParentId">Optional parent game object in the same scene.</param>
    /// <param name="IsActive">The object's activation value.</param>
    /// <param name="LocalTransform">Local transform relative to the parent or placement.</param>
    /// <param name="PointerEvents">Unique pointer events enabled for this object.</param>
    /// <param name="DragMode">Local pointer-following behavior, or null when not draggable.</param>
    public sealed record MasonryGameObject(
        ObjectId Id,
        GameObjectKind Kind,
        ParentScene ParentScene,
        ObjectId? ParentId,
        bool IsActive,
        LocalTransform LocalTransform,
        IReadOnlyList<PointerEvent> PointerEvents,
        DragMode? DragMode = null
    )
    {
        public MasonryGameObject(ObjectId id, GameObjectKind kind)
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

        /// <summary>A Masonry-owned image quad.</summary>
        public sealed record Image(ImageState State) : GameObjectKind;

        /// <summary>A world-space TextMesh Pro object.</summary>
        public sealed record Text(TextState State) : GameObjectKind;

        /// <summary>A standard camera.</summary>
        public sealed record Camera(CameraState State) : GameObjectKind;

        /// <summary>A standard light.</summary>
        public sealed record Light(LightState State) : GameObjectKind;

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

    /// <summary>Complete state for a Masonry-owned image quad.</summary>
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
        bool FacesCamera
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
        FontAddress Font,
        double Size,
        Color Color,
        HorizontalAlignment HorizontalAlignment,
        VerticalAlignment VerticalAlignment,
        double? WrapWidth,
        bool IsRichText,
        bool FacesCamera
    )
    {
        public TextState(string text, FontAddress font)
            : this(
                text,
                font,
                1,
                Masonry.Color.White,
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
        bool IsEnabled,
        CameraProjection Projection,
        double FieldOfView,
        double OrthographicSize,
        double NearClip,
        double FarClip,
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
                Masonry.Color.Black
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
        bool IsEnabled,
        LightType Type,
        Color Color,
        double Intensity,
        double Range,
        double OuterSpotAngle,
        double InnerSpotAngle,
        ShadowMode Shadows
    )
    {
        public LightState()
            : this(true, LightType.Point, Masonry.Color.White, 1, 10, 30, 0, ShadowMode.None) { }
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

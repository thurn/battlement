//! Snapshot descriptions of scenes and game objects.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    CameraClearMode, CameraProjection, Color, FontAddress, HorizontalAlignment, ImageFit,
    LightType, LocalTransform, MaterialAddress, ObjectId, PointerEvent, PrefabAddress, RgbColor,
    SceneAddress, SceneId, ShadowMode, TextureAddress, VerticalAlignment,
};

/// One additively loaded Addressable content-scene instance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Scene {
    /// Identity of this scene instance within the session.
    pub scene_id: SceneId,
    /// Prepared Addressables scene address to load.
    pub address: SceneAddress,
}

impl Scene {
    /// Creates a content-scene declaration.
    #[must_use]
    pub fn new(scene_id: SceneId, address: impl Into<SceneAddress>) -> Self {
        Self {
            scene_id,
            address: address.into(),
        }
    }
}

/// A complete game object from a snapshot or `object.create` command.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GameObject {
    /// Session-unique identity of the game object.
    pub object_id: ObjectId,
    /// Scene that owns the game object.
    pub parent_scene: ParentScene,
    /// Optional parent game object in the same scene.
    pub parent_id: Option<ObjectId>,
    /// The Unity GameObject's `activeSelf` value. Omission means `true`.
    ///
    /// This is the value passed to `GameObject.SetActive`; `activeInHierarchy`
    /// can still be false because of an inactive parent. It is separate from
    /// component `enabled` flags and from Unity's active Scene.
    pub active: bool,
    /// Local transform relative to the parent or placement container.
    pub local_transform: LocalTransform,
    /// Unique pointer events enabled for this object.
    pub pointer_events: Vec<PointerEvent>,
    /// Kind-specific object content and component state.
    pub kind: GameObjectKind,
}

impl GameObject {
    /// Creates a game object in the current primary scene with `activeSelf` set.
    #[must_use]
    pub fn new(object_id: ObjectId, kind: impl Into<GameObjectKind>) -> Self {
        Self {
            object_id,
            parent_scene: ParentScene::PrimaryScene,
            parent_id: None,
            active: true,
            local_transform: LocalTransform::default(),
            pointer_events: Vec::new(),
            kind: kind.into(),
        }
    }
}

/// The scene container that owns a game object.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub enum ParentScene {
    /// The primary content scene at the time the object is created.
    #[default]
    PrimaryScene,
    /// A specific loaded content-scene instance.
    Scene(SceneId),
    /// Masonry's bootstrap-scene container for objects that survive scene unloads.
    Persistent,
}

/// The concrete content created for a game object.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum GameObjectKind {
    /// An empty GameObject.
    Empty,
    /// Unity's standard cube primitive.
    Cube {
        /// Ordered prepared-material assignments with unique renderer slots.
        materials: Vec<MaterialAssignment>,
    },
    /// Unity's standard sphere primitive.
    Sphere {
        /// Ordered prepared-material assignments with unique renderer slots.
        materials: Vec<MaterialAssignment>,
    },
    /// Unity's standard capsule primitive.
    Capsule {
        /// Ordered prepared-material assignments with unique renderer slots.
        materials: Vec<MaterialAssignment>,
    },
    /// Unity's standard cylinder primitive.
    Cylinder {
        /// Ordered prepared-material assignments with unique renderer slots.
        materials: Vec<MaterialAssignment>,
    },
    /// Unity's standard plane primitive.
    Plane {
        /// Ordered prepared-material assignments with unique renderer slots.
        materials: Vec<MaterialAssignment>,
    },
    /// Unity's standard quad primitive.
    Quad {
        /// Ordered prepared-material assignments with unique renderer slots.
        materials: Vec<MaterialAssignment>,
    },
    /// A Masonry-owned image quad.
    Image {
        /// Complete image component state.
        image: ImageState,
    },
    /// A world-space TextMesh Pro object.
    Text {
        /// Complete text component state.
        text: TextState,
    },
    /// A standard Unity camera.
    Camera {
        /// Complete camera component state.
        camera: CameraState,
    },
    /// A standard Unity light.
    Light {
        /// Complete light component state.
        light: LightState,
    },
    /// An instance of a prepared prefab.
    Prefab {
        /// Prepared prefab address.
        address: PrefabAddress,
        /// Ordered prepared-material assignments with unique renderer slots.
        materials: Vec<MaterialAssignment>,
        /// Stable Animator state, when the prefab has an Animator.
        animator: Option<AnimatorState>,
    },
}

impl GameObjectKind {
    /// Creates a prefab instance without material or Animator overrides.
    #[must_use]
    pub fn prefab(address: impl Into<PrefabAddress>) -> Self {
        Self::Prefab {
            address: address.into(),
            materials: Vec::new(),
            animator: None,
        }
    }

    /// Creates a cube without material overrides.
    #[must_use]
    pub fn cube() -> Self {
        Self::Cube {
            materials: Vec::new(),
        }
    }

    /// Creates a sphere without material overrides.
    #[must_use]
    pub fn sphere() -> Self {
        Self::Sphere {
            materials: Vec::new(),
        }
    }

    /// Creates a capsule without material overrides.
    #[must_use]
    pub fn capsule() -> Self {
        Self::Capsule {
            materials: Vec::new(),
        }
    }

    /// Creates a cylinder without material overrides.
    #[must_use]
    pub fn cylinder() -> Self {
        Self::Cylinder {
            materials: Vec::new(),
        }
    }

    /// Creates a plane without material overrides.
    #[must_use]
    pub fn plane() -> Self {
        Self::Plane {
            materials: Vec::new(),
        }
    }

    /// Creates a quad without material overrides.
    #[must_use]
    pub fn quad() -> Self {
        Self::Quad {
            materials: Vec::new(),
        }
    }
}

impl From<ImageState> for GameObjectKind {
    fn from(image: ImageState) -> Self {
        Self::Image { image }
    }
}

impl From<TextState> for GameObjectKind {
    fn from(text: TextState) -> Self {
        Self::Text { text }
    }
}

impl From<CameraState> for GameObjectKind {
    fn from(camera: CameraState) -> Self {
        Self::Camera { camera }
    }
}

impl From<LightState> for GameObjectKind {
    fn from(light: LightState) -> Self {
        Self::Light { light }
    }
}

/// One prepared material assigned to a prefab renderer slot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MaterialAssignment {
    /// Zero-based index in the renderer's shared-material array.
    pub slot: u32,
    /// Prepared material address assigned to the slot.
    pub address: MaterialAddress,
}

impl MaterialAssignment {
    /// Creates one renderer material-slot assignment.
    #[must_use]
    pub fn new(slot: u32, address: impl Into<MaterialAddress>) -> Self {
        Self {
            slot,
            address: address.into(),
        }
    }
}

/// Complete state for a Masonry-owned image quad.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ImageState {
    /// Prepared texture address.
    pub texture: TextureAddress,
    /// Positive world-space width around a centered pivot.
    pub width: f64,
    /// Positive world-space height around a centered pivot.
    pub height: f64,
    /// How the texture fits the requested dimensions.
    pub fit: ImageFit,
    /// Linear RGB tint; opacity is controlled separately.
    pub tint: RgbColor,
    /// Opacity in the inclusive range `[0, 1]`.
    pub opacity: f64,
    /// Whether the image rotates to face the input camera.
    pub face_camera: bool,
}

impl ImageState {
    /// Creates opaque, untinted image state using stretch fitting.
    #[must_use]
    pub fn new(texture: impl Into<TextureAddress>, width: f64, height: f64) -> Self {
        Self {
            texture: texture.into(),
            width,
            height,
            fit: ImageFit::Stretch,
            tint: RgbColor::WHITE,
            opacity: 1.0,
            face_camera: false,
        }
    }
}

/// Complete state for a world-space TextMesh Pro object.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TextState {
    /// Displayed text content.
    pub text: String,
    /// Prepared TextMesh Pro font address.
    pub font: FontAddress,
    /// Positive world-space text size.
    pub size: f64,
    /// Linear text color.
    pub color: Color,
    /// Horizontal text alignment.
    pub horizontal: HorizontalAlignment,
    /// Vertical text alignment.
    pub vertical: VerticalAlignment,
    /// Positive wrapping width; [`None`] disables wrapping.
    pub wrap_width: Option<f64>,
    /// Whether TextMesh Pro rich-text tags are interpreted.
    pub rich_text: bool,
    /// Whether the text rotates to face the input camera.
    pub face_camera: bool,
}

impl TextState {
    /// Creates centered, unwrapped, opaque-white world-text state at size one.
    #[must_use]
    pub fn new(text: impl Into<String>, font: impl Into<FontAddress>) -> Self {
        Self {
            text: text.into(),
            font: font.into(),
            size: 1.0,
            color: Color::WHITE,
            horizontal: HorizontalAlignment::Center,
            vertical: VerticalAlignment::Middle,
            wrap_width: None,
            rich_text: false,
            face_camera: false,
        }
    }
}

/// Complete state for a standard Unity camera.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct CameraState {
    /// Whether the Camera component is enabled.
    pub enabled: bool,
    /// Perspective or orthographic projection.
    pub projection: CameraProjection,
    /// Perspective vertical field of view in degrees, strictly between 1 and 179.
    pub field_of_view: f64,
    /// Positive orthographic half-height.
    pub orthographic_size: f64,
    /// Positive near clipping distance.
    pub near: f64,
    /// Far clipping distance, which must be greater than `near`.
    pub far: f64,
    /// Camera clear behavior.
    pub clear_mode: CameraClearMode,
    /// Linear color used by solid-color clearing.
    pub clear_color: Color,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            enabled: true,
            projection: CameraProjection::Perspective,
            field_of_view: 60.0,
            orthographic_size: 5.0,
            near: 0.3,
            far: 1000.0,
            clear_mode: CameraClearMode::Skybox,
            clear_color: Color::BLACK,
        }
    }
}

impl CameraState {
    /// Creates camera state with its defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Complete state for a standard Unity light.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct LightState {
    /// Whether the Light component is enabled.
    pub enabled: bool,
    /// Directional, point, or spot behavior.
    pub light_type: LightType,
    /// Linear light color.
    pub color: Color,
    /// Nonnegative light intensity.
    pub intensity: f64,
    /// Positive range for point and spot lights.
    pub range: f64,
    /// Outer spot angle in degrees, strictly between 0 and 179.
    pub outer_spot_angle: f64,
    /// Inner spot angle in degrees, between zero and `outer_spot_angle`.
    pub inner_spot_angle: f64,
    /// Shadow rendering mode.
    pub shadows: ShadowMode,
}

impl Default for LightState {
    fn default() -> Self {
        Self {
            enabled: true,
            light_type: LightType::Point,
            color: Color::WHITE,
            intensity: 1.0,
            range: 10.0,
            outer_spot_angle: 30.0,
            inner_spot_angle: 0.0,
            shadows: ShadowMode::None,
        }
    }
}

impl LightState {
    /// Creates light state with its defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Stable Animator state reconstructed by a snapshot.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AnimatorState {
    /// Animator state name to play.
    pub state: String,
    /// Nonnegative Animator layer index.
    pub layer: u32,
    /// Normalized starting time in the inclusive range `[0, 1]`.
    pub normalized_start_time: f64,
    /// Persistent boolean parameters, ordered by name for deterministic output.
    pub bool_parameters: BTreeMap<String, bool>,
    /// Persistent signed 32-bit integer parameters, ordered by name.
    pub int_parameters: BTreeMap<String, i32>,
    /// Persistent finite floating-point parameters, ordered by name.
    pub float_parameters: BTreeMap<String, f64>,
    /// Nonnegative Animator playback speed.
    pub speed: f64,
}

impl AnimatorState {
    /// Creates stable Animator state on layer zero at normalized time zero.
    #[must_use]
    pub fn new(state: impl Into<String>) -> Self {
        Self {
            state: state.into(),
            layer: 0,
            normalized_start_time: 0.0,
            bool_parameters: BTreeMap::new(),
            int_parameters: BTreeMap::new(),
            float_parameters: BTreeMap::new(),
            speed: 1.0,
        }
    }
}

//! Snapshot descriptions of scenes and game objects.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    CameraClearMode, CameraProjection, Color, FontAddress, HorizontalAlignment, ImageFit,
    LightType, LocalTransform, MaterialAddress, ObjectId, PointerEvent, PrefabAddress, RgbColor,
    SceneAddress, SceneId, ShadowMode, TextureAddress, VerticalAlignment,
};

/// One additively loaded Addressable content-scene instance.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
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
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct GameObject {
    /// Session-unique identity of the game object.
    pub object_id: ObjectId,
    /// Scene container that owns the game object.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub placement: GameObjectPlacement,
    /// Optional parent game object in the same placement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<ObjectId>,
    /// The Unity GameObject's `activeSelf` value. Omission means `true`.
    ///
    /// This is the value passed to `GameObject.SetActive`; `activeInHierarchy`
    /// can still be false because of an inactive parent. It is separate from
    /// component `enabled` flags and from Unity's active Scene.
    #[serde(
        default = "crate::serialization::default_true",
        skip_serializing_if = "crate::serialization::is_true"
    )]
    pub active: bool,
    /// Local transform relative to the parent or placement container.
    #[serde(
        default,
        skip_serializing_if = "crate::serialization::is_default_transform"
    )]
    pub local_transform: LocalTransform,
    /// Unique pointer events enabled for this object.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("uniqueItems" = true))]
    pub pointer_events: Vec<PointerEvent>,
    /// Kind-specific object content and component state.
    #[serde(flatten)]
    pub kind: GameObjectKind,
}

impl GameObject {
    /// Creates a game object in the current primary scene with `activeSelf` set.
    #[must_use]
    pub fn new(object_id: ObjectId, kind: GameObjectKind) -> Self {
        Self {
            object_id,
            placement: GameObjectPlacement::PrimaryScene,
            parent_id: None,
            active: true,
            local_transform: LocalTransform::default(),
            pointer_events: Vec::new(),
            kind,
        }
    }
}

/// The scene container that owns a game object.
#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub enum GameObjectPlacement {
    /// The primary content scene at the time the object is created.
    #[default]
    PrimaryScene,
    /// A specific loaded content-scene instance.
    Scene(SceneId),
    /// Masonry's bootstrap-scene container for objects that survive scene unloads.
    Persistent,
}

/// The concrete content created for a game object.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum GameObjectKind {
    /// An empty GameObject.
    Empty,
    /// Unity's standard cube primitive.
    Cube {
        /// Ordered prepared-material assignments with unique renderer slots.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        materials: Vec<MaterialAssignment>,
    },
    /// Unity's standard sphere primitive.
    Sphere {
        /// Ordered prepared-material assignments with unique renderer slots.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        materials: Vec<MaterialAssignment>,
    },
    /// Unity's standard capsule primitive.
    Capsule {
        /// Ordered prepared-material assignments with unique renderer slots.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        materials: Vec<MaterialAssignment>,
    },
    /// Unity's standard cylinder primitive.
    Cylinder {
        /// Ordered prepared-material assignments with unique renderer slots.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        materials: Vec<MaterialAssignment>,
    },
    /// Unity's standard plane primitive.
    Plane {
        /// Ordered prepared-material assignments with unique renderer slots.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        materials: Vec<MaterialAssignment>,
    },
    /// Unity's standard quad primitive.
    Quad {
        /// Ordered prepared-material assignments with unique renderer slots.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
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
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        materials: Vec<MaterialAssignment>,
        /// Stable Animator state, when the prefab has an Animator.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        animator: Option<AnimatorState>,
    },
}

impl GameObjectKind {
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

/// One prepared material assigned to a prefab renderer slot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
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
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ImageState {
    /// Prepared texture address.
    pub texture: TextureAddress,
    /// Positive world-space width around a centered pivot.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub width: f64,
    /// Positive world-space height around a centered pivot.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub height: f64,
    /// How the texture fits the requested dimensions.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub fit: ImageFit,
    /// Linear RGB tint; opacity is controlled separately.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub tint: RgbColor,
    /// Opacity in the inclusive range `[0, 1]`.
    #[serde(
        default = "crate::serialization::default_one",
        skip_serializing_if = "crate::serialization::is_one"
    )]
    #[schemars(range(min = 0.0, max = 1.0))]
    pub opacity: f64,
    /// Whether the image rotates to face the input camera.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
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
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TextState {
    /// Displayed text content.
    #[schemars(length(max = 65_536))]
    pub text: String,
    /// Prepared TextMesh Pro font address.
    pub font: FontAddress,
    /// Positive world-space text size.
    #[serde(
        default = "crate::serialization::default_one",
        skip_serializing_if = "crate::serialization::is_one"
    )]
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub size: f64,
    /// Linear text color.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub color: Color,
    /// Horizontal text alignment.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub horizontal: HorizontalAlignment,
    /// Vertical text alignment.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub vertical: VerticalAlignment,
    /// Whether wrapping is enabled.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub wrapping: bool,
    /// Positive wrap width, required when wrapping is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub wrap_width: Option<f64>,
    /// Whether TextMesh Pro rich-text tags are interpreted.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub rich_text: bool,
    /// Whether the text rotates to face the input camera.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
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
            wrapping: false,
            wrap_width: None,
            rich_text: false,
            face_camera: false,
        }
    }
}

/// Complete state for a standard Unity camera.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CameraState {
    /// Whether the Camera component is enabled.
    #[serde(
        default = "crate::serialization::default_true",
        skip_serializing_if = "crate::serialization::is_true"
    )]
    pub enabled: bool,
    /// Perspective or orthographic projection.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub projection: CameraProjection,
    /// Perspective vertical field of view in degrees, strictly between 1 and 179.
    #[serde(
        default = "crate::serialization::default_field_of_view",
        skip_serializing_if = "crate::serialization::is_default_field_of_view"
    )]
    #[schemars(range(min = 1.0, max = 179.0))]
    #[schemars(
        extend("exclusiveMinimum" = 1.0),
        extend("exclusiveMaximum" = 179.0)
    )]
    pub field_of_view: f64,
    /// Positive orthographic half-height.
    #[serde(
        default = "crate::serialization::default_orthographic_size",
        skip_serializing_if = "crate::serialization::is_default_orthographic_size"
    )]
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub orthographic_size: f64,
    /// Positive near clipping distance.
    #[serde(
        default = "crate::serialization::default_near_clip",
        skip_serializing_if = "crate::serialization::is_default_near_clip"
    )]
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub near: f64,
    /// Far clipping distance, which must be greater than `near`.
    #[serde(
        default = "crate::serialization::default_far_clip",
        skip_serializing_if = "crate::serialization::is_default_far_clip"
    )]
    #[schemars(range(min = 0.0))]
    pub far: f64,
    /// Camera clear behavior.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub clear_mode: CameraClearMode,
    /// Linear color used by solid-color clearing.
    #[serde(
        default = "crate::serialization::default_black",
        skip_serializing_if = "crate::serialization::is_default_black"
    )]
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

/// Complete state for a standard Unity light.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct LightState {
    /// Whether the Light component is enabled.
    #[serde(
        default = "crate::serialization::default_true",
        skip_serializing_if = "crate::serialization::is_true"
    )]
    pub enabled: bool,
    /// Directional, point, or spot behavior.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub light_type: LightType,
    /// Linear light color.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub color: Color,
    /// Nonnegative light intensity.
    #[serde(
        default = "crate::serialization::default_one",
        skip_serializing_if = "crate::serialization::is_one"
    )]
    #[schemars(range(min = 0.0))]
    pub intensity: f64,
    /// Positive range for point and spot lights.
    #[serde(
        default = "crate::serialization::default_light_range",
        skip_serializing_if = "crate::serialization::is_default_light_range"
    )]
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub range: f64,
    /// Outer spot angle in degrees, strictly between 0 and 179.
    #[serde(
        default = "crate::serialization::default_outer_spot_angle",
        skip_serializing_if = "crate::serialization::is_default_outer_spot_angle"
    )]
    #[schemars(range(min = 0.0, max = 179.0))]
    #[schemars(
        extend("exclusiveMinimum" = 0.0),
        extend("exclusiveMaximum" = 179.0)
    )]
    pub outer_spot_angle: f64,
    /// Inner spot angle in degrees, between zero and `outer_spot_angle`.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    #[schemars(range(min = 0.0, max = 179.0))]
    pub inner_spot_angle: f64,
    /// Shadow rendering mode.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
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

/// Stable Animator state reconstructed by a snapshot.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AnimatorState {
    /// Animator state name to play.
    #[schemars(length(max = 65_536))]
    pub state: String,
    /// Nonnegative Animator layer index.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub layer: u32,
    /// Normalized starting time in the inclusive range `[0, 1]`.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    #[schemars(range(min = 0.0, max = 1.0))]
    pub normalized_start_time: f64,
    /// Persistent boolean parameters, ordered by name for deterministic output.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bool_parameters: BTreeMap<String, bool>,
    /// Persistent signed 32-bit integer parameters, ordered by name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub int_parameters: BTreeMap<String, i32>,
    /// Persistent finite floating-point parameters, ordered by name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub float_parameters: BTreeMap<String, f64>,
    /// Nonnegative Animator playback speed.
    #[serde(
        default = "crate::serialization::default_one",
        skip_serializing_if = "crate::serialization::is_one"
    )]
    #[schemars(range(min = 0.0))]
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prefab_materials_are_explicit_ordered_records() {
        let object = GameObject::new(
            "cc847d6e-1468-42c6-9bec-9af5b5aa5c03".parse().unwrap(),
            GameObjectKind::Prefab {
                address: "mygame/pieces/knight".into(),
                materials: vec![
                    MaterialAssignment {
                        slot: 1,
                        address: "mygame/materials/trim".into(),
                    },
                    MaterialAssignment {
                        slot: 0,
                        address: "mygame/materials/body".into(),
                    },
                ],
                animator: None,
            },
        );

        assert_eq!(
            serde_json::to_value(object).unwrap()["materials"],
            json!([
                { "slot": 1, "address": "mygame/materials/trim" },
                { "slot": 0, "address": "mygame/materials/body" }
            ])
        );
    }
}

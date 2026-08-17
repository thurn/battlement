//! Snapshot descriptions of scenes and runtime object roots.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    CameraClearMode, CameraProjection, Color, HorizontalAlignment, ImageFit, LightType,
    LocalTransform, ObjectId, PointerEvent, RgbColor, SceneId, ShadowMode, VerticalAlignment,
    default_true, is_false, is_one_f64, is_true, is_zero_u32,
};

/// One additively loaded Addressable content-scene instance.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    /// Identity of this scene instance within the session.
    pub scene_id: SceneId,
    /// Prepared Addressables scene address to load.
    #[schemars(length(max = 65_536))]
    pub address: String,
}

impl Scene {
    /// Creates a content-scene declaration.
    #[must_use]
    pub fn new(scene_id: SceneId, address: impl Into<String>) -> Self {
        Self {
            scene_id,
            address: address.into(),
        }
    }
}

/// A complete runtime object root from a snapshot or `object.create` command.
///
/// Placement defaults to the primary content scene. Setting `persistent` to
/// `true` instead places the root in Masonry's bootstrap-scene container;
/// `scene_id` and `persistent: true` are mutually exclusive.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeObject {
    /// Session-unique identity of the runtime object root.
    pub object_id: ObjectId,
    /// Explicit content-scene placement, or the primary scene when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene_id: Option<SceneId>,
    /// Whether the object belongs to Masonry's persistent bootstrap container.
    #[serde(default, skip_serializing_if = "is_false")]
    pub persistent: bool,
    /// Optional runtime-object parent in the same placement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<ObjectId>,
    /// Whether the root is active. Omission means `true`.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub active: bool,
    /// Local transform relative to the parent or placement container.
    #[serde(default, skip_serializing_if = "is_default_transform")]
    pub local_transform: LocalTransform,
    /// Unique pointer events enabled for this object.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("uniqueItems" = true))]
    pub pointer_events: Vec<PointerEvent>,
    /// Kind-specific object content and component state.
    #[serde(flatten)]
    pub kind: RuntimeObjectKind,
}

impl RuntimeObject {
    /// Creates an active object in the current primary scene with identity transform.
    #[must_use]
    pub fn new(object_id: ObjectId, kind: RuntimeObjectKind) -> Self {
        Self {
            object_id,
            scene_id: None,
            persistent: false,
            parent_id: None,
            active: true,
            local_transform: LocalTransform::default(),
            pointer_events: Vec::new(),
            kind,
        }
    }
}

fn is_default_transform(value: &LocalTransform) -> bool {
    *value == LocalTransform::default()
}

/// The concrete content created for a runtime object root.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RuntimeObjectKind {
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
        #[schemars(length(max = 65_536))]
        address: String,
        /// Ordered prepared-material assignments with unique renderer slots.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        materials: Vec<MaterialAssignment>,
        /// Stable root Animator state, when the prefab has an Animator.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        animator: Option<AnimatorState>,
    },
}

impl RuntimeObjectKind {
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

/// One prepared material assigned to a prefab root-renderer slot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct MaterialAssignment {
    /// Zero-based index in the root renderer's shared-material array.
    pub slot: u32,
    /// Prepared material address assigned to the slot.
    #[schemars(length(max = 65_536))]
    pub address: String,
}

impl MaterialAssignment {
    /// Creates one root-renderer material-slot assignment.
    #[must_use]
    pub fn new(slot: u32, address: impl Into<String>) -> Self {
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
    #[schemars(length(max = 65_536))]
    pub texture: String,
    /// Positive world-space width around a centered pivot.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub width: f64,
    /// Positive world-space height around a centered pivot.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub height: f64,
    /// How the texture fits the requested dimensions.
    #[serde(default, skip_serializing_if = "is_default_fit")]
    pub fit: ImageFit,
    /// Linear RGB tint; opacity is controlled separately.
    #[serde(default, skip_serializing_if = "is_white_rgb")]
    pub tint: RgbColor,
    /// Opacity in the inclusive range `[0, 1]`.
    #[serde(default = "one", skip_serializing_if = "is_one_f64")]
    #[schemars(range(min = 0.0, max = 1.0))]
    pub opacity: f64,
    /// Whether the image rotates to face the input camera.
    #[serde(default, skip_serializing_if = "is_false")]
    pub face_camera: bool,
}

impl ImageState {
    /// Creates opaque, untinted image state using stretch fitting.
    #[must_use]
    pub fn new(texture: impl Into<String>, width: f64, height: f64) -> Self {
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

fn one() -> f64 {
    1.0
}

fn is_default_fit(value: &ImageFit) -> bool {
    *value == ImageFit::default()
}

fn is_white_rgb(value: &RgbColor) -> bool {
    *value == RgbColor::WHITE
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
    #[schemars(length(max = 65_536))]
    pub font: String,
    /// Positive world-space text size.
    #[serde(default = "one", skip_serializing_if = "is_one_f64")]
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub size: f64,
    /// Linear text color.
    #[serde(default, skip_serializing_if = "is_white")]
    pub color: Color,
    /// Horizontal text alignment.
    #[serde(default, skip_serializing_if = "is_default_horizontal")]
    pub horizontal: HorizontalAlignment,
    /// Vertical text alignment.
    #[serde(default, skip_serializing_if = "is_default_vertical")]
    pub vertical: VerticalAlignment,
    /// Whether wrapping is enabled.
    #[serde(default, skip_serializing_if = "is_false")]
    pub wrapping: bool,
    /// Positive wrap width, required when wrapping is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub wrap_width: Option<f64>,
    /// Whether TextMesh Pro rich-text tags are interpreted.
    #[serde(default, skip_serializing_if = "is_false")]
    pub rich_text: bool,
    /// Whether the text rotates to face the input camera.
    #[serde(default, skip_serializing_if = "is_false")]
    pub face_camera: bool,
}

impl TextState {
    /// Creates centered, unwrapped, opaque-white world-text state at size one.
    #[must_use]
    pub fn new(text: impl Into<String>, font: impl Into<String>) -> Self {
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

fn is_white(value: &Color) -> bool {
    *value == Color::WHITE
}

fn is_default_horizontal(value: &HorizontalAlignment) -> bool {
    *value == HorizontalAlignment::default()
}

fn is_default_vertical(value: &VerticalAlignment) -> bool {
    *value == VerticalAlignment::default()
}

/// Complete state for a standard Unity camera.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CameraState {
    /// Whether the Camera component is enabled.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,
    /// Perspective or orthographic projection.
    #[serde(default, skip_serializing_if = "is_default_projection")]
    pub projection: CameraProjection,
    /// Perspective vertical field of view in degrees, strictly between 1 and 179.
    #[serde(
        default = "default_field_of_view",
        skip_serializing_if = "is_default_field_of_view"
    )]
    #[schemars(range(min = 1.0, max = 179.0))]
    #[schemars(
        extend("exclusiveMinimum" = 1.0),
        extend("exclusiveMaximum" = 179.0)
    )]
    pub field_of_view: f64,
    /// Positive orthographic half-height.
    #[serde(
        default = "default_orthographic_size",
        skip_serializing_if = "is_default_orthographic_size"
    )]
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub orthographic_size: f64,
    /// Positive near clipping distance.
    #[serde(default = "default_near", skip_serializing_if = "is_default_near")]
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub near: f64,
    /// Far clipping distance, which must be greater than `near`.
    #[serde(default = "default_far", skip_serializing_if = "is_default_far")]
    #[schemars(range(min = 0.0))]
    pub far: f64,
    /// Camera clear behavior.
    #[serde(default, skip_serializing_if = "is_default_clear_mode")]
    pub clear_mode: CameraClearMode,
    /// Linear color used by solid-color clearing.
    #[serde(default = "black", skip_serializing_if = "is_black")]
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

fn default_field_of_view() -> f64 {
    60.0
}
fn is_default_field_of_view(value: &f64) -> bool {
    *value == 60.0
}
fn default_orthographic_size() -> f64 {
    5.0
}
fn is_default_orthographic_size(value: &f64) -> bool {
    *value == 5.0
}
fn default_near() -> f64 {
    0.3
}
fn is_default_near(value: &f64) -> bool {
    *value == 0.3
}
fn default_far() -> f64 {
    1000.0
}
fn is_default_far(value: &f64) -> bool {
    *value == 1000.0
}
fn black() -> Color {
    Color::BLACK
}
fn is_black(value: &Color) -> bool {
    *value == Color::BLACK
}
fn is_default_projection(value: &CameraProjection) -> bool {
    *value == CameraProjection::default()
}
fn is_default_clear_mode(value: &CameraClearMode) -> bool {
    *value == CameraClearMode::default()
}

/// Complete state for a standard Unity light.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct LightState {
    /// Whether the Light component is enabled.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,
    /// Directional, point, or spot behavior.
    #[serde(default, skip_serializing_if = "is_default_light_type")]
    pub light_type: LightType,
    /// Linear light color.
    #[serde(default, skip_serializing_if = "is_white")]
    pub color: Color,
    /// Nonnegative light intensity.
    #[serde(default = "one", skip_serializing_if = "is_one_f64")]
    #[schemars(range(min = 0.0))]
    pub intensity: f64,
    /// Positive range for point and spot lights.
    #[serde(default = "default_range", skip_serializing_if = "is_default_range")]
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub range: f64,
    /// Outer spot angle in degrees, strictly between 0 and 179.
    #[serde(
        default = "default_outer_spot_angle",
        skip_serializing_if = "is_default_outer_spot_angle"
    )]
    #[schemars(range(min = 0.0, max = 179.0))]
    #[schemars(
        extend("exclusiveMinimum" = 0.0),
        extend("exclusiveMaximum" = 179.0)
    )]
    pub outer_spot_angle: f64,
    /// Inner spot angle in degrees, between zero and `outer_spot_angle`.
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[schemars(range(min = 0.0, max = 179.0))]
    pub inner_spot_angle: f64,
    /// Shadow rendering mode.
    #[serde(default, skip_serializing_if = "is_default_shadow_mode")]
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

fn default_range() -> f64 {
    10.0
}
fn is_default_range(value: &f64) -> bool {
    *value == 10.0
}
fn default_outer_spot_angle() -> f64 {
    30.0
}
fn is_default_outer_spot_angle(value: &f64) -> bool {
    *value == 30.0
}
fn is_zero_f64(value: &f64) -> bool {
    *value == 0.0
}
fn is_default_light_type(value: &LightType) -> bool {
    *value == LightType::default()
}
fn is_default_shadow_mode(value: &ShadowMode) -> bool {
    *value == ShadowMode::default()
}

/// Stable root Animator state reconstructed by a snapshot.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AnimatorState {
    /// Animator state name to play.
    #[schemars(length(max = 65_536))]
    pub state: String,
    /// Nonnegative Animator layer index.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub layer: u32,
    /// Normalized starting time in the inclusive range `[0, 1]`.
    #[serde(default, skip_serializing_if = "is_zero_f64")]
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
    #[serde(default = "one", skip_serializing_if = "is_one_f64")]
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
        let object = RuntimeObject::new(
            "cc847d6e-1468-42c6-9bec-9af5b5aa5c03".parse().unwrap(),
            RuntimeObjectKind::Prefab {
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

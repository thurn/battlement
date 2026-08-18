//! Validation for protocol invariants that JSON Schema cannot express.

use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
};

use serde::Serialize;
use serde_json::Value;

use crate::*;

/// A schema-inexpressible protocol invariant that was violated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationError {
    /// A floating-point value was NaN or infinite.
    NonFiniteNumber,
    /// A prepared asset address appeared more than once.
    DuplicatePreparedAddress,
    /// A scene identifier or address appeared more than once.
    DuplicateScene,
    /// The primary scene selection was missing or did not name a listed scene.
    InvalidPrimaryScene,
    /// A game-object identifier appeared more than once.
    DuplicateObject,
    /// A scene, object, asset, or input-camera reference was invalid.
    InvalidReference,
    /// The game-object parent graph was cyclic, too deep, or crossed placements.
    InvalidHierarchy,
    /// A quaternion had zero length.
    ZeroQuaternion,
    /// A camera's far clipping distance was not greater than its near distance.
    InvalidClipping,
    /// A spot light's inner angle exceeded its outer angle.
    InvalidSpotAngles,
    /// A tween used an invalid duration and repetition combination.
    InvalidRepeat,
    /// Blocking behavior was incompatible with an infinite operation.
    InvalidBlocking,
    /// Camera clear color presence did not match the clear mode.
    InvalidClearColor,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NonFiniteNumber => "all numeric values must be finite",
            Self::DuplicatePreparedAddress => "prepared asset addresses must be unique",
            Self::DuplicateScene => "scene identifiers and addresses must be unique",
            Self::InvalidPrimaryScene => "the primary scene must name a listed scene",
            Self::DuplicateObject => "game-object identifiers must be unique",
            Self::InvalidReference => "a protocol reference is missing or has the wrong kind",
            Self::InvalidHierarchy => "the game-object hierarchy is invalid",
            Self::ZeroQuaternion => "quaternions must have nonzero length",
            Self::InvalidClipping => "camera far clipping must be greater than near clipping",
            Self::InvalidSpotAngles => "spot inner angle must not exceed outer angle",
            Self::InvalidRepeat => "zero-duration tweens cannot repeat",
            Self::InvalidBlocking => "the blocking flag is incompatible with this operation",
            Self::InvalidClearColor => "clear color must be present only for solid-color clearing",
        })
    }
}

impl Error for ValidationError {}

/// Validates protocol rules that cannot be represented by the generated schema.
pub trait Validate {
    /// Returns the first schema-inexpressible contract violation.
    fn validate(&self) -> Result<(), ValidationError>;
}

impl Validate for Snapshot {
    fn validate(&self) -> Result<(), ValidationError> {
        reject_non_finite(self, false)?;

        let prepared = prepared_assets(&self.prepared_assets)?;
        let primary_scene = validate_scenes(&self.scenes, self.primary_scene_id, &prepared)?;
        let objects = object_index(&self.objects)?;

        for object in &self.objects {
            if let ParentScene::Scene(scene_id) = object.parent_scene
                && !self.scenes.iter().any(|scene| scene.scene_id == scene_id)
            {
                return Err(ValidationError::InvalidReference);
            }
            validate_object(object, &prepared)?;
            validate_parent_chain(object, primary_scene, &objects)?;
        }

        let input_camera = objects
            .get(&self.input_camera_id)
            .ok_or(ValidationError::InvalidReference)?;
        match &input_camera.kind {
            GameObjectKind::Camera { camera } if camera.enabled => {}
            _ => return Err(ValidationError::InvalidReference),
        }
        validate_active_chain(input_camera, &objects)?;

        Ok(())
    }
}

impl Validate for Command {
    fn validate(&self) -> Result<(), ValidationError> {
        reject_non_finite(self, true)?;

        match &self.body {
            CommandBody::AssetsReplaceSet(value) => {
                prepared_assets(&value.payload.assets)?;
            }
            CommandBody::ObjectCreate(value) => {
                validate_object_shape(&value.payload.object)?;
                validate_material_slots(materials(&value.payload.object.kind))?;
            }
            CommandBody::CameraSetClipping(value) => {
                validate_clipping(value.payload.near, value.payload.far)?;
            }
            CommandBody::CameraSetClear(value) => {
                let has_color = value.payload.clear_color.is_some();
                if matches!(value.payload.clear_mode, CameraClearMode::SolidColor) != has_color {
                    return Err(ValidationError::InvalidClearColor);
                }
            }
            CommandBody::LightSetSpotAngle(value) => {
                validate_spot_angles(
                    value.payload.inner_spot_angle,
                    value.payload.outer_spot_angle,
                )?;
            }
            CommandBody::TransformSetLocalRotation(value)
            | CommandBody::TransformSetWorldRotation(value) => {
                validate_quaternion(value.payload.rotation)?;
            }
            CommandBody::TransformTweenLocalRotation(value)
            | CommandBody::TransformTweenWorldRotation(value) => {
                validate_quaternion(value.payload.rotation)?;
            }
            CommandBody::AudioPlay(value) if value.payload.r#loop && self.blocking => {
                return Err(ValidationError::InvalidBlocking);
            }
            CommandBody::ParticlePlay(_) if self.blocking => {
                return Err(ValidationError::InvalidBlocking);
            }
            CommandBody::TimeWait(_) if !self.blocking => {
                return Err(ValidationError::InvalidBlocking);
            }
            _ => {}
        }

        if let Some(tween) = command_tween(&self.body) {
            validate_tween(*tween, self.blocking)?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum PreparedKind {
    Scene,
    Prefab,
    ParticleEffect,
    Material,
    Texture,
    AudioClip,
    Font,
}

fn prepared_assets(
    assets: &[PreparedAsset],
) -> Result<HashMap<&str, PreparedKind>, ValidationError> {
    let mut prepared = HashMap::with_capacity(assets.len());
    for asset in assets {
        let (address, kind) = match asset {
            PreparedAsset::Scene(value) => (value.as_str(), PreparedKind::Scene),
            PreparedAsset::Prefab(value) => (value.as_str(), PreparedKind::Prefab),
            PreparedAsset::ParticleEffect(value) => (value.as_str(), PreparedKind::ParticleEffect),
            PreparedAsset::Material(value) => (value.as_str(), PreparedKind::Material),
            PreparedAsset::Texture(value) => (value.as_str(), PreparedKind::Texture),
            PreparedAsset::AudioClip(value) => (value.as_str(), PreparedKind::AudioClip),
            PreparedAsset::Font(value) => (value.as_str(), PreparedKind::Font),
        };
        if prepared.insert(address, kind).is_some() {
            return Err(ValidationError::DuplicatePreparedAddress);
        }
    }
    Ok(prepared)
}

fn validate_scenes(
    scenes: &[Scene],
    selected: Option<SceneId>,
    prepared: &HashMap<&str, PreparedKind>,
) -> Result<SceneId, ValidationError> {
    let first = scenes.first().ok_or(ValidationError::InvalidPrimaryScene)?;
    let mut ids = HashSet::with_capacity(scenes.len());
    let mut addresses = HashSet::with_capacity(scenes.len());
    for scene in scenes {
        if !ids.insert(scene.scene_id) || !addresses.insert(scene.address.as_str()) {
            return Err(ValidationError::DuplicateScene);
        }
        require_asset(prepared, scene.address.as_str(), PreparedKind::Scene)?;
    }

    let primary = match (scenes.len(), selected) {
        (1, None) => first.scene_id,
        (_, Some(scene_id)) if ids.contains(&scene_id) => scene_id,
        _ => return Err(ValidationError::InvalidPrimaryScene),
    };
    Ok(primary)
}

fn object_index(objects: &[GameObject]) -> Result<HashMap<ObjectId, &GameObject>, ValidationError> {
    let mut index = HashMap::with_capacity(objects.len());
    for object in objects {
        if index.insert(object.object_id, object).is_some() {
            return Err(ValidationError::DuplicateObject);
        }
    }
    Ok(index)
}

fn validate_parent_chain(
    object: &GameObject,
    primary_scene: SceneId,
    objects: &HashMap<ObjectId, &GameObject>,
) -> Result<(), ValidationError> {
    let object_placement = placement(object, primary_scene);
    let mut visited = HashSet::new();
    let mut current = object;
    let mut depth = 0;

    while let Some(parent_id) = current.parent_id {
        if !visited.insert(current.object_id) {
            return Err(ValidationError::InvalidHierarchy);
        }
        let parent = objects
            .get(&parent_id)
            .ok_or(ValidationError::InvalidReference)?;
        if placement(parent, primary_scene) != object_placement {
            return Err(ValidationError::InvalidHierarchy);
        }
        depth += 1;
        if depth > 256 {
            return Err(ValidationError::InvalidHierarchy);
        }
        current = parent;
    }
    Ok(())
}

fn validate_active_chain(
    object: &GameObject,
    objects: &HashMap<ObjectId, &GameObject>,
) -> Result<(), ValidationError> {
    let mut current = object;
    loop {
        if !current.active {
            return Err(ValidationError::InvalidReference);
        }
        let Some(parent_id) = current.parent_id else {
            return Ok(());
        };
        current = objects
            .get(&parent_id)
            .ok_or(ValidationError::InvalidReference)?;
    }
}

fn placement(object: &GameObject, primary_scene: SceneId) -> Option<SceneId> {
    match object.parent_scene {
        ParentScene::PrimaryScene => Some(primary_scene),
        ParentScene::Scene(scene_id) => Some(scene_id),
        ParentScene::Persistent => None,
    }
}

fn validate_object(
    object: &GameObject,
    prepared: &HashMap<&str, PreparedKind>,
) -> Result<(), ValidationError> {
    validate_object_shape(object)?;

    match &object.kind {
        GameObjectKind::Image { image } => {
            require_asset(prepared, image.texture.as_str(), PreparedKind::Texture)?;
        }
        GameObjectKind::Text { text } => {
            require_asset(prepared, text.font.as_str(), PreparedKind::Font)?;
        }
        GameObjectKind::Prefab {
            address, materials, ..
        } => {
            require_asset(prepared, address.as_str(), PreparedKind::Prefab)?;
            validate_materials(materials, prepared)?;
        }
        GameObjectKind::Cube { materials }
        | GameObjectKind::Sphere { materials }
        | GameObjectKind::Capsule { materials }
        | GameObjectKind::Cylinder { materials }
        | GameObjectKind::Plane { materials }
        | GameObjectKind::Quad { materials } => validate_materials(materials, prepared)?,
        GameObjectKind::Empty | GameObjectKind::Camera { .. } | GameObjectKind::Light { .. } => {}
    }
    Ok(())
}

fn validate_object_shape(object: &GameObject) -> Result<(), ValidationError> {
    validate_quaternion(object.local_transform.rotation)?;
    match &object.kind {
        GameObjectKind::Text { .. } => Ok(()),
        GameObjectKind::Camera { camera } => validate_clipping(camera.near, camera.far),
        GameObjectKind::Light { light } => {
            validate_spot_angles(light.inner_spot_angle, light.outer_spot_angle)
        }
        _ => Ok(()),
    }
}

fn validate_materials(
    materials: &[MaterialAssignment],
    prepared: &HashMap<&str, PreparedKind>,
) -> Result<(), ValidationError> {
    validate_material_slots(materials)?;
    for material in materials {
        require_asset(prepared, material.address.as_str(), PreparedKind::Material)?;
    }
    Ok(())
}

fn validate_material_slots(materials: &[MaterialAssignment]) -> Result<(), ValidationError> {
    let mut slots = HashSet::with_capacity(materials.len());
    if materials.iter().all(|material| slots.insert(material.slot)) {
        Ok(())
    } else {
        Err(ValidationError::InvalidReference)
    }
}

fn materials(kind: &GameObjectKind) -> &[MaterialAssignment] {
    match kind {
        GameObjectKind::Cube { materials }
        | GameObjectKind::Sphere { materials }
        | GameObjectKind::Capsule { materials }
        | GameObjectKind::Cylinder { materials }
        | GameObjectKind::Plane { materials }
        | GameObjectKind::Quad { materials }
        | GameObjectKind::Prefab { materials, .. } => materials,
        _ => &[],
    }
}

fn require_asset(
    prepared: &HashMap<&str, PreparedKind>,
    address: &str,
    expected: PreparedKind,
) -> Result<(), ValidationError> {
    if prepared.get(address) == Some(&expected) {
        Ok(())
    } else {
        Err(ValidationError::InvalidReference)
    }
}

fn validate_quaternion(value: Quaternion) -> Result<(), ValidationError> {
    let squared_length =
        value.x * value.x + value.y * value.y + value.z * value.z + value.w * value.w;
    if squared_length > 0.0 {
        Ok(())
    } else {
        Err(ValidationError::ZeroQuaternion)
    }
}

fn validate_clipping(near: f64, far: f64) -> Result<(), ValidationError> {
    if far > near {
        Ok(())
    } else {
        Err(ValidationError::InvalidClipping)
    }
}

fn validate_spot_angles(inner: f64, outer: f64) -> Result<(), ValidationError> {
    if inner <= outer {
        Ok(())
    } else {
        Err(ValidationError::InvalidSpotAngles)
    }
}

fn validate_tween(tween: Tween, blocking: bool) -> Result<(), ValidationError> {
    if tween.duration_ms == 0 && !matches!(tween.repeat, TweenRepeat::Once) {
        return Err(ValidationError::InvalidRepeat);
    }
    if blocking && matches!(tween.repeat, TweenRepeat::Forever(_)) {
        return Err(ValidationError::InvalidBlocking);
    }
    Ok(())
}

fn command_tween(body: &CommandBody) -> Option<&Tween> {
    match body {
        CommandBody::TransformTweenLocalPosition(value)
        | CommandBody::TransformTweenWorldPosition(value) => Some(&value.payload.tween),
        CommandBody::TransformTweenLocalRotation(value)
        | CommandBody::TransformTweenWorldRotation(value) => Some(&value.payload.tween),
        CommandBody::TransformTweenLocalScale(value) => Some(&value.payload.tween),
        CommandBody::CameraTweenFieldOfView(value) => Some(&value.payload.tween),
        CommandBody::CameraTweenOrthographicSize(value) => Some(&value.payload.tween),
        CommandBody::LightTweenColor(value) | CommandBody::TextTweenColor(value) => {
            Some(&value.payload.tween)
        }
        CommandBody::LightTweenIntensity(value) => Some(&value.payload.tween),
        CommandBody::ImageTweenTint(value) => Some(&value.payload.tween),
        CommandBody::ImageTweenOpacity(value) => Some(&value.payload.tween),
        CommandBody::TextTweenSize(value) => Some(&value.payload.tween),
        CommandBody::AudioTweenVolume(value) => Some(&value.payload.tween),
        _ => None,
    }
}

fn reject_non_finite<T: Serialize>(
    value: &T,
    allow_null_parent: bool,
) -> Result<(), ValidationError> {
    let value = serde_json::to_value(value).map_err(|_| ValidationError::NonFiniteNumber)?;
    if has_unexpected_null(&value, allow_null_parent, None) {
        Err(ValidationError::NonFiniteNumber)
    } else {
        Ok(())
    }
}

fn has_unexpected_null(value: &Value, allow_null_parent: bool, key: Option<&str>) -> bool {
    match value {
        Value::Null => !(allow_null_parent && key == Some("parentId")),
        Value::Array(values) => values
            .iter()
            .any(|value| has_unexpected_null(value, allow_null_parent, None)),
        Value::Object(values) => values
            .iter()
            .any(|(key, value)| has_unexpected_null(value, allow_null_parent, Some(key.as_str()))),
        _ => false,
    }
}

//! Validation for protocol invariants.

use std::{
  collections::{HashMap, HashSet},
  error::Error,
  fmt,
};

use serde::{Serialize, ser};

use crate::*;

/// A protocol invariant that was violated.
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
  /// Controller settings or vibration intensity were outside protocol bounds.
  InvalidControllerInput,
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
      Self::InvalidControllerInput => "controller input settings are invalid",
    })
  }
}

impl Error for ValidationError {}

/// Validates protocol rules spanning multiple fields or records.
pub trait Validate {
  /// Returns the first contract violation.
  fn validate(&self) -> Result<(), ValidationError>;
}

impl Validate for Snapshot {
  fn validate(&self) -> Result<(), ValidationError> {
    reject_non_finite(self)?;

    let prepared = prepared_assets(&self.prepared_assets)?;
    let primary_scene = validate_scenes(&self.scenes, self.primary_scene_id, &prepared)?;
    let objects = object_index(&self.objects)?;
    let mut ui_identities = validate_documents(&self.ui).map_err(map_ui_error)?;
    validate_ui_assets(&self.ui, &prepared)?;
    validate_panel_input_configuration(&self.panel_input_configuration).map_err(map_ui_error)?;

    for document in &self.ui {
      let object = objects
        .get(&document.document_id)
        .ok_or(ValidationError::InvalidReference)?;
      validate_parent_chain(object, primary_scene, &objects)?;
      reject_nested_ui_document(object, &objects)?;
      match &object.kind {
        GameObjectKind::UiDocument(state) if state.root_id() == document.root_id => {
          validate_panel_settings(&state.panel_settings).map_err(map_ui_error)?;
          if let Some(target) = &state.panel_settings.target_texture {
            require_asset(&prepared, target.as_str(), PreparedKind::RenderTexture)?;
          }
          if state.world_space_size.width == 0 || state.world_space_size.height == 0 {
            return Err(ValidationError::InvalidReference);
          }
          let uses_default_world_geometry = state.world_space_size == ScreenSize::new(1920, 1080)
            && state.pivot_reference_size == PivotReferenceSize::BoundingBox
            && state.pivot == DocumentPivot::Center;
          match state.panel_settings.render_mode {
            PanelRenderMode::ScreenSpaceOverlay => {
              let uses_screen_space_defaults = state.position == DocumentPosition::Relative
                && state.world_space_size_mode == WorldSpaceSizeMode::Fixed;
              if !uses_screen_space_defaults || !uses_default_world_geometry {
                return Err(ValidationError::InvalidReference);
              }
            }
            PanelRenderMode::WorldSpace => {
              if state.world_space_size_mode == WorldSpaceSizeMode::Dynamic
                && state.world_space_size != ScreenSize::new(1920, 1080)
              {
                return Err(ValidationError::InvalidReference);
              }
            }
          }
        }
        _ => return Err(ValidationError::InvalidReference),
      }
      ui_identities.remove(&document.document_id);
    }

    for object in &self.objects {
      if matches!(object.kind, GameObjectKind::UiDocument(_))
        && !self
          .ui
          .iter()
          .any(|document| document.document_id == object.object_id)
      {
        return Err(ValidationError::InvalidReference);
      }
    }
    if ui_identities
      .iter()
      .any(|identity| objects.contains_key(identity))
    {
      return Err(ValidationError::DuplicateObject);
    }

    for object in &self.objects {
      if let ParentScene::Scene(scene_id) = object.parent_scene
        && !self.scenes.iter().any(|scene| scene.scene_id == scene_id)
      {
        return Err(ValidationError::InvalidReference);
      }
      validate_object(object, &prepared)?;
      validate_parent_chain(object, primary_scene, &objects)?;
    }

    if let Some(input_camera_id) = self.input_camera_id {
      let input_camera = objects
        .get(&input_camera_id)
        .ok_or(ValidationError::InvalidReference)?;
      match &input_camera.kind {
        GameObjectKind::Camera { camera } if camera.enabled => {}
        _ => return Err(ValidationError::InvalidReference),
      }
      validate_active_chain(input_camera, &objects)?;
    }

    if let Some(settings) = &self.controller_input {
      validate_controller_settings(settings)?;
    }

    Ok(())
  }
}

impl Validate for Command {
  fn validate(&self) -> Result<(), ValidationError> {
    reject_non_finite(self)?;

    match &self.body {
      CommandBody::AssetsReplaceSet(value) => {
        prepared_assets(&value.assets)?;
      }
      CommandBody::ObjectCreate(value) => {
        validate_object_shape(&value.object)?;
        validate_material_slots(materials(&value.object.kind))?;
      }
      CommandBody::CameraSetClipping(value) => {
        validate_clipping(value.near, value.far)?;
      }
      CommandBody::CameraSetClear(value) => {
        let has_color = value.clear_color.is_some();
        if matches!(value.clear_mode, CameraClearMode::SolidColor) != has_color {
          return Err(ValidationError::InvalidClearColor);
        }
      }
      CommandBody::LightSetSpotAngle(value) => {
        validate_spot_angles(value.inner_spot_angle, value.outer_spot_angle)?;
      }
      CommandBody::TransformSetLocalRotation(value)
      | CommandBody::TransformSetWorldRotation(value) => {
        validate_quaternion(value.payload.rotation)?;
      }
      CommandBody::TransformTweenLocalRotation(value)
      | CommandBody::TransformTweenWorldRotation(value) => {
        validate_quaternion(value.payload.rotation)?;
      }
      CommandBody::AudioPlay(value) if value.r#loop && self.blocking => {
        return Err(ValidationError::InvalidBlocking);
      }
      CommandBody::ParticlePlay(_) if self.blocking => {
        return Err(ValidationError::InvalidBlocking);
      }
      CommandBody::TimeWait(_) if !self.blocking => {
        return Err(ValidationError::InvalidBlocking);
      }
      CommandBody::InputSetController(value) => validate_controller_settings(value)?,
      CommandBody::VisualElementCreate(value) => {
        validate_create_subtree(&value.node).map_err(map_ui_error)?;
      }
      CommandBody::VisualElementUpdate(value) => {
        if let VisualElementUpdate::Properties { element, .. } = value.as_ref() {
          validate_element_update(element).map_err(map_ui_error)?;
        }
      }
      CommandBody::ControllerVibrate(value)
        if !(0.0..=1.0).contains(&value.low_frequency)
          || !(0.0..=1.0).contains(&value.high_frequency) =>
      {
        return Err(ValidationError::InvalidControllerInput);
      }
      CommandBody::Diagnostics(command) => {
        if !self.blocking || command.validate().is_err() {
          return Err(ValidationError::InvalidBlocking);
        }
      }
      _ => {}
    }

    if let Some(tween) = command_tween(&self.body) {
      validate_tween(*tween, self.blocking)?;
    }

    Ok(())
  }
}

fn validate_controller_settings(settings: &ControllerInputSettings) -> Result<(), ValidationError> {
  let invalid_dead_zone = settings
    .stick_dead_zone
    .is_some_and(|value| !(0.0..1.0).contains(&value));
  let invalid_repeat =
    settings.repeat_delay_ms == Some(0) || settings.repeat_interval_ms == Some(0);
  if invalid_dead_zone || invalid_repeat {
    return Err(ValidationError::InvalidControllerInput);
  }
  if settings.buttons.iter().collect::<HashSet<_>>().len() != settings.buttons.len() {
    return Err(ValidationError::InvalidControllerInput);
  }
  Ok(())
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum PreparedKind {
  Scene,
  Prefab,
  ParticleEffect,
  Material,
  Texture,
  Sprite,
  VectorImage,
  RenderTexture,
  AudioClip,
  TextMeshProFont,
  UiFont,
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
      PreparedAsset::Sprite(value) => (value.as_str(), PreparedKind::Sprite),
      PreparedAsset::VectorImage(value) => (value.as_str(), PreparedKind::VectorImage),
      PreparedAsset::RenderTexture(value) => (value.as_str(), PreparedKind::RenderTexture),
      PreparedAsset::AudioClip(value) => (value.as_str(), PreparedKind::AudioClip),
      PreparedAsset::TextMeshProFont(value) => (value.as_str(), PreparedKind::TextMeshProFont),
      PreparedAsset::UiFont(value) => (value.as_str(), PreparedKind::UiFont),
    };
    if prepared.insert(address, kind).is_some() {
      return Err(ValidationError::DuplicatePreparedAddress);
    }
  }
  Ok(prepared)
}

fn validate_ui_assets(
  documents: &[UiDocument],
  prepared: &HashMap<&str, PreparedKind>,
) -> Result<(), ValidationError> {
  for document in documents {
    validate_style_assets(&document.element.style, prepared)?;
    for child in &document.children {
      validate_ui_node_assets(child, prepared)?;
    }
  }
  Ok(())
}

fn validate_ui_node_assets(
  node: &UiNode,
  prepared: &HashMap<&str, PreparedKind>,
) -> Result<(), ValidationError> {
  if let UiElement::Image(image) = &node.element
    && let battlement_ui::Prop::Set(source) = &image.source
  {
    let (address, kind) = match source {
      ImageSource::Texture(value) => (value.as_str(), PreparedKind::Texture),
      ImageSource::Sprite(value) => (value.as_str(), PreparedKind::Sprite),
      ImageSource::VectorImage(value) => (value.as_str(), PreparedKind::VectorImage),
      ImageSource::RenderTexture(value) => (value.as_str(), PreparedKind::RenderTexture),
    };
    require_asset(prepared, address, kind)?;
  }
  if let UiElement::Button(button) = &node.element
    && let battlement_ui::Prop::Set(source) = &button.icon
  {
    let (address, kind) = match source {
      IconSource::Texture(value) => (value.as_str(), PreparedKind::Texture),
      IconSource::Sprite(value) => (value.as_str(), PreparedKind::Sprite),
      IconSource::VectorImage(value) => (value.as_str(), PreparedKind::VectorImage),
      IconSource::RenderTexture(value) => (value.as_str(), PreparedKind::RenderTexture),
    };
    require_asset(prepared, address, kind)?;
  }
  validate_style_assets(&node.element.visual_element().style, prepared)?;
  for child in &node.children {
    validate_ui_node_assets(child, prepared)?;
  }
  Ok(())
}

fn validate_style_assets(
  style: &battlement_ui::Style,
  prepared: &HashMap<&str, PreparedKind>,
) -> Result<(), ValidationError> {
  if let battlement_ui::Prop::Set(battlement_ui::StyleValue::Value(address)) =
    &style.unity_font_definition
  {
    require_asset(prepared, address.as_str(), PreparedKind::UiFont)?;
  }
  Ok(())
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

fn reject_nested_ui_document(
  object: &GameObject,
  objects: &HashMap<ObjectId, &GameObject>,
) -> Result<(), ValidationError> {
  let mut current = object;
  while let Some(parent_id) = current.parent_id {
    let parent = objects
      .get(&parent_id)
      .ok_or(ValidationError::InvalidReference)?;
    if matches!(parent.kind, GameObjectKind::UiDocument(_)) {
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
      require_asset(prepared, text.font.as_str(), PreparedKind::TextMeshProFont)?;
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
    GameObjectKind::Empty
    | GameObjectKind::UiDocument(_)
    | GameObjectKind::Camera { .. }
    | GameObjectKind::Light { .. } => {}
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

fn map_ui_error(value: UiValidationError) -> ValidationError {
  match value {
    UiValidationError::DuplicateObject => ValidationError::DuplicateObject,
    UiValidationError::InvalidReference | UiValidationError::InvalidProperty => {
      ValidationError::InvalidReference
    }
    UiValidationError::InvalidHierarchy => ValidationError::InvalidHierarchy,
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

fn reject_non_finite<T: Serialize>(value: &T) -> Result<(), ValidationError> {
  value.serialize(FiniteValueValidator)
}

/// Walks a Serde value solely to reject non-finite floating-point numbers.
///
/// This never produces bytes or chooses an encoding. Implementing
/// [`ser::Serializer`] gives validation complete coverage of nested protocol
/// values, including fields added later, without converting them into a
/// format-specific value tree or allocating an intermediate representation.
/// Serde requires serializers to describe every possible data-model shape;
/// most methods below therefore recurse into children or deliberately do
/// nothing, while `serialize_f32` and `serialize_f64` perform the only checks.
/// Explicitly listing every float field would be shorter here but would silently
/// miss new fields unless every future author remembered to update that list.
struct FiniteValueValidator;

macro_rules! ignore_scalar {
    ($($method:ident($type:ty)),+ $(,)?) => {
        $(
            fn $method(self, _value: $type) -> Result<Self::Ok, Self::Error> {
                Ok(())
            }
        )+
    };
}

impl ser::Error for ValidationError {
  fn custom<T>(_message: T) -> Self
  where
    T: fmt::Display,
  {
    Self::NonFiniteNumber
  }
}

impl ser::Serializer for FiniteValueValidator {
  type Error = ValidationError;
  type Ok = ();
  type SerializeMap = Self;
  type SerializeSeq = Self;
  type SerializeStruct = Self;
  type SerializeStructVariant = Self;
  type SerializeTuple = Self;
  type SerializeTupleStruct = Self;
  type SerializeTupleVariant = Self;

  ignore_scalar! {
      serialize_bool(bool),
      serialize_i8(i8),
      serialize_i16(i16),
      serialize_i32(i32),
      serialize_i64(i64),
      serialize_i128(i128),
      serialize_u8(u8),
      serialize_u16(u16),
      serialize_u32(u32),
      serialize_u64(u64),
      serialize_u128(u128),
      serialize_char(char),
  }

  fn serialize_f32(self, value: f32) -> Result<Self::Ok, Self::Error> {
    finite(value.is_finite())
  }

  fn serialize_f64(self, value: f64) -> Result<Self::Ok, Self::Error> {
    finite(value.is_finite())
  }

  fn serialize_str(self, _value: &str) -> Result<Self::Ok, Self::Error> {
    Ok(())
  }

  fn serialize_bytes(self, _value: &[u8]) -> Result<Self::Ok, Self::Error> {
    Ok(())
  }

  fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
    Ok(())
  }

  fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(self)
  }

  fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
    Ok(())
  }

  fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
    Ok(())
  }

  fn serialize_unit_variant(
    self,
    _name: &'static str,
    _variant_index: u32,
    _variant: &'static str,
  ) -> Result<Self::Ok, Self::Error> {
    Ok(())
  }

  fn serialize_newtype_struct<T>(
    self,
    _name: &'static str,
    value: &T,
  ) -> Result<Self::Ok, Self::Error>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(self)
  }

  fn serialize_newtype_variant<T>(
    self,
    _name: &'static str,
    _variant_index: u32,
    _variant: &'static str,
    value: &T,
  ) -> Result<Self::Ok, Self::Error>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(self)
  }

  fn serialize_seq(self, _length: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
    Ok(self)
  }

  fn serialize_tuple(self, _length: usize) -> Result<Self::SerializeTuple, Self::Error> {
    Ok(self)
  }

  fn serialize_tuple_struct(
    self,
    _name: &'static str,
    _length: usize,
  ) -> Result<Self::SerializeTupleStruct, Self::Error> {
    Ok(self)
  }

  fn serialize_tuple_variant(
    self,
    _name: &'static str,
    _variant_index: u32,
    _variant: &'static str,
    _length: usize,
  ) -> Result<Self::SerializeTupleVariant, Self::Error> {
    Ok(self)
  }

  fn serialize_map(self, _length: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
    Ok(self)
  }

  fn serialize_struct(
    self,
    _name: &'static str,
    _length: usize,
  ) -> Result<Self::SerializeStruct, Self::Error> {
    Ok(self)
  }

  fn serialize_struct_variant(
    self,
    _name: &'static str,
    _variant_index: u32,
    _variant: &'static str,
    _length: usize,
  ) -> Result<Self::SerializeStructVariant, Self::Error> {
    Ok(self)
  }
}

macro_rules! validate_collection {
  ($trait:ident, $method:ident) => {
    impl ser::$trait for FiniteValueValidator {
      type Error = ValidationError;
      type Ok = ();

      fn $method<T>(&mut self, value: &T) -> Result<(), Self::Error>
      where
        T: ?Sized + Serialize,
      {
        value.serialize(FiniteValueValidator)
      }

      fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
      }
    }
  };
}

validate_collection!(SerializeSeq, serialize_element);
validate_collection!(SerializeTuple, serialize_element);
validate_collection!(SerializeTupleStruct, serialize_field);
validate_collection!(SerializeTupleVariant, serialize_field);

impl ser::SerializeMap for FiniteValueValidator {
  type Error = ValidationError;
  type Ok = ();

  fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
  where
    T: ?Sized + Serialize,
  {
    key.serialize(FiniteValueValidator)
  }

  fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(FiniteValueValidator)
  }

  fn end(self) -> Result<Self::Ok, Self::Error> {
    Ok(())
  }
}

macro_rules! validate_struct {
  ($trait:ident) => {
    impl ser::$trait for FiniteValueValidator {
      type Error = ValidationError;
      type Ok = ();

      fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<(), Self::Error>
      where
        T: ?Sized + Serialize,
      {
        value.serialize(FiniteValueValidator)
      }

      fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
      }
    }
  };
}

validate_struct!(SerializeStruct);
validate_struct!(SerializeStructVariant);

fn finite(is_finite: bool) -> Result<(), ValidationError> {
  if is_finite {
    Ok(())
  } else {
    Err(ValidationError::NonFiniteNumber)
  }
}

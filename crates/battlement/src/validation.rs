//! Validation for protocol invariants.

use std::{
  collections::{HashMap, HashSet},
  error::Error,
  fmt,
};

use crate::*;

/// A protocol invariant that was violated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationError {
  /// A floating-point value was NaN or infinite.
  NonFiniteNumber,
  /// Shared audio gains were outside the inclusive range zero through one.
  InvalidAudioMix,
  /// The requested frame-rate ceiling was outside protocol bounds.
  InvalidFramePacing,
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
      Self::InvalidFramePacing => "frame-rate ceiling must be between 1 and 1000",
      Self::InvalidAudioMix => "audio mixer gains must be between zero and one",
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
    validate_snapshot_numbers(self)?;

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
      validate_world_motion_projection_camera(object, self.input_camera_id, &objects)?;
    }

    crate::material_validation::prepared(self)?;

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

fn validate_world_motion_projection_camera(
  object: &GameObject,
  input_camera_id: Option<ObjectId>,
  objects: &HashMap<ObjectId, &GameObject>,
) -> Result<(), ValidationError> {
  let Some(projection) = object
    .motion
    .as_ref()
    .and_then(|motion| motion.layout.as_ref())
    .and_then(|layout| layout.projection)
  else {
    return Ok(());
  };
  let camera_id = match projection.camera {
    MotionProjectionCamera::Input => input_camera_id.ok_or(ValidationError::InvalidReference)?,
    MotionProjectionCamera::Object(camera_id) => camera_id,
  };
  let camera = objects
    .get(&camera_id)
    .ok_or(ValidationError::InvalidReference)?;
  match &camera.kind {
    GameObjectKind::Camera { camera: state } if state.enabled => {
      validate_active_chain(camera, objects)
    }
    _ => Err(ValidationError::InvalidReference),
  }
}

impl Validate for Command {
  fn validate(&self) -> Result<(), ValidationError> {
    validate_command_numbers(&self.body)?;
    if let CommandBody::BoxHitRegionSetGeometry(value) = &self.body
      && !value.region.is_valid()
    {
      return Err(ValidationError::InvalidReference);
    }

    match &self.body {
      CommandBody::AssetsReplaceSet(value) => {
        prepared_assets(&value.assets)?;
      }
      CommandBody::MotionSetWorldDescriptor(value) => {
        crate::world_motion::validate(value.object_id, value.motion.as_deref())?;
      }
      CommandBody::RendererSetInstances(value) => {
        crate::material_validation::instances(&value.instances)?
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
      CommandBody::AudioSetMix(value)
        if [value.master, value.music, value.effects]
          .iter()
          .any(|gain| !(0.0..=1.0).contains(gain)) =>
      {
        return Err(ValidationError::InvalidAudioMix);
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
      CommandBody::ApplicationSetFramePacing(value)
        if !(1..=1000).contains(&value.maximum_frame_rate) =>
      {
        return Err(ValidationError::InvalidFramePacing);
      }
      CommandBody::Diagnostics(command) if !self.blocking || command.validate().is_err() => {
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
  Mesh,
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
      PreparedAsset::Mesh(value) => (value.as_str(), PreparedKind::Mesh),
      PreparedAsset::Scene(value) => (value.as_str(), PreparedKind::Scene),
      PreparedAsset::Prefab(value) => (value.as_str(), PreparedKind::Prefab),
      PreparedAsset::ParticleEffect(value) => (value.as_str(), PreparedKind::ParticleEffect),
      PreparedAsset::Material(value) => (value.as_str(), PreparedKind::Material),
      PreparedAsset::MaterialParameters { address, .. } => {
        (address.as_str(), PreparedKind::Material)
      }
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
  for m in &object.material_instances {
    require_asset(prepared, m.address.as_str(), PreparedKind::Material)?;
  }

  match &object.kind {
    GameObjectKind::Image { image } => {
      require_asset(prepared, image.texture.as_str(), PreparedKind::Texture)?;
    }
    GameObjectKind::Text { text } => {
      require_asset(prepared, text.font.as_str(), PreparedKind::TextMeshProFont)?;
    }
    GameObjectKind::Mesh { address, materials } => {
      require_asset(prepared, address.as_str(), PreparedKind::Mesh)?;
      validate_materials(materials, prepared)?;
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
    | GameObjectKind::BoxHitRegion { .. }
    | GameObjectKind::UiDocument(_)
    | GameObjectKind::Camera { .. }
    | GameObjectKind::Light { .. } => {}
  }
  Ok(())
}

fn validate_object_shape(object: &GameObject) -> Result<(), ValidationError> {
  crate::world_motion::validate(object.object_id, object.motion.as_deref())?;
  crate::material_validation::instances(&object.material_instances)?;
  validate_quaternion(object.local_transform.rotation)?;
  match &object.kind {
    GameObjectKind::BoxHitRegion { region } if !region.is_valid() => {
      Err(ValidationError::InvalidReference)
    }
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
    | GameObjectKind::Mesh { materials, .. }
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

fn validate_snapshot_numbers(snapshot: &Snapshot) -> Result<(), ValidationError> {
  for object in &snapshot.objects {
    validate_object_numbers(object)?;
  }
  Ok(())
}

fn validate_object_numbers(object: &GameObject) -> Result<(), ValidationError> {
  validate_transform(object.local_transform)?;
  match &object.kind {
    GameObjectKind::Image { image } => finite_all(&[
      image.width,
      image.height,
      image.tint.r,
      image.tint.g,
      image.tint.b,
      image.opacity,
    ]),
    GameObjectKind::Text { text } => {
      finite_all(&[
        text.size,
        text.color.r,
        text.color.g,
        text.color.b,
        text.color.a,
      ])?;
      finite(text.wrap_width.is_none_or(f64::is_finite))
    }
    GameObjectKind::Camera { camera } => finite_all(&[
      camera.field_of_view,
      camera.orthographic_size,
      camera.near,
      camera.far,
      camera.clear_color.r,
      camera.clear_color.g,
      camera.clear_color.b,
      camera.clear_color.a,
    ]),
    GameObjectKind::Light { light } => finite_all(&[
      light.color.r,
      light.color.g,
      light.color.b,
      light.color.a,
      light.intensity,
      light.range,
      light.outer_spot_angle,
      light.inner_spot_angle,
    ]),
    GameObjectKind::Prefab {
      animator: Some(animator),
      ..
    } => {
      finite_all(&[animator.normalized_start_time, animator.speed])?;
      finite(
        animator
          .float_parameters
          .values()
          .all(|value| value.is_finite()),
      )
    }
    _ => Ok(()),
  }
}

#[allow(clippy::too_many_lines)]
fn validate_command_numbers(body: &CommandBody) -> Result<(), ValidationError> {
  match body {
    CommandBody::ObjectCreate(value) => validate_object_numbers(&value.object),
    CommandBody::TransformSetLocalPosition(value)
    | CommandBody::TransformSetWorldPosition(value) => validate_vector(value.payload.position),
    CommandBody::TransformTweenLocalPosition(value)
    | CommandBody::TransformTweenWorldPosition(value) => validate_vector(value.payload.position),
    CommandBody::TransformSetLocalRotation(value)
    | CommandBody::TransformSetWorldRotation(value) => {
      validate_quaternion_numbers(value.payload.rotation)
    }
    CommandBody::TransformTweenLocalRotation(value)
    | CommandBody::TransformTweenWorldRotation(value) => {
      validate_quaternion_numbers(value.payload.rotation)
    }
    CommandBody::TransformSetLocalScale(value) => validate_vector(value.payload.scale),
    CommandBody::TransformTweenLocalScale(value) => validate_vector(value.payload.scale),
    CommandBody::CameraSetPerspective(value) => finite(value.payload.field_of_view.is_finite()),
    CommandBody::CameraTweenFieldOfView(value) => finite(value.payload.field_of_view.is_finite()),
    CommandBody::CameraSetOrthographic(value) => finite(value.payload.size.is_finite()),
    CommandBody::CameraTweenOrthographicSize(value) => finite(value.payload.size.is_finite()),
    CommandBody::CameraSetClipping(value) => finite_all(&[value.near, value.far]),
    CommandBody::CameraSetClear(value) => value.clear_color.map_or(Ok(()), validate_color),
    CommandBody::LightSetColor(value) | CommandBody::TextSetColor(value) => {
      validate_color(value.payload.color)
    }
    CommandBody::LightTweenColor(value) | CommandBody::TextTweenColor(value) => {
      validate_color(value.payload.color)
    }
    CommandBody::LightSetIntensity(value) => finite(value.payload.intensity.is_finite()),
    CommandBody::LightTweenIntensity(value) => finite(value.payload.intensity.is_finite()),
    CommandBody::LightSetRange(value) => finite(value.range.is_finite()),
    CommandBody::LightSetSpotAngle(value) => {
      finite_all(&[value.inner_spot_angle, value.outer_spot_angle])
    }
    CommandBody::ImageSetSize(value) => finite_all(&[value.width, value.height]),
    CommandBody::ImageSetTint(value) => validate_rgb(value.payload.tint),
    CommandBody::ImageTweenTint(value) => validate_rgb(value.payload.tint),
    CommandBody::ImageSetOpacity(value) => finite(value.payload.opacity.is_finite()),
    CommandBody::ImageTweenOpacity(value) => finite(value.payload.opacity.is_finite()),
    CommandBody::TextSetSize(value) => finite(value.payload.size.is_finite()),
    CommandBody::TextTweenSize(value) => finite(value.payload.size.is_finite()),
    CommandBody::TextSetWrapping(value) => finite(value.wrap_width.is_none_or(f64::is_finite)),
    CommandBody::AnimatorPlay(value) => finite(value.normalized_start_time.is_finite()),
    CommandBody::AnimatorCrossFade(value) => finite(value.normalized_start_time.is_finite()),
    CommandBody::AnimatorSetFloat(value) => finite(value.value.is_finite()),
    CommandBody::AnimatorSetSpeed(value) => finite(value.speed.is_finite()),
    CommandBody::ParticleSpawn(value) => match value.location {
      ParticleSpawnLocation::WorldPosition(position) => validate_vector(position),
      ParticleSpawnLocation::GameObject(_) => Ok(()),
    },
    CommandBody::AudioPlay(value) => finite_all(&[value.volume, value.pitch]),
    CommandBody::AudioSetMix(value) => finite_all(&[value.master, value.music, value.effects]),
    CommandBody::AudioSetVolume(value) => finite(value.payload.volume.is_finite()),
    CommandBody::AudioTweenVolume(value) => finite(value.payload.volume.is_finite()),
    CommandBody::InputSetController(value) => {
      finite(value.stick_dead_zone.is_none_or(f64::is_finite))
    }
    CommandBody::ControllerVibrate(value) => {
      finite_all(&[value.low_frequency, value.high_frequency])
    }
    CommandBody::VisualElementPerformAction(value) => match &value.action {
      VisualElementAction::ParticleStreaks { streaks } => finite(streaks.iter().all(|streak| {
        streak.rotation.is_finite()
          && streak.origin.iter().all(|value| value.is_finite())
          && streak.travel.iter().all(|value| value.is_finite())
          && streak.size.iter().all(|value| value.is_finite())
          && [
            streak.color.r,
            streak.color.g,
            streak.color.b,
            streak.color.a,
          ]
          .into_iter()
          .all(f64::is_finite)
      })),
      _ => Ok(()),
    },
    CommandBody::MotionValue(value) => validate_motion_value_command(&value.command),
    CommandBody::MotionValuePlayback(value) => validate_motion_playback_command(value.command),
    CommandBody::MotionPlayback(value) => validate_motion_playback_command(value.command),
    CommandBody::MotionControl(value) => validate_motion_control_command(&value.command),
    CommandBody::MotionScope(value) => validate_motion_scope_command(&value.command),
    CommandBody::MotionDragControl(value) => {
      finite(value.point.x.is_finite() && value.point.y.is_finite())
    }
    CommandBody::AccessibilityUpdate(value) => validate_accessibility_numbers(value),
    _ => Ok(()),
  }
}

fn validate_motion_value_command(value: &MotionValueCommand) -> Result<(), ValidationError> {
  match value {
    MotionValueCommand::Set(value) | MotionValueCommand::Jump(value) => {
      finite(value.validate().is_ok())
    }
    MotionValueCommand::Animate {
      target, transition, ..
    } => finite(target.validate().is_ok() && transition.validate().is_ok()),
    MotionValueCommand::Stop => Ok(()),
  }
}

fn validate_motion_playback_command(value: MotionPlaybackCommand) -> Result<(), ValidationError> {
  match value {
    MotionPlaybackCommand::SetSpeed { value } => finite(value.is_finite()),
    _ => Ok(()),
  }
}

fn validate_motion_control_command(value: &MotionControlCommand) -> Result<(), ValidationError> {
  match value {
    MotionControlCommand::Start { target, .. } | MotionControlCommand::Set(target) => {
      validate_motion_control_target(target)
    }
    MotionControlCommand::Stop | MotionControlCommand::Clear => Ok(()),
  }
}

fn validate_motion_control_target(value: &MotionControlTarget) -> Result<(), ValidationError> {
  match value {
    MotionControlTarget::Target(value) => finite(value.validate().is_ok()),
    MotionControlTarget::Variant(_) => Ok(()),
  }
}

fn validate_motion_scope_command(value: &MotionScopeCommand) -> Result<(), ValidationError> {
  match value {
    MotionScopeCommand::Start { entries, .. } => {
      finite(crate::validate_motion_sequence(entries).is_ok())
    }
    MotionScopeCommand::Set { target, .. } => finite(target.validate().is_ok()),
    MotionScopeCommand::Stop(_) => Ok(()),
  }
}

fn validate_accessibility_numbers(value: &AccessibilityUpdate) -> Result<(), ValidationError> {
  let Some(snapshot) = &value.snapshot else {
    return Ok(());
  };
  finite(snapshot.nodes.iter().all(|node| {
    node.value.as_ref().is_none_or(|value| {
      [value.current, value.minimum, value.maximum]
        .into_iter()
        .all(f64::is_finite)
    })
  }))
}

fn validate_transform(value: LocalTransform) -> Result<(), ValidationError> {
  validate_vector(value.position)?;
  validate_quaternion_numbers(value.rotation)?;
  validate_vector(value.scale)
}

fn validate_vector(value: Vector3) -> Result<(), ValidationError> {
  finite_all(&[value.x, value.y, value.z])
}

fn validate_quaternion_numbers(value: Quaternion) -> Result<(), ValidationError> {
  finite_all(&[value.x, value.y, value.z, value.w])
}

fn validate_rgb(value: RgbColor) -> Result<(), ValidationError> {
  finite_all(&[value.r, value.g, value.b])
}

fn validate_color(value: Color) -> Result<(), ValidationError> {
  finite_all(&[value.r, value.g, value.b, value.a])
}

fn finite_all(values: &[f64]) -> Result<(), ValidationError> {
  finite(values.iter().all(|value| value.is_finite()))
}

fn finite(is_finite: bool) -> Result<(), ValidationError> {
  if is_finite {
    Ok(())
  } else {
    Err(ValidationError::NonFiniteNumber)
  }
}

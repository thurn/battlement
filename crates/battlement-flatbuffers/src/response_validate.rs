use std::collections::{HashMap, HashSet};

use crate::{
  ProtocolError, common_generated as common,
  response_generated::battlement::flat_buffers::generated as response_wire,
  ui_generated as ui_wire, world_generated as world_wire,
};

type Id = [u8; 16];

pub(crate) fn validate_snapshot(value: response_wire::Snapshot<'_>) -> Result<(), ProtocolError> {
  let mut scene_ids = HashSet::new();
  for scene in value.scenes() {
    require_uuid(scene.scene_id(), "snapshot scene")?;
    if !scene_ids.insert(id(scene.scene_id())) {
      return Err(ProtocolError::new("snapshot repeats a scene UUID"));
    }
  }
  if let Some(primary) = value.primary_scene_id() {
    require_uuid(primary, "primary scene")?;
    if !scene_ids.contains(&id(primary)) {
      return Err(ProtocolError::new("snapshot primary scene is not loaded"));
    }
  }

  let mut object_ids = HashSet::new();
  for object in value.objects() {
    require_uuid(object.object_id(), "snapshot object")?;
    if !object_ids.insert(id(object.object_id())) {
      return Err(ProtocolError::new("snapshot repeats an object UUID"));
    }
    validate_game_object(object, &scene_ids)?;
  }
  for object in value.objects() {
    if let Some(parent) = object.parent_id() {
      require_uuid(parent, "object parent")?;
      if !object_ids.contains(&id(parent)) {
        return Err(ProtocolError::new("snapshot object parent is unavailable"));
      }
      if id(parent) == id(object.object_id()) {
        return Err(ProtocolError::new("snapshot object cannot parent itself"));
      }
    }
  }
  if let Some(camera) = value.input_camera_id() {
    require_uuid(camera, "input camera")?;
    if !object_ids.contains(&id(camera)) {
      return Err(ProtocolError::new("snapshot input camera is unavailable"));
    }
  }

  let mut all_ui_ids = HashSet::new();
  let mut document_ids = HashSet::new();
  for document in value.ui() {
    validate_document(document, &mut all_ui_ids, &mut document_ids)?;
  }
  Ok(())
}

pub(crate) fn validate_create(
  value: ui_wire::VisualElementCreatePayload<'_>,
) -> Result<(), ProtocolError> {
  require_uuid(value.parent_id(), "visual create parent")?;
  require_uuid(value.root_id(), "visual create root")?;
  let nodes = value.nodes();
  if nodes.is_empty() {
    return Err(ProtocolError::new("visual create subtree is empty"));
  }
  validate_subtree(value.root_id(), nodes)
}

pub(crate) fn validate_update(
  value: ui_wire::VisualElementUpdatePayload<'_>,
) -> Result<(), ProtocolError> {
  require_uuid(value.object_id(), "visual update object")?;
  match value.kind() {
    ui_wire::VisualElementUpdateKind::Properties => {
      let element = value
        .element()
        .ok_or_else(|| ProtocolError::new("property update has no element"))?;
      if value.parent_id().is_some() || value.child_index().is_some() {
        return Err(ProtocolError::new(
          "property update carries hierarchy fields",
        ));
      }
      validate_element(element)
    }
    ui_wire::VisualElementUpdateKind::Parent => {
      let parent = value
        .parent_id()
        .ok_or_else(|| ProtocolError::new("parent update has no parent"))?;
      require_uuid(parent, "visual update parent")?;
      if value.element().is_some() {
        return Err(ProtocolError::new(
          "parent update carries element properties",
        ));
      }
      Ok(())
    }
    ui_wire::VisualElementUpdateKind::Index => {
      if value.element().is_some() || value.parent_id().is_some() || value.child_index().is_none() {
        return Err(ProtocolError::new("index update has noncanonical fields"));
      }
      Ok(())
    }
    _ => Err(ProtocolError::new("visual update kind is unknown")),
  }
}

pub(crate) fn validate_destroy(
  value: ui_wire::VisualElementDestroyPayload<'_>,
) -> Result<(), ProtocolError> {
  require_uuid(value.object_id(), "visual destroy object")
}

pub(crate) fn validate_action(
  value: ui_wire::VisualElementActionPayload<'_>,
) -> Result<(), ProtocolError> {
  require_uuid(value.object_id(), "visual action object")?;
  match value.kind() {
    ui_wire::VisualElementActionKind::ParticleStreaks => {
      let streaks = value
        .streaks()
        .ok_or_else(|| ProtocolError::new("particle action has no streak vector"))?;
      if streaks.len() > 128 {
        return Err(ProtocolError::new("particle action exceeds 128 streaks"));
      }
      for streak in streaks {
        let finite = [
          streak.origin().x(),
          streak.origin().y(),
          streak.travel().x(),
          streak.travel().y(),
          streak.size().x(),
          streak.size().y(),
          streak.rotation(),
        ]
        .into_iter()
        .all(f32::is_finite);
        if !finite
          || !(0.0..=1.0).contains(&streak.origin().x())
          || !(0.0..=1.0).contains(&streak.origin().y())
          || streak.size().x() <= 0.0
          || streak.size().y() <= 0.0
          || streak.size().x() > 1024.0
          || streak.size().y() > 1024.0
          || streak.travel().x().abs() > 1024.0
          || streak.travel().y().abs() > 1024.0
          || !(1..=1000).contains(&streak.lifetime_ms())
          || streak.delay_ms() > 1000
        {
          return Err(ProtocolError::new(
            "particle action contains an invalid streak",
          ));
        }
      }
      Ok(())
    }
    ui_wire::VisualElementActionKind::Focus | ui_wire::VisualElementActionKind::Blur => Ok(()),
    ui_wire::VisualElementActionKind::CapturePointer
    | ui_wire::VisualElementActionKind::ReleasePointer => Ok(()),
    ui_wire::VisualElementActionKind::ScrollTo => require_uuid(
      value
        .descendant_id()
        .ok_or_else(|| ProtocolError::new("scroll action has no descendant"))?,
      "scroll descendant",
    ),
    ui_wire::VisualElementActionKind::SelectText => Ok(()),
    _ => Err(ProtocolError::new("visual action kind is unknown")),
  }
}

fn validate_game_object(
  value: world_wire::GameObject<'_>,
  scene_ids: &HashSet<Id>,
) -> Result<(), ProtocolError> {
  let parent_scene = value.parent_scene();
  match parent_scene.kind() {
    world_wire::ParentSceneKind::PrimaryScene | world_wire::ParentSceneKind::Persistent => {
      if parent_scene.scene_id().is_some() {
        return Err(ProtocolError::new(
          "object parent scene has a noncanonical UUID",
        ));
      }
    }
    world_wire::ParentSceneKind::Scene => {
      let scene = parent_scene
        .scene_id()
        .ok_or_else(|| ProtocolError::new("object parent scene has no UUID"))?;
      require_uuid(scene, "object parent scene")?;
      if !scene_ids.contains(&id(scene)) {
        return Err(ProtocolError::new("object parent scene is unavailable"));
      }
    }
    _ => return Err(ProtocolError::new("object parent scene kind is unknown")),
  }
  if value.kind().variant_name().is_none() || value.content_type().variant_name().is_none() {
    return Err(ProtocolError::new("game object kind is unknown"));
  }
  let expected = match value.kind() {
    world_wire::GameObjectKind::UiDocument => world_wire::GameObjectContent::UiDocumentObject,
    world_wire::GameObjectKind::Empty => world_wire::GameObjectContent::EmptyObject,
    world_wire::GameObjectKind::Cube
    | world_wire::GameObjectKind::Sphere
    | world_wire::GameObjectKind::Capsule
    | world_wire::GameObjectKind::Cylinder
    | world_wire::GameObjectKind::Plane
    | world_wire::GameObjectKind::Quad => world_wire::GameObjectContent::PrimitiveObject,
    world_wire::GameObjectKind::Image => world_wire::GameObjectContent::ImageObject,
    world_wire::GameObjectKind::Text => world_wire::GameObjectContent::TextObject,
    world_wire::GameObjectKind::Camera => world_wire::GameObjectContent::CameraObject,
    world_wire::GameObjectKind::Light => world_wire::GameObjectContent::LightObject,
    world_wire::GameObjectKind::Mesh => world_wire::GameObjectContent::MeshObject,
    world_wire::GameObjectKind::Prefab => world_wire::GameObjectContent::PrefabObject,
    _ => return Err(ProtocolError::new("game object kind is unknown")),
  };
  if value.content_type() != expected {
    return Err(ProtocolError::new(
      "game object kind and content payload do not match",
    ));
  }
  Ok(())
}

fn validate_document(
  value: ui_wire::UiDocument<'_>,
  all_ids: &mut HashSet<Id>,
  document_ids: &mut HashSet<Id>,
) -> Result<(), ProtocolError> {
  require_uuid(value.document_id(), "UI document")?;
  require_uuid(value.root_id(), "UI document root")?;
  if !document_ids.insert(id(value.document_id())) {
    return Err(ProtocolError::new("snapshot repeats a UI document UUID"));
  }
  if !all_ids.insert(id(value.root_id())) {
    return Err(ProtocolError::new(
      "snapshot repeats a UI node UUID across documents",
    ));
  }
  let root = value.root_element();
  if root.kind() != ui_wire::UiElementKind::VisualElement
    || root.usage_hints() != 0
    || !root.part_styles().is_empty()
    || root.properties().iter().any(|property| {
      let key = property.key();
      !matches!(
        key,
        ui_wire::UiPropertyKey::Name
          | ui_wire::UiPropertyKey::Enabled
          | ui_wire::UiPropertyKey::PickingMode
          | ui_wire::UiPropertyKey::LanguageDirection
          | ui_wire::UiPropertyKey::Focusable
          | ui_wire::UiPropertyKey::TabIndex
          | ui_wire::UiPropertyKey::DelegatesFocus
          | ui_wire::UiPropertyKey::AutoFocus
          | ui_wire::UiPropertyKey::Inert
          | ui_wire::UiPropertyKey::Classes
          | ui_wire::UiPropertyKey::Events
          | ui_wire::UiPropertyKey::EventSubscriptions
      ) && !(key.0 >= ui_wire::UiPropertyKey::StyleAlignContent.0
        && key.0 <= ui_wire::UiPropertyKey::StyleWordSpacing.0)
    })
  {
    return Err(ProtocolError::new(
      "UI document root contains non-document state",
    ));
  }
  validate_element(root)?;
  validate_forest(
    value.root_id(),
    value.nodes(),
    &value.root_child_ids().iter().collect::<Vec<_>>(),
  )?;
  for node in value.nodes() {
    if !all_ids.insert(id(node.object_id())) {
      return Err(ProtocolError::new(
        "snapshot repeats a UI node UUID across documents",
      ));
    }
  }
  Ok(())
}

fn validate_forest<'a>(
  root_id: &common::Uuid,
  nodes: flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<ui_wire::UiNode<'a>>>,
  root_children: &[&common::Uuid],
) -> Result<(), ProtocolError> {
  let root = id(root_id);
  let mut by_id = HashMap::with_capacity(nodes.len());
  for node in nodes {
    require_uuid(node.object_id(), "UI node")?;
    let node_id = id(node.object_id());
    if node_id == root || by_id.insert(node_id, node).is_some() {
      return Err(ProtocolError::new("UI forest repeats a node UUID"));
    }
    validate_element(node.element())?;
  }
  let mut parent = HashSet::new();
  let mut stack = root_children
    .iter()
    .rev()
    .map(|value| (id(value), 1_usize))
    .collect::<Vec<_>>();
  while let Some((node_id, depth)) = stack.pop() {
    if depth > 1024 {
      return Err(ProtocolError::new("UI forest exceeds logical depth 1024"));
    }
    if !parent.insert(node_id) {
      return Err(ProtocolError::new(
        "UI forest gives one node multiple parents",
      ));
    }
    let node = by_id
      .get(&node_id)
      .ok_or_else(|| ProtocolError::new("UI forest has a dangling child UUID"))?;
    stack.extend(
      node
        .child_ids()
        .iter()
        .rev()
        .map(|value| (id(value), depth + 1)),
    );
  }
  if parent.len() != by_id.len() {
    return Err(ProtocolError::new(
      "UI forest contains unreachable or cyclic nodes",
    ));
  }
  Ok(())
}

fn validate_subtree<'a>(
  root_id: &common::Uuid,
  nodes: flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<ui_wire::UiNode<'a>>>,
) -> Result<(), ProtocolError> {
  let root = id(root_id);
  let mut by_id = HashMap::with_capacity(nodes.len());
  for node in nodes {
    require_uuid(node.object_id(), "UI subtree node")?;
    if by_id.insert(id(node.object_id()), node).is_some() {
      return Err(ProtocolError::new("UI subtree repeats a node UUID"));
    }
    validate_element(node.element())?;
  }
  if !by_id.contains_key(&root) {
    return Err(ProtocolError::new("UI subtree root UUID is unavailable"));
  }
  let mut visited = HashSet::new();
  let mut stack = vec![(root, 0_usize)];
  while let Some((node_id, depth)) = stack.pop() {
    if depth > 1024 {
      return Err(ProtocolError::new("UI subtree exceeds logical depth 1024"));
    }
    if !visited.insert(node_id) {
      return Err(ProtocolError::new(
        "UI subtree gives one node multiple parents",
      ));
    }
    let node = by_id
      .get(&node_id)
      .ok_or_else(|| ProtocolError::new("UI subtree has a dangling child UUID"))?;
    stack.extend(
      node
        .child_ids()
        .iter()
        .rev()
        .map(|value| (id(value), depth + 1)),
    );
  }
  if visited.len() != by_id.len() {
    return Err(ProtocolError::new(
      "UI subtree contains unreachable or cyclic nodes",
    ));
  }
  Ok(())
}

fn validate_element(value: ui_wire::UiElement<'_>) -> Result<(), ProtocolError> {
  if value.kind().variant_name().is_none() {
    return Err(ProtocolError::new("UI element kind is unknown"));
  }
  if value.usage_hints() & !0b1111 != 0 {
    return Err(ProtocolError::new("UI usage-hint mask has unknown bits"));
  }
  let mut keys = HashSet::new();
  for property in value.properties() {
    validate_property(property, &mut keys)?;
    validate_element_property(value.kind(), property)?;
  }
  for subscription in value.event_subscriptions() {
    if subscription.kind().variant_name().is_none()
      || subscription.phases() == 0
      || subscription.phases() & !0b111 != 0
    {
      return Err(ProtocolError::new("UI event subscription is noncanonical"));
    }
  }
  let mut parts = HashSet::new();
  for part in value.part_styles() {
    if part.part().variant_name().is_none() || !parts.insert((part.part().0, part.index())) {
      return Err(ProtocolError::new(
        "UI part styles repeat or use an unknown part",
      ));
    }
    let mut part_keys = HashSet::new();
    for property in part.properties() {
      validate_property(property, &mut part_keys)?;
    }
  }
  Ok(())
}

fn validate_element_property(
  kind: ui_wire::UiElementKind,
  value: ui_wire::UiProperty<'_>,
) -> Result<(), ProtocolError> {
  if !matches!(
    value.key(),
    ui_wire::UiPropertyKey::DelayMs | ui_wire::UiPropertyKey::IntervalMs
  ) {
    return Ok(());
  }
  if kind != ui_wire::UiElementKind::RepeatButton {
    return Err(ProtocolError::new(
      "repeat timing is only valid on a repeat button",
    ));
  }
  if value.state() != ui_wire::PropState::Set {
    return Ok(());
  }
  if value.value_type() != ui_wire::UiPropertyValue::UIntPropertyValue {
    return Err(ProtocolError::new(
      "repeat timing requires an unsigned integer",
    ));
  }
  let timing = value
    .value_as_uint_property_value()
    .expect("verified unsigned repeat timing")
    .value();
  if value.key() == ui_wire::UiPropertyKey::IntervalMs && timing == 0 {
    return Err(ProtocolError::new(
      "repeat button interval must be positive",
    ));
  }
  Ok(())
}

fn validate_property(
  value: ui_wire::UiProperty<'_>,
  keys: &mut HashSet<u16>,
) -> Result<(), ProtocolError> {
  if value.key().variant_name().is_none() || !keys.insert(value.key().0) {
    return Err(ProtocolError::new(
      "UI properties repeat or use an unknown key",
    ));
  }
  if value.style_value_kind().variant_name().is_none() {
    return Err(ProtocolError::new("UI style value kind is unknown"));
  }
  let is_style = value.key().0 >= ui_wire::UiPropertyKey::StyleAlignContent.0
    && value.key().0 <= ui_wire::UiPropertyKey::StyleWordSpacing.0;
  if !is_style && value.style_value_kind() != ui_wire::StyleValueKind::Value {
    return Err(ProtocolError::new(
      "non-style UI property uses a style keyword",
    ));
  }
  let is_subscription_marker = value.key() == ui_wire::UiPropertyKey::EventSubscriptions
    && value.state() == ui_wire::PropState::Set
    && value.style_value_kind() == ui_wire::StyleValueKind::Value
    && value.value_type() == ui_wire::UiPropertyValue::NONE
    && value.value().is_none();
  if is_subscription_marker {
    return Ok(());
  }
  match value.state() {
    ui_wire::PropState::Set => {
      if value.style_value_kind() == ui_wire::StyleValueKind::Initial {
        if value.value_type() != ui_wire::UiPropertyValue::NONE || value.value().is_some() {
          return Err(ProtocolError::new(
            "initial UI style keyword carries a value",
          ));
        }
      } else {
        if value.value_type() == ui_wire::UiPropertyValue::NONE || value.value().is_none() {
          return Err(ProtocolError::new("UI Set property has no typed value"));
        }
        if value.value_type().variant_name().is_none() {
          return Err(ProtocolError::new("UI property value union is unknown"));
        }
        validate_property_payload(value)?;
      }
    }
    ui_wire::PropState::Reset => {
      if value.style_value_kind() != ui_wire::StyleValueKind::Value
        || value.value_type() != ui_wire::UiPropertyValue::NONE
        || value.value().is_some()
      {
        return Err(ProtocolError::new("UI Reset property carries a value"));
      }
    }
    ui_wire::PropState::Unset => {
      return Err(ProtocolError::new("present UI property uses Unset state"));
    }
    _ => return Err(ProtocolError::new("UI property state is unknown")),
  }
  Ok(())
}

fn validate_property_payload(value: ui_wire::UiProperty<'_>) -> Result<(), ProtocolError> {
  use ui_wire::UiPropertyValue as Value;
  match value.value_type() {
    Value::BoolPropertyValue
    | Value::IntPropertyValue
    | Value::UIntPropertyValue
    | Value::TextPropertyValue
    | Value::TextListPropertyValue
    | Value::UIntListPropertyValue => Ok(()),
    Value::FloatPropertyValue => require_finite32(
      value
        .value_as_float_property_value()
        .expect("verified UI float property")
        .value(),
      "UI float property",
    ),
    Value::EnumPropertyValue => {
      let value = value
        .value_as_enum_property_value()
        .expect("verified UI enum property");
      if value.catalog().variant_name().is_none() {
        return Err(ProtocolError::new("UI enum property catalog is unknown"));
      }
      Ok(())
    }
    Value::LengthPropertyValue => validate_length(
      value
        .value_as_length_property_value()
        .expect("verified UI length property"),
    ),
    Value::AspectRatioPropertyValue => {
      let value = value
        .value_as_aspect_ratio_property_value()
        .expect("verified UI aspect-ratio property");
      require_finite32(value.width(), "UI aspect-ratio width")?;
      require_finite32(value.height(), "UI aspect-ratio height")?;
      match value.kind() {
        ui_wire::AspectRatioKind::Auto if value.width() == 0.0 && value.height() == 0.0 => Ok(()),
        ui_wire::AspectRatioKind::Ratio if value.width() > 0.0 && value.height() > 0.0 => Ok(()),
        ui_wire::AspectRatioKind::Auto => Err(ProtocolError::new(
          "automatic UI aspect ratio carries dimensions",
        )),
        ui_wire::AspectRatioKind::Ratio => Err(ProtocolError::new(
          "UI aspect-ratio dimensions must be positive",
        )),
        _ => Err(ProtocolError::new("UI aspect-ratio kind is unknown")),
      }
    }
    Value::RotatePropertyValue => {
      let value = value
        .value_as_rotate_property_value()
        .expect("verified UI rotate property");
      require_finite32s(
        [value.x(), value.y(), value.z(), value.degrees()],
        "UI rotation",
      )
    }
    Value::ScalePropertyValue => {
      let value = value
        .value_as_scale_property_value()
        .expect("verified UI scale property");
      require_finite32s([value.x(), value.y()], "UI scale")
    }
    Value::FloatListPropertyValue => {
      let value = value
        .value_as_float_list_property_value()
        .expect("verified UI float-list property");
      for item in value.values() {
        require_finite32(item, "UI float-list item")?;
      }
      Ok(())
    }
    Value::EnumListPropertyValue => {
      let value = value
        .value_as_enum_list_property_value()
        .expect("verified UI enum-list property");
      if value.catalog().variant_name().is_none() {
        return Err(ProtocolError::new(
          "UI enum-list property catalog is unknown",
        ));
      }
      Ok(())
    }
    Value::TextAutoSizePropertyValue => {
      let value = value
        .value_as_text_auto_size_property_value()
        .expect("verified UI text-auto-size property");
      require_finite32s([value.min_size(), value.max_size()], "UI text-auto-size")?;
      match value.kind() {
        ui_wire::TextAutoSizeKind::None if value.min_size() == 0.0 && value.max_size() == 0.0 => {
          Ok(())
        }
        ui_wire::TextAutoSizeKind::BestFit
          if value.min_size() >= 0.0 && value.min_size() <= value.max_size() =>
        {
          Ok(())
        }
        ui_wire::TextAutoSizeKind::None => Err(ProtocolError::new(
          "disabled UI text auto-size carries bounds",
        )),
        ui_wire::TextAutoSizeKind::BestFit => {
          Err(ProtocolError::new("UI text auto-size bounds are invalid"))
        }
        _ => Err(ProtocolError::new("UI text auto-size kind is unknown")),
      }
    }
    Value::ColorPropertyValue => {
      let value = value
        .value_as_color_property_value()
        .expect("verified UI color property")
        .value();
      require_finite64s([value.r(), value.g(), value.b(), value.a()], "UI color")
    }
    Value::RectPropertyValue => {
      let value = value
        .value_as_rect_property_value()
        .expect("verified UI rectangle property")
        .value();
      require_finite64s(
        [value.x(), value.y(), value.width(), value.height()],
        "UI rectangle",
      )
    }
    Value::Vector2PropertyValue => {
      let value = value
        .value_as_vector_2_property_value()
        .expect("verified UI vector property")
        .value();
      require_finite64s([value.x(), value.y()], "UI vector")
    }
    Value::AssetPropertyValue => {
      let value = value
        .value_as_asset_property_value()
        .expect("verified UI asset property");
      if value.kind().variant_name().is_none() {
        return Err(ProtocolError::new("UI asset kind is unknown"));
      }
      Ok(())
    }
    Value::GridTracksPropertyValue => {
      let value = value
        .value_as_grid_tracks_property_value()
        .expect("verified UI grid-tracks property");
      for track in value.values() {
        if track.kind().variant_name().is_none() || !track.value().is_finite() {
          return Err(ProtocolError::new("UI grid track is invalid"));
        }
        if track.kind() != ui_wire::GridTrackKind::Auto && track.value() < 0.0 {
          return Err(ProtocolError::new("UI grid track value is negative"));
        }
        if track.kind() == ui_wire::GridTrackKind::Auto && track.value() != 0.0 {
          return Err(ProtocolError::new(
            "automatic UI grid track carries a value",
          ));
        }
      }
      Ok(())
    }
    Value::ChoicePropertyValue => {
      let value = value
        .value_as_choice_property_value()
        .expect("verified UI choice property");
      match value.kind() {
        ui_wire::ChoiceKind::None if value.index() == 0 && value.value().is_none() => Ok(()),
        ui_wire::ChoiceKind::Index if value.value().is_some() => Ok(()),
        ui_wire::ChoiceKind::None => Err(ProtocolError::new(
          "absent UI choice carries selection data",
        )),
        ui_wire::ChoiceKind::Index => Err(ProtocolError::new(
          "selected UI choice has no display value",
        )),
        _ => Err(ProtocolError::new("UI choice kind is unknown")),
      }
    }
    Value::LimitPropertyValue => {
      let value = value
        .value_as_limit_property_value()
        .expect("verified UI limit property");
      require_finite32(value.value(), "UI limit")?;
      match value.kind() {
        ui_wire::LimitKind::Unbounded if value.value() == 0.0 => Ok(()),
        ui_wire::LimitKind::Inclusive => Ok(()),
        ui_wire::LimitKind::Unbounded => {
          Err(ProtocolError::new("unbounded UI limit carries a value"))
        }
        _ => Err(ProtocolError::new("UI limit kind is unknown")),
      }
    }
    Value::BackgroundPositionPropertyValue => validate_length(
      value
        .value_as_background_position_property_value()
        .expect("verified UI background-position property")
        .offset(),
    ),
    Value::BackgroundRepeatPropertyValue => {
      let value = value
        .value_as_background_repeat_property_value()
        .expect("verified UI background-repeat property");
      if value.x() > 3 || value.y() > 3 {
        return Err(ProtocolError::new("UI background-repeat mode is unknown"));
      }
      Ok(())
    }
    Value::BackgroundSizePropertyValue => {
      let value = value
        .value_as_background_size_property_value()
        .expect("verified UI background-size property");
      match value.kind() {
        ui_wire::BackgroundSizeKind::Axes => {
          validate_length(
            value
              .x()
              .ok_or_else(|| ProtocolError::new("axis UI background size has no x value"))?,
          )?;
          validate_length(
            value
              .y()
              .ok_or_else(|| ProtocolError::new("axis UI background size has no y value"))?,
          )
        }
        ui_wire::BackgroundSizeKind::Auto
        | ui_wire::BackgroundSizeKind::Cover
        | ui_wire::BackgroundSizeKind::Contain
          if value.x().is_none() && value.y().is_none() =>
        {
          Ok(())
        }
        ui_wire::BackgroundSizeKind::Auto
        | ui_wire::BackgroundSizeKind::Cover
        | ui_wire::BackgroundSizeKind::Contain => Err(ProtocolError::new(
          "keyword UI background size carries axis values",
        )),
        _ => Err(ProtocolError::new("UI background-size kind is unknown")),
      }
    }
    Value::CursorPropertyValue => {
      let value = value
        .value_as_cursor_property_value()
        .expect("verified UI cursor property");
      match value.kind() {
        ui_wire::CursorKind::Default if value.address().is_none() && value.hotspot().is_none() => {
          Ok(())
        }
        ui_wire::CursorKind::Texture => {
          if value.address().is_none() {
            return Err(ProtocolError::new("texture UI cursor has no address"));
          }
          let hotspot = value
            .hotspot()
            .ok_or_else(|| ProtocolError::new("texture UI cursor has no hotspot"))?;
          require_finite64s([hotspot.x(), hotspot.y()], "UI cursor hotspot")
        }
        ui_wire::CursorKind::Default => {
          Err(ProtocolError::new("default UI cursor carries texture data"))
        }
        _ => Err(ProtocolError::new("UI cursor kind is unknown")),
      }
    }
    Value::TextShadowPropertyValue => {
      let value = value
        .value_as_text_shadow_property_value()
        .expect("verified UI text-shadow property");
      require_finite32s(
        [value.x(), value.y(), value.blur_radius()],
        "UI text shadow",
      )?;
      let color = value.color();
      require_finite64s(
        [color.r(), color.g(), color.b(), color.a()],
        "UI text-shadow color",
      )
    }
    Value::TransformOriginPropertyValue => {
      let value = value
        .value_as_transform_origin_property_value()
        .expect("verified UI transform-origin property");
      validate_length(value.x())?;
      validate_length(value.y())?;
      require_finite32(value.z(), "UI transform-origin z")
    }
    Value::TranslatePropertyValue => {
      let value = value
        .value_as_translate_property_value()
        .expect("verified UI translate property");
      validate_length(value.x())?;
      validate_length(value.y())?;
      require_finite32(value.z(), "UI translate z")
    }
    Value::OverlayPlacementPropertyValue => {
      let value = value
        .value_as_overlay_placement_property_value()
        .expect("verified UI overlay-placement property");
      require_finite32s(
        [
          value.main_offset(),
          value.cross_offset(),
          value.collision_padding(),
        ],
        "UI overlay placement",
      )?;
      if value.side().variant_name().is_none() || value.align().variant_name().is_none() {
        return Err(ProtocolError::new("UI overlay placement enum is unknown"));
      }
      match value.kind() {
        ui_wire::OverlayPlacementKind::PopoverLayer | ui_wire::OverlayPlacementKind::ModalLayer
          if value.anchor().is_none()
            && value.initial_focus().is_none()
            && value.restore_focus().is_none()
            && overlay_defaults(value) =>
        {
          Ok(())
        }
        ui_wire::OverlayPlacementKind::Popover => {
          require_uuid(
            value
              .anchor()
              .ok_or_else(|| ProtocolError::new("UI popover has no anchor"))?,
            "UI popover anchor",
          )?;
          if value.initial_focus().is_some() || value.restore_focus().is_some() {
            return Err(ProtocolError::new("UI popover carries modal focus UUIDs"));
          }
          Ok(())
        }
        ui_wire::OverlayPlacementKind::Modal => {
          if value.anchor().is_some() || !overlay_defaults(value) {
            return Err(ProtocolError::new(
              "UI modal carries popover placement data",
            ));
          }
          for focus in [value.initial_focus(), value.restore_focus()]
            .into_iter()
            .flatten()
          {
            require_uuid(focus, "UI modal focus target")?;
          }
          Ok(())
        }
        ui_wire::OverlayPlacementKind::PopoverLayer | ui_wire::OverlayPlacementKind::ModalLayer => {
          Err(ProtocolError::new(
            "UI overlay layer carries placement data",
          ))
        }
        _ => Err(ProtocolError::new("UI overlay-placement kind is unknown")),
      }
    }
    Value::GridItemPropertyValue => {
      let value = value
        .value_as_grid_item_property_value()
        .expect("verified UI grid-item property");
      if value.row_span() == 0 || value.column_span() == 0 {
        return Err(ProtocolError::new("UI grid-item span must be positive"));
      }
      Ok(())
    }
    Value::StackItemPropertyValue => {
      let value = value
        .value_as_stack_item_property_value()
        .expect("verified UI stack-item property");
      require_optional_finite32s(
        [value.top(), value.right(), value.bottom(), value.left()],
        "UI stack-item inset",
      )
    }
    Value::StickyPropertyValue => {
      let value = value
        .value_as_sticky_property_value()
        .expect("verified UI sticky property");
      require_optional_finite32s(
        [value.top(), value.right(), value.bottom(), value.left()],
        "UI sticky inset",
      )
    }
    Value::PaintStylePropertyValue | Value::MotionDescriptorPropertyValue => Ok(()),
    _ => Err(ProtocolError::new("UI property value union is unknown")),
  }
}

fn validate_length(value: ui_wire::LengthPropertyValue<'_>) -> Result<(), ProtocolError> {
  require_finite32s([value.pixels(), value.percentage()], "UI length")?;
  match value.kind() {
    ui_wire::LengthKind::Pixels if value.percentage() == 0.0 => Ok(()),
    ui_wire::LengthKind::Percent if value.pixels() == 0.0 => Ok(()),
    ui_wire::LengthKind::Calc => Ok(()),
    ui_wire::LengthKind::Auto if value.pixels() == 0.0 && value.percentage() == 0.0 => Ok(()),
    ui_wire::LengthKind::Pixels | ui_wire::LengthKind::Percent | ui_wire::LengthKind::Auto => {
      Err(ProtocolError::new("UI length carries inactive scalar data"))
    }
    _ => Err(ProtocolError::new("UI length kind is unknown")),
  }
}

fn require_finite32(value: f32, name: &str) -> Result<(), ProtocolError> {
  if value.is_finite() {
    Ok(())
  } else {
    Err(ProtocolError::new(format!("{name} must be finite")))
  }
}

fn require_finite32s<const N: usize>(values: [f32; N], name: &str) -> Result<(), ProtocolError> {
  if values.into_iter().all(f32::is_finite) {
    Ok(())
  } else {
    Err(ProtocolError::new(format!("{name} must be finite")))
  }
}

fn require_finite64s<const N: usize>(values: [f64; N], name: &str) -> Result<(), ProtocolError> {
  if values.into_iter().all(f64::is_finite) {
    Ok(())
  } else {
    Err(ProtocolError::new(format!("{name} must be finite")))
  }
}

fn require_optional_finite32s<const N: usize>(
  values: [Option<f32>; N],
  name: &str,
) -> Result<(), ProtocolError> {
  if values.into_iter().flatten().all(f32::is_finite) {
    Ok(())
  } else {
    Err(ProtocolError::new(format!("{name} must be finite")))
  }
}

fn overlay_defaults(value: ui_wire::OverlayPlacementPropertyValue<'_>) -> bool {
  value.side() == ui_wire::PlacementSide::Bottom
    && value.align() == ui_wire::PlacementAlign::Start
    && value.main_offset() == 0.0
    && value.cross_offset() == 0.0
    && value.collision_padding() == 0.0
    && !value.flip()
    && !value.shift()
}

fn require_uuid(value: &common::Uuid, name: &str) -> Result<(), ProtocolError> {
  if id(value).iter().all(|value| *value == 0) {
    Err(ProtocolError::new(format!("{name} UUID is zero")))
  } else {
    Ok(())
  }
}

fn id(value: &common::Uuid) -> Id {
  std::array::from_fn(|index| value.bytes().get(index))
}

#[cfg(test)]
mod tests {
  use flatbuffers::FlatBufferBuilder;

  use super::*;

  #[test]
  fn accepts_value_less_initial_only_for_set_style_properties() {
    assert!(
      validate_test_property(
        ui_wire::UiPropertyKey::StyleColor,
        ui_wire::PropState::Set,
        ui_wire::StyleValueKind::Initial,
      )
      .is_ok()
    );

    let non_style = validate_test_property(
      ui_wire::UiPropertyKey::Text,
      ui_wire::PropState::Set,
      ui_wire::StyleValueKind::Initial,
    )
    .unwrap_err();
    assert!(non_style.to_string().contains("non-style"));

    let reset_keyword = validate_test_property(
      ui_wire::UiPropertyKey::StyleColor,
      ui_wire::PropState::Reset,
      ui_wire::StyleValueKind::Initial,
    )
    .unwrap_err();
    assert!(reset_keyword.to_string().contains("Reset"));
  }

  #[test]
  fn rejects_nonfinite_and_noncanonical_typed_ui_values() {
    let nonfinite = validate_test_set_property(
      ui_wire::UiPropertyKey::StyleOpacity,
      ui_wire::UiPropertyValue::FloatPropertyValue,
      |builder| {
        ui_wire::FloatPropertyValue::create(
          builder,
          &ui_wire::FloatPropertyValueArgs { value: f32::NAN },
        )
        .as_union_value()
      },
    )
    .unwrap_err();
    assert!(nonfinite.to_string().contains("finite"));

    let inactive = validate_test_set_property(
      ui_wire::UiPropertyKey::StyleWidth,
      ui_wire::UiPropertyValue::LengthPropertyValue,
      |builder| {
        ui_wire::LengthPropertyValue::create(
          builder,
          &ui_wire::LengthPropertyValueArgs {
            kind: ui_wire::LengthKind::Pixels,
            pixels: 12.0,
            percentage: 5.0,
          },
        )
        .as_union_value()
      },
    )
    .unwrap_err();
    assert!(inactive.to_string().contains("inactive scalar"));
  }

  #[test]
  fn rejects_noncanonical_repeat_timing() {
    let zero_interval = validate_test_set_property_for(
      ui_wire::UiElementKind::RepeatButton,
      ui_wire::UiPropertyKey::IntervalMs,
      ui_wire::UiPropertyValue::UIntPropertyValue,
      |builder| {
        ui_wire::UIntPropertyValue::create(builder, &ui_wire::UIntPropertyValueArgs { value: 0 })
          .as_union_value()
      },
    )
    .unwrap_err();
    assert!(zero_interval.to_string().contains("positive"));

    let wrong_element = validate_test_set_property_for(
      ui_wire::UiElementKind::Label,
      ui_wire::UiPropertyKey::DelayMs,
      ui_wire::UiPropertyValue::UIntPropertyValue,
      |builder| {
        ui_wire::UIntPropertyValue::create(builder, &ui_wire::UIntPropertyValueArgs { value: 200 })
          .as_union_value()
      },
    )
    .unwrap_err();
    assert!(wrong_element.to_string().contains("repeat button"));
  }

  #[test]
  fn accepts_the_value_less_event_subscription_marker_only() {
    validate_test_property(
      ui_wire::UiPropertyKey::EventSubscriptions,
      ui_wire::PropState::Set,
      ui_wire::StyleValueKind::Value,
    )
    .unwrap();

    let error = validate_test_property(
      ui_wire::UiPropertyKey::Text,
      ui_wire::PropState::Set,
      ui_wire::StyleValueKind::Value,
    )
    .unwrap_err();
    assert!(error.to_string().contains("no typed value"));
  }

  fn validate_test_property(
    key: ui_wire::UiPropertyKey,
    state: ui_wire::PropState,
    style_value_kind: ui_wire::StyleValueKind,
  ) -> Result<(), ProtocolError> {
    let mut builder = FlatBufferBuilder::new();
    let property = ui_wire::UiProperty::create(
      &mut builder,
      &ui_wire::UiPropertyArgs {
        key,
        state,
        style_value_kind,
        value_type: ui_wire::UiPropertyValue::NONE,
        value: None,
      },
    );
    let properties = builder.create_vector(&[property]);
    let event_subscriptions =
      builder.create_vector::<flatbuffers::WIPOffset<ui_wire::UiEventSubscriptionValue<'_>>>(&[]);
    let part_styles = builder.create_vector::<flatbuffers::WIPOffset<ui_wire::PartStyle<'_>>>(&[]);
    let element = ui_wire::UiElement::create(
      &mut builder,
      &ui_wire::UiElementArgs {
        kind: ui_wire::UiElementKind::VisualElement,
        properties: Some(properties),
        event_subscriptions: Some(event_subscriptions),
        part_styles: Some(part_styles),
        ..Default::default()
      },
    );
    builder.finish(element, None);
    let element = flatbuffers::root::<ui_wire::UiElement<'_>>(builder.finished_data()).unwrap();
    validate_element(element)
  }

  fn validate_test_set_property(
    key: ui_wire::UiPropertyKey,
    value_type: ui_wire::UiPropertyValue,
    value: impl for<'a> FnOnce(
      &mut FlatBufferBuilder<'a>,
    ) -> flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
  ) -> Result<(), ProtocolError> {
    validate_test_set_property_for(
      ui_wire::UiElementKind::VisualElement,
      key,
      value_type,
      value,
    )
  }

  fn validate_test_set_property_for(
    kind: ui_wire::UiElementKind,
    key: ui_wire::UiPropertyKey,
    value_type: ui_wire::UiPropertyValue,
    value: impl for<'a> FnOnce(
      &mut FlatBufferBuilder<'a>,
    ) -> flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
  ) -> Result<(), ProtocolError> {
    let mut builder = FlatBufferBuilder::new();
    let value = value(&mut builder);
    let property = ui_wire::UiProperty::create(
      &mut builder,
      &ui_wire::UiPropertyArgs {
        key,
        state: ui_wire::PropState::Set,
        style_value_kind: ui_wire::StyleValueKind::Value,
        value_type,
        value: Some(value),
      },
    );
    let properties = builder.create_vector(&[property]);
    let event_subscriptions =
      builder.create_vector::<flatbuffers::WIPOffset<ui_wire::UiEventSubscriptionValue<'_>>>(&[]);
    let part_styles = builder.create_vector::<flatbuffers::WIPOffset<ui_wire::PartStyle<'_>>>(&[]);
    let element = ui_wire::UiElement::create(
      &mut builder,
      &ui_wire::UiElementArgs {
        kind,
        properties: Some(properties),
        event_subscriptions: Some(event_subscriptions),
        part_styles: Some(part_styles),
        ..Default::default()
      },
    );
    builder.finish(element, None);
    let element = flatbuffers::root::<ui_wire::UiElement<'_>>(builder.finished_data()).unwrap();
    validate_element(element)
  }
}

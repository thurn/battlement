//! Verified response decoding for the retained in-memory fake world.

use battlement::RenderOrder;

use battlement::{
  Batch, BatchStart, Command, ParallelCommandGroup, Response, ResponseMessage, SessionId,
};
use battlement_flatbuffers::schema_generated::response_generated::battlement::flat_buffers::generated as wire;
use battlement_flatbuffers::schema_generated::{
  client_message_generated as client_wire, command_core_generated as command_wire,
  common_generated as common, world_generated as world_wire,
};

/// Verifies and decodes a core response for test assertions.
pub fn read(bytes: &[u8]) -> Result<Response, String> {
  let verified =
    battlement_flatbuffers::ResponseView::read(bytes).map_err(|error| error.to_string())?;
  let root = wire::size_prefixed_root_as_response(bytes).map_err(|error| error.to_string())?;
  let session_id = session(verified.session_id())?;
  let mut messages = Vec::with_capacity(root.messages().len());
  for entry in root.messages() {
    messages.push(match entry.message_type() {
      wire::ResponseMessage::Snapshot => {
        let value = entry
          .message_as_snapshot()
          .ok_or_else(|| "response snapshot payload is missing".to_owned())?;
        ResponseMessage::Snapshot(read_snapshot(value)?)
      }
      wire::ResponseMessage::Batch => {
        let value = entry
          .message_as_batch()
          .ok_or_else(|| "response batch payload is missing".to_owned())?;
        ResponseMessage::Batch(read_batch(value)?)
      }
      _ => return Err("response message union tag is unknown".to_owned()),
    });
  }
  Ok(Response {
    session_id,
    messages,
  })
}

fn read_batch(value: wire::Batch<'_>) -> Result<Batch, String> {
  let mut groups = Vec::with_capacity(value.groups().len());
  for group in value.groups() {
    let mut commands = Vec::with_capacity(group.commands().len());
    for entry in group.commands() {
      if entry.command_type() != wire::CommandEntryPayload::CoreCommand {
        return Err("the core fake cannot execute a custom response command".to_owned());
      }
      let value = entry
        .command_as_core_command()
        .ok_or_else(|| "core response command payload is missing".to_owned())?;
      commands.push(read_command(value)?);
    }
    groups.push(ParallelCommandGroup::new(commands));
  }
  let mut batch = Batch::new(
    battlement::BatchId::from_uuid(uuid(value.batch_id())?).map_err(|error| error.to_string())?,
    SessionId::from_uuid(uuid(value.session_id())?).map_err(|error| error.to_string())?,
    groups,
  );
  batch.caused_by_action_id = value
    .caused_by_action_id()
    .map(uuid)
    .transpose()?
    .map(|value| battlement::ActionId::from_uuid(value).expect("verified nonzero action UUID"));
  batch.work_scope = value.work_scope();
  batch.cancel_scope = value.cancel_scope();
  batch.start = match value.start() {
    wire::BatchStart::Now => BatchStart::Now,
    wire::BatchStart::AfterEarlierBlockingWork => BatchStart::AfterEarlierBlockingWork,
    wire::BatchStart::AfterEarlierAssetPreparation => BatchStart::AfterEarlierAssetPreparation,
    _ => return Err("response batch start is unknown".to_owned()),
  };
  Ok(batch)
}

fn read_snapshot(value: wire::Snapshot<'_>) -> Result<battlement::Snapshot, String> {
  let session_id =
    SessionId::from_uuid(uuid(value.session_id())?).map_err(|error| error.to_string())?;
  let prepared_assets = value
    .prepared_assets()
    .iter()
    .map(read_prepared_asset)
    .collect::<Result<Vec<_>, _>>()?;
  let scenes = value
    .scenes()
    .iter()
    .map(|scene| {
      Ok(battlement::Scene::new(
        battlement::SceneId::from_uuid(uuid(scene.scene_id())?)
          .map_err(|error| error.to_string())?,
        scene.address(),
      ))
    })
    .collect::<Result<Vec<_>, String>>()?;
  let objects = value
    .objects()
    .iter()
    .map(read_object)
    .collect::<Result<Vec<_>, _>>()?;
  let ui = value
    .ui()
    .iter()
    .map(crate::response_ui_reader::read_document)
    .collect::<Result<Vec<_>, _>>()?;
  let input = value.panel_input_configuration();
  let panel_input_configuration = battlement::PanelInputConfiguration {
    interaction_layers: battlement::InteractionLayerMask(input.interaction_layers()),
    maximum_interaction_distance: match input.distance_kind() {
      world_wire::InteractionDistanceKind::Unbounded => battlement::InteractionDistance::Unbounded,
      world_wire::InteractionDistanceKind::Inclusive => {
        battlement::InteractionDistance::Inclusive(input.maximum_interaction_distance())
      }
      _ => return Err("unknown panel interaction distance".to_owned()),
    },
    input_redirection: match input.input_redirection() {
      world_wire::PanelInputRedirection::AutoSwitch => {
        battlement::PanelInputRedirection::AutoSwitch
      }
      world_wire::PanelInputRedirection::Never => battlement::PanelInputRedirection::Never,
      world_wire::PanelInputRedirection::Always => battlement::PanelInputRedirection::Always,
      _ => return Err("unknown panel input redirection".to_owned()),
    },
  };
  let global_keys = value
    .global_keys()
    .iter()
    .map(physical_key)
    .collect::<Result<Vec<_>, _>>()?;
  let controller_input = value
    .controller_input()
    .map(read_controller_input)
    .transpose()?;
  Ok(battlement::Snapshot {
    session_id,
    prepared_assets,
    scenes,
    primary_scene_id: value
      .primary_scene_id()
      .map(uuid)
      .transpose()?
      .map(|value| battlement::SceneId::from_uuid(value).expect("verified scene UUID")),
    objects,
    ui,
    panel_input_configuration,
    input_camera_id: value
      .input_camera_id()
      .map(uuid)
      .transpose()?
      .map(|value| battlement::ObjectId::from_uuid(value).expect("verified object UUID")),
    input_disabled: value.input_disabled(),
    global_keys,
    controller_input,
  })
}

fn read_command(value: wire::CoreCommand<'_>) -> Result<Command, String> {
  crate::response_command::read(value)
}

pub(crate) fn read_prepared_asset(
  value: world_wire::PreparedAsset<'_>,
) -> Result<battlement::PreparedAsset, String> {
  let address = value.address();
  Ok(match value.kind() {
    world_wire::PreparedAssetKind::Mesh => battlement::PreparedAsset::Mesh(address.into()),
    world_wire::PreparedAssetKind::Scene => battlement::PreparedAsset::Scene(address.into()),
    world_wire::PreparedAssetKind::Prefab => battlement::PreparedAsset::Prefab(address.into()),
    world_wire::PreparedAssetKind::ParticleEffect => {
      battlement::PreparedAsset::ParticleEffect(address.into())
    }
    world_wire::PreparedAssetKind::Material => battlement::PreparedAsset::Material(address.into()),
    world_wire::PreparedAssetKind::MaterialParameters => {
      battlement::PreparedAsset::MaterialParameters {
        address: address.into(),
        parameters: crate::material::declarations(value.parameters())?,
      }
    }
    world_wire::PreparedAssetKind::Texture => battlement::PreparedAsset::Texture(address.into()),
    world_wire::PreparedAssetKind::Sprite => battlement::PreparedAsset::Sprite(address.into()),
    world_wire::PreparedAssetKind::VectorImage => {
      battlement::PreparedAsset::VectorImage(address.into())
    }
    world_wire::PreparedAssetKind::RenderTexture => {
      battlement::PreparedAsset::RenderTexture(address.into())
    }
    world_wire::PreparedAssetKind::AudioClip => {
      battlement::PreparedAsset::AudioClip(address.into())
    }
    world_wire::PreparedAssetKind::TextMeshProFont => {
      battlement::PreparedAsset::TextMeshProFont(address.into())
    }
    world_wire::PreparedAssetKind::UiFont => battlement::PreparedAsset::UiFont(address.into()),
    _ => return Err("unknown prepared asset kind".to_owned()),
  })
}

pub(crate) fn read_object(
  value: world_wire::GameObject<'_>,
) -> Result<battlement::GameObject, String> {
  let parent_scene = match value.parent_scene().kind() {
    world_wire::ParentSceneKind::PrimaryScene => battlement::ParentScene::PrimaryScene,
    world_wire::ParentSceneKind::Scene => battlement::ParentScene::Scene(
      battlement::SceneId::from_uuid(uuid(
        value
          .parent_scene()
          .scene_id()
          .ok_or_else(|| "scene parent is missing its identity".to_owned())?,
      )?)
      .map_err(|error| error.to_string())?,
    ),
    world_wire::ParentSceneKind::Persistent => battlement::ParentScene::Persistent,
    _ => return Err("unknown parent scene kind".to_owned()),
  };
  let transform = value.local_transform();
  let kind = read_object_kind(value)?;
  Ok(battlement::GameObject {
    object_id: object_id(value.object_id())?,
    parent_scene,
    parent_id: value.parent_id().map(object_id).transpose()?,
    active: value.active(),
    local_transform: battlement::LocalTransform {
      position: vector3(transform.position()),
      rotation: quaternion(transform.rotation()),
      scale: vector3(transform.scale()),
    },
    pointer_events: value
      .pointer_events()
      .iter()
      .map(|event| {
        Ok(match event {
          world_wire::PointerEventKind::Enter => battlement::PointerEvent::Enter,
          world_wire::PointerEventKind::Exit => battlement::PointerEvent::Exit,
          world_wire::PointerEventKind::Down => battlement::PointerEvent::Down,
          world_wire::PointerEventKind::Up => battlement::PointerEvent::Up,
          world_wire::PointerEventKind::Click => battlement::PointerEvent::Click,
          _ => return Err("unknown pointer event kind".to_owned()),
        })
      })
      .collect::<Result<Vec<_>, String>>()?,
    render_order: read_render_order(value.render_order())?,
    world_pointer: value.world_pointer().map(read_world_pointer),
    motion: value
      .motion()
      .map(crate::response_motion_descriptor_reader::descriptor)
      .transpose()?
      .map(Box::new),
    material_instances: crate::material::instances(value.material_instances())?,
    drag_mode: match value.drag_mode() {
      world_wire::DragMode::None => None,
      world_wire::DragMode::SnapToPointer => Some(battlement::DragMode::SnapToPointer),
      world_wire::DragMode::PreserveOffset => Some(battlement::DragMode::PreserveOffset),
      _ => return Err("unknown drag mode".to_owned()),
    },
    kind,
  })
}

pub(crate) fn read_render_order(
  value: Option<world_wire::RenderOrder<'_>>,
) -> Result<Option<RenderOrder>, String> {
  value
    .map(|value| match value.kind() {
      world_wire::RenderOrderKind::Group => Ok(RenderOrder::Group(value.order())),
      world_wire::RenderOrderKind::Layer => Ok(RenderOrder::Layer(value.order())),
      _ => Err("unknown render order kind".to_owned()),
    })
    .transpose()
}

fn read_object_kind(
  value: world_wire::GameObject<'_>,
) -> Result<battlement::GameObjectKind, String> {
  Ok(match value.kind() {
    world_wire::GameObjectKind::UiDocument => {
      battlement::GameObjectKind::UiDocument(read_ui_document_state(
        value
          .content_as_ui_document_object()
          .ok_or_else(|| "UI document object payload is missing".to_owned())?,
      )?)
    }
    world_wire::GameObjectKind::Empty => battlement::GameObjectKind::Empty,
    world_wire::GameObjectKind::BoxHitRegion => battlement::GameObjectKind::BoxHitRegion {
      region: read_box_region(
        value
          .content_as_box_hit_region_object()
          .ok_or("box region missing")?,
      ),
    },
    kind @ (world_wire::GameObjectKind::Cube
    | world_wire::GameObjectKind::Sphere
    | world_wire::GameObjectKind::Capsule
    | world_wire::GameObjectKind::Cylinder
    | world_wire::GameObjectKind::Plane
    | world_wire::GameObjectKind::Quad) => {
      let materials = read_materials(
        value
          .content_as_primitive_object()
          .ok_or_else(|| "primitive object payload is missing".to_owned())?
          .materials(),
      );
      match kind {
        world_wire::GameObjectKind::Cube => battlement::GameObjectKind::Cube { materials },
        world_wire::GameObjectKind::Sphere => battlement::GameObjectKind::Sphere { materials },
        world_wire::GameObjectKind::Capsule => battlement::GameObjectKind::Capsule { materials },
        world_wire::GameObjectKind::Cylinder => battlement::GameObjectKind::Cylinder { materials },
        world_wire::GameObjectKind::Plane => battlement::GameObjectKind::Plane { materials },
        world_wire::GameObjectKind::Quad => battlement::GameObjectKind::Quad { materials },
        _ => unreachable!(),
      }
    }
    world_wire::GameObjectKind::Image => {
      let image = value
        .content_as_image_object()
        .ok_or_else(|| "image object payload is missing".to_owned())?;
      battlement::GameObjectKind::Image {
        image: battlement::ImageState {
          texture: image.texture().into(),
          width: image.width(),
          height: image.height(),
          fit: match image.fit() {
            world_wire::ImageFit::Stretch => battlement::ImageFit::Stretch,
            world_wire::ImageFit::Contain => battlement::ImageFit::Contain,
            world_wire::ImageFit::Cover => battlement::ImageFit::Cover,
            _ => return Err("unknown image fit".to_owned()),
          },
          tint: rgb(image.tint()),
          opacity: image.opacity(),
          face_camera: image.face_camera(),
        },
      }
    }
    world_wire::GameObjectKind::Text => {
      let text = value
        .content_as_text_object()
        .ok_or_else(|| "text object payload is missing".to_owned())?;
      battlement::GameObjectKind::Text {
        text: battlement::TextState {
          text: text.text().to_owned(),
          font: text.font().into(),
          size: text.size(),
          color: color(text.color()),
          horizontal: match text.horizontal() {
            world_wire::HorizontalAlignment::Left => battlement::HorizontalAlignment::Left,
            world_wire::HorizontalAlignment::Center => battlement::HorizontalAlignment::Center,
            world_wire::HorizontalAlignment::Right => battlement::HorizontalAlignment::Right,
            world_wire::HorizontalAlignment::Justified => {
              battlement::HorizontalAlignment::Justified
            }
            _ => return Err("unknown horizontal alignment".to_owned()),
          },
          vertical: match text.vertical() {
            world_wire::VerticalAlignment::Top => battlement::VerticalAlignment::Top,
            world_wire::VerticalAlignment::Middle => battlement::VerticalAlignment::Middle,
            world_wire::VerticalAlignment::Bottom => battlement::VerticalAlignment::Bottom,
            _ => return Err("unknown vertical alignment".to_owned()),
          },
          wrap_width: text.wrap_width(),
          rich_text: text.rich_text(),
          face_camera: text.face_camera(),
        },
      }
    }
    world_wire::GameObjectKind::Camera => {
      let camera = value
        .content_as_camera_object()
        .ok_or_else(|| "camera object payload is missing".to_owned())?;
      battlement::GameObjectKind::Camera {
        camera: battlement::CameraState {
          enabled: camera.enabled(),
          projection: match camera.projection() {
            world_wire::CameraProjection::Perspective => battlement::CameraProjection::Perspective,
            world_wire::CameraProjection::Orthographic => {
              battlement::CameraProjection::Orthographic
            }
            _ => return Err("unknown camera projection".to_owned()),
          },
          field_of_view: camera.field_of_view(),
          orthographic_size: camera.orthographic_size(),
          near: camera.near(),
          far: camera.far(),
          clear_mode: match camera.clear_mode() {
            world_wire::CameraClearMode::Skybox => battlement::CameraClearMode::Skybox,
            world_wire::CameraClearMode::SolidColor => battlement::CameraClearMode::SolidColor,
            world_wire::CameraClearMode::Depth => battlement::CameraClearMode::Depth,
            world_wire::CameraClearMode::Nothing => battlement::CameraClearMode::Nothing,
            _ => return Err("unknown camera clear mode".to_owned()),
          },
          clear_color: color(camera.clear_color()),
        },
      }
    }
    world_wire::GameObjectKind::Light => {
      let light = value
        .content_as_light_object()
        .ok_or_else(|| "light object payload is missing".to_owned())?;
      battlement::GameObjectKind::Light {
        light: battlement::LightState {
          enabled: light.enabled(),
          light_type: match light.light_type() {
            world_wire::LightType::Directional => battlement::LightType::Directional,
            world_wire::LightType::Point => battlement::LightType::Point,
            world_wire::LightType::Spot => battlement::LightType::Spot,
            _ => return Err("unknown light type".to_owned()),
          },
          color: color(light.color()),
          intensity: light.intensity(),
          range: light.range(),
          outer_spot_angle: light.outer_spot_angle(),
          inner_spot_angle: light.inner_spot_angle(),
          shadows: match light.shadows() {
            world_wire::ShadowMode::None => battlement::ShadowMode::None,
            world_wire::ShadowMode::Hard => battlement::ShadowMode::Hard,
            world_wire::ShadowMode::Soft => battlement::ShadowMode::Soft,
            _ => return Err("unknown shadow mode".to_owned()),
          },
        },
      }
    }
    world_wire::GameObjectKind::Mesh => {
      let mesh = value
        .content_as_mesh_object()
        .ok_or_else(|| "mesh object payload is missing".to_owned())?;
      battlement::GameObjectKind::Mesh {
        address: mesh.address().into(),
        materials: read_materials(mesh.materials()),
      }
    }
    world_wire::GameObjectKind::Prefab => {
      let prefab = value
        .content_as_prefab_object()
        .ok_or_else(|| "prefab object payload is missing".to_owned())?;
      battlement::GameObjectKind::Prefab {
        address: prefab.address().into(),
        materials: read_materials(prefab.materials()),
        animator: prefab.animator().map(read_animator),
      }
    }
    _ => return Err("unknown game object kind".to_owned()),
  })
}

fn read_materials(
  values: flatbuffers::Vector<'_, flatbuffers::ForwardsUOffset<world_wire::MaterialAssignment<'_>>>,
) -> Vec<battlement::MaterialAssignment> {
  values
    .iter()
    .map(|value| battlement::MaterialAssignment::new(value.slot(), value.address()))
    .collect()
}

fn read_animator(value: world_wire::AnimatorState<'_>) -> battlement::AnimatorState {
  battlement::AnimatorState {
    state: value.state().to_owned(),
    layer: value.layer(),
    normalized_start_time: value.normalized_start_time(),
    bool_parameters: value
      .bool_parameters()
      .iter()
      .map(|item| (item.name().to_owned(), item.value()))
      .collect(),
    int_parameters: value
      .int_parameters()
      .iter()
      .map(|item| (item.name().to_owned(), item.value()))
      .collect(),
    float_parameters: value
      .float_parameters()
      .iter()
      .map(|item| (item.name().to_owned(), item.value()))
      .collect(),
    speed: value.speed(),
  }
}

fn read_ui_document_state(
  value: world_wire::UiDocumentObject<'_>,
) -> Result<battlement::UiDocumentState, String> {
  let mut state = battlement::UiDocumentState::new(object_id(value.root_id())?);
  state.position = match value.position() {
    world_wire::DocumentPosition::Relative => battlement::DocumentPosition::Relative,
    world_wire::DocumentPosition::Absolute => battlement::DocumentPosition::Absolute,
    _ => return Err("unknown document position".to_owned()),
  };
  state.world_space_size_mode = match value.world_space_size_mode() {
    world_wire::WorldSpaceSizeMode::Fixed => battlement::WorldSpaceSizeMode::Fixed,
    world_wire::WorldSpaceSizeMode::Dynamic => battlement::WorldSpaceSizeMode::Dynamic,
    _ => return Err("unknown world-space size mode".to_owned()),
  };
  state.world_space_size = battlement::ScreenSize::new(
    value.world_space_size().width(),
    value.world_space_size().height(),
  );
  state.pivot_reference_size = match value.pivot_reference_size() {
    world_wire::PivotReferenceSize::BoundingBox => battlement::PivotReferenceSize::BoundingBox,
    world_wire::PivotReferenceSize::Layout => battlement::PivotReferenceSize::Layout,
    _ => return Err("unknown pivot reference size".to_owned()),
  };
  state.pivot = document_pivot(value.pivot())?;
  state.sorting_order = value.sorting_order();
  state.panel_settings = read_panel_settings(value.panel_settings())?;
  Ok(state)
}

fn read_panel_settings(
  value: world_wire::PanelSettings<'_>,
) -> Result<battlement::PanelSettings, String> {
  let atlas = value.dynamic_atlas();
  let dynamic_atlas = battlement::DynamicAtlasSettings {
    min_atlas_size: atlas.min_atlas_size(),
    max_atlas_size: atlas.max_atlas_size(),
    max_sub_texture_size: atlas.max_sub_texture_size(),
    filters: atlas
      .filters()
      .iter()
      .map(|value| {
        Ok(match value {
          world_wire::DynamicAtlasFilter::Readability => {
            battlement::DynamicAtlasFilter::Readability
          }
          world_wire::DynamicAtlasFilter::Size => battlement::DynamicAtlasFilter::Size,
          world_wire::DynamicAtlasFilter::Format => battlement::DynamicAtlasFilter::Format,
          world_wire::DynamicAtlasFilter::ColorSpace => battlement::DynamicAtlasFilter::ColorSpace,
          world_wire::DynamicAtlasFilter::FilterMode => battlement::DynamicAtlasFilter::FilterMode,
          _ => return Err("unknown dynamic atlas filter".to_owned()),
        })
      })
      .collect::<Result<Vec<_>, String>>()?,
  };
  let scale_mode = match value.scale_mode() {
    world_wire::PanelScaleMode::ConstantPixelSize => {
      battlement::PanelScaleMode::constant_pixel_size(value.scale())
    }
    world_wire::PanelScaleMode::ConstantLogicalPixelSize => {
      battlement::PanelScaleMode::ConstantLogicalPixelSize
    }
    world_wire::PanelScaleMode::ConstantPhysicalSize => {
      battlement::PanelScaleMode::constant_physical_size(
        value.reference_dpi(),
        value.fallback_dpi(),
      )
    }
    world_wire::PanelScaleMode::ScaleWithScreenSize => {
      let size = value.reference_resolution();
      let mode = match value.screen_match_mode() {
        world_wire::PanelScreenMatchMode::MatchWidthOrHeight => {
          battlement::PanelScreenMatchMode::match_width_or_height(value.match_factor())
        }
        world_wire::PanelScreenMatchMode::Shrink => battlement::PanelScreenMatchMode::Shrink,
        world_wire::PanelScreenMatchMode::Expand => battlement::PanelScreenMatchMode::Expand,
        _ => return Err("unknown panel screen match mode".to_owned()),
      };
      battlement::PanelScaleMode::scale_with_screen_size(
        battlement::ScreenSize::new(size.width(), size.height()),
        mode,
      )
    }
    _ => return Err("unknown panel scale mode".to_owned()),
  };
  Ok(battlement::PanelSettings {
    render_mode: match value.render_mode() {
      world_wire::PanelRenderMode::ScreenSpaceOverlay => {
        battlement::PanelRenderMode::ScreenSpaceOverlay
      }
      world_wire::PanelRenderMode::WorldSpace => battlement::PanelRenderMode::WorldSpace,
      _ => return Err("unknown panel render mode".to_owned()),
    },
    scale_mode,
    reference_sprite_pixels_per_unit: value.reference_sprite_pixels_per_unit(),
    target_display: value.target_display(),
    target_texture: value.target_texture().map(Into::into),
    clear_depth_stencil: value.clear_depth_stencil(),
    clear_color: value.clear_color(),
    color_clear_value: color(value.color_clear_value()),
    dynamic_atlas,
  })
}

fn read_controller_input(
  value: command_wire::ControllerInputSettings<'_>,
) -> Result<battlement::ControllerInputSettings, String> {
  Ok(battlement::ControllerInputSettings {
    buttons: value
      .buttons()
      .iter()
      .map(|value| {
        Ok(match value {
          client_wire::ControllerButton::South => battlement::ControllerButton::South,
          client_wire::ControllerButton::East => battlement::ControllerButton::East,
          client_wire::ControllerButton::West => battlement::ControllerButton::West,
          client_wire::ControllerButton::North => battlement::ControllerButton::North,
          client_wire::ControllerButton::LeftShoulder => battlement::ControllerButton::LeftShoulder,
          client_wire::ControllerButton::RightShoulder => {
            battlement::ControllerButton::RightShoulder
          }
          client_wire::ControllerButton::LeftStickButton => {
            battlement::ControllerButton::LeftStickButton
          }
          client_wire::ControllerButton::RightStickButton => {
            battlement::ControllerButton::RightStickButton
          }
          client_wire::ControllerButton::Select => battlement::ControllerButton::Select,
          client_wire::ControllerButton::Start => battlement::ControllerButton::Start,
          _ => return Err("unknown controller button".to_owned()),
        })
      })
      .collect::<Result<Vec<_>, String>>()?,
    navigation_enabled: value.navigation_enabled(),
    stick_dead_zone: value.stick_dead_zone(),
    repeat_delay_ms: value.repeat_delay_ms(),
    repeat_interval_ms: value.repeat_interval_ms(),
  })
}

fn document_pivot(value: world_wire::DocumentPivot) -> Result<battlement::DocumentPivot, String> {
  Ok(match value {
    world_wire::DocumentPivot::TopLeft => battlement::DocumentPivot::TopLeft,
    world_wire::DocumentPivot::TopCenter => battlement::DocumentPivot::TopCenter,
    world_wire::DocumentPivot::TopRight => battlement::DocumentPivot::TopRight,
    world_wire::DocumentPivot::MiddleLeft => battlement::DocumentPivot::MiddleLeft,
    world_wire::DocumentPivot::Center => battlement::DocumentPivot::Center,
    world_wire::DocumentPivot::MiddleRight => battlement::DocumentPivot::MiddleRight,
    world_wire::DocumentPivot::BottomLeft => battlement::DocumentPivot::BottomLeft,
    world_wire::DocumentPivot::BottomCenter => battlement::DocumentPivot::BottomCenter,
    world_wire::DocumentPivot::BottomRight => battlement::DocumentPivot::BottomRight,
    _ => return Err("unknown document pivot".to_owned()),
  })
}
pub(crate) fn object_id(value: &common::Uuid) -> Result<battlement::ObjectId, String> {
  battlement::ObjectId::from_uuid(uuid(value)?).map_err(|error| error.to_string())
}
fn vector3(value: &common::Vector3d) -> battlement::Vector3 {
  battlement::Vector3::new(value.x(), value.y(), value.z())
}
fn quaternion(value: &common::Quaterniond) -> battlement::Quaternion {
  battlement::Quaternion::new(value.x(), value.y(), value.z(), value.w())
}
fn rgb(value: &common::RgbColor) -> battlement::RgbColor {
  battlement::RgbColor::rgb(value.r(), value.g(), value.b())
}
fn color(value: &common::RgbaColor) -> battlement::Color {
  battlement::Color {
    r: value.r(),
    g: value.g(),
    b: value.b(),
    a: value.a(),
  }
}

fn physical_key(
  value: battlement_flatbuffers::schema_generated::ui_event_generated::PhysicalKey,
) -> Result<battlement::PhysicalKey, String> {
  battlement_flatbuffers::physical_key_from_ordinal(value.0)
    .ok_or_else(|| "unknown physical key".to_owned())
}

fn uuid(
  value: &battlement_flatbuffers::schema_generated::common_generated::Uuid,
) -> Result<uuid::Uuid, String> {
  let bytes = std::array::from_fn(|index| value.bytes().get(index));
  let value = uuid::Uuid::from_bytes(bytes);
  if value.is_nil() {
    Err("protocol UUID is zero".to_owned())
  } else {
    Ok(value)
  }
}

fn session(bytes: [u8; 16]) -> Result<SessionId, String> {
  SessionId::from_uuid(uuid::Uuid::from_bytes(bytes)).map_err(|error| error.to_string())
}

pub(crate) fn read_box_region(
  value: world_wire::BoxHitRegionObject<'_>,
) -> battlement::BoxHitRegionState {
  battlement::BoxHitRegionState {
    size: vector3(value.size()),
    center: vector3(value.center()),
  }
}

pub(crate) fn read_world_pointer(
  value: world_wire::WorldPointerSettings<'_>,
) -> battlement::WorldPointerSettings {
  battlement::WorldPointerSettings {
    interaction_layer: value.interaction_layer(),
    order: value.order(),
    capture_on_press: value.capture_on_press(),
    focusable: value.focusable(),
  }
}

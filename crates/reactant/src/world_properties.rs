//! Existing component commands preserve compatible world host identities.

use battlement::{
  CameraClearMode, CameraClearPayload, CameraClippingPayload, CameraProjection, CameraState,
  ColorPayload, CommandBody, GameObjectKind, ImageFitPayload, ImageSizePayload, ImageState,
  IntensityPayload, LightRangePayload, LightShadowsPayload, LightState, LightType,
  LightTypePayload, ObjectEnabledPayload, ObjectId, OpacityPayload, OrthographicPayload,
  PerspectivePayload, PropertyCommand, SetTexturePayload, SpotAnglePayload, TintPayload,
};

use crate::world_text;

pub(crate) fn compatible(previous: &GameObjectKind, desired: &GameObjectKind) -> bool {
  match (previous, desired) {
    (GameObjectKind::BoxHitRegion { .. }, GameObjectKind::BoxHitRegion { .. })
    | (GameObjectKind::Image { .. }, GameObjectKind::Image { .. })
    | (GameObjectKind::Text { .. }, GameObjectKind::Text { .. })
    | (GameObjectKind::Camera { .. }, GameObjectKind::Camera { .. })
    | (GameObjectKind::Light { .. }, GameObjectKind::Light { .. }) => true,
    _ => previous == desired,
  }
}

pub(crate) fn commands(
  object_id: ObjectId,
  previous: &GameObjectKind,
  desired: &GameObjectKind,
) -> Vec<CommandBody> {
  let mut output = Vec::new();
  match (previous, desired) {
    (
      GameObjectKind::BoxHitRegion { region: previous },
      GameObjectKind::BoxHitRegion { region: desired },
    ) if previous != desired => {
      output.push(CommandBody::BoxHitRegionSetGeometry(
        battlement::BoxHitRegionPayload {
          object_id,
          region: *desired,
        },
      ));
    }
    (GameObjectKind::Text { text: previous }, GameObjectKind::Text { text: desired }) => {
      world_text::commands(object_id, previous, desired, &mut output);
    }
    (GameObjectKind::Image { image: previous }, GameObjectKind::Image { image: desired }) => {
      self::sprite(object_id, previous, desired, &mut output);
    }
    (GameObjectKind::Camera { camera: previous }, GameObjectKind::Camera { camera: desired }) => {
      self::camera(object_id, previous, desired, &mut output);
    }
    (GameObjectKind::Light { light: previous }, GameObjectKind::Light { light: desired }) => {
      self::light(object_id, previous, desired, &mut output);
    }
    _ => {}
  }
  output
}

fn sprite(object_id: ObjectId, old: &ImageState, new: &ImageState, out: &mut Vec<CommandBody>) {
  if old.texture != new.texture {
    out.push(CommandBody::ImageSetTexture(SetTexturePayload {
      object_id,
      address: new.texture.clone(),
    }));
  }
  if old.width != new.width || old.height != new.height {
    out.push(CommandBody::ImageSetSize(ImageSizePayload {
      object_id,
      width: new.width,
      height: new.height,
    }));
  }
  if old.fit != new.fit {
    out.push(CommandBody::ImageSetFit(ImageFitPayload {
      object_id,
      fit: new.fit,
    }));
  }
  if old.tint != new.tint {
    out.push(CommandBody::ImageSetTint(PropertyCommand::canceling(
      TintPayload {
        object_id,
        tint: new.tint,
      },
    )));
  }
  if old.opacity != new.opacity {
    out.push(CommandBody::ImageSetOpacity(PropertyCommand::canceling(
      OpacityPayload {
        object_id,
        opacity: new.opacity,
      },
    )));
  }
  if old.face_camera != new.face_camera {
    out.push(CommandBody::ImageSetFaceCamera(ObjectEnabledPayload {
      object_id,
      enabled: new.face_camera,
    }));
  }
}

fn camera(object_id: ObjectId, old: &CameraState, new: &CameraState, out: &mut Vec<CommandBody>) {
  if old.enabled != new.enabled {
    out.push(CommandBody::CameraSetEnabled(ObjectEnabledPayload {
      object_id,
      enabled: new.enabled,
    }));
  }
  let projection_changed = old.projection != new.projection;
  match new.projection {
    CameraProjection::Perspective
      if projection_changed || old.field_of_view != new.field_of_view =>
    {
      out.push(CommandBody::CameraSetPerspective(
        PropertyCommand::canceling(PerspectivePayload {
          object_id,
          field_of_view: new.field_of_view,
        }),
      ));
    }
    CameraProjection::Orthographic
      if projection_changed || old.orthographic_size != new.orthographic_size =>
    {
      out.push(CommandBody::CameraSetOrthographic(
        PropertyCommand::canceling(OrthographicPayload {
          object_id,
          size: new.orthographic_size,
        }),
      ));
    }
    _ => {}
  }
  if old.near != new.near || old.far != new.far {
    out.push(CommandBody::CameraSetClipping(CameraClippingPayload {
      object_id,
      near: new.near,
      far: new.far,
    }));
  }
  if old.clear_mode != new.clear_mode || old.clear_color != new.clear_color {
    out.push(CommandBody::CameraSetClear(CameraClearPayload {
      object_id,
      clear_mode: new.clear_mode,
      clear_color: (new.clear_mode == CameraClearMode::SolidColor).then_some(new.clear_color),
    }));
  }
}

fn light(object_id: ObjectId, old: &LightState, new: &LightState, out: &mut Vec<CommandBody>) {
  if old.enabled != new.enabled {
    out.push(CommandBody::LightSetEnabled(ObjectEnabledPayload {
      object_id,
      enabled: new.enabled,
    }));
  }
  let type_changed = old.light_type != new.light_type;
  if type_changed {
    out.push(CommandBody::LightSetType(LightTypePayload {
      object_id,
      light_type: new.light_type,
    }));
  }
  if old.color != new.color {
    out.push(CommandBody::LightSetColor(PropertyCommand::canceling(
      ColorPayload {
        object_id,
        color: new.color,
      },
    )));
  }
  if old.intensity != new.intensity {
    out.push(CommandBody::LightSetIntensity(PropertyCommand::canceling(
      IntensityPayload {
        object_id,
        intensity: new.intensity,
      },
    )));
  }
  if new.light_type != LightType::Directional && (type_changed || old.range != new.range) {
    out.push(CommandBody::LightSetRange(LightRangePayload {
      object_id,
      range: new.range,
    }));
  }
  let angles_changed =
    old.inner_spot_angle != new.inner_spot_angle || old.outer_spot_angle != new.outer_spot_angle;
  if new.light_type == LightType::Spot && (type_changed || angles_changed) {
    out.push(CommandBody::LightSetSpotAngle(SpotAnglePayload {
      object_id,
      inner_spot_angle: new.inner_spot_angle,
      outer_spot_angle: new.outer_spot_angle,
    }));
  }
  if old.shadows != new.shadows {
    out.push(CommandBody::LightSetShadows(LightShadowsPayload {
      object_id,
      shadows: new.shadows,
    }));
  }
}

//! Owned fake-state copies made only after response verification.

use battlement::{
  Command, CommandBody, ConflictPolicy, PropertyCommand,
  display::DisplayCommand,
  host_settings::{DisplayConfiguration, DisplayMode, DisplayResolution},
};
use battlement_flatbuffers::schema_generated::{
  accessibility_generated as accessibility, command_core_generated as payload,
  common_generated as common, geometry_generated as geometry,
  response_generated::battlement::flat_buffers::generated as wire, ui_generated as ui_wire,
};

pub(crate) fn read(value: wire::CoreCommand<'_>) -> Result<Command, String> {
  let body = read_body(value)?;
  let mut command = Command::new(command_id(value.command_id())?, body);
  command.blocking = value.blocking();
  Ok(command)
}

fn read_body(value: wire::CoreCommand<'_>) -> Result<CommandBody, String> {
  use wire::CoreCommandKind as Kind;
  let missing = || format!("response command payload is missing for {:?}", value.kind());
  Ok(match value.kind() {
    Kind::ApplicationOpenUrl => {
      CommandBody::ApplicationOpenUrl(battlement::application::ExternalUrlRequest {
        url: value
          .payload_as_external_url_payload()
          .ok_or_else(missing)?
          .url()
          .to_owned(),
      })
    }
    Kind::ApplicationDisplay => {
      let body = value
        .payload_as_display_command_payload()
        .expect("validated display payload");
      CommandBody::ApplicationDisplay(match body.operation() {
        payload::DisplayOperation::Preview => {
          let configuration = body
            .configuration()
            .expect("validated display configuration");
          let resolution = configuration.resolution();
          DisplayCommand::Preview(DisplayConfiguration {
            mode: match configuration.mode().0 {
              0 => DisplayMode::Windowed,
              1 => DisplayMode::Borderless,
              2 => DisplayMode::Fullscreen,
              _ => unreachable!("validated display mode"),
            },
            resolution: DisplayResolution {
              width: resolution.width(),
              height: resolution.height(),
              refresh_numerator: resolution.refresh_numerator(),
              refresh_denominator: resolution.refresh_denominator(),
            },
          })
        }
        payload::DisplayOperation::Confirm => DisplayCommand::Confirm(command_id(
          body.preview_id().expect("validated preview ID"),
        )?),
        payload::DisplayOperation::Cancel => DisplayCommand::Cancel(command_id(
          body.preview_id().expect("validated preview ID"),
        )?),
        _ => unreachable!("validated display operation"),
      })
    }
    Kind::ApplicationSetFramePacing => {
      let body = value
        .payload_as_frame_pacing_payload()
        .expect("validated pacing payload");
      CommandBody::ApplicationSetFramePacing(battlement::frame_pacing::FramePacing {
        maximum_frame_rate: body.maximum_frame_rate(),
        vsync: body.vsync(),
      })
    }
    Kind::Diagnostics => {
      let body = value.payload_as_diagnostics_payload().ok_or_else(missing)?;
      CommandBody::Diagnostics(match body.operation() {
        payload::DiagnosticsOperation::SetMetadata => {
          battlement_cloud::diagnostics::DiagnosticsCommand::SetMetadata(
            battlement_cloud::diagnostics::DiagnosticsMetadata {
              key: body.key().ok_or_else(missing)?.to_owned(),
              value: body.value().map(str::to_owned),
            },
          )
        }
        payload::DiagnosticsOperation::SetReporting => {
          battlement_cloud::diagnostics::DiagnosticsCommand::SetReporting(body.enabled())
        }
        _ => return Err(missing()),
      })
    }
    Kind::AssetsReplaceSet => {
      let body = value
        .payload_as_replace_asset_set_payload()
        .ok_or_else(missing)?;
      CommandBody::AssetsReplaceSet(battlement::ReplaceAssetSetPayload {
        assets: body
          .assets()
          .iter()
          .map(crate::response_reader::read_prepared_asset)
          .collect::<Result<_, _>>()?,
      })
    }
    Kind::SceneLoad => {
      let body = value.payload_as_scene_load_payload().ok_or_else(missing)?;
      CommandBody::SceneLoad(battlement::SceneLoadPayload {
        scene_id: scene_id(body.scene_id())?,
        address: body.address().into(),
        make_primary: body.make_primary(),
      })
    }
    Kind::SceneUnload | Kind::SceneSetPrimary => {
      let body = value.payload_as_scene_id_payload().ok_or_else(missing)?;
      let payload = battlement::SceneIdPayload {
        scene_id: scene_id(body.scene_id())?,
      };
      if value.kind() == Kind::SceneUnload {
        CommandBody::SceneUnload(payload)
      } else {
        CommandBody::SceneSetPrimary(payload)
      }
    }
    Kind::ObjectCreate => {
      let body = value
        .payload_as_object_create_payload()
        .ok_or_else(missing)?;
      CommandBody::ObjectCreate(Box::new(battlement::ObjectCreatePayload {
        object: crate::response_reader::read_object(body.object())?,
      }))
    }
    Kind::ObjectDestroy | Kind::InputSetCamera => {
      let body = value.payload_as_object_id_payload().ok_or_else(missing)?;
      let payload = battlement::ObjectIdPayload {
        object_id: object_id(body.object_id())?,
      };
      if value.kind() == Kind::ObjectDestroy {
        CommandBody::ObjectDestroy(payload)
      } else {
        CommandBody::InputSetCamera(payload)
      }
    }
    Kind::BoxHitRegionSetGeometry => {
      let body = value
        .payload_as_box_hit_region_payload()
        .ok_or("box region payload")?;
      CommandBody::BoxHitRegionSetGeometry(battlement::BoxHitRegionPayload {
        object_id: object_id(body.object_id())?,
        region: crate::response_reader::read_box_region(body.region()),
      })
    }
    Kind::RendererSetInstances => {
      let body = value
        .payload_as_renderer_instances_payload()
        .ok_or("material instances payload")?;
      CommandBody::RendererSetInstances(battlement::RendererInstancesPayload {
        object_id: object_id(body.object_id())?,
        instances: crate::material::instances(Some(body.instances()))?,
      })
    }
    Kind::ObjectSetRenderOrder => {
      let body = value
        .payload_as_object_render_order_payload()
        .ok_or_else(missing)?;
      CommandBody::ObjectSetRenderOrder(battlement::ObjectRenderOrderPayload {
        object_id: object_id(body.object_id())?,
        render_order: crate::response_reader::read_render_order(body.render_order())?,
      })
    }
    Kind::ObjectSetActive => {
      let body = value
        .payload_as_object_set_active_payload()
        .ok_or_else(missing)?;
      CommandBody::ObjectSetActive(battlement::ObjectSetActivePayload {
        object_id: object_id(body.object_id())?,
        active: body.active(),
      })
    }
    Kind::ObjectReparent => {
      let body = value
        .payload_as_object_reparent_payload()
        .ok_or_else(missing)?;
      CommandBody::ObjectReparent(battlement::ObjectReparentPayload {
        object_id: object_id(body.object_id())?,
        parent_id: body.parent_id().map(object_id).transpose()?,
        world_position_stays: body.world_position_stays(),
      })
    }
    Kind::TransformSetLocalPosition | Kind::TransformSetWorldPosition => {
      let body = value.payload_as_position_payload().ok_or_else(missing)?;
      let body = property(
        body.on_conflict(),
        battlement::PositionPayload {
          object_id: object_id(body.object_id())?,
          position: vector3(body.position()),
        },
      )?;
      if value.kind() == Kind::TransformSetLocalPosition {
        CommandBody::TransformSetLocalPosition(body)
      } else {
        CommandBody::TransformSetWorldPosition(body)
      }
    }
    Kind::TransformTweenLocalPosition | Kind::TransformTweenWorldPosition => {
      let body = value
        .payload_as_tween_position_payload()
        .ok_or_else(missing)?;
      let body = property(
        body.on_conflict(),
        battlement::TweenPositionPayload {
          object_id: object_id(body.object_id())?,
          position: vector3(body.position()),
          tween: tween(body.tween())?,
        },
      )?;
      if value.kind() == Kind::TransformTweenLocalPosition {
        CommandBody::TransformTweenLocalPosition(body)
      } else {
        CommandBody::TransformTweenWorldPosition(body)
      }
    }
    Kind::TransformSetLocalRotation | Kind::TransformSetWorldRotation => {
      let body = value.payload_as_rotation_payload().ok_or_else(missing)?;
      let body = property(
        body.on_conflict(),
        battlement::RotationPayload {
          object_id: object_id(body.object_id())?,
          rotation: quaternion(body.rotation()),
        },
      )?;
      if value.kind() == Kind::TransformSetLocalRotation {
        CommandBody::TransformSetLocalRotation(body)
      } else {
        CommandBody::TransformSetWorldRotation(body)
      }
    }
    Kind::TransformTweenLocalRotation | Kind::TransformTweenWorldRotation => {
      let body = value
        .payload_as_tween_rotation_payload()
        .ok_or_else(missing)?;
      let body = property(
        body.on_conflict(),
        battlement::TweenRotationPayload {
          object_id: object_id(body.object_id())?,
          rotation: quaternion(body.rotation()),
          tween: tween(body.tween())?,
        },
      )?;
      if value.kind() == Kind::TransformTweenLocalRotation {
        CommandBody::TransformTweenLocalRotation(body)
      } else {
        CommandBody::TransformTweenWorldRotation(body)
      }
    }
    Kind::TransformSetLocalScale => {
      let body = value.payload_as_scale_payload().ok_or_else(missing)?;
      CommandBody::TransformSetLocalScale(property(
        body.on_conflict(),
        battlement::ScalePayload {
          object_id: object_id(body.object_id())?,
          scale: vector3(body.scale()),
        },
      )?)
    }
    Kind::TransformTweenLocalScale => {
      let body = value.payload_as_tween_scale_payload().ok_or_else(missing)?;
      CommandBody::TransformTweenLocalScale(property(
        body.on_conflict(),
        battlement::TweenScalePayload {
          object_id: object_id(body.object_id())?,
          scale: vector3(body.scale()),
          tween: tween(body.tween())?,
        },
      )?)
    }
    Kind::RendererSetMaterial => {
      let body = value
        .payload_as_set_material_payload()
        .ok_or_else(missing)?;
      CommandBody::RendererSetMaterial(property(
        body.on_conflict(),
        battlement::SetMaterialPayload {
          object_id: object_id(body.object_id())?,
          address: body.address().into(),
          slot: body.slot(),
        },
      )?)
    }
    Kind::CameraSetEnabled
    | Kind::LightSetEnabled
    | Kind::ImageSetFaceCamera
    | Kind::TextSetRichText
    | Kind::TextSetFaceCamera => {
      let body = value
        .payload_as_object_enabled_payload()
        .ok_or_else(missing)?;
      let body = battlement::ObjectEnabledPayload {
        object_id: object_id(body.object_id())?,
        enabled: body.enabled(),
      };
      match value.kind() {
        Kind::CameraSetEnabled => CommandBody::CameraSetEnabled(body),
        Kind::LightSetEnabled => CommandBody::LightSetEnabled(body),
        Kind::ImageSetFaceCamera => CommandBody::ImageSetFaceCamera(body),
        Kind::TextSetRichText => CommandBody::TextSetRichText(body),
        Kind::TextSetFaceCamera => CommandBody::TextSetFaceCamera(body),
        _ => unreachable!(),
      }
    }
    Kind::CameraSetPerspective => {
      let body = value.payload_as_perspective_payload().ok_or_else(missing)?;
      CommandBody::CameraSetPerspective(property(
        body.on_conflict(),
        battlement::PerspectivePayload {
          object_id: object_id(body.object_id())?,
          field_of_view: body.field_of_view(),
        },
      )?)
    }
    Kind::CameraTweenFieldOfView => {
      let body = value
        .payload_as_tween_field_of_view_payload()
        .ok_or_else(missing)?;
      CommandBody::CameraTweenFieldOfView(property(
        body.on_conflict(),
        battlement::TweenFieldOfViewPayload {
          object_id: object_id(body.object_id())?,
          field_of_view: body.field_of_view(),
          tween: tween(body.tween())?,
        },
      )?)
    }
    Kind::CameraSetOrthographic => {
      let body = value
        .payload_as_orthographic_payload()
        .ok_or_else(missing)?;
      CommandBody::CameraSetOrthographic(property(
        body.on_conflict(),
        battlement::OrthographicPayload {
          object_id: object_id(body.object_id())?,
          size: body.size(),
        },
      )?)
    }
    Kind::CameraTweenOrthographicSize => {
      let body = value
        .payload_as_tween_orthographic_size_payload()
        .ok_or_else(missing)?;
      CommandBody::CameraTweenOrthographicSize(property(
        body.on_conflict(),
        battlement::TweenOrthographicSizePayload {
          object_id: object_id(body.object_id())?,
          size: body.size(),
          tween: tween(body.tween())?,
        },
      )?)
    }
    Kind::CameraSetClipping => {
      let body = value
        .payload_as_camera_clipping_payload()
        .ok_or_else(missing)?;
      CommandBody::CameraSetClipping(battlement::CameraClippingPayload {
        object_id: object_id(body.object_id())?,
        near: body.near(),
        far: body.far(),
      })
    }
    Kind::CameraSetClear => {
      let body = value
        .payload_as_camera_clear_payload()
        .ok_or_else(missing)?;
      CommandBody::CameraSetClear(battlement::CameraClearPayload {
        object_id: object_id(body.object_id())?,
        clear_mode: camera_clear(body.clear_mode())?,
        clear_color: body.clear_color().map(color),
      })
    }
    Kind::LightSetType => {
      let body = value.payload_as_light_type_payload().ok_or_else(missing)?;
      CommandBody::LightSetType(battlement::LightTypePayload {
        object_id: object_id(body.object_id())?,
        light_type: light_type(body.light_type())?,
      })
    }
    Kind::LightSetColor | Kind::TextSetColor => {
      let body = value.payload_as_color_payload().ok_or_else(missing)?;
      let body = property(
        body.on_conflict(),
        battlement::ColorPayload {
          object_id: object_id(body.object_id())?,
          color: color(body.color()),
        },
      )?;
      if value.kind() == Kind::LightSetColor {
        CommandBody::LightSetColor(body)
      } else {
        CommandBody::TextSetColor(body)
      }
    }
    Kind::LightTweenColor | Kind::TextTweenColor => {
      let body = value.payload_as_tween_color_payload().ok_or_else(missing)?;
      let body = property(
        body.on_conflict(),
        battlement::TweenColorPayload {
          object_id: object_id(body.object_id())?,
          color: color(body.color()),
          tween: tween(body.tween())?,
        },
      )?;
      if value.kind() == Kind::LightTweenColor {
        CommandBody::LightTweenColor(body)
      } else {
        CommandBody::TextTweenColor(body)
      }
    }
    Kind::LightSetIntensity => {
      let body = value.payload_as_intensity_payload().ok_or_else(missing)?;
      CommandBody::LightSetIntensity(property(
        body.on_conflict(),
        battlement::IntensityPayload {
          object_id: object_id(body.object_id())?,
          intensity: body.intensity(),
        },
      )?)
    }
    Kind::LightTweenIntensity => {
      let body = value
        .payload_as_tween_intensity_payload()
        .ok_or_else(missing)?;
      CommandBody::LightTweenIntensity(property(
        body.on_conflict(),
        battlement::TweenIntensityPayload {
          object_id: object_id(body.object_id())?,
          intensity: body.intensity(),
          tween: tween(body.tween())?,
        },
      )?)
    }
    Kind::LightSetRange => {
      let body = value.payload_as_light_range_payload().ok_or_else(missing)?;
      CommandBody::LightSetRange(battlement::LightRangePayload {
        object_id: object_id(body.object_id())?,
        range: body.range(),
      })
    }
    Kind::LightSetSpotAngle => {
      let body = value.payload_as_spot_angle_payload().ok_or_else(missing)?;
      CommandBody::LightSetSpotAngle(battlement::SpotAnglePayload {
        object_id: object_id(body.object_id())?,
        outer_spot_angle: body.outer_spot_angle(),
        inner_spot_angle: body.inner_spot_angle(),
      })
    }
    Kind::LightSetShadows => {
      let body = value
        .payload_as_light_shadows_payload()
        .ok_or_else(missing)?;
      CommandBody::LightSetShadows(battlement::LightShadowsPayload {
        object_id: object_id(body.object_id())?,
        shadows: shadow_mode(body.shadows())?,
      })
    }
    Kind::ImageSetTexture => {
      let body = value.payload_as_set_texture_payload().ok_or_else(missing)?;
      CommandBody::ImageSetTexture(battlement::SetTexturePayload {
        object_id: object_id(body.object_id())?,
        address: body.address().into(),
      })
    }
    Kind::TextSetFont => {
      let body = value.payload_as_set_font_payload().ok_or_else(missing)?;
      CommandBody::TextSetFont(battlement::SetFontPayload {
        object_id: object_id(body.object_id())?,
        address: body.address().into(),
      })
    }
    Kind::ImageSetSize => {
      let body = value.payload_as_image_size_payload().ok_or_else(missing)?;
      CommandBody::ImageSetSize(battlement::ImageSizePayload {
        object_id: object_id(body.object_id())?,
        width: body.width(),
        height: body.height(),
      })
    }
    Kind::ImageSetFit => {
      let body = value.payload_as_image_fit_payload().ok_or_else(missing)?;
      CommandBody::ImageSetFit(battlement::ImageFitPayload {
        object_id: object_id(body.object_id())?,
        fit: image_fit(body.fit())?,
      })
    }
    Kind::ImageSetTint => {
      let body = value.payload_as_tint_payload().ok_or_else(missing)?;
      CommandBody::ImageSetTint(property(
        body.on_conflict(),
        battlement::TintPayload {
          object_id: object_id(body.object_id())?,
          tint: rgb(body.tint()),
        },
      )?)
    }
    Kind::ImageTweenTint => {
      let body = value.payload_as_tween_tint_payload().ok_or_else(missing)?;
      CommandBody::ImageTweenTint(property(
        body.on_conflict(),
        battlement::TweenTintPayload {
          object_id: object_id(body.object_id())?,
          tint: rgb(body.tint()),
          tween: tween(body.tween())?,
        },
      )?)
    }
    Kind::ImageSetOpacity => {
      let body = value.payload_as_opacity_payload().ok_or_else(missing)?;
      CommandBody::ImageSetOpacity(property(
        body.on_conflict(),
        battlement::OpacityPayload {
          object_id: object_id(body.object_id())?,
          opacity: body.opacity(),
        },
      )?)
    }
    Kind::ImageTweenOpacity => {
      let body = value
        .payload_as_tween_opacity_payload()
        .ok_or_else(missing)?;
      CommandBody::ImageTweenOpacity(property(
        body.on_conflict(),
        battlement::TweenOpacityPayload {
          object_id: object_id(body.object_id())?,
          opacity: body.opacity(),
          tween: tween(body.tween())?,
        },
      )?)
    }
    Kind::TextSetContent => {
      let body = value
        .payload_as_text_content_payload()
        .ok_or_else(missing)?;
      CommandBody::TextSetContent(battlement::TextContentPayload {
        object_id: object_id(body.object_id())?,
        content: body.content().to_owned(),
      })
    }
    Kind::TextSetSize => {
      let body = value.payload_as_text_size_payload().ok_or_else(missing)?;
      CommandBody::TextSetSize(property(
        body.on_conflict(),
        battlement::TextSizePayload {
          object_id: object_id(body.object_id())?,
          size: body.size(),
        },
      )?)
    }
    Kind::TextTweenSize => {
      let body = value
        .payload_as_tween_text_size_payload()
        .ok_or_else(missing)?;
      CommandBody::TextTweenSize(property(
        body.on_conflict(),
        battlement::TweenTextSizePayload {
          object_id: object_id(body.object_id())?,
          size: body.size(),
          tween: tween(body.tween())?,
        },
      )?)
    }
    Kind::TextSetAlignment => {
      let body = value
        .payload_as_text_alignment_payload()
        .ok_or_else(missing)?;
      CommandBody::TextSetAlignment(battlement::TextAlignmentPayload {
        object_id: object_id(body.object_id())?,
        horizontal: horizontal(body.horizontal())?,
        vertical: vertical(body.vertical())?,
      })
    }
    Kind::TextSetWrapping => {
      let body = value
        .payload_as_text_wrapping_payload()
        .ok_or_else(missing)?;
      CommandBody::TextSetWrapping(battlement::TextWrappingPayload {
        object_id: object_id(body.object_id())?,
        wrap_width: body.wrap_width(),
      })
    }
    Kind::AnimatorPlay => {
      let body = value
        .payload_as_animator_play_payload()
        .ok_or_else(missing)?;
      CommandBody::AnimatorPlay(battlement::AnimatorPlayPayload {
        object_id: object_id(body.object_id())?,
        state: body.state().to_owned(),
        layer: body.layer(),
        normalized_start_time: body.normalized_start_time(),
        wait_ms: body.wait_ms(),
      })
    }
    Kind::AnimatorCrossFade => {
      let body = value
        .payload_as_animator_cross_fade_payload()
        .ok_or_else(missing)?;
      CommandBody::AnimatorCrossFade(battlement::AnimatorCrossFadePayload {
        object_id: object_id(body.object_id())?,
        state: body.state().to_owned(),
        layer: body.layer(),
        normalized_start_time: body.normalized_start_time(),
        wait_ms: body.wait_ms(),
        cross_fade_ms: body.cross_fade_ms(),
      })
    }
    Kind::AnimatorSetBool => {
      let body = value
        .payload_as_animator_bool_payload()
        .ok_or_else(missing)?;
      CommandBody::AnimatorSetBool(battlement::AnimatorBoolPayload {
        object_id: object_id(body.object_id())?,
        parameter: body.parameter().to_owned(),
        value: body.value(),
      })
    }
    Kind::AnimatorSetInt => {
      let body = value
        .payload_as_animator_int_payload()
        .ok_or_else(missing)?;
      CommandBody::AnimatorSetInt(battlement::AnimatorIntPayload {
        object_id: object_id(body.object_id())?,
        parameter: body.parameter().to_owned(),
        value: body.value(),
      })
    }
    Kind::AnimatorSetFloat => {
      let body = value
        .payload_as_animator_float_payload()
        .ok_or_else(missing)?;
      CommandBody::AnimatorSetFloat(battlement::AnimatorFloatPayload {
        object_id: object_id(body.object_id())?,
        parameter: body.parameter().to_owned(),
        value: body.value(),
      })
    }
    Kind::AnimatorSetTrigger => {
      let body = value
        .payload_as_animator_parameter_payload()
        .ok_or_else(missing)?;
      CommandBody::AnimatorSetTrigger(battlement::AnimatorParameterPayload {
        object_id: object_id(body.object_id())?,
        parameter: body.parameter().to_owned(),
      })
    }
    Kind::AnimatorSetSpeed => {
      let body = value
        .payload_as_animator_speed_payload()
        .ok_or_else(missing)?;
      CommandBody::AnimatorSetSpeed(battlement::AnimatorSpeedPayload {
        object_id: object_id(body.object_id())?,
        speed: body.speed(),
      })
    }
    Kind::ParticlePlay => {
      let body = value
        .payload_as_particle_play_payload()
        .ok_or_else(missing)?;
      CommandBody::ParticlePlay(battlement::ParticlePlayPayload {
        object_id: object_id(body.object_id())?,
        restart: body.restart(),
      })
    }
    Kind::ParticleStop => {
      let body = value
        .payload_as_particle_stop_payload()
        .ok_or_else(missing)?;
      CommandBody::ParticleStop(battlement::ParticleStopPayload {
        object_id: object_id(body.object_id())?,
        clear: body.clear(),
      })
    }
    Kind::ParticleSpawn => {
      let body = value
        .payload_as_particle_spawn_payload()
        .ok_or_else(missing)?;
      let location = match body.location_kind() {
        payload::ParticleSpawnLocationKind::GameObject => {
          battlement::ParticleSpawnLocation::GameObject(object_id(
            body.object_id().ok_or_else(missing)?,
          )?)
        }
        payload::ParticleSpawnLocationKind::WorldPosition => {
          battlement::ParticleSpawnLocation::WorldPosition(vector3(
            body.world_position().ok_or_else(missing)?,
          ))
        }
        _ => return Err("unknown particle spawn location".to_owned()),
      };
      CommandBody::ParticleSpawn(battlement::ParticleSpawnPayload {
        address: body.address().into(),
        location,
        lifetime_ms: body.lifetime_ms(),
      })
    }
    Kind::AudioPlay => {
      let body = value.payload_as_audio_play_payload().ok_or_else(missing)?;
      CommandBody::AudioPlay(battlement::AudioPlayPayload {
        address: body.address().into(),
        bus: audio_bus(body.bus())?,
        volume: body.volume(),
        pitch: body.pitch(),
        r#loop: body.loop_(),
        fade_in_ms: body.fade_in_ms(),
      })
    }
    Kind::AudioSetMix => {
      let body = value.payload_as_audio_mix_payload().ok_or_else(missing)?;
      CommandBody::AudioSetMix(battlement::AudioMix {
        master: body.master(),
        music: body.music(),
        effects: body.effects(),
        muted: body.muted(),
      })
    }
    Kind::AudioStop => {
      let body = value.payload_as_audio_stop_payload().ok_or_else(missing)?;
      CommandBody::AudioStop(battlement::AudioStopPayload {
        audio_command_id: command_id(body.audio_command_id())?,
        fade_out_ms: body.fade_out_ms(),
      })
    }
    Kind::AudioPause | Kind::AudioResume => {
      let body = value
        .payload_as_audio_playback_payload()
        .ok_or_else(missing)?;
      let body = battlement::AudioPlaybackPayload {
        audio_command_id: command_id(body.audio_command_id())?,
      };
      if value.kind() == Kind::AudioPause {
        CommandBody::AudioPause(body)
      } else {
        CommandBody::AudioResume(body)
      }
    }
    Kind::AudioSeek => {
      let body = value.payload_as_audio_seek_payload().ok_or_else(missing)?;
      CommandBody::AudioSeek(battlement::AudioSeekPayload {
        audio_command_id: command_id(body.audio_command_id())?,
        position_ms: body.position_ms(),
      })
    }
    Kind::AudioSetBuffering => {
      let body = value
        .payload_as_audio_buffering_payload()
        .ok_or_else(missing)?;
      CommandBody::AudioSetBuffering(battlement::AudioBufferingPayload {
        audio_command_id: command_id(body.audio_command_id())?,
        buffering: body.buffering(),
      })
    }
    Kind::AudioReplace => {
      let body = value
        .payload_as_audio_replace_payload()
        .ok_or_else(missing)?;
      CommandBody::AudioReplace(battlement::AudioReplacePayload {
        audio_command_id: command_id(body.audio_command_id())?,
        address: body.address().into(),
      })
    }
    Kind::AudioSetVolume => {
      let body = value
        .payload_as_audio_volume_payload()
        .ok_or_else(missing)?;
      CommandBody::AudioSetVolume(property(
        body.on_conflict(),
        battlement::AudioVolumePayload {
          audio_command_id: command_id(body.audio_command_id())?,
          volume: body.volume(),
        },
      )?)
    }
    Kind::AudioTweenVolume => {
      let body = value
        .payload_as_tween_audio_volume_payload()
        .ok_or_else(missing)?;
      CommandBody::AudioTweenVolume(property(
        body.on_conflict(),
        battlement::TweenAudioVolumePayload {
          audio_command_id: command_id(body.audio_command_id())?,
          volume: body.volume(),
          tween: tween(body.tween())?,
        },
      )?)
    }
    Kind::TimeWait => {
      let body = value.payload_as_wait_payload().ok_or_else(missing)?;
      CommandBody::TimeWait(battlement::WaitPayload {
        duration_ms: body.duration_ms(),
      })
    }
    Kind::OperationCancel => {
      let body = value
        .payload_as_cancel_operation_payload()
        .ok_or_else(missing)?;
      CommandBody::OperationCancel(battlement::CancelOperationPayload {
        command_id: command_id(body.command_id())?,
      })
    }
    Kind::InputSetEnabled => {
      let body = value
        .payload_as_set_input_enabled_payload()
        .ok_or_else(missing)?;
      CommandBody::InputSetEnabled(battlement::SetInputEnabledPayload {
        enabled: body.enabled(),
      })
    }
    Kind::MotionSetWorldDescriptor => {
      let value = value
        .payload_as_world_motion_payload()
        .ok_or_else(|| "world motion payload is missing".to_owned())?;
      CommandBody::MotionSetWorldDescriptor(battlement::WorldMotionPayload {
        object_id: object_id(value.object_id())?,
        motion: value
          .motion()
          .map(crate::response_motion_descriptor_reader::descriptor)
          .transpose()?
          .map(Box::new),
      })
    }
    Kind::InputSetWorldPointer => {
      let value = value
        .payload_as_world_pointer_payload()
        .ok_or_else(|| "missing world pointer payload".to_owned())?;
      CommandBody::InputSetWorldPointer(battlement::WorldPointerPayload {
        object_id: object_id(value.object_id())?,
        settings: value
          .settings()
          .map(crate::response_reader::read_world_pointer),
      })
    }
    Kind::InputSetPointerEvents => {
      let body = value
        .payload_as_pointer_events_payload()
        .ok_or_else(missing)?;
      CommandBody::InputSetPointerEvents(battlement::PointerEventsPayload {
        object_id: object_id(body.object_id())?,
        events: body
          .events()
          .iter()
          .map(pointer_event)
          .collect::<Result<_, _>>()?,
      })
    }
    Kind::InputSetGlobalKeys => {
      let body = value.payload_as_global_keys_payload().ok_or_else(missing)?;
      CommandBody::InputSetGlobalKeys(battlement::GlobalKeysPayload {
        keys: body
          .keys()
          .iter()
          .map(|key| {
            battlement_flatbuffers::physical_key_from_ordinal(key.0)
              .ok_or_else(|| "unknown physical key".to_owned())
          })
          .collect::<Result<_, _>>()?,
      })
    }
    Kind::InputCapture => {
      let body = value
        .payload_as_input_capture_payload()
        .ok_or_else(missing)?;
      let id = object_id(body.capture_id())?;
      CommandBody::InputCapture(match body.operation() {
        payload::InputCaptureOperation::BeginKeyboard => {
          battlement::InputCaptureCommand::Begin(battlement::InputCaptureRequest {
            id,
            device: battlement::InputCaptureDevice::Keyboard,
          })
        }
        payload::InputCaptureOperation::BeginController => {
          battlement::InputCaptureCommand::Begin(battlement::InputCaptureRequest {
            id,
            device: battlement::InputCaptureDevice::Controller,
          })
        }
        payload::InputCaptureOperation::End => battlement::InputCaptureCommand::End(id),
        _ => return Err("unknown capture operation".to_owned()),
      })
    }
    Kind::InputSetController => {
      let body = value
        .payload_as_controller_input_settings()
        .ok_or_else(missing)?;
      CommandBody::InputSetController(controller_input(body)?)
    }
    Kind::ControllerVibrate => {
      let body = value
        .payload_as_controller_vibration_payload()
        .ok_or_else(missing)?;
      CommandBody::ControllerVibrate(battlement::ControllerVibrationPayload {
        low_frequency: body.low_frequency(),
        high_frequency: body.high_frequency(),
        duration_ms: body.duration_ms(),
      })
    }
    Kind::DebugUi => {
      let body = value.payload_as_debug_ui_payload().ok_or_else(missing)?;
      let surface = match body.surface() {
        payload::DebugUiSurface::LogViewer => battlement::DebugUiSurface::LogViewer,
        payload::DebugUiSurface::FpsViewer => battlement::DebugUiSurface::FpsViewer,
        _ => return Err("unknown debug UI surface".to_owned()),
      };
      CommandBody::DebugUi(battlement::DebugUiPayload {
        surface,
        visible: body.visible(),
      })
    }
    Kind::VisualElementCreate => {
      let body = value
        .payload_as_visual_element_create_payload()
        .ok_or_else(missing)?;
      let root_id = object_id(body.root_id())?;
      CommandBody::VisualElementCreate(Box::new(battlement::VisualElementCreate {
        parent_id: object_id(body.parent_id())?,
        child_index: body.child_index(),
        node: crate::response_ui_reader::read_subtree(root_id, body.nodes())?,
      }))
    }
    Kind::VisualElementUpdate => {
      let body = value
        .payload_as_visual_element_update_payload()
        .ok_or_else(missing)?;
      let target_id = object_id(body.object_id())?;
      let update = match body.kind() {
        ui_wire::VisualElementUpdateKind::Properties => {
          battlement::VisualElementUpdate::Properties {
            object_id: target_id,
            element: Box::new(crate::response_ui_reader::read_element(
              body
                .element()
                .ok_or_else(|| "UI property update element is missing".to_owned())?,
            )?),
          }
        }
        ui_wire::VisualElementUpdateKind::Parent => battlement::VisualElementUpdate::Parent {
          object_id: target_id,
          parent_id: object_id(
            body
              .parent_id()
              .ok_or_else(|| "UI parent update identity is missing".to_owned())?,
          )?,
          child_index: body.child_index(),
        },
        ui_wire::VisualElementUpdateKind::Index => battlement::VisualElementUpdate::Index {
          object_id: target_id,
          child_index: body
            .child_index()
            .ok_or_else(|| "UI index update value is missing".to_owned())?,
        },
        _ => return Err("UI element update kind is unknown".to_owned()),
      };
      CommandBody::VisualElementUpdate(Box::new(update))
    }
    Kind::VisualElementDestroy => {
      let body = value
        .payload_as_visual_element_destroy_payload()
        .ok_or_else(missing)?;
      CommandBody::VisualElementDestroy(battlement::VisualElementDestroy {
        object_id: object_id(body.object_id())?,
      })
    }
    Kind::VisualElementPerformAction => {
      let body = value
        .payload_as_visual_element_action_payload()
        .ok_or_else(missing)?;
      let action = match body.kind() {
        ui_wire::VisualElementActionKind::ParticleStreaks => {
          battlement::VisualElementAction::ParticleStreaks {
            streaks: body
              .streaks()
              .ok_or_else(|| "UI particle streak list is missing".to_owned())?
              .iter()
              .map(|value| battlement::UiParticleStreak {
                origin: [value.origin().x(), value.origin().y()],
                travel: [value.travel().x(), value.travel().y()],
                size: [value.size().x(), value.size().y()],
                rotation: value.rotation(),
                color: color(value.color()),
                lifetime_ms: value.lifetime_ms(),
                delay_ms: value.delay_ms(),
              })
              .collect(),
          }
        }
        ui_wire::VisualElementActionKind::Focus => battlement::VisualElementAction::Focus,
        ui_wire::VisualElementActionKind::Blur => battlement::VisualElementAction::Blur,
        ui_wire::VisualElementActionKind::CapturePointer => {
          battlement::VisualElementAction::CapturePointer {
            pointer_id: body.pointer_id(),
          }
        }
        ui_wire::VisualElementActionKind::ReleasePointer => {
          battlement::VisualElementAction::ReleasePointer {
            pointer_id: body.pointer_id(),
          }
        }
        ui_wire::VisualElementActionKind::ScrollTo => battlement::VisualElementAction::ScrollTo {
          descendant_id: object_id(
            body
              .descendant_id()
              .ok_or_else(|| "UI scroll target is missing".to_owned())?,
          )?,
        },
        ui_wire::VisualElementActionKind::SelectText => {
          battlement::VisualElementAction::SelectText {
            cursor_index: body.cursor_index(),
            selection_index: body.selection_index(),
          }
        }
        _ => return Err("UI element action kind is unknown".to_owned()),
      };
      CommandBody::VisualElementPerformAction(battlement::VisualElementPerformAction {
        object_id: object_id(body.object_id())?,
        action,
      })
    }
    Kind::MotionValue => CommandBody::MotionValue(crate::response_motion_reader::value_operation(
      value
        .payload_as_motion_value_operation()
        .ok_or_else(missing)?,
    )?),
    Kind::MotionValuePlayback => {
      CommandBody::MotionValuePlayback(crate::response_motion_reader::value_playback_operation(
        value
          .payload_as_motion_value_playback_operation()
          .ok_or_else(missing)?,
      )?)
    }
    Kind::MotionPlayback => {
      CommandBody::MotionPlayback(crate::response_motion_reader::playback_operation(
        value
          .payload_as_motion_playback_operation()
          .ok_or_else(missing)?,
      )?)
    }
    Kind::MotionControlledClock => {
      CommandBody::MotionControlledClock(crate::response_motion_reader::controlled_clock_operation(
        value
          .payload_as_motion_controlled_clock_operation()
          .ok_or_else(missing)?,
      )?)
    }
    Kind::MotionControl => {
      CommandBody::MotionControl(crate::response_motion_reader::control_operation(
        value
          .payload_as_motion_control_operation()
          .ok_or_else(missing)?,
      )?)
    }
    Kind::MotionScope => CommandBody::MotionScope(crate::response_motion_reader::scope_operation(
      value
        .payload_as_motion_scope_operation()
        .ok_or_else(missing)?,
    )?),
    Kind::MotionDragControl => {
      CommandBody::MotionDragControl(crate::response_motion_reader::drag_control_operation(
        value
          .payload_as_motion_drag_control_operation()
          .ok_or_else(missing)?,
      )?)
    }
    Kind::GeometryObservationUpdate => {
      let body = value
        .payload_as_geometry_observation_update()
        .ok_or_else(missing)?;
      CommandBody::GeometryObservationUpdate(read_geometry_update(body)?)
    }
    Kind::AccessibilityUpdate => {
      let body = value
        .payload_as_accessibility_update()
        .ok_or_else(missing)?;
      CommandBody::AccessibilityUpdate(read_accessibility_update(body)?)
    }
    _ => {
      return Err(format!(
        "response command kind is unknown: {:?}",
        value.kind()
      ));
    }
  })
}

fn read_accessibility_update(
  value: accessibility::AccessibilityUpdate<'_>,
) -> Result<battlement::AccessibilityUpdate, String> {
  Ok(battlement::AccessibilityUpdate {
    snapshot: value
      .snapshot()
      .map(read_accessibility_snapshot)
      .transpose()?,
    announcements: value.announcements().iter().map(str::to_owned).collect(),
  })
}

fn read_accessibility_snapshot(
  value: accessibility::AccessibilitySnapshot<'_>,
) -> Result<battlement::AccessibilitySnapshot, String> {
  Ok(battlement::AccessibilitySnapshot {
    commit_sequence: value.commit_sequence(),
    roots: value
      .roots()
      .iter()
      .map(object_id)
      .collect::<Result<_, _>>()?,
    nodes: value
      .nodes()
      .iter()
      .map(read_accessibility_node)
      .collect::<Result<_, _>>()?,
  })
}

fn read_accessibility_node(
  value: accessibility::AccessibilityNodeSnapshot<'_>,
) -> Result<battlement::AccessibilityNodeSnapshot, String> {
  let state = value.state();
  let actions = value.actions();
  Ok(battlement::AccessibilityNodeSnapshot {
    object_id: object_id(value.object_id())?,
    parent_id: value.parent_id().map(object_id).transpose()?,
    children: value
      .children()
      .iter()
      .map(object_id)
      .collect::<Result<_, _>>()?,
    role: semantic_role(value.role())?,
    label: value.label().map(str::to_owned),
    hint: value.hint().map(str::to_owned),
    state: battlement::SemanticState {
      disabled: state.disabled(),
      checked: state.checked().map(checked_state).transpose()?,
      selected: state.selected(),
      expanded: state.expanded(),
      popup: state
        .popup()
        .map(|value| match value {
          accessibility::PopupKind::ListBox => Ok(battlement::PopupKind::ListBox),
          _ => Err("unknown accessibility popup kind".to_owned()),
        })
        .transpose()?,
      busy: state.busy(),
      current: state
        .current()
        .map(|value| match value {
          accessibility::CurrentPage::Page => Ok(battlement::CurrentPage::Page),
          _ => Err("unknown accessibility current-page kind".to_owned()),
        })
        .transpose()?,
    },
    value: value
      .value()
      .map(|range| battlement::AccessibilityRangeValue {
        current: range.current(),
        minimum: range.minimum(),
        maximum: range.maximum(),
        text: range.text().map(str::to_owned),
      }),
    actions: battlement::AccessibilityActionSet {
      activate: actions.activate(),
      increment: actions.increment(),
      decrement: actions.decrement(),
      dismiss: actions.dismiss(),
      scroll: actions
        .scroll()
        .iter()
        .map(accessibility_scroll_direction)
        .collect::<Result<_, _>>()?,
    },
    heading_level: value.heading_level(),
    scroll_axis: value
      .scroll_axis()
      .map(|axis| match axis {
        accessibility::AccessibilityScrollAxis::Horizontal => {
          Ok(battlement::AccessibilityScrollAxis::Horizontal)
        }
        accessibility::AccessibilityScrollAxis::Vertical => {
          Ok(battlement::AccessibilityScrollAxis::Vertical)
        }
        _ => Err("unknown accessibility scroll axis".to_owned()),
      })
      .transpose()?,
  })
}

fn semantic_role(value: accessibility::SemanticRole) -> Result<battlement::SemanticRole, String> {
  use accessibility::SemanticRole as W;
  use battlement::SemanticRole as R;
  Ok(match value {
    W::Button => R::Button,
    W::Checkbox => R::Checkbox,
    W::Switch => R::Switch,
    W::Radio => R::Radio,
    W::RadioGroup => R::RadioGroup,
    W::Slider => R::Slider,
    W::Progress => R::Progress,
    W::Disclosure => R::Disclosure,
    W::ScrollArea => R::ScrollArea,
    W::Tab => R::Tab,
    W::TabList => R::TabList,
    W::TabPanel => R::TabPanel,
    W::Dialog => R::Dialog,
    W::Heading => R::Heading,
    W::Image => R::Image,
    W::StaticText => R::StaticText,
    W::Group => R::Group,
    W::ListBox => R::ListBox,
    W::Option => R::Option,
    W::Table => R::Table,
    W::Row => R::Row,
    W::ColumnHeader => R::ColumnHeader,
    W::RowHeader => R::RowHeader,
    W::Cell => R::Cell,
    W::Link => R::Link,
    W::Navigation => R::Navigation,
    W::Region => R::Region,
    _ => return Err("unknown accessibility semantic role".to_owned()),
  })
}

fn checked_state(value: accessibility::CheckedState) -> Result<battlement::CheckedState, String> {
  match value {
    accessibility::CheckedState::False => Ok(battlement::CheckedState::False),
    accessibility::CheckedState::True => Ok(battlement::CheckedState::True),
    accessibility::CheckedState::Mixed => Ok(battlement::CheckedState::Mixed),
    _ => Err("unknown accessibility checked state".to_owned()),
  }
}

fn accessibility_scroll_direction(
  value: accessibility::AccessibilityScrollDirection,
) -> Result<battlement::AccessibilityScrollDirection, String> {
  match value {
    accessibility::AccessibilityScrollDirection::Forward => {
      Ok(battlement::AccessibilityScrollDirection::Forward)
    }
    accessibility::AccessibilityScrollDirection::Backward => {
      Ok(battlement::AccessibilityScrollDirection::Backward)
    }
    _ => Err("unknown accessibility scroll direction".to_owned()),
  }
}

fn property<T>(policy: payload::ConflictPolicy, payload: T) -> Result<PropertyCommand<T>, String> {
  Ok(PropertyCommand {
    on_conflict: conflict(policy)?,
    payload,
  })
}

fn conflict(value: payload::ConflictPolicy) -> Result<ConflictPolicy, String> {
  match value {
    payload::ConflictPolicy::Cancel => Ok(ConflictPolicy::Cancel),
    payload::ConflictPolicy::Wait => Ok(ConflictPolicy::Wait),
    _ => Err("unknown conflict policy".to_owned()),
  }
}

fn tween(value: &payload::Tween) -> Result<battlement::Tween, String> {
  const EASINGS: [battlement::Easing; 31] = [
    battlement::Easing::Linear,
    battlement::Easing::InSine,
    battlement::Easing::OutSine,
    battlement::Easing::InOutSine,
    battlement::Easing::InQuad,
    battlement::Easing::OutQuad,
    battlement::Easing::InOutQuad,
    battlement::Easing::InCubic,
    battlement::Easing::OutCubic,
    battlement::Easing::InOutCubic,
    battlement::Easing::InQuart,
    battlement::Easing::OutQuart,
    battlement::Easing::InOutQuart,
    battlement::Easing::InQuint,
    battlement::Easing::OutQuint,
    battlement::Easing::InOutQuint,
    battlement::Easing::InExpo,
    battlement::Easing::OutExpo,
    battlement::Easing::InOutExpo,
    battlement::Easing::InCirc,
    battlement::Easing::OutCirc,
    battlement::Easing::InOutCirc,
    battlement::Easing::InBack,
    battlement::Easing::OutBack,
    battlement::Easing::InOutBack,
    battlement::Easing::InElastic,
    battlement::Easing::OutElastic,
    battlement::Easing::InOutElastic,
    battlement::Easing::InBounce,
    battlement::Easing::OutBounce,
    battlement::Easing::InOutBounce,
  ];
  let easing = *EASINGS
    .get(usize::from(value.easing().0))
    .ok_or_else(|| "unknown easing".to_owned())?;
  let mode = match value.repeat_mode() {
    payload::RepeatMode::Restart => battlement::RepeatMode::Restart,
    payload::RepeatMode::PingPong => battlement::RepeatMode::PingPong,
    _ => return Err("unknown repeat mode".to_owned()),
  };
  let repeat = match value.repeat_kind() {
    payload::TweenRepeatKind::Once => battlement::TweenRepeat::Once,
    payload::TweenRepeatKind::Count => battlement::TweenRepeat::Count {
      additional_traversals: value.repeat_count(),
      mode,
    },
    payload::TweenRepeatKind::Forever => battlement::TweenRepeat::Forever(mode),
    _ => return Err("unknown tween repeat kind".to_owned()),
  };
  Ok(battlement::Tween {
    delay_ms: value.delay_ms(),
    duration_ms: value.duration_ms(),
    easing,
    repeat,
  })
}

fn command_id(value: &common::Uuid) -> Result<battlement::CommandId, String> {
  battlement::CommandId::from_uuid(uuid(value)?).map_err(|error| error.to_string())
}

fn scene_id(value: &common::Uuid) -> Result<battlement::SceneId, String> {
  battlement::SceneId::from_uuid(uuid(value)?).map_err(|error| error.to_string())
}

fn object_id(value: &common::Uuid) -> Result<battlement::ObjectId, String> {
  battlement::ObjectId::from_uuid(uuid(value)?).map_err(|error| error.to_string())
}

fn uuid(value: &common::Uuid) -> Result<uuid::Uuid, String> {
  let value = uuid::Uuid::from_bytes(std::array::from_fn(|index| value.bytes().get(index)));
  (!value.is_nil())
    .then_some(value)
    .ok_or_else(|| "protocol UUID is zero".to_owned())
}

fn vector3(value: &common::Vector3d) -> battlement::Vector3 {
  battlement::Vector3::new(value.x(), value.y(), value.z())
}

fn quaternion(value: &common::Quaterniond) -> battlement::Quaternion {
  battlement::Quaternion::new(value.x(), value.y(), value.z(), value.w())
}

fn color(value: &common::RgbaColor) -> battlement::Color {
  battlement::Color {
    r: value.r(),
    g: value.g(),
    b: value.b(),
    a: value.a(),
  }
}

fn rgb(value: &common::RgbColor) -> battlement::RgbColor {
  battlement::RgbColor::rgb(value.r(), value.g(), value.b())
}

fn camera_clear(
  value: battlement_flatbuffers::schema_generated::world_generated::CameraClearMode,
) -> Result<battlement::CameraClearMode, String> {
  use battlement_flatbuffers::schema_generated::world_generated::CameraClearMode as W;
  match value {
    W::Skybox => Ok(battlement::CameraClearMode::Skybox),
    W::SolidColor => Ok(battlement::CameraClearMode::SolidColor),
    W::Depth => Ok(battlement::CameraClearMode::Depth),
    W::Nothing => Ok(battlement::CameraClearMode::Nothing),
    _ => Err("unknown camera clear mode".to_owned()),
  }
}

fn light_type(
  value: battlement_flatbuffers::schema_generated::world_generated::LightType,
) -> Result<battlement::LightType, String> {
  use battlement_flatbuffers::schema_generated::world_generated::LightType as W;
  match value {
    W::Directional => Ok(battlement::LightType::Directional),
    W::Point => Ok(battlement::LightType::Point),
    W::Spot => Ok(battlement::LightType::Spot),
    _ => Err("unknown light type".to_owned()),
  }
}

fn shadow_mode(
  value: battlement_flatbuffers::schema_generated::world_generated::ShadowMode,
) -> Result<battlement::ShadowMode, String> {
  use battlement_flatbuffers::schema_generated::world_generated::ShadowMode as W;
  match value {
    W::None => Ok(battlement::ShadowMode::None),
    W::Hard => Ok(battlement::ShadowMode::Hard),
    W::Soft => Ok(battlement::ShadowMode::Soft),
    _ => Err("unknown shadow mode".to_owned()),
  }
}

fn image_fit(
  value: battlement_flatbuffers::schema_generated::world_generated::ImageFit,
) -> Result<battlement::ImageFit, String> {
  use battlement_flatbuffers::schema_generated::world_generated::ImageFit as W;
  match value {
    W::Stretch => Ok(battlement::ImageFit::Stretch),
    W::Contain => Ok(battlement::ImageFit::Contain),
    W::Cover => Ok(battlement::ImageFit::Cover),
    _ => Err("unknown image fit".to_owned()),
  }
}

fn horizontal(
  value: battlement_flatbuffers::schema_generated::world_generated::HorizontalAlignment,
) -> Result<battlement::HorizontalAlignment, String> {
  use battlement_flatbuffers::schema_generated::world_generated::HorizontalAlignment as W;
  match value {
    W::Left => Ok(battlement::HorizontalAlignment::Left),
    W::Center => Ok(battlement::HorizontalAlignment::Center),
    W::Right => Ok(battlement::HorizontalAlignment::Right),
    W::Justified => Ok(battlement::HorizontalAlignment::Justified),
    _ => Err("unknown horizontal alignment".to_owned()),
  }
}

fn vertical(
  value: battlement_flatbuffers::schema_generated::world_generated::VerticalAlignment,
) -> Result<battlement::VerticalAlignment, String> {
  use battlement_flatbuffers::schema_generated::world_generated::VerticalAlignment as W;
  match value {
    W::Top => Ok(battlement::VerticalAlignment::Top),
    W::Middle => Ok(battlement::VerticalAlignment::Middle),
    W::Bottom => Ok(battlement::VerticalAlignment::Bottom),
    _ => Err("unknown vertical alignment".to_owned()),
  }
}

fn pointer_event(
  value: battlement_flatbuffers::schema_generated::world_generated::PointerEventKind,
) -> Result<battlement::PointerEvent, String> {
  use battlement_flatbuffers::schema_generated::world_generated::PointerEventKind as W;
  match value {
    W::Enter => Ok(battlement::PointerEvent::Enter),
    W::Exit => Ok(battlement::PointerEvent::Exit),
    W::Down => Ok(battlement::PointerEvent::Down),
    W::Up => Ok(battlement::PointerEvent::Up),
    W::Click => Ok(battlement::PointerEvent::Click),
    _ => Err("unknown pointer event".to_owned()),
  }
}

fn controller_input(
  value: payload::ControllerInputSettings<'_>,
) -> Result<battlement::ControllerInputSettings, String> {
  use battlement_flatbuffers::schema_generated::client_message_generated::ControllerButton as W;
  let buttons = value
    .buttons()
    .iter()
    .map(|button| match button {
      W::South => Ok(battlement::ControllerButton::South),
      W::East => Ok(battlement::ControllerButton::East),
      W::West => Ok(battlement::ControllerButton::West),
      W::North => Ok(battlement::ControllerButton::North),
      W::LeftShoulder => Ok(battlement::ControllerButton::LeftShoulder),
      W::RightShoulder => Ok(battlement::ControllerButton::RightShoulder),
      W::LeftStickButton => Ok(battlement::ControllerButton::LeftStickButton),
      W::RightStickButton => Ok(battlement::ControllerButton::RightStickButton),
      W::Select => Ok(battlement::ControllerButton::Select),
      W::Start => Ok(battlement::ControllerButton::Start),
      _ => Err("unknown controller button".to_owned()),
    })
    .collect::<Result<_, _>>()?;
  Ok(battlement::ControllerInputSettings {
    buttons,
    navigation_enabled: value.navigation_enabled(),
    stick_dead_zone: value.stick_dead_zone(),
    repeat_delay_ms: value.repeat_delay_ms(),
    repeat_interval_ms: value.repeat_interval_ms(),
  })
}

fn read_geometry_update(
  value: geometry::GeometryObservationUpdate<'_>,
) -> Result<battlement::GeometryObservationUpdate, String> {
  let added = value
    .added()
    .iter()
    .map(|value| {
      let target = value.target();
      let camera = || match target.camera_kind() {
        geometry::CameraTargetKind::Input => Ok(battlement::CameraTarget::Input),
        geometry::CameraTargetKind::Object => Ok(battlement::CameraTarget::Object(object_id(
          target
            .camera_object_id()
            .ok_or_else(|| "geometry camera object is missing".to_owned())?,
        )?)),
        _ => Err("unknown geometry camera target".to_owned()),
      };
      let target = match target.kind() {
        geometry::GeometryTargetKind::UiElement => {
          battlement::GeometryObservationTarget::UiElement {
            object_id: object_id(
              target
                .object_id()
                .ok_or_else(|| "geometry UI object is missing".to_owned())?,
            )?,
          }
        }
        geometry::GeometryTargetKind::Viewport => battlement::GeometryObservationTarget::Viewport {
          display_id: battlement::DisplayId(target.display_id()),
        },
        geometry::GeometryTargetKind::WorldOrigin => {
          battlement::GeometryObservationTarget::WorldOrigin {
            object_id: object_id(
              target
                .object_id()
                .ok_or_else(|| "geometry world object is missing".to_owned())?,
            )?,
            camera: camera()?,
          }
        }
        geometry::GeometryTargetKind::WorldAnchor => {
          battlement::GeometryObservationTarget::WorldAnchor {
            object_id: object_id(
              target
                .object_id()
                .ok_or_else(|| "geometry anchor object is missing".to_owned())?,
            )?,
            anchor: target
              .anchor()
              .ok_or_else(|| "geometry anchor is missing".to_owned())?
              .into(),
            camera: camera()?,
          }
        }
        geometry::GeometryTargetKind::WorldRenderedBounds => {
          battlement::GeometryObservationTarget::WorldRenderedBounds {
            object_id: object_id(
              target
                .object_id()
                .ok_or_else(|| "geometry bounds object is missing".to_owned())?,
            )?,
            camera: camera()?,
          }
        }
        geometry::GeometryTargetKind::WorldRestBounds => {
          battlement::GeometryObservationTarget::WorldRestBounds {
            object_id: object_id(
              target
                .object_id()
                .ok_or_else(|| "geometry rest-bounds object is missing".to_owned())?,
            )?,
            request_id: object_id(
              target
                .request_id()
                .ok_or_else(|| "geometry rest-bounds request is missing".to_owned())?,
            )?,
          }
        }
        geometry::GeometryTargetKind::PresentationWork => {
          battlement::GeometryObservationTarget::PresentationWork
        }
        _ => return Err("unknown geometry observation target".to_owned()),
      };
      Ok(battlement::GeometryObservation {
        observation_id: battlement::GeometryObservationId(object_id(value.observation_id())?),
        target,
      })
    })
    .collect::<Result<_, String>>()?;
  let removed = value
    .removed()
    .iter()
    .map(|value| Ok(battlement::GeometryObservationId(object_id(value)?)))
    .collect::<Result<_, String>>()?;
  Ok(battlement::GeometryObservationUpdate { added, removed })
}

pub(crate) fn audio_bus(value: common::AudioBus) -> Result<battlement::AudioBus, String> {
  match value {
    common::AudioBus::Music => Ok(battlement::AudioBus::Music),
    common::AudioBus::Effects => Ok(battlement::AudioBus::Effects),
    _ => Err("audio bus is unknown".to_owned()),
  }
}

use std::{env, error::Error, fs, path::Path};

use masonry::*;
use schemars::{JsonSchema, generate::SchemaSettings};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let output_directory = arguments
        .next()
        .ok_or("usage: masonry-schema-export <output-directory>")?;
    if arguments.next().is_some() {
        return Err("expected exactly one output directory".into());
    }

    let output_directory = Path::new(&output_directory);
    fs::create_dir_all(output_directory)?;

    macro_rules! export {
        ($filename:literal, $type:ty) => {
            write_schema::<$type>(output_directory, $filename)?;
        };
    }

    export!("connect.schema.json", Connect);
    export!("response.schema.json", Response);
    export!("client-message.schema.json", ClientMessage);
    export!("snapshot.schema.json", Snapshot);
    export!("batch.schema.json", Batch);
    export!("command.schema.json", Command);
    export!("quicktype-bundle.schema.json", QuicktypeBundle);

    Ok(())
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct QuicktypeBundle {
    connect: Connect,
    client_message: ClientMessage,
    snapshot: Snapshot,
    animator_bool_payload: AnimatorBoolPayload,
    animator_cross_fade_payload: AnimatorCrossFadePayload,
    animator_float_payload: AnimatorFloatPayload,
    animator_int_payload: AnimatorIntPayload,
    animator_parameter_payload: AnimatorParameterPayload,
    animator_play_payload: AnimatorPlayPayload,
    animator_speed_payload: AnimatorSpeedPayload,
    audio_play_payload: AudioPlayPayload,
    audio_stop_payload: AudioStopPayload,
    audio_volume_payload: AudioVolumePayload,
    camera_clear_payload: CameraClearPayload,
    camera_clipping_payload: CameraClippingPayload,
    cancel_operation_payload: CancelOperationPayload,
    color_payload: ColorPayload,
    global_keys_payload: GlobalKeysPayload,
    image_fit_payload: ImageFitPayload,
    image_size_payload: ImageSizePayload,
    intensity_payload: IntensityPayload,
    light_range_payload: LightRangePayload,
    light_shadows_payload: LightShadowsPayload,
    light_type_payload: LightTypePayload,
    object_create_payload: ObjectCreatePayload,
    object_enabled_payload: ObjectEnabledPayload,
    object_id_payload: ObjectIdPayload,
    object_reparent_payload: ObjectReparentPayload,
    object_set_active_payload: ObjectSetActivePayload,
    opacity_payload: OpacityPayload,
    orthographic_payload: OrthographicPayload,
    particle_play_payload: ParticlePlayPayload,
    particle_spawn_payload: ParticleSpawnPayload,
    particle_stop_payload: ParticleStopPayload,
    perspective_payload: PerspectivePayload,
    pointer_events_payload: PointerEventsPayload,
    position_payload: PositionPayload,
    replace_asset_set_payload: ReplaceAssetSetPayload,
    rotation_payload: RotationPayload,
    scale_payload: ScalePayload,
    scene_id_payload: SceneIdPayload,
    scene_load_payload: SceneLoadPayload,
    set_font_payload: SetFontPayload,
    set_input_enabled_payload: SetInputEnabledPayload,
    set_material_payload: SetMaterialPayload,
    set_texture_payload: SetTexturePayload,
    spot_angle_payload: SpotAnglePayload,
    text_alignment_payload: TextAlignmentPayload,
    text_content_payload: TextContentPayload,
    text_size_payload: TextSizePayload,
    text_wrapping_payload: TextWrappingPayload,
    tint_payload: TintPayload,
    tween_audio_volume_payload: TweenAudioVolumePayload,
    tween_color_payload: TweenColorPayload,
    tween_field_of_view_payload: TweenFieldOfViewPayload,
    tween_intensity_payload: TweenIntensityPayload,
    tween_opacity_payload: TweenOpacityPayload,
    tween_orthographic_size_payload: TweenOrthographicSizePayload,
    tween_position_payload: TweenPositionPayload,
    tween_rotation_payload: TweenRotationPayload,
    tween_scale_payload: TweenScalePayload,
    tween_text_size_payload: TweenTextSizePayload,
    tween_tint_payload: TweenTintPayload,
    wait_payload: WaitPayload,
}

fn write_schema<T: JsonSchema>(
    output_directory: &Path,
    filename: &str,
) -> Result<(), Box<dyn Error>> {
    let schema = SchemaSettings::draft07()
        .into_generator()
        .into_root_schema_for::<T>();
    let mut json = serde_json::to_string_pretty(&schema)?;
    json.push('\n');
    fs::write(output_directory.join(filename), json)?;
    Ok(())
}

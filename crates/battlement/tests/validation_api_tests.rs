use battlement::{
  AnimatorSpeedPayload, AudioClipAddress, AudioPlayPayload, CameraClearMode, CameraClearPayload,
  CameraClippingPayload, CameraState, Color, Command, CommandBody, GameObject, GameObjectKind,
  MaterialAddress, MaterialAssignment, ParentScene, ParticlePlayPayload, PreparedAsset,
  PropertyCommand, Quaternion, RepeatMode, RotationPayload, Scene, SceneAddress, Snapshot,
  SpotAnglePayload, TextMeshProFontAddress, Tween, TweenRepeat, TweenScalePayload, Validate,
  ValidationError, Vector3, WaitPayload,
};

const SESSION_ID: &str = "94fa422b-301d-442d-b9a7-10ea54318e78";
const SCENE_ID: &str = "ca64d87d-33d9-4a19-be6e-597035312d01";
const SECOND_SCENE_ID: &str = "a12e5b12-0fa7-4afb-bc2b-dcb9db398e48";
const CAMERA_ID: &str = "cc847d6e-1468-42c6-9bec-9af5b5aa5c03";
const OBJECT_ID: &str = "e6d89383-f87e-465d-a755-8c1d67bf3874";
const COMMAND_ID: &str = "565e76aa-b480-43c2-900b-1cb9d90e4602";

#[test]
fn valid_snapshot_defaults_pass_cross_field_validation() {
  assert_eq!(base_snapshot().validate(), Ok(()));
}

#[test]
fn snapshot_validation_rejects_representative_cross_field_failures() {
  let mut missing_primary = base_snapshot();
  missing_primary
    .prepared_assets
    .push(PreparedAsset::Scene(SceneAddress::new("scene/second")));
  missing_primary
    .scenes
    .push(Scene::new(SECOND_SCENE_ID.parse().unwrap(), "scene/second"));

  let mut duplicate_asset = base_snapshot();
  duplicate_asset
    .prepared_assets
    .push(PreparedAsset::TextMeshProFont(TextMeshProFontAddress::new(
      "scene/main",
    )));

  let mut missing_parent = base_snapshot();
  missing_parent.objects[0].parent_id = Some(OBJECT_ID.parse().unwrap());

  let mut zero_quaternion = base_snapshot();
  zero_quaternion.objects[0].local_transform.rotation = Quaternion {
    x: 0.0,
    y: 0.0,
    z: 0.0,
    w: 0.0,
  };

  let mut invalid_clipping = base_snapshot();
  let GameObjectKind::Camera { camera } = &mut invalid_clipping.objects[0].kind else {
    panic!("fixture camera missing");
  };
  camera.near = 10.0;
  camera.far = 1.0;

  for (snapshot, expected) in [
    (missing_primary, ValidationError::InvalidPrimaryScene),
    (duplicate_asset, ValidationError::DuplicatePreparedAddress),
    (missing_parent, ValidationError::InvalidReference),
    (zero_quaternion, ValidationError::ZeroQuaternion),
    (invalid_clipping, ValidationError::InvalidClipping),
  ] {
    assert_eq!(snapshot.validate(), Err(expected));
  }
}

#[test]
fn snapshot_validation_rejects_non_finite_numbers() {
  let mut snapshot = base_snapshot();
  let GameObjectKind::Camera { camera } = &mut snapshot.objects[0].kind else {
    panic!("fixture camera missing");
  };
  camera.near = f64::NAN;

  assert_eq!(snapshot.validate(), Err(ValidationError::NonFiniteNumber));
}

#[test]
fn snapshot_validation_rejects_duplicate_object_ids() {
  let mut snapshot = base_snapshot();
  snapshot.objects.push(snapshot.objects[0].clone());

  assert_eq!(snapshot.validate(), Err(ValidationError::DuplicateObject));
}

#[test]
fn snapshot_validation_rejects_duplicate_scene_ids() {
  let mut snapshot = base_snapshot();
  let scene_id = snapshot.scenes[0].scene_id;
  snapshot
    .prepared_assets
    .push(PreparedAsset::Scene(SceneAddress::new("scene/second")));
  snapshot.scenes.push(Scene::new(scene_id, "scene/second"));
  snapshot.primary_scene_id = Some(scene_id);

  assert_eq!(snapshot.validate(), Err(ValidationError::DuplicateScene));
}

#[test]
fn snapshot_validation_covers_scene_selection_and_active_camera_rules() {
  let mut duplicate_address = base_snapshot();
  duplicate_address
    .scenes
    .push(Scene::new(SECOND_SCENE_ID.parse().unwrap(), "scene/main"));
  duplicate_address.primary_scene_id = Some(SCENE_ID.parse().unwrap());

  let mut unknown_primary = base_snapshot();
  unknown_primary.primary_scene_id = Some(SECOND_SCENE_ID.parse().unwrap());

  let mut inactive_camera = base_snapshot();
  inactive_camera.objects[0].active = false;

  for (snapshot, expected) in [
    (duplicate_address, ValidationError::DuplicateScene),
    (unknown_primary, ValidationError::InvalidPrimaryScene),
    (inactive_camera, ValidationError::InvalidReference),
  ] {
    assert_eq!(snapshot.validate(), Err(expected));
  }
}

#[test]
fn snapshot_validation_rejects_cross_placement_parents_and_duplicate_material_slots() {
  let mut cross_placement = base_snapshot();
  let mut child = GameObject::new(OBJECT_ID.parse().unwrap(), GameObjectKind::Empty);
  child.parent_scene = ParentScene::Persistent;
  child.parent_id = Some(CAMERA_ID.parse().unwrap());
  cross_placement.objects.push(child);

  let mut duplicate_slots = base_snapshot();
  let material = MaterialAddress::new("material/main");
  duplicate_slots
    .prepared_assets
    .push(PreparedAsset::Material(material.clone()));
  duplicate_slots.objects.push(GameObject::new(
    OBJECT_ID.parse().unwrap(),
    GameObjectKind::Cube {
      materials: vec![
        MaterialAssignment::new(0, material.clone()),
        MaterialAssignment::new(0, material),
      ],
    },
  ));

  assert_eq!(
    cross_placement.validate(),
    Err(ValidationError::InvalidHierarchy)
  );
  assert_eq!(
    duplicate_slots.validate(),
    Err(ValidationError::InvalidReference)
  );
}

#[test]
fn command_validation_rejects_cross_field_and_blocking_failures() {
  let command_id = COMMAND_ID.parse().unwrap();
  let object_id = OBJECT_ID.parse().unwrap();
  let camera_id = CAMERA_ID.parse().unwrap();
  let invalid_clipping = Command::new(
    command_id,
    CommandBody::CameraSetClipping(CameraClippingPayload {
      object_id: camera_id,
      near: 4.0,
      far: 2.0,
    }),
  );
  let invalid_repeat = Command::new(
    command_id,
    CommandBody::TransformTweenLocalScale(PropertyCommand::canceling(TweenScalePayload {
      object_id,
      scale: Vector3::new(2.0, 2.0, 2.0),
      tween: Tween {
        repeat: TweenRepeat::Count {
          additional_traversals: 1,
          mode: RepeatMode::Restart,
        },
        ..Tween::default()
      },
    })),
  );
  let blocking_loop = Command::new(
    command_id,
    CommandBody::AudioPlay(AudioPlayPayload {
      address: AudioClipAddress::new("audio/loop"),
      volume: 1.0,
      pitch: 1.0,
      r#loop: true,
      fade_in_ms: 0,
    }),
  );
  let nonblocking_wait = Command::new(
    command_id,
    CommandBody::TimeWait(WaitPayload { duration_ms: 10 }),
  )
  .nonblocking();

  for (command, expected) in [
    (invalid_clipping, ValidationError::InvalidClipping),
    (invalid_repeat, ValidationError::InvalidRepeat),
    (blocking_loop, ValidationError::InvalidBlocking),
    (nonblocking_wait, ValidationError::InvalidBlocking),
  ] {
    assert_eq!(command.validate(), Err(expected));
  }
}

#[test]
fn command_validation_covers_clear_spot_rotation_and_particle_rules() {
  let command_id = COMMAND_ID.parse().unwrap();
  let object_id = OBJECT_ID.parse().unwrap();
  let invalid_clear = Command::new(
    command_id,
    CommandBody::CameraSetClear(CameraClearPayload {
      object_id,
      clear_mode: CameraClearMode::SolidColor,
      clear_color: None,
    }),
  );
  let invalid_spot = Command::new(
    command_id,
    CommandBody::LightSetSpotAngle(SpotAnglePayload {
      object_id,
      outer_spot_angle: 20.0,
      inner_spot_angle: 21.0,
    }),
  );
  let zero_rotation = Command::new(
    command_id,
    CommandBody::TransformSetLocalRotation(PropertyCommand::canceling(RotationPayload {
      object_id,
      rotation: Quaternion {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 0.0,
      },
    })),
  );
  let blocking_particle = Command::new(
    command_id,
    CommandBody::ParticlePlay(ParticlePlayPayload {
      object_id,
      restart: false,
    }),
  );
  let non_finite = Command::new(
    command_id,
    CommandBody::AnimatorSetSpeed(AnimatorSpeedPayload {
      object_id,
      speed: f64::NAN,
    }),
  );

  for (command, expected) in [
    (invalid_clear, ValidationError::InvalidClearColor),
    (invalid_spot, ValidationError::InvalidSpotAngles),
    (zero_rotation, ValidationError::ZeroQuaternion),
    (blocking_particle, ValidationError::InvalidBlocking),
    (non_finite, ValidationError::NonFiniteNumber),
  ] {
    assert_eq!(command.validate(), Err(expected));
  }

  let valid_solid_clear = Command::new(
    command_id,
    CommandBody::CameraSetClear(CameraClearPayload {
      object_id,
      clear_mode: CameraClearMode::SolidColor,
      clear_color: Some(Color::BLACK),
    }),
  );
  assert_eq!(valid_solid_clear.validate(), Ok(()));
}

fn base_snapshot() -> Snapshot {
  let session_id = SESSION_ID.parse().unwrap();
  let scene_id = SCENE_ID.parse().unwrap();
  let camera_id = CAMERA_ID.parse().unwrap();
  Snapshot::new(
    session_id,
    vec![PreparedAsset::Scene(SceneAddress::new("scene/main"))],
    vec![Scene::new(scene_id, "scene/main")],
    vec![GameObject::new(
      camera_id,
      GameObjectKind::Camera {
        camera: CameraState::default(),
      },
    )],
    camera_id,
  )
}

#[test]
fn main_camera_snapshot_does_not_require_a_camera_object() {
  let snapshot = Snapshot::new_with_main_camera(
    SESSION_ID.parse().unwrap(),
    vec![PreparedAsset::Scene(SceneAddress::new("scene/main"))],
    vec![Scene::new(SCENE_ID.parse().unwrap(), "scene/main")],
    Vec::new(),
  );

  assert_eq!(snapshot.validate(), Ok(()));
}

#[test]
fn controller_settings_and_vibration_enforce_bounds() {
  let mut snapshot = base_snapshot();
  snapshot.controller_input = Some(battlement::ControllerInputSettings {
    stick_dead_zone: Some(1.0),
    ..battlement::ControllerInputSettings::new()
  });
  assert_eq!(
    snapshot.validate(),
    Err(ValidationError::InvalidControllerInput)
  );

  snapshot.controller_input = Some(battlement::ControllerInputSettings {
    repeat_delay_ms: Some(0),
    ..battlement::ControllerInputSettings::new()
  });
  assert_eq!(
    snapshot.validate(),
    Err(ValidationError::InvalidControllerInput)
  );

  let command = Command::new_v4(CommandBody::ControllerVibrate(
    battlement::ControllerVibrationPayload {
      low_frequency: 1.1,
      high_frequency: 0.2,
      duration_ms: 50,
    },
  ));
  assert_eq!(
    command.validate(),
    Err(ValidationError::InvalidControllerInput)
  );
}

use battlement_ditto::wire::job::{
  Capability, Command, Job, KeyAction, Motion, ObjectState, Orientation, Platform, StepKind,
  VideoStep,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

const SHARED_JOB_FIXTURE: &str =
  include_str!("../../../Packages/com.battlement.client/Tests/Fixtures/Ditto/job-contract.json");

#[test]
fn shared_csharp_job_fixture_has_matching_acceptance() {
  let fixture: Value = serde_json::from_str(SHARED_JOB_FIXTURE).unwrap();
  let valid = fixture["valid"].clone();
  let job: Job = serde_json::from_value(valid.clone()).unwrap();
  job.validate().unwrap();

  for case in fixture["invalid"].as_array().unwrap() {
    let pointer = case["pointer"].as_str().unwrap();
    let mut changed = valid.clone();
    if case["remove"].as_bool() == Some(true) {
      let (parent, field) = pointer.rsplit_once('/').unwrap();
      changed
        .pointer_mut(parent)
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove(field);
    } else {
      changed = with(&changed, pointer, case["value"].clone());
    }
    if let Ok(job) = serde_json::from_value::<Job>(changed) {
      assert!(
        job.validate().is_err(),
        "{} unexpectedly validated",
        case["name"].as_str().unwrap()
      );
    }
  }
}

#[test]
fn complete_job_round_trips_every_step_and_union_variant() {
  let job = job();
  job.validate().unwrap();
  let encoded = serde_json::to_string(&job).unwrap();
  let decoded: Job = serde_json::from_str(&encoded).unwrap();
  assert_eq!(decoded, job);
  assert!(encoded.contains(r#""wait":{"frames":2}"#));
  assert!(encoded.contains(r#""wait":{"object":"4aac8ca0-af3d-409e-958e-62954e6cb3d1""#));
  assert!(encoded.contains(r#""video":{"action":"start""#));
  assert!(encoded.contains(r#""video":{"action":"stop"}"#));
}

#[test]
fn every_closed_enum_variant_has_its_exact_kebab_case_wire_value() {
  round_trip(&[Command::Run, Command::Capture]);
  round_trip(&[Platform::Macos, Platform::Webgl, Platform::IosSimulator]);
  round_trip(&[
    Orientation::Portrait,
    Orientation::PortraitUpsideDown,
    Orientation::LandscapeLeft,
    Orientation::LandscapeRight,
  ]);
  round_trip(&[
    Capability::Click,
    Capability::Hover,
    Capability::Drag,
    Capability::Key,
    Capability::Png,
    Capability::Video,
  ]);
  round_trip(&[Motion::Instant, Motion::Controlled, Motion::RealTime]);
  round_trip(&[KeyAction::Down, KeyAction::Up, KeyAction::Tap]);
  round_trip(&[
    ObjectState::Exists,
    ObjectState::Absent,
    ObjectState::Visible,
    ObjectState::Hidden,
    ObjectState::Enabled,
    ObjectState::Disabled,
  ]);
  assert_eq!(
    serde_json::to_value(Command::Capture).unwrap(),
    json!("capture")
  );
  assert_eq!(
    serde_json::to_value(Orientation::PortraitUpsideDown).unwrap(),
    json!("portrait-upside-down")
  );
  assert_eq!(
    serde_json::to_value(Platform::IosSimulator).unwrap(),
    json!("ios-simulator")
  );
}

#[test]
fn all_platform_profiles_validate_with_their_supported_capabilities() {
  let macos = job();
  macos.validate().unwrap();

  let mut webgl = job();
  webgl.profile.platform = Platform::Webgl;
  webgl.profile.capabilities.pop();
  webgl.scenarios[0]
    .steps
    .retain(|step| !matches!(step.action, StepKind::Video(_)));
  reindex(&mut webgl);
  webgl.validate().unwrap();

  let mut ios = job();
  ios.profile.platform = Platform::IosSimulator;
  ios.profile.display.orientation = Some(battlement_ditto::wire::job::Orientation::Portrait);
  ios.profile.display.safe_area = [0, 24, 1280, 672];
  ios.profile.capabilities.remove(1);
  ios.scenarios[0]
    .steps
    .retain(|step| !matches!(step.action, StepKind::Hover { .. }));
  reindex(&mut ios);
  ios.validate().unwrap();
}

#[test]
fn serde_rejects_unknown_fields_variants_and_malformed_unions() {
  let base = serde_json::to_value(job()).unwrap();
  let cases = [
    with(&base, "/unexpected", json!(true)),
    with(&base, "/profile/unexpected", json!(true)),
    with(&base, "/profile/display/unexpected", json!(true)),
    with(&base, "/scenarios/0/unexpected", json!(true)),
    with(&base, "/scenarios/0/steps/0/unexpected", json!(true)),
    with(
      &base,
      "/scenarios/0/steps/0/action/click/unexpected",
      json!(true),
    ),
    with(
      &base,
      "/scenarios/0/steps/6/action/wait/unexpected",
      json!(true),
    ),
    with(
      &base,
      "/scenarios/0/steps/7/action/wait/unexpected",
      json!(true),
    ),
    with(
      &base,
      "/scenarios/0/steps/8/action/assert/unexpected",
      json!(true),
    ),
    with(
      &base,
      "/scenarios/0/steps/9/action/screenshot/unexpected",
      json!(true),
    ),
    with(
      &base,
      "/scenarios/0/steps/9/action/screenshot/comparison/unexpected",
      json!(true),
    ),
    with(
      &base,
      "/scenarios/0/steps/10/action/video/unexpected",
      json!(true),
    ),
    with(&base, "/command", json!("unknown")),
    with(&base, "/profile/platform", json!("unknown")),
    with(&base, "/profile/display/orientation", json!("sideways")),
    with(&base, "/profile/capabilities/0", json!("unknown")),
    with(&base, "/scenarios/0/motion", json!("unknown")),
    with(&base, "/scenarios/0/steps/0/action", json!({"unknown": {}})),
    with(
      &base,
      "/scenarios/0/steps/3/action/key/action",
      json!("unknown"),
    ),
    with(
      &base,
      "/scenarios/0/steps/8/action/assert/state",
      json!("unknown"),
    ),
    with(
      &base,
      "/scenarios/0/steps/10/action/video/action",
      json!("unknown"),
    ),
    with(
      &base,
      "/scenarios/0/steps/0/action/click/target",
      json!({"object": UUID}),
    ),
    with(
      &base,
      "/scenarios/0/steps/6/action/wait",
      json!({"frames": 2, "object": UUID, "state": "visible"}),
    ),
    with(
      &base,
      "/scenarios/0/steps/10/action/video",
      json!({"action": "stop", "name": "clip"}),
    ),
  ];
  for (index, case) in cases.into_iter().enumerate() {
    assert!(
      serde_json::from_value::<Job>(case).is_err(),
      "case {index} unexpectedly parsed"
    );
  }
}

#[test]
fn job_identity_profile_and_collection_invariants_are_enforced() {
  invalid("job_id UUID", |job| job.job_id = "not-a-uuid".to_owned());
  invalid("canonical UUID", |job| job.run_id = UUID.to_uppercase());
  invalid("run timeout", |job| job.remaining_run_timeout_ms = 0);
  invalid("redaction empty", |job| {
    job.log_redactions.push(String::new())
  });
  invalid("redaction duplicate", |job| {
    job.log_redactions.push("secret".to_owned())
  });
  invalid("profile name", |job| job.profile.name = String::new());
  invalid("build hash", |job| {
    job.profile.build_fingerprint = "A".repeat(64)
  });
  invalid("source hash", |job| {
    job.profile.source_fingerprint.pop();
  });
  invalid("display width", |job| job.profile.display.width = 0);
  invalid("display scale", |job| job.profile.display.scale = f64::NAN);
  invalid("safe area", |job| {
    job.profile.display.safe_area = [1200, 0, 100, 720]
  });
  invalid("desktop orientation", |job| {
    job.profile.display.orientation = Some(battlement_ditto::wire::job::Orientation::Portrait)
  });
  invalid("capability duplicate", |job| {
    job.profile.capabilities.push(Capability::Click)
  });
  invalid("scenario ID duplicate", |job| {
    let mut duplicate = job.scenarios[0].clone();
    duplicate.name = "other".to_owned();
    duplicate.run_index = 2;
    job.scenarios.push(duplicate);
  });
  invalid("scenario name duplicate", |job| {
    let mut duplicate = job.scenarios[0].clone();
    duplicate.id = "d1bbd0ad-fcb7-48d7-b409-d221adc9eac6".to_owned();
    duplicate.run_index = 2;
    job.scenarios.push(duplicate);
  });
  invalid("run index order", |job| {
    let mut second = job.scenarios[0].clone();
    second.id = "d1bbd0ad-fcb7-48d7-b409-d221adc9eac6".to_owned();
    second.name = "other".to_owned();
    job.scenarios.push(second);
  });
  let mut too_many = job();
  too_many.scenarios = (0..129)
    .map(|index| {
      let mut scenario = too_many.scenarios[0].clone();
      scenario.id = format!("00000000-0000-4000-8000-{index:012}");
      scenario.name = format!("scenario {index}");
      scenario.run_index = index;
      scenario
    })
    .collect();
  assert!(too_many.validate().is_err());
}

#[test]
fn scenario_step_input_wait_and_comparison_invariants_are_enforced() {
  invalid("scenario timeout", |job| job.scenarios[0].timeout_ms = 0);
  invalid("scenario deadline", |job| {
    job.scenarios[0].timeout_ms = 20_001
  });
  invalid("empty steps", |job| job.scenarios[0].steps.clear());
  invalid("step index", |job| job.scenarios[0].steps[0].index = 1);
  invalid("step timeout", |job| {
    job.scenarios[0].steps[0].timeout_ms = 0
  });
  invalid("step deadline", |job| {
    job.scenarios[0].steps[0].timeout_ms = 10_001
  });
  invalid("duplicate step name", |job| {
    job.scenarios[0].steps[1].name = Some("click object".to_owned())
  });
  invalid("object UUID", |job| {
    job.scenarios[0].steps[0].action = StepKind::Click {
      target: battlement_ditto::wire::job::InputTarget::Object("alias".to_owned()),
    }
  });
  invalid("coordinate range", |job| {
    job.scenarios[0].steps[1].action = StepKind::Hover {
      target: battlement_ditto::wire::job::InputTarget::Coordinates([1.1, 0.5]),
    }
  });
  invalid("frame count", |job| {
    job.scenarios[0].steps[6].action =
      StepKind::Wait(battlement_ditto::wire::job::WaitStep::Frames(
        battlement_ditto::wire::job::FrameWait { frames: 0 },
      ))
  });
  invalid("frame motion", |job| {
    job.scenarios[0].motion = Motion::RealTime
  });
  invalid("condition UUID", |job| {
    if let StepKind::Assert(condition) = &mut job.scenarios[0].steps[8].action {
      condition.object = "bad".to_owned();
    }
  });
  invalid("checkpoint duplicate", |job| {
    if let StepKind::Screenshot(screenshot) = &mut job.scenarios[0].steps[11].action {
      screenshot.name = "ready".to_owned();
    }
  });
  for decimal in ["", "-1", "+1", "1e0", ".1", "1.", "00.1", "1.1"] {
    invalid(decimal, |job| {
      if let StepKind::Screenshot(screenshot) = &mut job.scenarios[0].steps[9].action {
        screenshot.comparison.threshold = decimal.to_owned();
      }
    });
  }
  invalid("percent range", |job| {
    if let StepKind::Screenshot(screenshot) = &mut job.scenarios[0].steps[9].action {
      screenshot.comparison.max_changed_percent = "100.01".to_owned();
    }
  });
  let mut too_many = job();
  too_many.scenarios[0].steps = (0..129)
    .map(|index| {
      let mut step = too_many.scenarios[0].steps[8].clone();
      step.index = index;
      step.name = None;
      step
    })
    .collect();
  assert!(too_many.validate().is_err());
}

#[test]
fn key_video_and_capability_state_is_validated_across_steps() {
  invalid("unreleased key", |job| {
    job.scenarios[0].steps.remove(4);
  });
  invalid("key up without down", |job| {
    job.scenarios[0].steps.remove(3);
  });
  invalid("tap held key", |job| {
    job.scenarios[0].steps[4].action = StepKind::Key {
      key: "Space".to_owned(),
      action: KeyAction::Tap,
    };
  });
  invalid("invalid key", |job| {
    if let StepKind::Key { key, .. } = &mut job.scenarios[0].steps[5].action {
      *key = "Left Shift".to_owned();
    }
  });
  invalid("video overlap", |job| {
    job.scenarios[0].steps[11].action = job.scenarios[0].steps[10].action.clone()
  });
  invalid("video stop", |job| {
    job.scenarios[0].steps.remove(10);
  });
  invalid("video unclosed", |job| {
    job.scenarios[0].steps.pop();
  });
  invalid("video instant", |job| {
    if let StepKind::Video(VideoStep::Start { motion, .. }) = &mut job.scenarios[0].steps[10].action
    {
      *motion = Motion::Instant;
    }
  });
  invalid("video duration", |job| {
    if let StepKind::Video(VideoStep::Start {
      max_duration_ms, ..
    }) = &mut job.scenarios[0].steps[10].action
    {
      *max_duration_ms = 30_001;
    }
  });
  invalid("unsupported video", |job| {
    job.profile.platform = Platform::Webgl;
    job.profile.capabilities.pop();
  });
  invalid("unsupported hover", |job| {
    job.profile.platform = Platform::IosSimulator;
    job.profile.display.orientation = Some(battlement_ditto::wire::job::Orientation::Portrait);
    job.profile.capabilities.remove(1);
  });
}

fn invalid(description: &str, mutate: impl FnOnce(&mut Job)) {
  let mut value = job();
  mutate(&mut value);
  assert!(value.validate().is_err(), "expected invalid {description}");
}

fn round_trip<T>(values: &[T])
where
  T: Clone + std::fmt::Debug + DeserializeOwned + PartialEq + Serialize,
{
  let encoded = serde_json::to_string(values).unwrap();
  let decoded: Vec<T> = serde_json::from_str(&encoded).unwrap();
  assert_eq!(decoded, values);
}

fn with(base: &Value, pointer: &str, value: Value) -> Value {
  let mut changed = base.clone();
  if let Some(target) = changed.pointer_mut(pointer) {
    *target = value;
    return changed;
  }
  let (parent, field) = pointer.rsplit_once('/').unwrap();
  changed
    .pointer_mut(parent)
    .unwrap()
    .as_object_mut()
    .unwrap()
    .insert(field.to_owned(), value);
  changed
}

fn reindex(job: &mut Job) {
  for (index, step) in job.scenarios[0].steps.iter_mut().enumerate() {
    step.index = index as u32;
  }
}

fn job() -> Job {
  serde_json::from_str(VALID_JOB).unwrap()
}

const UUID: &str = "4aac8ca0-af3d-409e-958e-62954e6cb3d1";

const VALID_JOB: &str = r#"{
  "job_id":"0197b35f-6c59-7b98-b1f0-a39f5ee54db8",
  "run_id":"0197b35f-6c59-7b98-b1f0-a39f5ee54db8",
  "remaining_run_timeout_ms":20000,
  "log_redactions":["secret"],
  "command":"run",
  "profile":{
    "name":"macos-local",
    "platform":"macos",
    "display":{"width":1280,"height":720,"scale":1.0,"orientation":null,"safe_area":[0,0,1280,720]},
    "build_fingerprint":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    "source_fingerprint":"fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
    "capabilities":["click","hover","drag","key","png","video"]
  },
  "scenarios":[{
    "id":"1f160ce4-dcdc-47ac-9613-31011f8afc96",
    "run_index":1,
    "name":"all wire shapes",
    "motion":"controlled",
    "timeout_ms":10000,
    "steps":[
      {"index":0,"name":"click object","timeout_ms":1000,"action":{"click":{"target":"4aac8ca0-af3d-409e-958e-62954e6cb3d1"}}},
      {"index":1,"name":null,"timeout_ms":1000,"action":{"hover":{"target":[0.5,0.75]}}},
      {"index":2,"name":null,"timeout_ms":1000,"action":{"drag":{"from":"4aac8ca0-af3d-409e-958e-62954e6cb3d1","to":[0.75,0.75]}}},
      {"index":3,"name":null,"timeout_ms":1000,"action":{"key":{"key":"Space","action":"down"}}},
      {"index":4,"name":null,"timeout_ms":1000,"action":{"key":{"key":"Space","action":"up"}}},
      {"index":5,"name":null,"timeout_ms":1000,"action":{"key":{"key":"Enter","action":"tap"}}},
      {"index":6,"name":null,"timeout_ms":1000,"action":{"wait":{"frames":2}}},
      {"index":7,"name":null,"timeout_ms":1000,"action":{"wait":{"object":"4aac8ca0-af3d-409e-958e-62954e6cb3d1","state":"visible"}}},
      {"index":8,"name":null,"timeout_ms":1000,"action":{"assert":{"object":"4aac8ca0-af3d-409e-958e-62954e6cb3d1","state":"enabled"}}},
      {"index":9,"name":null,"timeout_ms":1000,"action":{"screenshot":{"name":"ready","comparison":{"threshold":"0.05","anti_alias":false,"max_changed_percent":"0"}}}},
      {"index":10,"name":null,"timeout_ms":1000,"action":{"video":{"action":"start","name":"clip","motion":"real-time","max_duration_ms":5000}}},
      {"index":11,"name":null,"timeout_ms":1000,"action":{"screenshot":{"name":"recording","comparison":{"threshold":"1.0","anti_alias":true,"max_changed_percent":"100.0"}}}},
      {"index":12,"name":null,"timeout_ms":1000,"action":{"video":{"action":"stop"}}}
    ]
  }]
}"#;

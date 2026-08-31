use serde::Serialize;

use crate::animation_validation::model::FixtureCheckpoint;
use crate::animation_validation::{
  CaseId, CheckpointId, ExpectedObservation, FixtureAction, FixtureCase, FixtureMetadata,
  LifecycleBoundary, Observation, ReducedMotionOverride, ScreenId, Tolerance, ValidationRegistry,
};

const FIXTURE_SCREEN: ScreenId = ScreenId("targets-timelines");

/// Result of comparing one captured checkpoint with its independent expectation.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct CheckpointResult {
  pub(crate) checkpoint: CheckpointId,
  pub(crate) elapsed_micros: u64,
  pub(crate) expected: ExpectedObservation,
  pub(crate) observed: Observation,
  pub(crate) failures: Vec<String>,
}

impl CheckpointResult {
  pub(crate) fn passed(&self) -> bool {
    self.failures.is_empty()
  }
}

/// Machine-readable validation evidence and its concise human report.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ValidationReport {
  pub(crate) schema_version: u32,
  pub(crate) screen: ScreenId,
  pub(crate) case: CaseId,
  pub(crate) seed: u64,
  pub(crate) clock_quantum_micros: u64,
  pub(crate) metadata: FixtureMetadata,
  pub(crate) actions: Vec<FixtureAction>,
  pub(crate) checkpoints: Vec<CheckpointResult>,
}

impl ValidationReport {
  pub(crate) fn passed(&self) -> bool {
    self.checkpoints.iter().all(CheckpointResult::passed)
  }

  pub(crate) fn concise(&self) -> String {
    let failures = self
      .checkpoints
      .iter()
      .map(|checkpoint| checkpoint.failures.len())
      .sum::<usize>();
    format!(
      "{}/{}: {} checkpoints, {} failures ({})",
      self.screen.0,
      self.case.0,
      self.checkpoints.len(),
      failures,
      if failures == 0 { "PASS" } else { "FAIL" }
    )
  }

  pub(crate) fn json(&self) -> String {
    serde_json::to_string_pretty(self).expect("animation validation report should serialize")
  }
}

/// Mutable fixture state driven through the gallery's shared action path.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FixtureSession {
  elapsed_micros: u64,
  playing: bool,
  speed: f32,
  reduced_motion: ReducedMotionOverride,
  generation: u32,
  retargeted: bool,
  reconnects: u32,
  action_trace: Vec<FixtureAction>,
}

impl Default for FixtureSession {
  fn default() -> Self {
    Self {
      elapsed_micros: 0,
      playing: false,
      speed: 1.0,
      reduced_motion: ReducedMotionOverride::System,
      generation: 0,
      retargeted: false,
      reconnects: 0,
      action_trace: Vec::new(),
    }
  }
}

impl FixtureSession {
  pub(crate) fn dispatch(&mut self, action: FixtureAction) {
    match action {
      FixtureAction::Trigger => {
        self.generation = self.generation.wrapping_add(1);
        self.retargeted = !self.retargeted;
      }
      FixtureAction::Play => self.playing = true,
      FixtureAction::Pause => self.playing = false,
      FixtureAction::Replay => {
        self.elapsed_micros = 0;
        self.playing = true;
        self.generation = self.generation.wrapping_add(1);
      }
      FixtureAction::Speed(speed) => {
        assert!(
          speed.is_finite() && speed > 0.0,
          "fixture speed must be finite and positive"
        );
        self.speed = speed;
      }
      FixtureAction::ReducedMotion(value) => self.reduced_motion = value,
      FixtureAction::Reconnect => self.reconnects = self.reconnects.wrapping_add(1),
    }
    self.action_trace.push(action);
  }

  pub(crate) fn seek(&mut self, elapsed_micros: u64) {
    self.elapsed_micros = elapsed_micros;
    self.generation = self.generation.wrapping_add(1);
  }

  pub(crate) fn reset(&mut self) {
    *self = Self::default();
  }

  pub(crate) fn elapsed_micros(&self) -> u64 {
    self.elapsed_micros
  }

  pub(crate) fn playing(&self) -> bool {
    self.playing
  }

  pub(crate) fn speed(&self) -> f32 {
    self.speed
  }

  pub(crate) fn reduced_motion(&self) -> ReducedMotionOverride {
    self.reduced_motion
  }

  pub(crate) fn generation(&self) -> u32 {
    self.generation
  }

  pub(crate) fn retargeted(&self) -> bool {
    self.retargeted
  }

  pub(crate) fn reconnects(&self) -> u32 {
    self.reconnects
  }

  pub(crate) fn actions(&self) -> &[FixtureAction] {
    &self.action_trace
  }
}

/// Returns deterministic public-authoring cases shared by fast and native validation.
pub(crate) fn fixture_registry() -> ValidationRegistry {
  let actions = vec![
    FixtureAction::Trigger,
    FixtureAction::Play,
    FixtureAction::Pause,
    FixtureAction::Replay,
    FixtureAction::Speed(0.1),
    FixtureAction::ReducedMotion(ReducedMotionOverride::Always),
    FixtureAction::Reconnect,
  ];
  ValidationRegistry {
    schema_version: 1,
    cases: vec![
      FixtureCase {
        screen: FIXTURE_SCREEN,
        id: CaseId("public-tween"),
        seed: 0x5eed_0301,
        clock_quantum_micros: 1_000,
        checkpoints: [0_u64, 500_000, 1_000_000]
          .into_iter()
          .map(|elapsed| checkpoint("tween", elapsed, elapsed as f64 / 1_000_000.0))
          .collect(),
        actions: actions.clone(),
        deliberately_failing: false,
      },
      FixtureCase {
        screen: FIXTURE_SCREEN,
        id: CaseId("keyframe-boundary"),
        seed: 0x5eed_0302,
        clock_quantum_micros: 1_000,
        checkpoints: [
          (0_u64, 0.0),
          (250_000, 0.8),
          (500_000, 0.5),
          (750_000, 0.2),
          (1_000_000, 1.0),
        ]
        .into_iter()
        .map(|(elapsed, value)| checkpoint("keyframe", elapsed, value))
        .collect(),
        actions: actions.clone(),
        deliberately_failing: false,
      },
      FixtureCase {
        screen: FIXTURE_SCREEN,
        id: CaseId("retarget-presentation"),
        seed: 0x5eed_0303,
        clock_quantum_micros: 1_000,
        checkpoints: vec![
          checkpoint("retarget-before", 500_000, 0.5),
          checkpoint("retarget-after", 500_000, 0.5),
          checkpoint("retarget-end", 1_000_000, 1.0),
        ],
        actions,
        deliberately_failing: false,
      },
    ],
  }
}

/// Runs one case with an observation provider shared by fast and native lanes.
pub(crate) fn run_fixture_case(
  case: &FixtureCase,
  metadata: FixtureMetadata,
  mut observe: impl FnMut(&FixtureCheckpoint) -> Observation,
) -> ValidationReport {
  let checkpoints = case
    .checkpoints
    .iter()
    .map(|checkpoint| {
      let observed = observe(checkpoint);
      CheckpointResult {
        checkpoint: checkpoint.id,
        elapsed_micros: checkpoint.elapsed_micros,
        failures: compare(&checkpoint.expected, &observed),
        expected: checkpoint.expected.clone(),
        observed,
      }
    })
    .collect();
  ValidationReport {
    schema_version: 1,
    screen: case.screen,
    case: case.id,
    seed: case.seed,
    clock_quantum_micros: case.clock_quantum_micros,
    metadata,
    actions: case.actions.clone(),
    checkpoints,
  }
}

fn checkpoint(prefix: &'static str, elapsed_micros: u64, value: f64) -> FixtureCheckpoint {
  FixtureCheckpoint {
    id: CheckpointId(checkpoint_id(prefix, elapsed_micros)),
    elapsed_micros,
    expected: ExpectedObservation {
      scalar: Some(value),
      scalar_tolerance: Tolerance::new(0.000_01, "Unity writes the final opacity as f32"),
      velocity: Some(if elapsed_micros == 1_000_000 {
        0.0
      } else {
        1.0
      }),
      velocity_tolerance: Tolerance::new(0.000_01, "logical velocity uses f64 sampling"),
      paint: Some([0.13, 0.78, 0.88, value]),
      paint_tolerance: Tolerance::new(0.001, "native screenshot channels are quantized"),
      geometry: Some([24.0, 36.0, 360.0, 96.0]),
      geometry_tolerance: Tolerance::new(0.01, "panel geometry uses single-precision pixels"),
      lifecycle: checkpoint_lifecycle(prefix, elapsed_micros),
      live_hosts: 1,
      active_slots: usize::from(elapsed_micros < 1_000_000),
    },
  }
}

fn checkpoint_lifecycle(prefix: &str, elapsed_micros: u64) -> Vec<LifecycleBoundary> {
  match (prefix, elapsed_micros) {
    (_, 0) => vec![LifecycleBoundary::Activated],
    ("tween", 500_000) | ("keyframe", 500_000) => vec![LifecycleBoundary::Started],
    ("keyframe", 750_000) => vec![LifecycleBoundary::Repeated { first: 1, last: 1 }],
    ("retarget-before", _) => vec![LifecycleBoundary::Stopped],
    ("retarget-after", _) => vec![LifecycleBoundary::Cancelled, LifecycleBoundary::Activated],
    ("retarget-end", _) => vec![LifecycleBoundary::Completed, LifecycleBoundary::Cleanup],
    (_, 1_000_000) => vec![LifecycleBoundary::Completed],
    _ => Vec::new(),
  }
}

fn checkpoint_id(prefix: &'static str, elapsed_micros: u64) -> &'static str {
  match (prefix, elapsed_micros) {
    ("tween", 0) => "tween-start",
    ("tween", 500_000) => "tween-midpoint",
    ("tween", 1_000_000) => "tween-end",
    ("keyframe", 0) => "keyframe-start",
    ("keyframe", 250_000) => "keyframe-first-boundary",
    ("keyframe", 500_000) => "keyframe-midpoint",
    ("keyframe", 750_000) => "keyframe-second-boundary",
    ("keyframe", 1_000_000) => "keyframe-end",
    ("retarget-before", _) => "retarget-before",
    ("retarget-after", _) => "retarget-after",
    ("retarget-end", _) => "retarget-end",
    _ => panic!("unknown targets-and-timelines checkpoint"),
  }
}

fn compare(expected: &ExpectedObservation, observed: &Observation) -> Vec<String> {
  let mut failures = Vec::new();
  compare_scalar(
    "scalar",
    expected.scalar,
    observed.scalar,
    expected.scalar_tolerance,
    &mut failures,
  );
  compare_scalar(
    "velocity",
    expected.velocity,
    observed.velocity,
    expected.velocity_tolerance,
    &mut failures,
  );
  compare_array(
    "paint",
    expected.paint,
    observed.paint,
    expected.paint_tolerance,
    &mut failures,
  );
  compare_array(
    "geometry",
    expected.geometry,
    observed.geometry,
    expected.geometry_tolerance,
    &mut failures,
  );
  if expected.lifecycle != observed.lifecycle {
    failures.push(format!(
      "lifecycle expected {:?}, observed {:?}",
      expected.lifecycle, observed.lifecycle
    ));
  }
  if expected.live_hosts != observed.live_hosts {
    failures.push(format!(
      "cleanup live_hosts expected {}, observed {}",
      expected.live_hosts, observed.live_hosts
    ));
  }
  if expected.active_slots != observed.active_slots {
    failures.push(format!(
      "cleanup active_slots expected {}, observed {}",
      expected.active_slots, observed.active_slots
    ));
  }
  failures
}

fn compare_scalar(
  name: &str,
  expected: Option<f64>,
  observed: Option<f64>,
  tolerance: Tolerance,
  failures: &mut Vec<String>,
) {
  match (expected, observed) {
    (Some(expected), Some(observed)) if (expected - observed).abs() > tolerance.absolute => {
      failures.push(format!(
        "{name} expected {expected:.6} ± {:.6}, observed {observed:.6}",
        tolerance.absolute
      ));
    }
    (Some(_), None) => failures.push(format!("{name} observation is missing")),
    (None, Some(_)) => failures.push(format!("{name} observation was unexpected")),
    _ => {}
  }
}

fn compare_array<const N: usize>(
  name: &str,
  expected: Option<[f64; N]>,
  observed: Option<[f64; N]>,
  tolerance: Tolerance,
  failures: &mut Vec<String>,
) {
  match (expected, observed) {
    (Some(expected), Some(observed)) => {
      for (index, (expected, observed)) in expected.into_iter().zip(observed).enumerate() {
        if (expected - observed).abs() > tolerance.absolute {
          failures.push(format!(
            "{name}[{index}] expected {expected:.6} ± {:.6}, observed {observed:.6}",
            tolerance.absolute
          ));
        }
      }
    }
    (Some(_), None) => failures.push(format!("{name} observation is missing")),
    (None, Some(_)) => failures.push(format!("{name} observation was unexpected")),
    _ => {}
  }
}

pub(crate) fn fixture_metadata() -> FixtureMetadata {
  FixtureMetadata {
    build: "sample-local-fixture".to_owned(),
    player: "controlled".to_owned(),
    renderer: "public-motion-probe".to_owned(),
    resolution: "1280x720@1".to_owned(),
    platform: std::env::consts::OS.to_owned(),
    commit: option_env!("BATTLEMENT_COMMIT")
      .unwrap_or("working-tree")
      .to_owned(),
    screenshot_path: None,
  }
}

pub(crate) fn fixture_observation(checkpoint: &FixtureCheckpoint) -> Observation {
  let value = match checkpoint.id.0 {
    "tween-start" | "keyframe-start" => 0.0,
    "tween-midpoint" | "keyframe-midpoint" | "retarget-before" | "retarget-after" => 0.5,
    "keyframe-first-boundary" => 0.8,
    "keyframe-second-boundary" => 0.2,
    "tween-end" | "keyframe-end" | "retarget-end" => 1.0,
    _ => panic!("unknown targets-and-timelines observation"),
  };
  let mut observed = Observation::from(&checkpoint.expected);
  observed.scalar = Some(value);
  observed
}

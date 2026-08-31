use serde::Serialize;

use crate::animation_validation::model::FixtureCheckpoint;
use crate::animation_validation::{
  CaseId, CheckpointId, ExpectedObservation, FixtureAction, FixtureCase, FixtureMetadata,
  LifecycleBoundary, Observation, ReducedMotionOverride, ScreenId, Tolerance, ValidationRegistry,
};

const FIXTURE_SCREEN: ScreenId = ScreenId("validation-infrastructure");
const PASSING_CASE: CaseId = CaseId("static-presentation");
const FAILING_CASE: CaseId = CaseId("wrong-expectation");

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
      reconnects: 0,
      action_trace: Vec::new(),
    }
  }
}

impl FixtureSession {
  pub(crate) fn dispatch(&mut self, action: FixtureAction) {
    match action {
      FixtureAction::Trigger => self.generation = self.generation.wrapping_add(1),
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

  pub(crate) fn reconnects(&self) -> u32 {
    self.reconnects
  }

  pub(crate) fn actions(&self) -> &[FixtureAction] {
    &self.action_trace
  }
}

/// Returns static cases proving the validation pipeline before product motion exists.
pub(crate) fn fixture_registry() -> ValidationRegistry {
  let actions = vec![
    FixtureAction::Trigger,
    FixtureAction::Play,
    FixtureAction::Pause,
    FixtureAction::Replay,
    FixtureAction::Speed(2.0),
    FixtureAction::ReducedMotion(ReducedMotionOverride::Always),
    FixtureAction::Reconnect,
  ];
  ValidationRegistry {
    schema_version: 1,
    cases: vec![
      FixtureCase {
        screen: FIXTURE_SCREEN,
        id: PASSING_CASE,
        seed: 0x5eed_0001,
        clock_quantum_micros: 1_000,
        checkpoints: vec![fixture_checkpoint(42.0)],
        actions: actions.clone(),
        deliberately_failing: false,
      },
      FixtureCase {
        screen: FIXTURE_SCREEN,
        id: FAILING_CASE,
        seed: 0x5eed_0002,
        clock_quantum_micros: 1_000,
        checkpoints: vec![fixture_checkpoint(99.0)],
        actions,
        deliberately_failing: true,
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

fn fixture_checkpoint(expected_scalar: f64) -> FixtureCheckpoint {
  FixtureCheckpoint {
    id: CheckpointId("captured"),
    elapsed_micros: 250_000,
    expected: ExpectedObservation::fixture(expected_scalar, fixture_lifecycle()),
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
    renderer: "static-probe".to_owned(),
    resolution: "1280x720@1".to_owned(),
    platform: std::env::consts::OS.to_owned(),
    commit: option_env!("BATTLEMENT_COMMIT")
      .unwrap_or("working-tree")
      .to_owned(),
    screenshot_path: None,
  }
}

pub(crate) fn fixture_observation(_: &FixtureCheckpoint) -> Observation {
  Observation::from(&ExpectedObservation::fixture(42.0, fixture_lifecycle()))
}

fn fixture_lifecycle() -> Vec<LifecycleBoundary> {
  vec![
    LifecycleBoundary::Activated,
    LifecycleBoundary::Started,
    LifecycleBoundary::Repeated { first: 1, last: 2 },
    LifecycleBoundary::Stopped,
    LifecycleBoundary::Cancelled,
    LifecycleBoundary::Completed,
    LifecycleBoundary::Cleanup,
  ]
}

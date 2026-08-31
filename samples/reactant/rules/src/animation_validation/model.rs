use std::collections::BTreeSet;

use serde::Serialize;

/// Stable screen identity shared by the sample and validation runners.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(crate) struct ScreenId(pub(crate) &'static str);

/// Stable case identity within one animation screen.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(crate) struct CaseId(pub(crate) &'static str);

/// Stable checkpoint identity within one validation case.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(crate) struct CheckpointId(pub(crate) &'static str);

/// An absolute numerical tolerance with an explicit justification.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(crate) struct Tolerance {
  pub(crate) absolute: f64,
  pub(crate) rationale: &'static str,
}

impl Tolerance {
  pub(crate) const fn exact() -> Self {
    Self {
      absolute: 0.0,
      rationale: "fixture value is represented exactly",
    }
  }

  pub(crate) const fn new(absolute: f64, rationale: &'static str) -> Self {
    Self {
      absolute,
      rationale,
    }
  }

  fn validate(self, context: &str) -> Result<(), String> {
    if !self.absolute.is_finite() || self.absolute < 0.0 {
      return Err(format!("{context} has an invalid absolute tolerance"));
    }
    if self.rationale.trim().is_empty() {
      return Err(format!("{context} has an unexplained tolerance"));
    }
    Ok(())
  }
}

/// One reliable lifecycle boundary expected at a checkpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(crate) enum LifecycleBoundary {
  Activated,
  Started,
  Repeated { first: u32, last: u32 },
  Completed,
  Stopped,
  Cancelled,
  Cleanup,
}

/// Expected presentation, lifecycle, and cleanup state at one checkpoint.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExpectedObservation {
  pub(crate) scalar: Option<f64>,
  pub(crate) scalar_tolerance: Tolerance,
  pub(crate) velocity: Option<f64>,
  pub(crate) velocity_tolerance: Tolerance,
  pub(crate) paint: Option<[f64; 4]>,
  pub(crate) paint_tolerance: Tolerance,
  pub(crate) geometry: Option<[f64; 4]>,
  pub(crate) geometry_tolerance: Tolerance,
  pub(crate) lifecycle: Vec<LifecycleBoundary>,
  pub(crate) live_hosts: usize,
  pub(crate) active_slots: usize,
}

impl ExpectedObservation {
  pub(crate) fn fixture(scalar: f64, lifecycle: Vec<LifecycleBoundary>) -> Self {
    Self {
      scalar: Some(scalar),
      scalar_tolerance: Tolerance::exact(),
      velocity: Some(0.0),
      velocity_tolerance: Tolerance::exact(),
      paint: Some([0.2, 0.8, 0.9, 1.0]),
      paint_tolerance: Tolerance::new(0.001, "linear color channels are rounded by the probe"),
      geometry: Some([24.0, 36.0, 180.0, 80.0]),
      geometry_tolerance: Tolerance::new(0.01, "panel geometry uses single-precision pixels"),
      lifecycle,
      live_hosts: 1,
      active_slots: 0,
    }
  }

  fn validate(&self, context: &str) -> Result<(), String> {
    self
      .scalar_tolerance
      .validate(&format!("{context}.scalar"))?;
    self
      .velocity_tolerance
      .validate(&format!("{context}.velocity"))?;
    self.paint_tolerance.validate(&format!("{context}.paint"))?;
    self
      .geometry_tolerance
      .validate(&format!("{context}.geometry"))?;
    for (name, values) in [("paint", self.paint), ("geometry", self.geometry)] {
      if values.is_some_and(|values| values.into_iter().any(|value| !value.is_finite())) {
        return Err(format!("{context}.{name} contains a non-finite value"));
      }
    }
    if self.scalar.is_some_and(|value| !value.is_finite()) {
      return Err(format!("{context}.scalar contains a non-finite value"));
    }
    if self.velocity.is_some_and(|value| !value.is_finite()) {
      return Err(format!("{context}.velocity contains a non-finite value"));
    }
    Ok(())
  }
}

/// One actual presentation observation captured by a validation probe.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Observation {
  pub(crate) scalar: Option<f64>,
  pub(crate) velocity: Option<f64>,
  pub(crate) paint: Option<[f64; 4]>,
  pub(crate) geometry: Option<[f64; 4]>,
  pub(crate) lifecycle: Vec<LifecycleBoundary>,
  pub(crate) live_hosts: usize,
  pub(crate) active_slots: usize,
}

impl From<&ExpectedObservation> for Observation {
  fn from(value: &ExpectedObservation) -> Self {
    Self {
      scalar: value.scalar,
      velocity: value.velocity,
      paint: value.paint,
      geometry: value.geometry,
      lifecycle: value.lifecycle.clone(),
      live_hosts: value.live_hosts,
      active_slots: value.active_slots,
    }
  }
}

/// One deterministic action routed through the common sample control path.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(crate) enum FixtureAction {
  Trigger,
  Play,
  Pause,
  Replay,
  Speed(f32),
  ReducedMotion(ReducedMotionOverride),
  Reconnect,
}

/// Deterministic reduced-motion selection used by the animation gallery.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(crate) enum ReducedMotionOverride {
  System,
  Always,
  Never,
}

/// One checkpoint and its expected values.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct FixtureCheckpoint {
  pub(crate) id: CheckpointId,
  pub(crate) elapsed_micros: u64,
  pub(crate) expected: ExpectedObservation,
}

/// A deterministic validation case shared by the fast and heavy lanes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct FixtureCase {
  pub(crate) screen: ScreenId,
  pub(crate) id: CaseId,
  pub(crate) seed: u64,
  pub(crate) clock_quantum_micros: u64,
  pub(crate) checkpoints: Vec<FixtureCheckpoint>,
  pub(crate) actions: Vec<FixtureAction>,
  pub(crate) deliberately_failing: bool,
}

/// Complete case registry used by every validation lane.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ValidationRegistry {
  pub(crate) schema_version: u32,
  pub(crate) cases: Vec<FixtureCase>,
}

impl ValidationRegistry {
  pub(crate) fn validate(&self) -> Result<(), String> {
    if self.schema_version != 1 {
      return Err("animation validation schema version must be 1".to_owned());
    }
    if self.cases.is_empty() {
      return Err("animation validation registry is empty".to_owned());
    }
    let mut cases = BTreeSet::new();
    for case in &self.cases {
      validate_identity("screen", case.screen.0)?;
      validate_identity("case", case.id.0)?;
      if !cases.insert((case.screen, case.id)) {
        return Err(format!("duplicate case {}/{}", case.screen.0, case.id.0));
      }
      if case.clock_quantum_micros == 0 {
        return Err(format!(
          "case {}/{} has a zero clock quantum",
          case.screen.0, case.id.0
        ));
      }
      if case.checkpoints.is_empty() {
        return Err(format!(
          "case {}/{} has no checkpoints",
          case.screen.0, case.id.0
        ));
      }
      let mut checkpoints = BTreeSet::new();
      for checkpoint in &case.checkpoints {
        validate_identity("checkpoint", checkpoint.id.0)?;
        if !checkpoints.insert(checkpoint.id) {
          return Err(format!(
            "duplicate checkpoint {}/{}/{}",
            case.screen.0, case.id.0, checkpoint.id.0
          ));
        }
        checkpoint.expected.validate(&format!(
          "{}/{}/{}",
          case.screen.0, case.id.0, checkpoint.id.0
        ))?;
      }
    }
    Ok(())
  }

  pub(crate) fn select(&self, screen: ScreenId, id: CaseId) -> Result<&FixtureCase, String> {
    self
      .cases
      .iter()
      .find(|case| case.screen == screen && case.id == id)
      .ok_or_else(|| format!("unknown animation validation case {}/{}", screen.0, id.0))
  }
}

/// Environment facts attached to a machine-readable validation report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct FixtureMetadata {
  pub(crate) build: String,
  pub(crate) player: String,
  pub(crate) renderer: String,
  pub(crate) resolution: String,
  pub(crate) platform: String,
  pub(crate) commit: String,
  pub(crate) screenshot_path: Option<String>,
}

fn validate_identity(kind: &str, value: &str) -> Result<(), String> {
  let valid = !value.is_empty()
    && value
      .bytes()
      .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
  if !valid {
    return Err(format!("invalid {kind} identity {value:?}"));
  }
  Ok(())
}

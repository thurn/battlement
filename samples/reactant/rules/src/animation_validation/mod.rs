//! Shared deterministic validation support for the Reactant animation gallery.

mod model;
mod runner;
mod screen;

pub(crate) use model::{
  CaseId, CheckpointId, ExpectedObservation, FixtureAction, FixtureCase, FixtureMetadata,
  LifecycleBoundary, Observation, ReducedMotionOverride, ScreenId, Tolerance, ValidationRegistry,
};
pub(crate) use runner::{FixtureSession, ValidationReport, fixture_registry, run_fixture_case};
pub(crate) use screen::{ValidationScreen, ValidationUiState};

#[cfg(test)]
mod tests;

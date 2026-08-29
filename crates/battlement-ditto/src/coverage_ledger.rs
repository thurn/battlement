//! Checked visual-state coverage for convention-based samples.

use std::{
  collections::{BTreeMap, BTreeSet},
  fs,
  path::Path,
};

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;

use crate::{
  baseline_manifest::ManifestSnapshot,
  config::{
    self,
    model::{Profile, StepKind, VideoStep},
  },
};

const LEDGER_NAME: &str = "ditto-coverage.toml";
const REGISTRY_NAME: &str = "ditto-visual-states.toml";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
  version: u32,
  samples: Vec<String>,
}

/// Coverage status for every convention-based sample in a repository.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverageReport {
  pub samples: Vec<SampleReport>,
}

/// Checked status for one discovered sample.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SampleReport {
  pub sample: String,
  pub state_count: usize,
  pub status: SampleStatus,
}

/// Whether a sample is complete or assigned to later migration tasks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SampleStatus {
  Complete,
  Pending { tasks: Vec<u32> },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
  version: u32,
  #[serde(default)]
  states: Vec<State>,
  #[serde(default)]
  transitions: Vec<Transition>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
  key: String,
  screen: String,
  #[serde(default)]
  transient: bool,
  #[serde(default)]
  unsupported_profiles: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
struct Transition {
  from: String,
  to: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
  version: u32,
  sample: String,
  #[serde(default)]
  pending_tasks: Vec<u32>,
  canonical_profile: Option<String>,
  #[serde(default)]
  mappings: Vec<Mapping>,
  #[serde(default)]
  transitions: Vec<Transition>,
  #[serde(default)]
  skips: Vec<PlatformSkip>,
  #[serde(default)]
  conditional_omissions: Vec<ConditionalOmission>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Mapping {
  state: String,
  scenario: String,
  checkpoint: Option<String>,
  video: Option<String>,
  owner: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlatformSkip {
  state: String,
  profile: String,
  reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConditionalOmission {
  condition: String,
  reason: String,
}

#[derive(Debug)]
struct SuiteFacts {
  profiles: BTreeSet<String>,
  canonical_profiles: BTreeSet<String>,
  scenarios: BTreeMap<String, ScenarioFacts>,
  baselines: BTreeSet<(String, String, String)>,
}

#[derive(Debug)]
struct ScenarioFacts {
  checkpoints: BTreeSet<String>,
  videos: BTreeSet<String>,
}

/// Discovers and checks every direct child of `samples` containing `sample.toml`.
pub fn check_repository(repository: &Path) -> Result<CoverageReport> {
  let samples = repository.join("samples");
  let catalog: Catalog = read_toml(&samples.join(LEDGER_NAME))?;
  ensure!(
    catalog.version == 1,
    "sample coverage catalog version must be 1"
  );
  let expected = catalog
    .samples
    .iter()
    .map(String::as_str)
    .collect::<BTreeSet<_>>();
  ensure!(
    expected.len() == catalog.samples.len(),
    "sample coverage catalog contains duplicates"
  );
  for sample in &expected {
    validate_name("sample", sample)?;
  }
  let mut reports = Vec::new();
  for entry in fs::read_dir(&samples).with_context(|| format!("read {}", samples.display()))? {
    let directory = entry?.path();
    if !directory.join("sample.toml").is_file() {
      continue;
    }
    reports.push(check_sample(&directory)?);
  }
  reports.sort_by(|left, right| left.sample.cmp(&right.sample));
  ensure!(
    !reports.is_empty(),
    "no convention-based samples were discovered"
  );
  exact_set(
    "repository",
    "sample",
    expected,
    reports
      .iter()
      .map(|report| report.sample.as_str())
      .collect(),
  )?;
  Ok(CoverageReport { samples: reports })
}

fn check_sample(directory: &Path) -> Result<SampleReport> {
  let sample = directory
    .file_name()
    .and_then(|name| name.to_str())
    .context("sample directory name is not UTF-8")?;
  let registry: Registry = read_toml(&directory.join(REGISTRY_NAME))?;
  let ledger: Ledger = read_toml(&directory.join(LEDGER_NAME))?;
  validate_registry(&registry).with_context(|| format!("sample {sample} registry"))?;
  ensure!(
    ledger.version == 1,
    "sample {sample} ledger version must be 1"
  );
  ensure!(
    ledger.sample == sample,
    "sample {sample} ledger names {}",
    ledger.sample
  );
  if !ledger.pending_tasks.is_empty() {
    validate_pending(sample, &ledger)?;
    return Ok(SampleReport {
      sample: sample.to_owned(),
      state_count: registry.states.len(),
      status: SampleStatus::Pending {
        tasks: ledger.pending_tasks,
      },
    });
  }
  let facts = suite_facts(directory)?;
  validate_complete(sample, &registry, &ledger, &facts)?;
  Ok(SampleReport {
    sample: sample.to_owned(),
    state_count: registry.states.len(),
    status: SampleStatus::Complete,
  })
}

fn validate_registry(registry: &Registry) -> Result<()> {
  ensure!(registry.version == 1, "version must be 1");
  let mut states = BTreeSet::new();
  for state in &registry.states {
    validate_name("state key", &state.key)?;
    validate_name("screen", &state.screen)?;
    for profile in &state.unsupported_profiles {
      validate_name("unsupported profile", profile)?;
    }
    ensure!(
      state
        .unsupported_profiles
        .iter()
        .collect::<BTreeSet<_>>()
        .len()
        == state.unsupported_profiles.len(),
      "state {} has duplicate unsupported profiles",
      state.key
    );
    ensure!(states.insert(&state.key), "duplicate state {}", state.key);
  }
  let mut transitions = BTreeSet::new();
  for transition in &registry.transitions {
    ensure!(
      states.contains(&transition.from),
      "unknown transition source {}",
      transition.from
    );
    ensure!(
      states.contains(&transition.to),
      "unknown transition destination {}",
      transition.to
    );
    ensure!(
      transitions.insert(transition),
      "duplicate transition {} -> {}",
      transition.from,
      transition.to
    );
  }
  Ok(())
}

fn validate_pending(sample: &str, ledger: &Ledger) -> Result<()> {
  ensure!(
    ledger
      .pending_tasks
      .iter()
      .all(|task| (41..=48).contains(task)),
    "sample {sample} has an invalid migration task"
  );
  ensure!(
    ledger
      .pending_tasks
      .iter()
      .copied()
      .collect::<BTreeSet<_>>()
      .len()
      == ledger.pending_tasks.len(),
    "sample {sample} has duplicate migration tasks"
  );
  ensure!(
    ledger.canonical_profile.is_none(),
    "sample {sample} pending ledger selects a canonical profile"
  );
  ensure!(
    ledger.mappings.is_empty(),
    "sample {sample} pending ledger contains state mappings"
  );
  ensure!(
    ledger.transitions.is_empty(),
    "sample {sample} pending ledger contains transitions"
  );
  ensure!(
    ledger.skips.is_empty(),
    "sample {sample} pending ledger contains platform skips"
  );
  ensure!(
    ledger.conditional_omissions.is_empty(),
    "sample {sample} pending ledger contains conditional omissions"
  );
  Ok(())
}

fn validate_complete(
  sample: &str,
  registry: &Registry,
  ledger: &Ledger,
  facts: &SuiteFacts,
) -> Result<()> {
  ensure!(
    !registry.states.is_empty(),
    "sample {sample} registry has no states"
  );
  let profile = ledger
    .canonical_profile
    .as_deref()
    .context("complete ledger requires canonical_profile")?;
  ensure!(
    facts.profiles.contains(profile),
    "sample {sample} canonical profile {profile} is missing"
  );
  ensure!(
    facts.canonical_profiles.contains(profile),
    "sample {sample} canonical profile {profile} must be 1280x720 macOS at scale 1"
  );
  let expected_states = registry
    .states
    .iter()
    .map(|state| state.key.as_str())
    .collect::<BTreeSet<_>>();
  let mut mapped_states = BTreeSet::new();
  let mut mapped_checkpoints = BTreeSet::new();
  let mut mapped_videos = BTreeSet::new();
  let mut mapped_baselines = BTreeSet::new();
  for mapping in &ledger.mappings {
    validate_name("test owner", &mapping.owner)?;
    ensure!(
      expected_states.contains(mapping.state.as_str()),
      "sample {sample} ledger has unknown state {}",
      mapping.state
    );
    ensure!(
      mapped_states.insert(mapping.state.as_str()),
      "sample {sample} state {} has multiple owners",
      mapping.state
    );
    let scenario = facts.scenarios.get(&mapping.scenario).with_context(|| {
      format!(
        "sample {sample} state {} references missing scenario {}",
        mapping.state, mapping.scenario
      )
    })?;
    let state = registry
      .states
      .iter()
      .find(|state| state.key == mapping.state)
      .expect("mapped state was checked above");
    if state.transient {
      let video = mapping
        .video
        .as_deref()
        .context("transient state mapping requires video")?;
      ensure!(
        mapping.checkpoint.is_none(),
        "sample {sample} transient state {} cannot name a checkpoint",
        mapping.state
      );
      ensure!(
        scenario.videos.contains(video),
        "sample {sample} scenario {} is missing video {video}",
        mapping.scenario
      );
      ensure!(
        mapped_videos.insert((mapping.scenario.clone(), video.to_owned())),
        "sample {sample} video {}/{} has multiple owners",
        mapping.scenario,
        video
      );
      continue;
    }
    let checkpoint = mapping
      .checkpoint
      .as_deref()
      .context("stable state mapping requires checkpoint")?;
    ensure!(
      mapping.video.is_none(),
      "sample {sample} stable state {} cannot name a video",
      mapping.state
    );
    ensure!(
      scenario.checkpoints.contains(checkpoint),
      "sample {sample} scenario {} is missing checkpoint {checkpoint}",
      mapping.scenario
    );
    ensure!(
      mapped_checkpoints.insert((mapping.scenario.clone(), checkpoint.to_owned())),
      "sample {sample} checkpoint {}/{} has multiple owners",
      mapping.scenario,
      checkpoint
    );
    ensure!(
      facts.baselines.contains(&(
        profile.to_owned(),
        mapping.scenario.clone(),
        checkpoint.to_owned()
      )),
      "sample {sample} missing baseline {profile}/{}/{checkpoint}",
      mapping.scenario
    );
    mapped_baselines.insert((
      profile.to_owned(),
      mapping.scenario.clone(),
      checkpoint.to_owned(),
    ));
  }
  exact_set(sample, "registry state", expected_states, mapped_states)?;
  let suite_checkpoints = facts
    .scenarios
    .iter()
    .flat_map(|(scenario, facts)| {
      facts
        .checkpoints
        .iter()
        .map(move |checkpoint| (scenario.clone(), checkpoint.clone()))
    })
    .collect();
  exact_set(sample, "checkpoint", suite_checkpoints, mapped_checkpoints)?;
  let suite_videos = facts
    .scenarios
    .iter()
    .flat_map(|(scenario, facts)| {
      facts
        .videos
        .iter()
        .map(move |video| (scenario.clone(), video.clone()))
    })
    .collect();
  exact_set(sample, "video", suite_videos, mapped_videos)?;
  exact_set(
    sample,
    "baseline",
    mapped_baselines,
    facts.baselines.clone(),
  )?;
  exact_set(
    sample,
    "transition",
    registry.transitions.iter().collect(),
    ledger.transitions.iter().collect(),
  )?;
  validate_skips(sample, registry, ledger, facts, profile)?;
  self::validate_conditional_omissions(sample, ledger)
}

fn validate_conditional_omissions(sample: &str, ledger: &Ledger) -> Result<()> {
  let mut conditions = BTreeSet::new();
  for omission in &ledger.conditional_omissions {
    validate_name("conditional omission", &omission.condition)?;
    ensure!(
      !omission.reason.trim().is_empty(),
      "sample {sample} conditional omission {} requires a reason",
      omission.condition
    );
    ensure!(
      conditions.insert(&omission.condition),
      "sample {sample} duplicate conditional omission {}",
      omission.condition
    );
  }
  Ok(())
}

fn validate_skips(
  sample: &str,
  registry: &Registry,
  ledger: &Ledger,
  facts: &SuiteFacts,
  canonical: &str,
) -> Result<()> {
  let states = registry
    .states
    .iter()
    .map(|state| state.key.as_str())
    .collect::<BTreeSet<_>>();
  let mut skips = BTreeSet::new();
  for skip in &ledger.skips {
    ensure!(
      states.contains(skip.state.as_str()),
      "sample {sample} skip has unknown state {}",
      skip.state
    );
    ensure!(
      facts.profiles.contains(&skip.profile),
      "sample {sample} skip has unknown profile {}",
      skip.profile
    );
    ensure!(
      skip.profile != canonical,
      "sample {sample} canonical profile cannot skip {}",
      skip.state
    );
    ensure!(
      !skip.reason.trim().is_empty(),
      "sample {sample} skip requires a reason"
    );
    ensure!(
      skips.insert((&skip.state, &skip.profile)),
      "sample {sample} duplicate skip for {}/{}",
      skip.profile,
      skip.state
    );
  }
  let expected = registry
    .states
    .iter()
    .flat_map(|state| {
      state
        .unsupported_profiles
        .iter()
        .map(move |profile| (&state.key, profile))
    })
    .collect();
  exact_set(sample, "platform skip", expected, skips)
}

fn suite_facts(directory: &Path) -> Result<SuiteFacts> {
  let suite = config::load(Some(&directory.join("ditto.toml")))?;
  let snapshot = ManifestSnapshot::read(&directory.join("ditto.lock"))?;
  let canonical_profiles = suite
    .profiles
    .iter()
    .filter_map(|(name, profile)| match profile {
      Profile::Macos { display }
        if display.width == 1280 && display.height == 720 && display.scale == 1.0 =>
      {
        Some(name.clone())
      }
      _ => None,
    })
    .collect();
  Ok(SuiteFacts {
    profiles: suite.profiles.into_keys().collect(),
    canonical_profiles,
    scenarios: suite
      .scenarios
      .into_iter()
      .map(|scenario| {
        let mut checkpoints = BTreeSet::new();
        let mut videos = BTreeSet::new();
        for step in scenario.steps {
          match step.action {
            StepKind::Screenshot(screenshot) => {
              checkpoints.insert(screenshot.name);
            }
            StepKind::Video(VideoStep::Start { name, .. }) => {
              videos.insert(name);
            }
            _ => {}
          }
        }
        (
          scenario.name,
          ScenarioFacts {
            checkpoints,
            videos,
          },
        )
      })
      .collect(),
    baselines: snapshot.manifest.map_or_else(BTreeSet::new, |manifest| {
      manifest
        .baselines
        .into_iter()
        .map(|entry| (entry.profile, entry.scenario, entry.checkpoint))
        .collect()
    }),
  })
}

fn exact_set<T: Ord + std::fmt::Debug>(
  sample: &str,
  kind: &str,
  expected: BTreeSet<T>,
  actual: BTreeSet<T>,
) -> Result<()> {
  if let Some(missing) = expected.difference(&actual).next() {
    bail!("sample {sample} missing {kind} {missing:?}");
  }
  if let Some(orphan) = actual.difference(&expected).next() {
    bail!("sample {sample} orphan {kind} {orphan:?}");
  }
  Ok(())
}

fn validate_name(field: &str, value: &str) -> Result<()> {
  ensure!(
    !value.is_empty()
      && value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_.:/".contains(&byte)),
    "{field} {value:?} is not canonical"
  );
  Ok(())
}

fn read_toml<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
  toml::from_str(&fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?)
    .with_context(|| format!("parse {}", path.display()))
}

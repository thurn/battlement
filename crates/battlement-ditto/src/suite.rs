use std::{
  collections::BTreeMap,
  fmt::{self, Display as FmtDisplay},
  fs,
  path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use battlement_tooling::{contained_path, repository_root};
use serde::Deserialize;

const CONFIG_NAME: &str = "ditto.toml";

/// A suite resolved independently from the caller's working directory.
#[derive(Debug, Eq, PartialEq)]
pub struct ListedSuite {
  pub name: String,
  pub config: PathBuf,
  pub repository: PathBuf,
  pub profiles: Vec<ListedProfile>,
  pub scenarios: Vec<ListedScenario>,
}

/// A resolved launch profile shown by `ditto list`.
#[derive(Debug, Eq, PartialEq)]
pub struct ListedProfile {
  pub name: String,
  pub target: Target,
  pub display: Display,
  pub selected: bool,
}

/// A render size shown by `ditto list`.
#[derive(Debug, Eq, PartialEq)]
pub struct Display {
  pub width: u32,
  pub height: u32,
  pub scale: String,
}

/// A supported Ditto player target.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Target {
  Macos,
}

/// A scenario and its ordered screenshot checkpoints.
#[derive(Debug, Eq, PartialEq)]
pub struct ListedScenario {
  pub name: String,
  pub checkpoints: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SuiteFile {
  name: String,
  default_profile: String,
  player: Player,
  profiles: BTreeMap<String, Profile>,
  scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Player {
  unity_project: PathBuf,
  scene: PathBuf,
  rust_manifest: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
  target: Target,
  display: DisplayFile,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DisplayFile {
  width: u32,
  height: u32,
  scale: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
  name: String,
  #[serde(default)]
  steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Step {
  screenshot: Option<Screenshot>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Screenshot {
  name: String,
}

pub(crate) fn load(explicit: Option<&Path>) -> Result<ListedSuite> {
  let current = std::env::current_dir().context("failed to read the current directory")?;
  let config = match explicit {
    Some(path) if path.is_absolute() => path.to_path_buf(),
    Some(path) => current.join(path),
    None => discover(&current)?,
  };
  let config = config
    .canonicalize()
    .with_context(|| format!("failed to resolve suite {}", config.display()))?;
  let directory = config.parent().expect("suite file has a parent");
  let repository = repository_root(directory)?;
  if !config.starts_with(&repository) {
    bail!("suite is outside repository root: {}", config.display());
  }
  let source = fs::read_to_string(&config)
    .with_context(|| format!("failed to read suite {}", config.display()))?;
  let suite: SuiteFile = toml::from_str(&source)
    .with_context(|| format!("failed to parse suite {}", config.display()))?;
  validate_paths(&suite.player, &repository, directory)?;
  validate(&suite)?;
  Ok(ListedSuite {
    name: suite.name,
    config,
    repository,
    profiles: suite
      .profiles
      .into_iter()
      .map(|(name, profile)| ListedProfile {
        selected: name == suite.default_profile,
        name,
        target: profile.target,
        display: Display {
          width: profile.display.width,
          height: profile.display.height,
          scale: format!("{:.1}", profile.display.scale),
        },
      })
      .collect(),
    scenarios: suite
      .scenarios
      .into_iter()
      .map(|scenario| ListedScenario {
        name: scenario.name,
        checkpoints: scenario
          .steps
          .into_iter()
          .filter_map(|step| step.screenshot.map(|screenshot| screenshot.name))
          .collect(),
      })
      .collect(),
  })
}

fn discover(start: &Path) -> Result<PathBuf> {
  start
    .ancestors()
    .map(|directory| directory.join(CONFIG_NAME))
    .find(|candidate| candidate.is_file())
    .ok_or_else(|| anyhow::anyhow!("could not find {CONFIG_NAME} from {}", start.display()))
}

fn validate_paths(player: &Player, repository: &Path, directory: &Path) -> Result<()> {
  contained_path(repository, directory, &player.unity_project)?;
  contained_path(repository, directory, &player.scene)?;
  contained_path(repository, directory, &player.rust_manifest)?;
  Ok(())
}

fn validate(suite: &SuiteFile) -> Result<()> {
  if suite.name.trim().is_empty() {
    bail!("suite name must not be empty");
  }
  if !suite.profiles.contains_key(&suite.default_profile) {
    bail!("default profile {:?} does not exist", suite.default_profile);
  }
  if suite.scenarios.is_empty() {
    bail!("suite must contain at least one scenario");
  }
  Ok(())
}

impl FmtDisplay for Target {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Macos => formatter.write_str("macos"),
    }
  }
}

impl FmtDisplay for ListedSuite {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    writeln!(formatter, "Suite: {}", self.name)?;
    writeln!(formatter, "Profiles:")?;
    for profile in &self.profiles {
      writeln!(
        formatter,
        "  {}{} [{}] {}x{} @ {}",
        if profile.selected { "* " } else { "  " },
        profile.name,
        profile.target,
        profile.display.width,
        profile.display.height,
        profile.display.scale
      )?;
    }
    writeln!(formatter, "Scenarios:")?;
    for scenario in &self.scenarios {
      writeln!(formatter, "  - {}", scenario.name)?;
      for checkpoint in &scenario.checkpoints {
        writeln!(formatter, "    screenshot: {checkpoint}")?;
      }
    }
    Ok(())
  }
}

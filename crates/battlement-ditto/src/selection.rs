//! Deterministic profile, scenario, and capability selection.

use std::collections::BTreeSet;

use anyhow::{Result, bail};

use crate::config::model::{Profile, Scenario, StepKind, Suite, Target};

/// Inputs that select one profile and an ordered scenario subset.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Options {
  pub profile: Option<String>,
  pub includes: Vec<String>,
  pub excludes: Vec<String>,
  pub allow_empty: bool,
}

/// A resolved suite selection with host-materialized skips.
#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
  pub profile_name: String,
  pub profile: Profile,
  pub scenarios: Vec<MaterializedScenario>,
}

/// One selected scenario in stable run order.
#[derive(Clone, Debug, PartialEq)]
pub struct MaterializedScenario {
  pub run_index: u32,
  pub source_index: usize,
  pub scenario: Scenario,
  pub disposition: Disposition,
}

/// Whether a selected scenario is runnable on the resolved profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Disposition {
  Runnable,
  Skipped { reason: String },
}

/// Resolves selection options against a validated suite.
pub fn resolve(suite: &Suite, options: &Options) -> Result<Selection> {
  let profile_name = options
    .profile
    .as_ref()
    .unwrap_or(&suite.default_profile)
    .clone();
  let profile = suite
    .profiles
    .get(&profile_name)
    .ok_or_else(|| anyhow::anyhow!("profile {profile_name:?} does not exist"))?
    .clone();
  let mut selected = if options.includes.is_empty() {
    (0..suite.scenarios.len()).collect()
  } else {
    matching_indices(&suite.scenarios, &options.includes, "scenario")?
  };
  let excluded = matching_indices(&suite.scenarios, &options.excludes, "exclude")?;
  selected.retain(|index| !excluded.contains(index));
  if selected.is_empty() && !options.allow_empty {
    bail!("scenario selection is empty; pass --allow-empty to accept it");
  }
  let scenarios = selected
    .into_iter()
    .enumerate()
    .map(|(run_index, source_index)| {
      let scenario = suite.scenarios[source_index].clone();
      MaterializedScenario {
        run_index: run_index as u32,
        disposition: skip_reason(profile.target(), &scenario)
          .map_or(Disposition::Runnable, |reason| Disposition::Skipped {
            reason,
          }),
        source_index,
        scenario,
      }
    })
    .collect();
  Ok(Selection {
    profile_name,
    profile,
    scenarios,
  })
}

fn matching_indices(
  scenarios: &[Scenario],
  patterns: &[String],
  description: &str,
) -> Result<BTreeSet<usize>> {
  let mut selected = BTreeSet::new();
  for pattern in patterns {
    if pattern.is_empty() {
      bail!("{description} selector must not be empty");
    }
    if pattern.contains(['[', ']']) {
      bail!("unsupported {description} glob {pattern:?}; use `*` and `?`");
    }
    let matches: Vec<usize> = scenarios
      .iter()
      .enumerate()
      .filter_map(|(index, scenario)| glob_matches(pattern, &scenario.name).then_some(index))
      .collect();
    if matches.is_empty() {
      bail!("{description} selector {pattern:?} matched no scenarios");
    }
    selected.extend(matches);
  }
  Ok(selected)
}

fn skip_reason(target: Target, scenario: &Scenario) -> Option<String> {
  for step in &scenario.steps {
    match (&step.action, target) {
      (StepKind::Hover { .. }, Target::IosSimulator) => {
        return Some("unsupported-input:hover".to_owned());
      }
      (StepKind::Video(_), Target::Webgl) => {
        return Some("unsupported-step:video".to_owned());
      }
      _ => {}
    }
  }
  None
}

fn glob_matches(pattern: &str, value: &str) -> bool {
  let pattern: Vec<char> = pattern.chars().collect();
  let value: Vec<char> = value.chars().collect();
  let mut previous = vec![false; value.len() + 1];
  previous[0] = true;
  for token in pattern {
    let mut current = vec![false; value.len() + 1];
    if token == '*' {
      current[0] = previous[0];
      for index in 1..=value.len() {
        current[index] = previous[index] || current[index - 1];
      }
    } else {
      for index in 1..=value.len() {
        current[index] = previous[index - 1] && (token == '?' || token == value[index - 1]);
      }
    }
    previous = current;
  }
  previous[value.len()]
}

//! Human-readable suite listing.

use std::{fmt, path::Path};

use anyhow::Result;

use crate::config::{
  self,
  model::{Orientation, Profile, StepKind, Suite, Target},
};
use crate::selection::{Disposition, Options};

/// A stable human-readable view of a validated suite.
#[derive(Debug, Eq, PartialEq)]
pub struct ListedSuite {
  pub name: String,
  pub profiles: Vec<ListedProfile>,
  pub scenarios: Vec<ListedScenario>,
}

/// A launch profile shown by `ditto list`.
#[derive(Debug, Eq, PartialEq)]
pub struct ListedProfile {
  pub name: String,
  pub target: Target,
  pub details: String,
  pub selected: bool,
}

/// A scenario and its ordered screenshot checkpoints.
#[derive(Debug, Eq, PartialEq)]
pub struct ListedScenario {
  pub name: String,
  pub checkpoints: Vec<String>,
  pub status: String,
}

pub(crate) fn load(explicit: Option<&Path>, options: Options) -> Result<ListedSuite> {
  let suite = config::load(explicit)?;
  let selection = crate::selection::resolve(&suite, &options)?;
  Ok(listing(suite, selection))
}

fn listing(suite: Suite, selection: crate::selection::Selection) -> ListedSuite {
  ListedSuite {
    name: suite.name,
    profiles: suite
      .profiles
      .into_iter()
      .map(|(name, profile)| ListedProfile {
        selected: name == selection.profile_name,
        target: profile.target(),
        details: profile_details(&profile),
        name,
      })
      .collect(),
    scenarios: selection
      .scenarios
      .into_iter()
      .map(|materialized| ListedScenario {
        name: materialized.scenario.name,
        status: match materialized.disposition {
          Disposition::Runnable => "run".to_owned(),
          Disposition::Skipped { reason } => format!("skip: {reason}"),
        },
        checkpoints: materialized
          .scenario
          .steps
          .into_iter()
          .filter_map(|step| match step.action {
            StepKind::Screenshot(screenshot) => Some(screenshot.name),
            _ => None,
          })
          .collect(),
      })
      .collect(),
  }
}

fn profile_details(profile: &Profile) -> String {
  match profile {
    Profile::Macos { display } | Profile::Webgl { display, .. } => {
      let scale = if display.scale.fract() == 0.0 {
        format!("{:.1}", display.scale)
      } else {
        display.scale.to_string()
      };
      format!("{}x{} @ {scale}", display.width, display.height)
    }
    Profile::IosSimulator {
      device,
      orientation,
    } => format!("{device} ({orientation})"),
  }
}

impl fmt::Display for Target {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Macos => formatter.write_str("macos"),
      Self::Webgl => formatter.write_str("webgl"),
      Self::IosSimulator => formatter.write_str("ios-simulator"),
    }
  }
}

impl fmt::Display for Orientation {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Portrait => formatter.write_str("portrait"),
      Self::PortraitUpsideDown => formatter.write_str("portrait-upside-down"),
      Self::LandscapeLeft => formatter.write_str("landscape-left"),
      Self::LandscapeRight => formatter.write_str("landscape-right"),
    }
  }
}

impl fmt::Display for ListedSuite {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    writeln!(formatter, "Suite: {}", self.name)?;
    writeln!(formatter, "Profiles:")?;
    for profile in &self.profiles {
      writeln!(
        formatter,
        "  {}{} [{}] {}",
        if profile.selected { "* " } else { "  " },
        profile.name,
        profile.target,
        profile.details
      )?;
    }
    writeln!(formatter, "Scenarios:")?;
    for scenario in &self.scenarios {
      writeln!(formatter, "  - {} [{}]", scenario.name, scenario.status)?;
      for checkpoint in &scenario.checkpoints {
        writeln!(formatter, "    screenshot: {checkpoint}")?;
      }
    }
    Ok(())
  }
}

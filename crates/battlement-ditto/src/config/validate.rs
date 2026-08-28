use std::{
  collections::BTreeMap,
  path::{Path, PathBuf},
};

use battlement_tooling::{contained_path, repository_root, resolve_nearest};
use uuid::Uuid;

use crate::config::{
  diagnostic::{ConfigError, invalid},
  model::{
    Baseline, Comparison, Defaults, Display, Motion, Orientation, Player, Profile, Suite, Timeouts,
  },
  raw::{
    RawBaseline, RawComparison, RawDecimal, RawDefaults, RawFragment, RawMotion, RawOrientation,
    RawProfile, RawSuite, RawTarget, RawTimeouts,
  },
  scenario,
  value::{DurationValue, ExactDecimal},
};

const MAX_DURATION_MS: u64 = 3_600_000;
const MAX_NAME_BYTES: usize = 128;

pub(super) struct Validation<'a> {
  pub path: &'a Path,
  pub source: &'a str,
  pub aliases: &'a BTreeMap<String, Uuid>,
  pub defaults: &'a Defaults,
  pub run_timeout: DurationValue,
}

pub(super) fn suite(
  raw: RawSuite,
  source_path: PathBuf,
  source: String,
) -> Result<Suite, ConfigError> {
  let directory = source_path.parent().expect("suite path has a parent");
  let repository = repository_root(directory)
    .map_err(|error| invalid(&source_path, &source, "suite", error.to_string()))?;
  name(&source_path, &source, "name", &raw.name)?;
  if raw.scenarios.is_empty() {
    return Err(invalid(
      &source_path,
      &source,
      "scenarios",
      "suite must contain at least one scenario",
    ));
  }
  if raw.scenarios.len() > 128 {
    return Err(invalid(
      &source_path,
      &source,
      "scenarios",
      "suite may contain at most 128 scenarios",
    ));
  }
  let player = Player {
    unity_project: player_path(
      &source_path,
      &source,
      &repository,
      directory,
      "player.unity_project",
      raw.player.unity_project,
    )?,
    scene: player_path(
      &source_path,
      &source,
      &repository,
      directory,
      "player.scene",
      raw.player.scene,
    )?,
    rust_manifest: player_path(
      &source_path,
      &source,
      &repository,
      directory,
      "player.rust_manifest",
      raw.player.rust_manifest,
    )?,
  };
  let timeouts = timeouts(&source_path, &source, raw.timeouts)?;
  let defaults = defaults(&source_path, &source, raw.defaults, timeouts)?;
  let aliases = aliases(&source_path, &source, raw.aliases)?;
  let baseline = baseline(&source_path, &source, directory, raw.baseline)?;
  let profiles = profiles(&source_path, &source, raw.profiles)?;
  if !profiles.contains_key(&raw.default_profile) {
    return Err(invalid(
      &source_path,
      &source,
      "default_profile",
      format!("profile {:?} does not exist", raw.default_profile),
    ));
  }
  let validation = Validation {
    path: &source_path,
    source: &source,
    aliases: &aliases,
    defaults: &defaults,
    run_timeout: timeouts.run,
  };
  let mut names = std::collections::BTreeSet::new();
  let mut scenarios = Vec::with_capacity(raw.scenarios.len());
  for (index, raw_scenario) in raw.scenarios.into_iter().enumerate() {
    name(
      &source_path,
      &source,
      &format!("scenarios.{index}.name"),
      &raw_scenario.name,
    )?;
    if !names.insert(raw_scenario.name.clone()) {
      return Err(invalid(
        &source_path,
        &source,
        format!("scenarios.{index}.name"),
        format!("duplicate scenario name {:?}", raw_scenario.name),
      ));
    }
    scenarios.push(scenario::validate(&validation, index, raw_scenario)?);
  }
  Ok(Suite {
    source: source_path,
    repository,
    name: raw.name,
    default_profile: raw.default_profile,
    player,
    timeouts,
    defaults,
    aliases,
    baseline,
    profiles,
    scenarios,
  })
}

pub(super) fn fragment(
  raw: RawFragment,
  base: &Suite,
  source_path: PathBuf,
  source: String,
  standard_input: bool,
) -> Result<Suite, ConfigError> {
  let fragment_name = raw.name.unwrap_or_else(|| {
    if standard_input {
      "standard-input".to_owned()
    } else {
      format!("{} fragment", base.name)
    }
  });
  name(&source_path, &source, "name", &fragment_name)?;
  if raw.scenarios.is_empty() || raw.scenarios.len() > 128 {
    return Err(invalid(
      &source_path,
      &source,
      "scenarios",
      "fragment must contain 1 through 128 scenarios",
    ));
  }
  let defaults = inherited_defaults(&source_path, &source, raw.defaults, &base.defaults)?;
  let mut merged_aliases = base.aliases.clone();
  for (alias, value) in aliases(&source_path, &source, raw.aliases)? {
    if let Some(inherited) = merged_aliases.get(&alias)
      && inherited != &value
    {
      return Err(invalid(
        &source_path,
        &source,
        format!("aliases.{alias}"),
        "fragment alias conflicts with its inherited UUID",
      ));
    }
    merged_aliases.insert(alias, value);
  }
  let validation = Validation {
    path: &source_path,
    source: &source,
    aliases: &merged_aliases,
    defaults: &defaults,
    run_timeout: base.timeouts.run,
  };
  let mut names = std::collections::BTreeSet::new();
  let mut scenarios = Vec::with_capacity(raw.scenarios.len());
  for (index, raw_scenario) in raw.scenarios.into_iter().enumerate() {
    name(
      &source_path,
      &source,
      &format!("scenarios.{index}.name"),
      &raw_scenario.name,
    )?;
    if !names.insert(raw_scenario.name.clone()) {
      return Err(invalid(
        &source_path,
        &source,
        format!("scenarios.{index}.name"),
        format!("duplicate scenario name {:?}", raw_scenario.name),
      ));
    }
    scenarios.push(scenario::validate(&validation, index, raw_scenario)?);
  }
  Ok(Suite {
    source: source_path,
    repository: base.repository.clone(),
    name: fragment_name,
    default_profile: base.default_profile.clone(),
    player: base.player.clone(),
    timeouts: base.timeouts,
    defaults,
    aliases: merged_aliases,
    baseline: None,
    profiles: base.profiles.clone(),
    scenarios,
  })
}

fn player_path(
  path: &Path,
  source: &str,
  repository: &std::path::Path,
  directory: &std::path::Path,
  key: &str,
  value: PathBuf,
) -> Result<PathBuf, ConfigError> {
  contained_path(repository, directory, &value)
    .map_err(|error| invalid(path, source, key, error.to_string()))
}

fn timeouts(path: &Path, source: &str, raw: RawTimeouts) -> Result<Timeouts, ConfigError> {
  Ok(Timeouts {
    run: duration(
      path,
      source,
      "timeouts.run",
      raw.run.as_deref().unwrap_or("5m"),
    )?,
    build: duration(
      path,
      source,
      "timeouts.build",
      raw.build.as_deref().unwrap_or("15m"),
    )?,
    launch: duration(
      path,
      source,
      "timeouts.launch",
      raw.launch.as_deref().unwrap_or("90s"),
    )?,
    baseline_download: duration(
      path,
      source,
      "timeouts.baseline_download",
      raw.baseline_download.as_deref().unwrap_or("2m"),
    )?,
    simulator_boot: duration(
      path,
      source,
      "timeouts.simulator_boot",
      raw.simulator_boot.as_deref().unwrap_or("5m"),
    )?,
  })
}

fn defaults(
  path: &Path,
  source: &str,
  raw: RawDefaults,
  timeouts: Timeouts,
) -> Result<Defaults, ConfigError> {
  let step_timeout = duration(
    path,
    source,
    "defaults.step_timeout",
    raw.step_timeout.as_deref().unwrap_or("2s"),
  )?;
  let scenario_timeout = duration(
    path,
    source,
    "defaults.scenario_timeout",
    raw.scenario_timeout.as_deref().unwrap_or("10s"),
  )?;
  if scenario_timeout > timeouts.run {
    return Err(invalid(
      path,
      source,
      "defaults.scenario_timeout",
      "scenario timeout may not exceed the run timeout",
    ));
  }
  if step_timeout > scenario_timeout {
    return Err(invalid(
      path,
      source,
      "defaults.step_timeout",
      "step timeout may not exceed the scenario timeout",
    ));
  }
  Ok(Defaults {
    step_timeout,
    scenario_timeout,
    motion: motion(raw.motion.unwrap_or(RawMotion::Instant)),
    comparison: comparison(path, source, None, raw.comparison)?,
  })
}

fn aliases(
  path: &Path,
  source: &str,
  raw: BTreeMap<String, String>,
) -> Result<BTreeMap<String, Uuid>, ConfigError> {
  raw
    .into_iter()
    .map(|(alias, value)| {
      identifier(path, source, &format!("aliases.{alias}"), &alias)?;
      if Uuid::parse_str(&alias).is_ok() {
        return Err(invalid(
          path,
          source,
          format!("aliases.{alias}"),
          "alias must not look like a UUID",
        ));
      }
      let uuid = Uuid::parse_str(&value).map_err(|_| {
        invalid(
          path,
          source,
          format!("aliases.{alias}"),
          "alias value must be a UUID",
        )
      })?;
      if uuid.hyphenated().to_string() != value {
        return Err(invalid(
          path,
          source,
          format!("aliases.{alias}"),
          "alias UUID must use canonical lowercase hyphenated form",
        ));
      }
      Ok((alias, uuid))
    })
    .collect()
}

fn inherited_defaults(
  path: &Path,
  source: &str,
  raw: RawDefaults,
  inherited: &Defaults,
) -> Result<Defaults, ConfigError> {
  let step_timeout = raw.step_timeout.as_deref().map_or_else(
    || Ok(inherited.step_timeout),
    |value| duration(path, source, "defaults.step_timeout", value),
  )?;
  let scenario_timeout = raw.scenario_timeout.as_deref().map_or_else(
    || Ok(inherited.scenario_timeout),
    |value| duration(path, source, "defaults.scenario_timeout", value),
  )?;
  if step_timeout > scenario_timeout {
    return Err(invalid(
      path,
      source,
      "defaults.step_timeout",
      "step timeout may not exceed the scenario timeout",
    ));
  }
  Ok(Defaults {
    step_timeout,
    scenario_timeout,
    motion: raw.motion.map_or(inherited.motion, motion),
    comparison: comparison(path, source, Some(&inherited.comparison), raw.comparison)?,
  })
}

fn baseline(
  path: &Path,
  source: &str,
  directory: &std::path::Path,
  raw: Option<RawBaseline>,
) -> Result<Option<Baseline>, ConfigError> {
  raw
    .map(|baseline| match baseline {
      RawBaseline::Filesystem { namespace, root } => {
        namespace_value(path, source, &namespace)?;
        let joined = if root.is_absolute() {
          root
        } else {
          directory.join(root)
        };
        let root = resolve_nearest(&joined)
          .map_err(|error| invalid(path, source, "baseline.root", error.to_string()))?;
        Ok(Baseline::Filesystem { namespace, root })
      }
      RawBaseline::R2 {
        namespace,
        public_base_url,
        account_id_env,
        bucket_env,
        access_key_id_env,
        secret_access_key_env,
      } => {
        namespace_value(path, source, &namespace)?;
        if !public_base_url.starts_with("https://") || public_base_url.contains(char::is_whitespace)
        {
          return Err(invalid(
            path,
            source,
            "baseline.public_base_url",
            "R2 public base URL must be an HTTPS URL",
          ));
        }
        for (key, value) in [
          ("account_id_env", &account_id_env),
          ("bucket_env", &bucket_env),
          ("access_key_id_env", &access_key_id_env),
          ("secret_access_key_env", &secret_access_key_env),
        ] {
          environment_name(path, source, &format!("baseline.{key}"), value)?;
        }
        Ok(Baseline::R2 {
          namespace,
          public_base_url,
          account_id_env,
          bucket_env,
          access_key_id_env,
          secret_access_key_env,
        })
      }
    })
    .transpose()
}

fn profiles(
  path: &Path,
  source: &str,
  raw: BTreeMap<String, RawProfile>,
) -> Result<BTreeMap<String, Profile>, ConfigError> {
  if raw.is_empty() {
    return Err(invalid(
      path,
      source,
      "profiles",
      "at least one profile is required",
    ));
  }
  raw
    .into_iter()
    .map(|(name_value, profile)| {
      name(path, source, &format!("profiles.{name_value}"), &name_value)?;
      let key = format!("profiles.{name_value}");
      let RawProfile {
        target,
        display,
        headless_command,
        device,
        orientation: profile_orientation,
      } = profile;
      let wrong_fields = match target {
        RawTarget::Macos => {
          headless_command.is_some() || device.is_some() || profile_orientation.is_some()
        }
        RawTarget::Webgl => device.is_some() || profile_orientation.is_some(),
        RawTarget::IosSimulator => display.is_some() || headless_command.is_some(),
      };
      if wrong_fields {
        return Err(invalid(
          path,
          source,
          &key,
          "profile contains fields for a different target",
        ));
      }
      let resolved = match target {
        RawTarget::Macos => Profile::Macos {
          display: require_display(path, source, &key, display)?,
        },
        RawTarget::Webgl => {
          if let Some(command) = &headless_command {
            if command.is_empty()
              || command.iter().filter(|arg| arg.as_str() == "{url}").count() != 1
            {
              return Err(invalid(
                path,
                source,
                format!("{key}.headless_command"),
                "headless command must contain exactly one `{url}` argument",
              ));
            }
          }
          Profile::Webgl {
            display: require_display(path, source, &key, display)?,
            headless_command,
          }
        }
        RawTarget::IosSimulator => Profile::IosSimulator {
          device: device
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
              invalid(
                path,
                source,
                format!("{key}.device"),
                "iOS profile requires a device",
              )
            })?,
          orientation: orientation(profile_orientation.ok_or_else(|| {
            invalid(
              path,
              source,
              format!("{key}.orientation"),
              "iOS profile requires an orientation",
            )
          })?),
        },
      };
      Ok((name_value, resolved))
    })
    .collect()
}

pub(super) fn comparison(
  path: &Path,
  source: &str,
  inherited: Option<&Comparison>,
  raw: RawComparison,
) -> Result<Comparison, ConfigError> {
  let default_threshold = ExactDecimal::parse("0.1", "0"..="1").expect("valid default");
  let default_percent = ExactDecimal::parse("0.01", "0"..="100").expect("valid default");
  Ok(Comparison {
    threshold: decimal(
      path,
      source,
      "comparison.threshold",
      raw.threshold,
      "0"..="1",
    )?
    .or_else(|| inherited.map(|value| value.threshold.clone()))
    .unwrap_or(default_threshold),
    anti_alias: raw
      .anti_alias
      .or_else(|| inherited.map(|value| value.anti_alias))
      .unwrap_or(true),
    max_changed_percent: decimal(
      path,
      source,
      "comparison.max_changed_percent",
      raw.max_changed_percent,
      "0"..="100",
    )?
    .or_else(|| inherited.map(|value| value.max_changed_percent.clone()))
    .unwrap_or(default_percent),
  })
}

fn decimal(
  path: &Path,
  source: &str,
  key: &str,
  raw: Option<RawDecimal>,
  range: std::ops::RangeInclusive<&str>,
) -> Result<Option<ExactDecimal>, ConfigError> {
  raw
    .map(|value| {
      let span = value.span();
      ExactDecimal::parse(source.get(span).unwrap_or_default().trim(), range)
        .map_err(|error| invalid(path, source, key, error.to_string()))
    })
    .transpose()
}

pub(super) fn duration(
  path: &Path,
  source: &str,
  key: &str,
  value: &str,
) -> Result<DurationValue, ConfigError> {
  let duration =
    DurationValue::parse(value).map_err(|error| invalid(path, source, key, error.to_string()))?;
  if duration.as_millis() > MAX_DURATION_MS {
    return Err(invalid(
      path,
      source,
      key,
      "duration may not exceed one hour",
    ));
  }
  Ok(duration)
}

pub(super) fn name(path: &Path, source: &str, key: &str, value: &str) -> Result<(), ConfigError> {
  if value.trim().is_empty() || value.len() > MAX_NAME_BYTES {
    return Err(invalid(
      path,
      source,
      key,
      "name must contain 1 through 128 UTF-8 bytes",
    ));
  }
  Ok(())
}

pub(super) fn motion(value: RawMotion) -> Motion {
  match value {
    RawMotion::Instant => Motion::Instant,
    RawMotion::Controlled => Motion::Controlled,
    RawMotion::RealTime => Motion::RealTime,
  }
}

fn orientation(value: RawOrientation) -> Orientation {
  match value {
    RawOrientation::Portrait => Orientation::Portrait,
    RawOrientation::PortraitUpsideDown => Orientation::PortraitUpsideDown,
    RawOrientation::LandscapeLeft => Orientation::LandscapeLeft,
    RawOrientation::LandscapeRight => Orientation::LandscapeRight,
  }
}

fn require_display(
  path: &Path,
  source: &str,
  key: &str,
  raw: Option<crate::config::raw::RawDisplay>,
) -> Result<Display, ConfigError> {
  let display = raw.ok_or_else(|| {
    invalid(
      path,
      source,
      format!("{key}.display"),
      "profile requires a display",
    )
  })?;
  let invalid_dimensions = display.width == 0 || display.height == 0;
  let invalid_scale = !display.scale.is_finite() || display.scale <= 0.0;
  if invalid_dimensions || invalid_scale {
    return Err(invalid(
      path,
      source,
      format!("{key}.display"),
      "display dimensions and scale must be finite and positive",
    ));
  }
  Ok(Display {
    width: display.width,
    height: display.height,
    scale: display.scale,
  })
}

fn identifier(path: &Path, source: &str, key: &str, value: &str) -> Result<(), ConfigError> {
  let mut bytes = value.bytes();
  let valid_start = bytes
    .next()
    .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_');
  let valid_tail = bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-');
  if !valid_start || !valid_tail {
    return Err(invalid(
      path,
      source,
      key,
      "alias must be a readable identifier",
    ));
  }
  name(path, source, key, value)
}

fn environment_name(path: &Path, source: &str, key: &str, value: &str) -> Result<(), ConfigError> {
  let mut bytes = value.bytes();
  let valid_start = bytes
    .next()
    .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_');
  if !valid_start || !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') {
    return Err(invalid(
      path,
      source,
      key,
      "environment variable name is invalid",
    ));
  }
  name(path, source, key, value)
}

fn namespace_value(path: &Path, source: &str, value: &str) -> Result<(), ConfigError> {
  if value.len() > MAX_NAME_BYTES || value.split('/').any(invalid_namespace_segment) {
    return Err(invalid(
      path,
      source,
      "baseline.namespace",
      "namespace contains an invalid segment",
    ));
  }
  Ok(())
}

fn invalid_namespace_segment(segment: &str) -> bool {
  if segment.is_empty() || segment == "." || segment == ".." {
    return true;
  }
  !segment
    .bytes()
    .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}

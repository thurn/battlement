use std::collections::BTreeSet;

use anyhow::{Result, ensure};
use uuid::Uuid;

use crate::wire::job::{
  Capability, Comparison, Display, InputTarget, Job, KeyAction, Motion, ObjectCondition, Platform,
  ResolvedScenario, ResolvedStep, StepKind, VideoStep, WaitStep,
};

pub(super) fn validate_job(job: &Job) -> Result<()> {
  identifier("job_id", &job.job_id)?;
  identifier("run_id", &job.run_id)?;
  ensure!(
    job.remaining_run_timeout_ms > 0 && job.remaining_run_timeout_ms <= 3_600_000,
    "remaining_run_timeout_ms must be from 1 through 3600000"
  );
  validate_redactions(&job.log_redactions)?;
  validate_profile(job)?;
  ensure!(
    job.scenarios.len() <= 128,
    "job may contain at most 128 scenarios"
  );
  let mut scenario_ids = BTreeSet::new();
  let mut scenario_names = BTreeSet::new();
  let mut previous_index = None;
  for scenario in &job.scenarios {
    identifier("scenario.id", &scenario.id)?;
    ensure!(
      scenario_ids.insert(&scenario.id),
      "scenario IDs must be unique"
    );
    name("scenario.name", &scenario.name)?;
    ensure!(
      scenario_names.insert(&scenario.name),
      "scenario names must be unique"
    );
    if let Some(index) = previous_index {
      ensure!(
        scenario.run_index > index,
        "scenario run_index values must increase"
      );
    }
    previous_index = Some(scenario.run_index);
    validate_scenario(job, scenario)?;
  }
  Ok(())
}

fn validate_redactions(redactions: &[String]) -> Result<()> {
  ensure!(
    redactions.len() <= 128,
    "log_redactions may contain at most 128 values"
  );
  let mut unique = BTreeSet::new();
  for value in redactions {
    ensure!(!value.is_empty(), "log redactions must not be empty");
    ensure!(
      value.len() <= 4096,
      "log redactions may contain at most 4096 UTF-8 bytes"
    );
    ensure!(unique.insert(value), "log redactions must be unique");
  }
  Ok(())
}

fn validate_profile(job: &Job) -> Result<()> {
  name("profile.name", &job.profile.name)?;
  sha256("profile.build_fingerprint", &job.profile.build_fingerprint)?;
  sha256(
    "profile.source_fingerprint",
    &job.profile.source_fingerprint,
  )?;
  display(job.profile.platform, &job.profile.display)?;
  let unique: BTreeSet<Capability> = job.profile.capabilities.iter().copied().collect();
  ensure!(
    unique.len() == job.profile.capabilities.len(),
    "profile capabilities must be unique"
  );
  let unsupported = match job.profile.platform {
    Platform::Macos => None,
    Platform::Webgl => Some(Capability::Video),
    Platform::IosSimulator => Some(Capability::Hover),
  };
  ensure!(
    unsupported.is_none_or(|capability| !unique.contains(&capability)),
    "profile contains a capability unsupported by its platform"
  );
  Ok(())
}

fn display(platform: Platform, display: &Display) -> Result<()> {
  ensure!(
    display.width > 0 && display.height > 0,
    "display dimensions must be positive"
  );
  ensure!(
    display.scale.is_finite() && display.scale > 0.0,
    "display scale must be finite and positive"
  );
  let [x, y, width, height] = display.safe_area;
  let inside_width = x
    .checked_add(width)
    .is_some_and(|edge| edge <= display.width);
  let inside_height = y
    .checked_add(height)
    .is_some_and(|edge| edge <= display.height);
  ensure!(
    width > 0 && height > 0,
    "display safe area must be nonempty"
  );
  ensure!(
    inside_width && inside_height,
    "display safe area must fit inside the framebuffer"
  );
  match platform {
    Platform::IosSimulator => {
      ensure!(
        display.orientation.is_some(),
        "iOS Simulator display requires an orientation"
      );
      Ok(())
    }
    Platform::Macos | Platform::Webgl => {
      ensure!(
        display.orientation.is_none(),
        "desktop display must not have an orientation"
      );
      ensure!(
        display.safe_area == [0, 0, display.width, display.height],
        "desktop safe area must equal the framebuffer"
      );
      Ok(())
    }
  }
}

fn validate_scenario(job: &Job, scenario: &ResolvedScenario) -> Result<()> {
  ensure!(scenario.timeout_ms > 0, "scenario timeout must be positive");
  ensure!(
    scenario.timeout_ms <= job.remaining_run_timeout_ms,
    "scenario timeout may not exceed the remaining run timeout"
  );
  ensure!(
    !scenario.steps.is_empty() && scenario.steps.len() <= 128,
    "scenario must contain 1 through 128 steps"
  );
  let mut state = ScenarioState::default();
  for (index, step) in scenario.steps.iter().enumerate() {
    ensure!(
      step.index == index as u32,
      "step indices must match authored order"
    );
    validate_step(job, scenario, step, &mut state)?;
  }
  ensure!(
    state.held_keys.is_empty(),
    "keys must be released before the scenario ends"
  );
  ensure!(
    state.active_video.is_none(),
    "video start must have a matching stop"
  );
  Ok(())
}

#[derive(Default)]
struct ScenarioState<'a> {
  names: BTreeSet<&'a String>,
  checkpoints: BTreeSet<&'a String>,
  videos: BTreeSet<&'a String>,
  held_keys: BTreeSet<&'a String>,
  active_video: Option<&'a String>,
}

fn validate_step<'a>(
  job: &Job,
  scenario: &ResolvedScenario,
  step: &'a ResolvedStep,
  state: &mut ScenarioState<'a>,
) -> Result<()> {
  ensure!(step.timeout_ms > 0, "step timeout must be positive");
  ensure!(
    step.timeout_ms <= scenario.timeout_ms,
    "step timeout may not exceed the scenario timeout"
  );
  if let Some(step_name) = &step.name {
    name("step.name", step_name)?;
    ensure!(
      state.names.insert(step_name),
      "step names must be unique within a scenario"
    );
  }
  match &step.action {
    StepKind::Click { target } => {
      capability(job, Capability::Click)?;
      input_target(target)
    }
    StepKind::Hover { target } => {
      capability(job, Capability::Hover)?;
      input_target(target)
    }
    StepKind::Drag { from, to } => {
      capability(job, Capability::Drag)?;
      input_target(from)?;
      input_target(to)
    }
    StepKind::Key { key, action } => {
      capability(job, Capability::Key)?;
      key_step(key, *action, state)
    }
    StepKind::Wait(wait) => wait_step(scenario.motion, wait),
    StepKind::Assert(condition) => object_condition(condition),
    StepKind::Screenshot(screenshot) => {
      capability(job, Capability::Png)?;
      name("screenshot.name", &screenshot.name)?;
      ensure!(
        state.checkpoints.insert(&screenshot.name),
        "screenshot names must be unique within a scenario"
      );
      comparison(&screenshot.comparison)
    }
    StepKind::Video(video) => {
      capability(job, Capability::Video)?;
      video_step(video, state)
    }
  }
}

fn capability(job: &Job, required: Capability) -> Result<()> {
  ensure!(
    job.profile.capabilities.contains(&required),
    "step requires unsupported capability {required:?}"
  );
  Ok(())
}

fn input_target(target: &InputTarget) -> Result<()> {
  match target {
    InputTarget::Object(value) => identifier("input target", value),
    InputTarget::Coordinates(coordinates) => {
      ensure!(
        coordinates
          .iter()
          .all(|value| value.is_finite() && (0.0..=1.0).contains(value)),
        "input coordinates must be finite and from 0.0 through 1.0"
      );
      Ok(())
    }
  }
}

fn object_condition(condition: &ObjectCondition) -> Result<()> {
  identifier("object condition", &condition.object)
}

fn wait_step(motion: Motion, wait: &WaitStep) -> Result<()> {
  match wait {
    WaitStep::Frames(wait) => {
      ensure!(wait.frames > 0, "frame wait must be positive");
      ensure!(
        motion == Motion::Controlled,
        "frame wait requires controlled motion"
      );
      Ok(())
    }
    WaitStep::Object(condition) => object_condition(condition),
  }
}

fn key_step<'a>(key: &'a String, action: KeyAction, state: &mut ScenarioState<'a>) -> Result<()> {
  ensure!(
    !key.is_empty() && key.len() <= 128 && key.bytes().all(|byte| byte.is_ascii_alphanumeric()),
    "key must be a Unity Input System Key enum name"
  );
  match action {
    KeyAction::Down => ensure!(state.held_keys.insert(key), "key is already held"),
    KeyAction::Up => ensure!(state.held_keys.remove(key), "key is not held"),
    KeyAction::Tap => ensure!(!state.held_keys.contains(key), "key is already held"),
  };
  Ok(())
}

fn comparison(comparison: &Comparison) -> Result<()> {
  decimal("comparison.threshold", &comparison.threshold, "1")?;
  decimal(
    "comparison.max_changed_percent",
    &comparison.max_changed_percent,
    "100",
  )
}

fn video_step<'a>(video: &'a VideoStep, state: &mut ScenarioState<'a>) -> Result<()> {
  match video {
    VideoStep::Start {
      name: video_name,
      motion,
      max_duration_ms,
    } => {
      name("video.name", video_name)?;
      ensure!(state.active_video.is_none(), "videos may not overlap");
      ensure!(
        state.videos.insert(video_name),
        "video names must be unique within a scenario"
      );
      ensure!(
        *motion != Motion::Instant,
        "video motion must not be instant"
      );
      ensure!(
        *max_duration_ms > 0 && *max_duration_ms <= 30_000,
        "video duration must be from 1 through 30000 milliseconds"
      );
      state.active_video = Some(video_name);
      Ok(())
    }
    VideoStep::Stop => {
      ensure!(
        state.active_video.take().is_some(),
        "video stop has no matching start"
      );
      Ok(())
    }
  }
}

fn name(field: &str, value: &str) -> Result<()> {
  ensure!(!value.is_empty(), "{field} must not be empty");
  ensure!(
    value.len() <= 128,
    "{field} may contain at most 128 UTF-8 bytes"
  );
  Ok(())
}

fn identifier(field: &str, value: &str) -> Result<()> {
  let parsed = Uuid::parse_str(value).map_err(|_| anyhow::anyhow!("{field} must be a UUID"))?;
  ensure!(!parsed.is_nil(), "{field} must not be nil");
  ensure!(
    parsed.to_string() == value,
    "{field} must use canonical lowercase UUID text"
  );
  Ok(())
}

fn sha256(field: &str, value: &str) -> Result<()> {
  ensure!(
    value.len() == 64
      && value
        .bytes()
        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
    "{field} must contain exactly 64 lowercase hexadecimal digits"
  );
  Ok(())
}

fn decimal(field: &str, value: &str, maximum: &str) -> Result<()> {
  ensure!(
    !value.is_empty(),
    "{field} must be an unsigned base-10 decimal"
  );
  ensure!(
    !value.starts_with(['+', '-']) && !value.contains(['e', 'E']),
    "{field} must be an unsigned base-10 decimal without an exponent"
  );
  let mut parts = value.split('.');
  let integer = parts.next().expect("split always returns one item");
  let fraction = parts.next();
  let integer_valid = !integer.is_empty() && integer.bytes().all(|byte| byte.is_ascii_digit());
  let fraction_valid = fraction
    .is_none_or(|digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()));
  ensure!(
    parts.next().is_none() && integer_valid && fraction_valid,
    "{field} must contain digits with at most one decimal point"
  );
  ensure!(
    integer.len() == 1 || !integer.starts_with('0'),
    "{field} must not contain a redundant leading zero"
  );
  let at_maximum =
    integer == maximum && fraction.is_none_or(|digits| digits.bytes().all(|byte| byte == b'0'));
  let within_range = integer.len() < maximum.len()
    || integer.len() == maximum.len() && integer < maximum
    || at_maximum;
  ensure!(within_range, "{field} must be from 0 through {maximum}");
  Ok(())
}

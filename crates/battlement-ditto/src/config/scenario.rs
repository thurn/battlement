use std::collections::BTreeSet;

use uuid::Uuid;

use crate::config::{
  diagnostic::{ConfigError, invalid},
  model::{
    InputTarget, KeyAction, Motion, ObjectCondition, ObjectState, Scenario, ScreenshotStep, Step,
    StepKind, VideoStep, WaitStep,
  },
  raw::{
    RawComparison, RawCondition, RawInputTarget, RawKeyAction, RawMotion, RawObjectState,
    RawScenario, RawStep, RawVideo, RawVideoAction, RawWait,
  },
  validate::{Validation, comparison, duration, motion, name},
  value::DurationValue,
};

pub(super) fn validate(
  validation: &Validation<'_>,
  scenario_index: usize,
  raw: RawScenario,
) -> Result<Scenario, ConfigError> {
  let key = format!("scenarios.{scenario_index}");
  if let Some(fixture) = &raw.fixture {
    name(
      validation.path,
      validation.source,
      &format!("{key}.fixture"),
      fixture,
    )?;
  }
  if raw.steps.is_empty() || raw.steps.len() > 128 {
    return Err(invalid(
      validation.path,
      validation.source,
      format!("{key}.steps"),
      "scenario must contain 1 through 128 steps",
    ));
  }
  let scenario_motion = raw.motion.map_or(validation.defaults.motion, motion);
  let timeout = raw.timeout.as_deref().map_or_else(
    || Ok(validation.defaults.scenario_timeout),
    |value| {
      duration(
        validation.path,
        validation.source,
        &format!("{key}.timeout"),
        value,
      )
    },
  )?;
  if timeout > validation.run_timeout {
    return Err(invalid(
      validation.path,
      validation.source,
      format!("{key}.timeout"),
      "scenario timeout may not exceed the run timeout",
    ));
  }
  let mut state = State::default();
  let steps = raw
    .steps
    .into_iter()
    .enumerate()
    .map(|(index, step)| {
      step_value(
        validation,
        &key,
        index,
        scenario_motion,
        timeout,
        step,
        &mut state,
      )
    })
    .collect::<Result<Vec<_>, _>>()?;
  if let Some(video) = state.active_video {
    return Err(invalid(
      validation.path,
      validation.source,
      format!("{key}.steps.video"),
      format!("video {video:?} has no stop step"),
    ));
  }
  if !state.held_keys.is_empty() {
    return Err(invalid(
      validation.path,
      validation.source,
      format!("{key}.steps.key"),
      format!(
        "keys remain held at scenario end: {}",
        join(&state.held_keys)
      ),
    ));
  }
  Ok(Scenario {
    name: raw.name,
    fixture: raw.fixture,
    motion: scenario_motion,
    timeout,
    steps,
  })
}

#[derive(Default)]
struct State {
  step_names: BTreeSet<String>,
  checkpoints: BTreeSet<String>,
  videos: BTreeSet<String>,
  held_keys: BTreeSet<String>,
  active_video: Option<String>,
}

fn step_value(
  validation: &Validation<'_>,
  scenario_key: &str,
  index: usize,
  scenario_motion: Motion,
  scenario_timeout: DurationValue,
  mut raw: RawStep,
  state: &mut State,
) -> Result<Step, ConfigError> {
  let key = format!("{scenario_key}.steps.{index}");
  if let Some(step_name) = &raw.name {
    name(
      validation.path,
      validation.source,
      &format!("{key}.name"),
      step_name,
    )?;
    if !state.step_names.insert(step_name.clone()) {
      return Err(invalid(
        validation.path,
        validation.source,
        format!("{key}.name"),
        format!("duplicate step name {step_name:?}"),
      ));
    }
  }
  let timeout = raw.timeout.as_deref().map_or_else(
    || Ok(validation.defaults.step_timeout),
    |value| {
      duration(
        validation.path,
        validation.source,
        &format!("{key}.timeout"),
        value,
      )
    },
  )?;
  if timeout > scenario_timeout {
    return Err(invalid(
      validation.path,
      validation.source,
      format!("{key}.timeout"),
      "step timeout may not exceed the scenario timeout",
    ));
  }
  let action_count = [
    raw.click.is_some(),
    raw.hover.is_some(),
    raw.drag.is_some(),
    raw.key.is_some(),
    raw.wait.is_some(),
    raw.assertion.is_some(),
    raw.screenshot.is_some(),
    raw.video.is_some(),
  ]
  .into_iter()
  .filter(|present| *present)
  .count();
  if action_count != 1 {
    return Err(invalid(
      validation.path,
      validation.source,
      &key,
      "step must contain exactly one action",
    ));
  }
  let action = if let Some(click) = raw.click.take() {
    StepKind::Click {
      target: input_target(validation, &format!("{key}.click.target"), click.target)?,
      settle: click.settle,
    }
  } else if let Some(hover) = raw.hover.take() {
    StepKind::Hover {
      target: input_target(validation, &format!("{key}.hover.target"), hover.target)?,
    }
  } else if let Some(drag) = raw.drag.take() {
    StepKind::Drag {
      from: input_target(validation, &format!("{key}.drag.from"), drag.from)?,
      to: input_target(validation, &format!("{key}.drag.to"), drag.to)?,
    }
  } else if let Some(key_step) = raw.key.take() {
    key_step_value(validation, &key, key_step.key, key_step.action, state)?
  } else if let Some(wait) = raw.wait.take() {
    StepKind::Wait(wait_step(validation, &key, scenario_motion, wait)?)
  } else if let Some(assertion) = raw.assertion.take() {
    StepKind::Assert(condition(validation, &format!("{key}.assert"), assertion)?)
  } else if let Some(screenshot) = raw.screenshot.take() {
    name(
      validation.path,
      validation.source,
      &format!("{key}.screenshot.name"),
      &screenshot.name,
    )?;
    if !state.checkpoints.insert(screenshot.name.clone()) {
      return Err(invalid(
        validation.path,
        validation.source,
        format!("{key}.screenshot.name"),
        format!("duplicate screenshot checkpoint {:?}", screenshot.name),
      ));
    }
    StepKind::Screenshot(ScreenshotStep {
      name: screenshot.name,
      comparison: comparison(
        validation.path,
        validation.source,
        Some(&validation.defaults.comparison),
        RawComparison {
          threshold: screenshot.threshold,
          anti_alias: screenshot.anti_alias,
          max_changed_percent: screenshot.max_changed_percent,
        },
      )?,
    })
  } else {
    StepKind::Video(video_step(
      validation,
      &key,
      raw.video.take().expect("one action is present"),
      state,
    )?)
  };
  Ok(Step {
    name: raw.name,
    timeout,
    action,
  })
}

fn input_target(
  validation: &Validation<'_>,
  key: &str,
  raw: RawInputTarget,
) -> Result<InputTarget, ConfigError> {
  match raw {
    RawInputTarget::Object(value) => {
      object_reference(validation, key, &value)?;
      Ok(InputTarget::Object(value))
    }
    RawInputTarget::Coordinates(coordinates) => {
      if coordinates
        .iter()
        .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
      {
        return Err(invalid(
          validation.path,
          validation.source,
          key,
          "input coordinates must be finite and from 0.0 through 1.0",
        ));
      }
      Ok(InputTarget::Coordinates(coordinates))
    }
  }
}

fn key_step_value(
  validation: &Validation<'_>,
  key: &str,
  value: String,
  raw_action: Option<RawKeyAction>,
  state: &mut State,
) -> Result<StepKind, ConfigError> {
  if value.is_empty()
    || value.len() > 128
    || !value.bytes().all(|byte| byte.is_ascii_alphanumeric())
  {
    return Err(invalid(
      validation.path,
      validation.source,
      format!("{key}.key.key"),
      "key must be a Unity Input System Key enum name",
    ));
  }
  let action = match raw_action.unwrap_or(RawKeyAction::Tap) {
    RawKeyAction::Down => {
      if !state.held_keys.insert(value.clone()) {
        return Err(key_state_error(validation, key, &value, "is already held"));
      }
      KeyAction::Down
    }
    RawKeyAction::Up => {
      if !state.held_keys.remove(&value) {
        return Err(key_state_error(validation, key, &value, "is not held"));
      }
      KeyAction::Up
    }
    RawKeyAction::Tap => {
      if state.held_keys.contains(&value) {
        return Err(key_state_error(validation, key, &value, "is already held"));
      }
      KeyAction::Tap
    }
  };
  Ok(StepKind::Key { key: value, action })
}

fn wait_step(
  validation: &Validation<'_>,
  key: &str,
  scenario_motion: Motion,
  raw: RawWait,
) -> Result<WaitStep, ConfigError> {
  match (raw.frames, raw.object, raw.state) {
    (Some(frames), None, None) if frames > 0 && scenario_motion == Motion::Controlled => {
      Ok(WaitStep::Frames(frames))
    }
    (Some(0), None, None) => Err(invalid(
      validation.path,
      validation.source,
      format!("{key}.wait.frames"),
      "frame wait must be positive",
    )),
    (Some(_), None, None) => Err(invalid(
      validation.path,
      validation.source,
      format!("{key}.wait.frames"),
      "frame wait requires controlled scenario motion",
    )),
    (None, Some(object), Some(state)) => Ok(WaitStep::Object(condition(
      validation,
      &format!("{key}.wait"),
      RawCondition { object, state },
    )?)),
    _ => Err(invalid(
      validation.path,
      validation.source,
      format!("{key}.wait"),
      "wait requires exactly frames or object with state",
    )),
  }
}

fn condition(
  validation: &Validation<'_>,
  key: &str,
  raw: RawCondition,
) -> Result<ObjectCondition, ConfigError> {
  object_reference(validation, &format!("{key}.object"), &raw.object)?;
  Ok(ObjectCondition {
    object: raw.object,
    state: match raw.state {
      RawObjectState::Exists => ObjectState::Exists,
      RawObjectState::Absent => ObjectState::Absent,
      RawObjectState::Visible => ObjectState::Visible,
      RawObjectState::Hidden => ObjectState::Hidden,
      RawObjectState::Enabled => ObjectState::Enabled,
      RawObjectState::Disabled => ObjectState::Disabled,
    },
  })
}

fn video_step(
  validation: &Validation<'_>,
  key: &str,
  raw: RawVideo,
  state: &mut State,
) -> Result<VideoStep, ConfigError> {
  match raw.action {
    RawVideoAction::Start => {
      if state.active_video.is_some() {
        return Err(invalid(
          validation.path,
          validation.source,
          format!("{key}.video"),
          "videos may not overlap",
        ));
      }
      let video_name = raw.name.ok_or_else(|| {
        invalid(
          validation.path,
          validation.source,
          format!("{key}.video.name"),
          "video start requires a name",
        )
      })?;
      name(
        validation.path,
        validation.source,
        &format!("{key}.video.name"),
        &video_name,
      )?;
      if !state.videos.insert(video_name.clone()) {
        return Err(invalid(
          validation.path,
          validation.source,
          format!("{key}.video.name"),
          format!("duplicate video name {video_name:?}"),
        ));
      }
      let video_motion = motion(raw.motion.unwrap_or(RawMotion::RealTime));
      if video_motion == Motion::Instant {
        return Err(invalid(
          validation.path,
          validation.source,
          format!("{key}.video.motion"),
          "video motion must be controlled or real-time",
        ));
      }
      let max_duration = raw.max_duration.as_deref().map_or_else(
        || Ok(DurationValue::from_millis(30_000)),
        |value| {
          duration(
            validation.path,
            validation.source,
            &format!("{key}.video.max_duration"),
            value,
          )
        },
      )?;
      if max_duration.as_millis() > 30_000 {
        return Err(invalid(
          validation.path,
          validation.source,
          format!("{key}.video.max_duration"),
          "video duration may not exceed 30 seconds",
        ));
      }
      state.active_video = Some(video_name.clone());
      Ok(VideoStep::Start {
        name: video_name,
        motion: video_motion,
        max_duration,
      })
    }
    RawVideoAction::Stop => {
      if raw.name.is_some() || raw.motion.is_some() || raw.max_duration.is_some() {
        return Err(invalid(
          validation.path,
          validation.source,
          format!("{key}.video"),
          "video stop accepts no fields other than action",
        ));
      }
      state.active_video.take().ok_or_else(|| {
        invalid(
          validation.path,
          validation.source,
          format!("{key}.video"),
          "video stop has no matching start",
        )
      })?;
      Ok(VideoStep::Stop)
    }
  }
}

fn object_reference(
  validation: &Validation<'_>,
  key: &str,
  value: &str,
) -> Result<(), ConfigError> {
  if validation.aliases.contains_key(value) {
    return Ok(());
  }
  let uuid = Uuid::parse_str(value).map_err(|_| {
    invalid(
      validation.path,
      validation.source,
      key,
      format!("unknown alias or invalid UUID {value:?}"),
    )
  })?;
  if uuid.hyphenated().to_string() != value {
    return Err(invalid(
      validation.path,
      validation.source,
      key,
      "UUID must use canonical lowercase hyphenated form",
    ));
  }
  Ok(())
}

fn key_state_error(
  validation: &Validation<'_>,
  key: &str,
  value: &str,
  reason: &str,
) -> ConfigError {
  invalid(
    validation.path,
    validation.source,
    format!("{key}.key"),
    format!("key {value:?} {reason}"),
  )
}

fn join(values: &BTreeSet<String>) -> String {
  values.iter().cloned().collect::<Vec<_>>().join(", ")
}

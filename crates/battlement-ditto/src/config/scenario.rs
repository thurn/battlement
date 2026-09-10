use std::collections::BTreeSet;

use uuid::Uuid;

use crate::config::{
  diagnostic::{ConfigError, invalid},
  model::{
    AccessibilityAction, AccessibilityAssertion, AccessibilityRole, AccessibilityTarget,
    InputTarget, Motion, ObjectCondition, ObjectState, Scenario, ScreenshotStep, Step, StepKind,
    VideoStep,
  },
  raw::{
    RawAccessibilityAction, RawAccessibilityRole, RawAccessibilityTarget, RawComparison,
    RawCondition, RawInputTarget, RawObjectState, RawPointerAction, RawScenario, RawStep, RawVideo,
    RawVideoAction,
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
  if scenario_motion == Motion::RealTime {
    return Err(invalid(
      validation.path,
      validation.source,
      format!("{key}.motion"),
      "real-time motion violates the deterministic execution contract",
    ));
  }
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
  Ok(Scenario {
    name: raw.name,
    fixture: raw.fixture,
    motion: scenario_motion,
    timeout,
    performance: None,
    steps,
  })
}

#[derive(Default)]
struct State {
  step_names: BTreeSet<String>,
  checkpoints: BTreeSet<String>,
  videos: BTreeSet<String>,
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
    raw.advance.is_some(),
    raw.wait.is_some(),
    raw.assertion.is_some(),
    raw.accessibility_assert.is_some(),
    raw.accessibility_action.is_some(),
    raw.pointer_action.is_some(),
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
    }
  } else if let Some(hover) = raw.hover.take() {
    let _ = hover.target;
    return Err(invalid(
      validation.path,
      validation.source,
      format!("{key}.hover"),
      "hover has no deterministic semantic delivery contract",
    ));
  } else if let Some(drag) = raw.drag.take() {
    let _ = (drag.from, drag.to);
    return Err(invalid(
      validation.path,
      validation.source,
      format!("{key}.drag"),
      "drag has no deterministic semantic delivery contract",
    ));
  } else if let Some(key_step) = raw.key.take() {
    let _ = (key_step.key, key_step.action);
    return Err(invalid(
      validation.path,
      validation.source,
      format!("{key}.key"),
      "physical key input has no deterministic semantic delivery contract",
    ));
  } else if let Some(advance) = raw.advance.take() {
    StepKind::Advance {
      frames: advance_step(validation, &key, scenario_motion, advance.frames)?,
    }
  } else if let Some(wait) = raw.wait.take() {
    StepKind::Wait(condition(
      validation,
      &format!("{key}.wait"),
      RawCondition {
        object: wait.object,
        state: wait.state,
      },
    )?)
  } else if let Some(assertion) = raw.assertion.take() {
    StepKind::Assert(condition(validation, &format!("{key}.assert"), assertion)?)
  } else if let Some(assertion) = raw.accessibility_assert.take() {
    StepKind::AccessibilityAssert(accessibility_assertion(
      validation,
      &format!("{key}.accessibility_assert"),
      assertion,
    )?)
  } else if let Some(action) = raw.accessibility_action.take() {
    StepKind::AccessibilityAction {
      target: accessibility_target(validation, &key, action.target)?,
      action: accessibility_action(action.action),
    }
  } else if let Some(action) = raw.pointer_action.take() {
    if action.completion.is_some() && !matches!(action.action, RawPointerAction::Click) {
      return Err(invalid(
        validation.path,
        validation.source,
        format!("{key}.pointer_action.completion"),
        "pointer completion witnesses require a click",
      ));
    }
    StepKind::PointerAction {
      target: accessibility_target(validation, &key, action.target)?,
      action: match action.action {
        RawPointerAction::Click => crate::config::model::PointerAction::Click,
        RawPointerAction::Hover => crate::config::model::PointerAction::Hover,
      },
      visual_witness: action
        .visual_witness
        .map(|target| accessibility_target(validation, &key, target))
        .transpose()?,
      completion: action
        .completion
        .map(|assertion| {
          accessibility_assertion(
            validation,
            &format!("{key}.pointer_action.completion"),
            assertion,
          )
        })
        .transpose()?,
    }
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
  if raw.measure && !matches!(action, StepKind::PointerAction { .. }) {
    return Err(invalid(
      validation.path,
      validation.source,
      format!("{key}.measure"),
      "measure is supported only on pointer_action steps",
    ));
  }
  if raw.measure && raw.name.as_deref().is_none_or(str::is_empty) {
    return Err(invalid(
      validation.path,
      validation.source,
      format!("{key}.name"),
      "measured pointer actions require a step name",
    ));
  }
  Ok(Step {
    name: raw.name,
    timeout,
    measure: raw.measure,
    action,
  })
}

fn accessibility_assertion(
  validation: &Validation<'_>,
  key: &str,
  assertion: crate::config::raw::RawAccessibilityAssertion,
) -> Result<AccessibilityAssertion, ConfigError> {
  Ok(AccessibilityAssertion {
    target: accessibility_target(validation, key, assertion.target)?,
    role: accessibility_role(assertion.role),
    name: required_accessible_name(validation, &format!("{key}.name"), assertion.name)?,
    selected: assertion.selected,
    checked: assertion.checked,
    disabled: assertion.disabled,
    current_page: assertion.current_page,
    parent: assertion
      .parent
      .map(|target| accessibility_target(validation, key, target))
      .transpose()?,
  })
}

fn accessibility_target(
  validation: &Validation<'_>,
  key: &str,
  target: RawAccessibilityTarget,
) -> Result<AccessibilityTarget, ConfigError> {
  Ok(AccessibilityTarget {
    role: accessibility_role(target.role),
    name: required_accessible_name(validation, &format!("{key}.target.name"), target.name)?,
  })
}

fn required_accessible_name(
  validation: &Validation<'_>,
  key: &str,
  value: String,
) -> Result<String, ConfigError> {
  if value.trim().is_empty() {
    return Err(invalid(
      validation.path,
      validation.source,
      key,
      "accessible name must not be empty",
    ));
  }
  Ok(value)
}

fn accessibility_role(value: RawAccessibilityRole) -> AccessibilityRole {
  match value {
    RawAccessibilityRole::Button => AccessibilityRole::Button,
    RawAccessibilityRole::Checkbox => AccessibilityRole::Checkbox,
    RawAccessibilityRole::Switch => AccessibilityRole::Switch,
    RawAccessibilityRole::Radio => AccessibilityRole::Radio,
    RawAccessibilityRole::RadioGroup => AccessibilityRole::RadioGroup,
    RawAccessibilityRole::Slider => AccessibilityRole::Slider,
    RawAccessibilityRole::Progress => AccessibilityRole::Progress,
    RawAccessibilityRole::Disclosure => AccessibilityRole::Disclosure,
    RawAccessibilityRole::ScrollArea => AccessibilityRole::ScrollArea,
    RawAccessibilityRole::Tab => AccessibilityRole::Tab,
    RawAccessibilityRole::TabList => AccessibilityRole::TabList,
    RawAccessibilityRole::TabPanel => AccessibilityRole::TabPanel,
    RawAccessibilityRole::Dialog => AccessibilityRole::Dialog,
    RawAccessibilityRole::Heading => AccessibilityRole::Heading,
    RawAccessibilityRole::Image => AccessibilityRole::Image,
    RawAccessibilityRole::StaticText => AccessibilityRole::StaticText,
    RawAccessibilityRole::Group => AccessibilityRole::Group,
    RawAccessibilityRole::ListBox => AccessibilityRole::ListBox,
    RawAccessibilityRole::Option => AccessibilityRole::Option,
    RawAccessibilityRole::Table => AccessibilityRole::Table,
    RawAccessibilityRole::Row => AccessibilityRole::Row,
    RawAccessibilityRole::ColumnHeader => AccessibilityRole::ColumnHeader,
    RawAccessibilityRole::RowHeader => AccessibilityRole::RowHeader,
    RawAccessibilityRole::Cell => AccessibilityRole::Cell,
    RawAccessibilityRole::Link => AccessibilityRole::Link,
    RawAccessibilityRole::Navigation => AccessibilityRole::Navigation,
    RawAccessibilityRole::Region => AccessibilityRole::Region,
  }
}

fn accessibility_action(value: RawAccessibilityAction) -> AccessibilityAction {
  match value {
    RawAccessibilityAction::Activate => AccessibilityAction::Activate,
    RawAccessibilityAction::Increment => AccessibilityAction::Increment,
    RawAccessibilityAction::Decrement => AccessibilityAction::Decrement,
    RawAccessibilityAction::Dismiss => AccessibilityAction::Dismiss,
    RawAccessibilityAction::ScrollForward => AccessibilityAction::ScrollForward,
    RawAccessibilityAction::ScrollBackward => AccessibilityAction::ScrollBackward,
  }
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
      let _ = coordinates;
      Err(invalid(
        validation.path,
        validation.source,
        key,
        "coordinate input has no deterministic semantic delivery contract",
      ))
    }
  }
}

fn advance_step(
  validation: &Validation<'_>,
  key: &str,
  scenario_motion: Motion,
  frames: u32,
) -> Result<u32, ConfigError> {
  if frames == 0 {
    Err(invalid(
      validation.path,
      validation.source,
      format!("{key}.advance.frames"),
      "frame advance must be positive",
    ))
  } else if scenario_motion != Motion::Controlled {
    Err(invalid(
      validation.path,
      validation.source,
      format!("{key}.advance.frames"),
      "frame advance requires controlled scenario motion",
    ))
  } else {
    Ok(frames)
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
      let Some(raw_motion) = raw.motion else {
        return Err(invalid(
          validation.path,
          validation.source,
          format!("{key}.video.motion"),
          "video motion must explicitly select controlled execution",
        ));
      };
      let video_motion = motion(raw_motion);
      if video_motion != Motion::Controlled {
        return Err(invalid(
          validation.path,
          validation.source,
          format!("{key}.video.motion"),
          "video motion must be controlled",
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

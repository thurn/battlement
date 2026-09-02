//! Conversion from validated authoring models to one immutable player job.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use uuid::Uuid;

use crate::{
  config::model::{
    Comparison as AuthoredComparison, InputTarget as AuthoredInputTarget,
    KeyAction as AuthoredKeyAction, Motion as AuthoredMotion,
    ObjectCondition as AuthoredObjectCondition, ObjectState as AuthoredObjectState, Profile,
    Step as AuthoredStep, StepKind as AuthoredStepKind, VideoStep as AuthoredVideoStep,
    WaitStep as AuthoredWaitStep,
  },
  selection::{Disposition, Selection},
  wire::job::{
    AccessibilityAction, AccessibilityAssertion, AccessibilityRole, AccessibilityTarget,
    Capability, Command, Comparison, Display, FrameWait, InputTarget, Job, KeyAction, Motion,
    ObjectCondition, ObjectState, Platform, ResolvedProfile, ResolvedScenario, ResolvedStep,
    ScreenshotStep, StepKind, VideoStep, WaitStep,
  },
};

pub(crate) fn resolve(
  selection: &Selection,
  aliases: &BTreeMap<String, Uuid>,
  command: Command,
  run_id: &str,
  build_fingerprint: &str,
  source_fingerprint: &str,
  timeout_ms: u64,
) -> Result<Job> {
  let (platform, display, capabilities) = match &selection.profile {
    Profile::Macos { display } => (
      Platform::Macos,
      display,
      vec![
        Capability::Click,
        Capability::Hover,
        Capability::Drag,
        Capability::Key,
        Capability::Png,
        Capability::Video,
      ],
    ),
    Profile::Webgl { display, .. } => (
      Platform::Webgl,
      display,
      vec![
        Capability::Click,
        Capability::Hover,
        Capability::Drag,
        Capability::Key,
        Capability::Png,
      ],
    ),
    Profile::IosSimulator { .. } => anyhow::bail!("iOS resolution requires observed display facts"),
  };
  self::resolve_inner(
    selection,
    aliases,
    command,
    run_id,
    build_fingerprint,
    source_fingerprint,
    timeout_ms,
    platform,
    Display {
      width: display.width,
      height: display.height,
      scale: display.scale,
      orientation: None,
      safe_area: [0, 0, display.width, display.height],
    },
    capabilities,
  )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve_ios(
  selection: &Selection,
  aliases: &BTreeMap<String, Uuid>,
  command: Command,
  run_id: &str,
  build_fingerprint: &str,
  source_fingerprint: &str,
  timeout_ms: u64,
  display: Display,
) -> Result<Job> {
  ensure_ios(selection)?;
  self::resolve_inner(
    selection,
    aliases,
    command,
    run_id,
    build_fingerprint,
    source_fingerprint,
    timeout_ms,
    Platform::IosSimulator,
    display,
    vec![
      Capability::Click,
      Capability::Drag,
      Capability::Key,
      Capability::Png,
      Capability::Video,
    ],
  )
}

fn ensure_ios(selection: &Selection) -> Result<()> {
  anyhow::ensure!(
    matches!(selection.profile, Profile::IosSimulator { .. }),
    "iOS resolver received another profile"
  );
  Ok(())
}

#[allow(clippy::too_many_arguments)]
fn resolve_inner(
  selection: &Selection,
  aliases: &BTreeMap<String, Uuid>,
  command: Command,
  run_id: &str,
  build_fingerprint: &str,
  source_fingerprint: &str,
  timeout_ms: u64,
  platform: Platform,
  display: Display,
  capabilities: Vec<Capability>,
) -> Result<Job> {
  let job = Job {
    job_id: Uuid::new_v4().to_string(),
    run_id: run_id.to_owned(),
    remaining_run_timeout_ms: timeout_ms,
    log_redactions: Vec::new(),
    command,
    profile: ResolvedProfile {
      name: selection.profile_name.clone(),
      platform,
      display,
      build_fingerprint: build_fingerprint.to_owned(),
      source_fingerprint: source_fingerprint.to_owned(),
      capabilities,
    },
    scenarios: selection
      .scenarios
      .iter()
      .filter(|scenario| scenario.disposition == Disposition::Runnable)
      .map(|scenario| {
        Ok(ResolvedScenario {
          id: Uuid::new_v4().to_string(),
          run_index: scenario.run_index,
          name: scenario.scenario.name.clone(),
          fixture: scenario.scenario.fixture.clone(),
          motion: motion(scenario.scenario.motion),
          timeout_ms: scenario.scenario.timeout.as_millis(),
          steps: scenario
            .scenario
            .steps
            .iter()
            .enumerate()
            .map(|(index, step)| resolved_step(index, step, aliases))
            .collect::<Result<_>>()?,
        })
      })
      .collect::<Result<_>>()?,
  };
  job.validate()?;
  Ok(job)
}

fn resolved_step(
  index: usize,
  step: &AuthoredStep,
  aliases: &BTreeMap<String, Uuid>,
) -> Result<ResolvedStep> {
  Ok(ResolvedStep {
    index: index as u32,
    name: step.name.clone(),
    timeout_ms: step.timeout.as_millis(),
    action: match &step.action {
      AuthoredStepKind::Click { target, settle } => StepKind::Click {
        target: input_target(target, aliases)?,
        settle: *settle,
      },
      AuthoredStepKind::Hover { target } => StepKind::Hover {
        target: input_target(target, aliases)?,
      },
      AuthoredStepKind::Drag { from, to } => StepKind::Drag {
        from: input_target(from, aliases)?,
        to: input_target(to, aliases)?,
      },
      AuthoredStepKind::Key { key, action } => StepKind::Key {
        key: key.clone(),
        action: key_action(*action),
      },
      AuthoredStepKind::Wait(AuthoredWaitStep::Frames(frames)) => {
        StepKind::Wait(WaitStep::Frames(FrameWait { frames: *frames }))
      }
      AuthoredStepKind::Wait(AuthoredWaitStep::Object(condition)) => {
        StepKind::Wait(WaitStep::Object(object_condition(condition, aliases)?))
      }
      AuthoredStepKind::Assert(condition) => {
        StepKind::Assert(object_condition(condition, aliases)?)
      }
      AuthoredStepKind::AccessibilityAssert(assertion) => {
        StepKind::AccessibilityAssert(AccessibilityAssertion {
          target: accessibility_target(&assertion.target),
          role: accessibility_role(assertion.role),
          name: assertion.name.clone(),
          selected: assertion.selected,
          checked: assertion.checked,
          disabled: assertion.disabled,
          current_page: assertion.current_page,
          parent: assertion.parent.as_ref().map(accessibility_target),
        })
      }
      AuthoredStepKind::AccessibilityAction { target, action } => StepKind::AccessibilityAction {
        target: accessibility_target(target),
        action: accessibility_action(*action),
      },
      AuthoredStepKind::Screenshot(screenshot) => StepKind::Screenshot(ScreenshotStep {
        name: screenshot.name.clone(),
        comparison: comparison(&screenshot.comparison),
      }),
      AuthoredStepKind::Video(AuthoredVideoStep::Start {
        name,
        motion: value,
        max_duration,
      }) => StepKind::Video(VideoStep::Start {
        name: name.clone(),
        motion: motion(*value),
        max_duration_ms: max_duration.as_millis(),
      }),
      AuthoredStepKind::Video(AuthoredVideoStep::Stop) => StepKind::Video(VideoStep::Stop),
    },
  })
}

fn accessibility_target(target: &crate::config::model::AccessibilityTarget) -> AccessibilityTarget {
  AccessibilityTarget {
    role: accessibility_role(target.role),
    name: target.name.clone(),
  }
}

fn accessibility_role(value: crate::config::model::AccessibilityRole) -> AccessibilityRole {
  match value {
    crate::config::model::AccessibilityRole::Button => AccessibilityRole::Button,
    crate::config::model::AccessibilityRole::Checkbox => AccessibilityRole::Checkbox,
    crate::config::model::AccessibilityRole::Switch => AccessibilityRole::Switch,
    crate::config::model::AccessibilityRole::Radio => AccessibilityRole::Radio,
    crate::config::model::AccessibilityRole::RadioGroup => AccessibilityRole::RadioGroup,
    crate::config::model::AccessibilityRole::Slider => AccessibilityRole::Slider,
    crate::config::model::AccessibilityRole::Progress => AccessibilityRole::Progress,
    crate::config::model::AccessibilityRole::Disclosure => AccessibilityRole::Disclosure,
    crate::config::model::AccessibilityRole::ScrollArea => AccessibilityRole::ScrollArea,
    crate::config::model::AccessibilityRole::Tab => AccessibilityRole::Tab,
    crate::config::model::AccessibilityRole::TabList => AccessibilityRole::TabList,
    crate::config::model::AccessibilityRole::TabPanel => AccessibilityRole::TabPanel,
    crate::config::model::AccessibilityRole::Dialog => AccessibilityRole::Dialog,
    crate::config::model::AccessibilityRole::Heading => AccessibilityRole::Heading,
    crate::config::model::AccessibilityRole::Image => AccessibilityRole::Image,
    crate::config::model::AccessibilityRole::StaticText => AccessibilityRole::StaticText,
    crate::config::model::AccessibilityRole::Group => AccessibilityRole::Group,
    crate::config::model::AccessibilityRole::ListBox => AccessibilityRole::ListBox,
    crate::config::model::AccessibilityRole::Option => AccessibilityRole::Option,
    crate::config::model::AccessibilityRole::Table => AccessibilityRole::Table,
    crate::config::model::AccessibilityRole::Row => AccessibilityRole::Row,
    crate::config::model::AccessibilityRole::ColumnHeader => AccessibilityRole::ColumnHeader,
    crate::config::model::AccessibilityRole::RowHeader => AccessibilityRole::RowHeader,
    crate::config::model::AccessibilityRole::Cell => AccessibilityRole::Cell,
    crate::config::model::AccessibilityRole::Link => AccessibilityRole::Link,
    crate::config::model::AccessibilityRole::Navigation => AccessibilityRole::Navigation,
    crate::config::model::AccessibilityRole::Region => AccessibilityRole::Region,
  }
}

fn accessibility_action(value: crate::config::model::AccessibilityAction) -> AccessibilityAction {
  match value {
    crate::config::model::AccessibilityAction::Activate => AccessibilityAction::Activate,
    crate::config::model::AccessibilityAction::Increment => AccessibilityAction::Increment,
    crate::config::model::AccessibilityAction::Decrement => AccessibilityAction::Decrement,
    crate::config::model::AccessibilityAction::Dismiss => AccessibilityAction::Dismiss,
    crate::config::model::AccessibilityAction::ScrollForward => AccessibilityAction::ScrollForward,
    crate::config::model::AccessibilityAction::ScrollBackward => {
      AccessibilityAction::ScrollBackward
    }
  }
}

fn input_target(
  target: &AuthoredInputTarget,
  aliases: &BTreeMap<String, Uuid>,
) -> Result<InputTarget> {
  Ok(match target {
    AuthoredInputTarget::Object(value) => InputTarget::Object(
      Uuid::parse_str(value)
        .ok()
        .or_else(|| aliases.get(value).copied())
        .with_context(|| format!("input target {value:?} has no resolved UUID"))?
        .to_string(),
    ),
    AuthoredInputTarget::Coordinates(value) => InputTarget::Coordinates(*value),
  })
}

fn object_condition(
  condition: &AuthoredObjectCondition,
  aliases: &BTreeMap<String, Uuid>,
) -> Result<ObjectCondition> {
  let InputTarget::Object(object) = input_target(
    &AuthoredInputTarget::Object(condition.object.clone()),
    aliases,
  )?
  else {
    unreachable!()
  };
  Ok(ObjectCondition {
    object,
    state: match condition.state {
      AuthoredObjectState::Exists => ObjectState::Exists,
      AuthoredObjectState::Absent => ObjectState::Absent,
      AuthoredObjectState::Visible => ObjectState::Visible,
      AuthoredObjectState::Hidden => ObjectState::Hidden,
      AuthoredObjectState::Enabled => ObjectState::Enabled,
      AuthoredObjectState::Disabled => ObjectState::Disabled,
    },
  })
}

fn comparison(value: &AuthoredComparison) -> Comparison {
  Comparison {
    threshold: value.threshold.as_str().to_owned(),
    anti_alias: value.anti_alias,
    max_changed_percent: value.max_changed_percent.as_str().to_owned(),
  }
}

fn motion(value: AuthoredMotion) -> Motion {
  match value {
    AuthoredMotion::Instant => Motion::Instant,
    AuthoredMotion::Controlled => Motion::Controlled,
    AuthoredMotion::RealTime => Motion::RealTime,
  }
}

fn key_action(value: AuthoredKeyAction) -> KeyAction {
  match value {
    AuthoredKeyAction::Down => KeyAction::Down,
    AuthoredKeyAction::Up => KeyAction::Up,
    AuthoredKeyAction::Tap => KeyAction::Tap,
  }
}

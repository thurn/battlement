use std::{collections::BTreeMap, path::PathBuf};

use uuid::Uuid;

use crate::config::value::{DurationValue, ExactDecimal};

/// A validated authoring suite with resolved filesystem paths and defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct Suite {
  pub source: PathBuf,
  pub repository: PathBuf,
  pub name: String,
  pub default_profile: String,
  pub player: Player,
  pub timeouts: Timeouts,
  pub defaults: Defaults,
  pub aliases: BTreeMap<String, Uuid>,
  pub baseline: Option<Baseline>,
  pub profiles: BTreeMap<String, Profile>,
  pub scenarios: Vec<Scenario>,
}

/// Resolved Unity player inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Player {
  pub unity_project: PathBuf,
  pub scene: PathBuf,
  pub rust_manifest: PathBuf,
}

/// Host-level operation deadlines.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Timeouts {
  pub run: DurationValue,
  pub build: DurationValue,
  pub launch: DurationValue,
  pub baseline_download: DurationValue,
  pub simulator_boot: DurationValue,
}

/// Defaults inherited by scenarios and steps.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Defaults {
  pub step_timeout: DurationValue,
  pub scenario_timeout: DurationValue,
  pub motion: Motion,
  pub comparison: Comparison,
}

/// Exact image comparison settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Comparison {
  pub threshold: ExactDecimal,
  pub anti_alias: bool,
  pub max_changed_percent: ExactDecimal,
}

/// A configured baseline store.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Baseline {
  Filesystem {
    namespace: String,
    root: PathBuf,
  },
  R2 {
    namespace: String,
    public_base_url: String,
    account_id_env: String,
    bucket_env: String,
    access_key_id_env: String,
    secret_access_key_env: String,
  },
}

/// One validated target-specific launch profile.
#[derive(Clone, Debug, PartialEq)]
pub enum Profile {
  Macos {
    display: Display,
  },
  Webgl {
    display: Display,
    headless_command: Option<Vec<String>>,
  },
  IosSimulator {
    device: String,
    orientation: Orientation,
  },
}

/// A fixed render size and scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Display {
  pub width: u32,
  pub height: u32,
  pub scale: f64,
}

/// A supported Ditto target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
  Macos,
  Webgl,
  IosSimulator,
}

/// An iOS Simulator orientation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Orientation {
  Portrait,
  PortraitUpsideDown,
  LandscapeLeft,
  LandscapeRight,
}

/// A validated scenario with inherited defaults applied.
#[derive(Clone, Debug, PartialEq)]
pub struct Scenario {
  pub name: String,
  pub fixture: Option<String>,
  pub motion: Motion,
  pub timeout: DurationValue,
  pub steps: Vec<Step>,
}

/// A scenario motion mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Motion {
  Instant,
  Controlled,
  RealTime,
}

/// One validated scenario step.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
  pub name: Option<String>,
  pub timeout: DurationValue,
  pub action: StepKind,
}

/// A supported scenario action.
#[derive(Clone, Debug, PartialEq)]
pub enum StepKind {
  Click {
    target: InputTarget,
    settle: bool,
  },
  Hover {
    target: InputTarget,
  },
  Drag {
    from: InputTarget,
    to: InputTarget,
  },
  Key {
    key: String,
    action: KeyAction,
  },
  Wait(WaitStep),
  Assert(ObjectCondition),
  AccessibilityAssert(AccessibilityAssertion),
  AccessibilityAction {
    target: AccessibilityTarget,
    action: AccessibilityAction,
  },
  Screenshot(ScreenshotStep),
  Video(VideoStep),
}

/// A semantic node selected by role and accessible name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessibilityTarget {
  pub role: AccessibilityRole,
  pub name: String,
}

/// Expected semantic values for one selected node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessibilityAssertion {
  pub target: AccessibilityTarget,
  pub role: AccessibilityRole,
  pub name: String,
}

/// A role supported by Battlement's accessibility surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessibilityRole {
  Button,
  Checkbox,
  Switch,
  Radio,
  RadioGroup,
  Slider,
  Progress,
  Disclosure,
  ScrollArea,
  Tab,
  TabList,
  TabPanel,
  Dialog,
  Heading,
  Image,
  StaticText,
  Group,
}

/// A direct action supported by the accessibility callback adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessibilityAction {
  Activate,
  Increment,
  Decrement,
  Dismiss,
  ScrollForward,
  ScrollBackward,
}

/// A production input target.
#[derive(Clone, Debug, PartialEq)]
pub enum InputTarget {
  Object(String),
  Coordinates([f64; 2]),
}

/// A virtual key transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyAction {
  Down,
  Up,
  Tap,
}

/// A frame or black-box object wait.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WaitStep {
  Frames(u32),
  Object(ObjectCondition),
}

/// A black-box object condition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectCondition {
  pub object: String,
  pub state: ObjectState,
}

/// A supported object condition state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectState {
  Exists,
  Absent,
  Visible,
  Hidden,
  Enabled,
  Disabled,
}

/// A screenshot checkpoint and its effective comparison settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScreenshotStep {
  pub name: String,
  pub comparison: Comparison,
}

/// A paired native video boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VideoStep {
  Start {
    name: String,
    motion: Motion,
    max_duration: DurationValue,
  },
  Stop,
}

impl Profile {
  /// Returns the platform selected by this profile.
  pub fn target(&self) -> Target {
    match self {
      Self::Macos { .. } => Target::Macos,
      Self::Webgl { .. } => Target::Webgl,
      Self::IosSimulator { .. } => Target::IosSimulator,
    }
  }
}

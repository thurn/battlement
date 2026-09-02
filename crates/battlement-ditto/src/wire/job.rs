//! Resolved job wire models.

use serde::{Deserialize, Deserializer, Serialize};

use crate::wire::validation;

/// An exact unsigned base-10 decimal carried without binary conversion.
pub type DecimalString = String;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Command {
  Run,
  Capture,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Platform {
  Macos,
  Webgl,
  IosSimulator,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Orientation {
  Portrait,
  PortraitUpsideDown,
  LandscapeLeft,
  LandscapeRight,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
  Click,
  Hover,
  Drag,
  Key,
  Png,
  Video,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Motion {
  Instant,
  Controlled,
  RealTime,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyAction {
  Down,
  Up,
  Tap,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObjectState {
  Exists,
  Absent,
  Visible,
  Hidden,
  Enabled,
  Disabled,
}

/// A fully resolved unit of player work.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
  pub job_id: String,
  pub run_id: String,
  pub remaining_run_timeout_ms: u64,
  pub log_redactions: Vec<String>,
  pub command: Command,
  pub profile: ResolvedProfile,
  pub scenarios: Vec<ResolvedScenario>,
}

/// The selected platform and exact launch identity.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedProfile {
  pub name: String,
  pub platform: Platform,
  pub display: Display,
  pub build_fingerprint: String,
  pub source_fingerprint: String,
  pub capabilities: Vec<Capability>,
}

/// The effective player framebuffer and safe area.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Display {
  pub width: u32,
  pub height: u32,
  pub scale: f64,
  pub orientation: Option<Orientation>,
  pub safe_area: [u32; 4],
}

/// One runnable scenario with its result-coordinate index.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedScenario {
  pub id: String,
  pub run_index: u32,
  pub name: String,
  pub fixture: Option<String>,
  pub motion: Motion,
  pub timeout_ms: u64,
  pub steps: Vec<ResolvedStep>,
}

/// One resolved authored step.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedStep {
  pub index: u32,
  pub name: Option<String>,
  pub timeout_ms: u64,
  pub action: StepKind,
}

/// A resolved player action.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AccessibilityTarget {
  pub role: AccessibilityRole,
  pub name: String,
}

/// Expected semantic values for one selected node.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AccessibilityAssertion {
  pub target: AccessibilityTarget,
  pub role: AccessibilityRole,
  pub name: String,
  pub selected: Option<bool>,
  pub checked: Option<bool>,
  pub disabled: Option<bool>,
  pub current_page: Option<bool>,
  pub parent: Option<AccessibilityTarget>,
}

/// A role supported by the runtime semantic mirror.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
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
  ListBox,
  Option,
  Table,
  Row,
  ColumnHeader,
  RowHeader,
  Cell,
  Link,
  Navigation,
  Region,
}

/// A direct action supported by the runtime callback adapter.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccessibilityAction {
  Activate,
  Increment,
  Decrement,
  Dismiss,
  ScrollForward,
  ScrollBackward,
}

/// An object UUID or normalized render coordinate.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum InputTarget {
  Object(String),
  Coordinates([f64; 2]),
}

/// An exact frame wait or black-box object wait.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum WaitStep {
  Frames(FrameWait),
  Object(ObjectCondition),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrameWait {
  pub frames: u32,
}

/// A condition observable through the Battlement presentation model.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectCondition {
  pub object: String,
  pub state: ObjectState,
}

/// A named PNG checkpoint and its resolved comparison settings.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScreenshotStep {
  pub name: String,
  pub comparison: Comparison,
}

/// Exact image-difference thresholds.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Comparison {
  pub threshold: DecimalString,
  pub anti_alias: bool,
  pub max_changed_percent: DecimalString,
}

/// A paired native-video boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "kebab-case", deny_unknown_fields)]
pub enum VideoStep {
  Start {
    name: String,
    motion: Motion,
    max_duration_ms: u64,
  },
  Stop,
}

impl Job {
  /// Validates semantic invariants that Serde cannot express.
  pub fn validate(&self) -> anyhow::Result<()> {
    validation::validate_job(self)
  }
}

impl<'de> Deserialize<'de> for VideoStep {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    Ok(match RawVideoStep::deserialize(deserializer)? {
      RawVideoStep::Start {
        name,
        motion,
        max_duration_ms,
      } => Self::Start {
        name,
        motion,
        max_duration_ms,
      },
      RawVideoStep::Stop {} => Self::Stop,
    })
  }
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case", deny_unknown_fields)]
enum RawVideoStep {
  Start {
    name: String,
    motion: Motion,
    max_duration_ms: u64,
  },
  Stop {},
}

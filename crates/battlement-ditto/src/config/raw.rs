use std::{collections::BTreeMap, ops::Range, path::PathBuf};

use serde::Deserialize;
use toml::Spanned;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawSuite {
  pub name: String,
  pub default_profile: String,
  pub player: RawPlayer,
  #[serde(default)]
  pub timeouts: RawTimeouts,
  #[serde(default)]
  pub defaults: RawDefaults,
  #[serde(default)]
  pub aliases: BTreeMap<String, String>,
  pub baseline: Option<RawBaseline>,
  pub profiles: BTreeMap<String, RawProfile>,
  pub scenarios: Vec<RawScenario>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawFragment {
  pub name: Option<String>,
  #[serde(default)]
  pub defaults: RawDefaults,
  #[serde(default)]
  pub aliases: BTreeMap<String, String>,
  pub scenarios: Vec<RawScenario>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawPlayer {
  pub unity_project: PathBuf,
  pub scene: PathBuf,
  pub rust_manifest: PathBuf,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawTimeouts {
  pub run: Option<String>,
  pub build: Option<String>,
  pub launch: Option<String>,
  pub baseline_download: Option<String>,
  pub simulator_boot: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawDefaults {
  pub step_timeout: Option<String>,
  pub scenario_timeout: Option<String>,
  pub motion: Option<RawMotion>,
  #[serde(default)]
  pub comparison: RawComparison,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawComparison {
  pub threshold: Option<RawDecimal>,
  pub anti_alias: Option<bool>,
  pub max_changed_percent: Option<RawDecimal>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub(super) struct RawDecimal(Spanned<toml::Value>);

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(super) enum RawBaseline {
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawProfile {
  pub target: RawTarget,
  pub display: Option<RawDisplay>,
  pub headless_command: Option<Vec<String>>,
  pub device: Option<String>,
  pub orientation: Option<RawOrientation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawDisplay {
  pub width: u32,
  pub height: u32,
  pub scale: f64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum RawTarget {
  Macos,
  Webgl,
  IosSimulator,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum RawOrientation {
  Portrait,
  PortraitUpsideDown,
  LandscapeLeft,
  LandscapeRight,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawScenario {
  pub name: String,
  pub fixture: Option<String>,
  pub motion: Option<RawMotion>,
  pub timeout: Option<String>,
  pub steps: Vec<RawStep>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum RawMotion {
  Instant,
  Controlled,
  RealTime,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawStep {
  pub name: Option<String>,
  pub timeout: Option<String>,
  pub click: Option<RawClick>,
  pub hover: Option<RawHover>,
  pub drag: Option<RawDrag>,
  pub key: Option<RawKey>,
  pub wait: Option<RawWait>,
  #[serde(rename = "assert")]
  pub assertion: Option<RawCondition>,
  pub accessibility_assert: Option<RawAccessibilityAssertion>,
  pub accessibility_action: Option<RawAccessibilityActionStep>,
  pub screenshot: Option<RawScreenshot>,
  pub video: Option<RawVideo>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawAccessibilityTarget {
  pub role: RawAccessibilityRole,
  pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawAccessibilityAssertion {
  pub target: RawAccessibilityTarget,
  pub role: RawAccessibilityRole,
  pub name: String,
  pub selected: Option<bool>,
  pub checked: Option<bool>,
  pub disabled: Option<bool>,
  pub current_page: Option<bool>,
  pub parent: Option<RawAccessibilityTarget>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawAccessibilityActionStep {
  pub target: RawAccessibilityTarget,
  pub action: RawAccessibilityAction,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum RawAccessibilityRole {
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

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum RawAccessibilityAction {
  Activate,
  Increment,
  Decrement,
  Dismiss,
  ScrollForward,
  ScrollBackward,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawClick {
  pub target: RawInputTarget,
  #[serde(default = "default_settle")]
  pub settle: bool,
}

fn default_settle() -> bool {
  true
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawHover {
  pub target: RawInputTarget,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawDrag {
  pub from: RawInputTarget,
  pub to: RawInputTarget,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum RawInputTarget {
  Object(String),
  Coordinates([f64; 2]),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawKey {
  pub key: String,
  pub action: Option<RawKeyAction>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum RawKeyAction {
  Down,
  Up,
  Tap,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawWait {
  pub frames: Option<u32>,
  pub object: Option<String>,
  pub state: Option<RawObjectState>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawCondition {
  pub object: String,
  pub state: RawObjectState,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum RawObjectState {
  Exists,
  Absent,
  Visible,
  Hidden,
  Enabled,
  Disabled,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawScreenshot {
  pub name: String,
  pub threshold: Option<RawDecimal>,
  pub anti_alias: Option<bool>,
  pub max_changed_percent: Option<RawDecimal>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawVideo {
  pub action: RawVideoAction,
  pub name: Option<String>,
  pub motion: Option<RawMotion>,
  pub max_duration: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum RawVideoAction {
  Start,
  Stop,
}

impl RawDecimal {
  pub fn span(&self) -> Range<usize> {
    self.0.span()
  }
}

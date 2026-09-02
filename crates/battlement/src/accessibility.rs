//! Resolved accessibility snapshots and direct actions.

use serde::{Deserialize, Serialize};

use crate::ObjectId;

/// Roles retained by the host-backed accessibility surface.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum SemanticRole {
  /// An activatable button.
  Button,
  /// A tri-state checkbox.
  Checkbox,
  /// A Boolean switch.
  Switch,
  /// One radio option.
  Radio,
  /// A radio-option container.
  RadioGroup,
  /// A single-thumb numeric slider.
  Slider,
  /// A determinate or busy progress indicator.
  Progress,
  /// A disclosure trigger.
  Disclosure,
  /// A scrollable region.
  ScrollArea,
  /// One tab.
  Tab,
  /// A tab container.
  TabList,
  /// The selected tab panel.
  TabPanel,
  /// A modal dialog.
  Dialog,
  /// A heading.
  Heading,
  /// An informative image.
  Image,
  /// Read-only text.
  StaticText,
  /// A structural group.
  Group,
  /// A named single-selection list.
  ListBox,
  /// One listbox choice.
  Option,
  /// A named table of rows.
  Table,
  /// One table row.
  Row,
  /// A header identifying a table column.
  ColumnHeader,
  /// A header identifying a table row.
  RowHeader,
  /// One table data cell.
  Cell,
  /// An activatable link.
  Link,
  /// A named navigation landmark.
  Navigation,
  /// A named content landmark.
  Region,
}

/// The current location represented by a button or link.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum CurrentPage {
  /// The currently displayed page.
  Page,
}

/// Kind of popup controlled by a semantic button.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum PopupKind {
  /// A selection list.
  ListBox,
}

/// Canonical checked state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum CheckedState {
  /// Not checked.
  False,
  /// Checked.
  True,
  /// Partially checked.
  Mixed,
}

/// Direction of one logical accessibility scroll action.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum AccessibilityScrollDirection {
  /// Increase the logical offset.
  Forward,
  /// Decrease the logical offset.
  Backward,
}

/// Axis owned by an accessible scroll area.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum AccessibilityScrollAxis {
  /// Horizontal scrolling.
  Horizontal,
  /// Vertical scrolling.
  Vertical,
}

/// Current canonical semantic state.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticState {
  /// Whether actions are currently unavailable.
  pub disabled: bool,
  /// Checked state when relevant.
  pub checked: Option<CheckedState>,
  /// Selection state when relevant.
  pub selected: Option<bool>,
  /// Expansion state when relevant.
  pub expanded: Option<bool>,
  /// Kind of popup controlled by this button. Requires expansion state.
  pub popup: Option<PopupKind>,
  /// Whether a progress indicator is indeterminate.
  pub busy: bool,
  /// Whether this button or link represents the current page.
  pub current: Option<CurrentPage>,
}

/// Resolved finite range value.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AccessibilityRangeValue {
  /// Current value.
  pub current: f64,
  /// Inclusive minimum.
  pub minimum: f64,
  /// Inclusive maximum.
  pub maximum: f64,
  /// Optional localized display text.
  pub text: Option<String>,
}

/// Direct callbacks currently declared by a semantic node.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AccessibilityActionSet {
  /// Supports activation.
  pub activate: bool,
  /// Supports range increment.
  pub increment: bool,
  /// Supports range decrement.
  pub decrement: bool,
  /// Supports dialog dismissal.
  pub dismiss: bool,
  /// Available logical scroll directions.
  pub scroll: Vec<AccessibilityScrollDirection>,
}

/// One resolved host-backed semantic node.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AccessibilityNodeSnapshot {
  /// Stable host identity.
  pub object_id: ObjectId,
  /// Nearest exposed logical semantic ancestor.
  pub parent_id: Option<ObjectId>,
  /// Exposed children in logical reading order.
  pub children: Vec<ObjectId>,
  /// Canonical role.
  pub role: SemanticRole,
  /// Resolved accessible name.
  pub label: Option<String>,
  /// Resolved accessible description.
  pub hint: Option<String>,
  /// Canonical state.
  pub state: SemanticState,
  /// Range value when relevant.
  pub value: Option<AccessibilityRangeValue>,
  /// Declared direct actions.
  pub actions: AccessibilityActionSet,
  /// Heading level from one through six.
  pub heading_level: Option<u8>,
  /// Scroll axis when the role is a scroll area.
  pub scroll_axis: Option<AccessibilityScrollAxis>,
}

/// One complete canonical semantic tree.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct AccessibilitySnapshot {
  /// Monotonic semantic commit sequence.
  pub commit_sequence: u64,
  /// Canonical semantic roots in document order.
  pub roots: Vec<ObjectId>,
  /// Nodes in depth-first logical reading order.
  pub nodes: Vec<AccessibilityNodeSnapshot>,
}

/// Atomic accessibility work attached to an ordinary response commit.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct AccessibilityUpdate {
  /// Complete replacement when canonical semantics changed.
  pub snapshot: Option<AccessibilitySnapshot>,
  /// Ordered one-shot messages, never retained for reconnect.
  pub announcements: Vec<String>,
}

/// One normalized accessibility callback.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum AccessibilityAction {
  /// Invoke the target.
  Activate,
  /// Increment its range value.
  Increment,
  /// Decrement its range value.
  Decrement,
  /// Dismiss its dialog.
  Dismiss,
  /// Scroll in one logical direction.
  Scroll(AccessibilityScrollDirection),
}

/// Payload emitted by the current Unity accessibility backend.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AccessibilityEvent {
  /// Live backend generation.
  pub backend_generation: u64,
  /// Stable semantic host target.
  pub target: ObjectId,
  /// Requested direct action.
  pub action: AccessibilityAction,
}

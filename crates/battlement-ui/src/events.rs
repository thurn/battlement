use battlement_types::{ObjectId, PointerButton};
use serde::{Deserialize, Serialize};

/// A two-dimensional position in panel pixels measured from the upper-left.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PanelPoint {
    /// Horizontal panel coordinate.
    pub x: f64,
    /// Vertical panel coordinate.
    pub y: f64,
}

impl PanelPoint {
    /// Creates a panel position from its coordinates.
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A physical modifier key held while a UI gesture occurred.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum KeyModifier {
    /// Alt or Option.
    Alt,
    /// Control.
    Control,
    /// Command on macOS or Windows on Windows and Linux.
    Command,
    /// Shift.
    Shift,
}

/// The canonical, duplicate-free modifier set carried by a UI event.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct KeyModifiers(Vec<KeyModifier>);

impl KeyModifiers {
    /// Creates a canonical modifier set, rejecting duplicates and noncanonical order.
    pub fn new(values: Vec<KeyModifier>) -> Result<Self, &'static str> {
        if values.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err("key modifiers must be unique and in canonical order");
        }
        Ok(Self(values))
    }

    /// Returns the modifiers in canonical order.
    #[must_use]
    pub fn as_slice(&self) -> &[KeyModifier] {
        &self.0
    }

    /// Returns whether no modifiers were held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// UI event kinds that an element can request from the Unity host.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum UiEventKind {
    /// A pointer or navigation activation of a clickable element.
    Click,
}

/// One subscribed UI event sent synchronously to the rules engine.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct UiEvent {
    /// Logical element on which the native event originated.
    pub target_id: ObjectId,
    /// Typed event payload.
    pub body: UiEventBody,
}

impl UiEvent {
    /// Creates a click event for one logical target.
    #[must_use]
    pub fn click(target_id: ObjectId, value: ClickEvent) -> Self {
        Self {
            target_id,
            body: UiEventBody::Click(value),
        }
    }

    /// Returns the event kind used for subscription checks.
    #[must_use]
    pub const fn kind(&self) -> UiEventKind {
        match self.body {
            UiEventBody::Click(_) => UiEventKind::Click,
        }
    }
}

/// The exact payload union for supported UI events.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum UiEventBody {
    /// Button activation.
    Click(ClickEvent),
}

/// How a clickable element was activated.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ClickEvent {
    /// Pointer press and release on the same logical target.
    Pointer {
        /// Stable Unity pointer identity.
        #[serde(default, skip_serializing_if = "crate::is_default")]
        pointer_id: i32,
        /// Upper-left-origin panel position.
        position: PanelPoint,
        /// Mouse-style button used for activation.
        #[serde(default, skip_serializing_if = "crate::is_default")]
        button: PointerButton,
        /// Native click count reported by UI Toolkit.
        click_count: u32,
        /// Held physical modifiers in canonical order.
        #[serde(default, skip_serializing_if = "KeyModifiers::is_empty")]
        modifiers: KeyModifiers,
    },
    /// Activation from Unity's `NavigationSubmitEvent` on a focused button.
    NavigationSubmit,
    /// Fixed callback activation from a repeat button.
    Repeat,
}

impl ClickEvent {
    /// Creates a pointer activation payload.
    #[must_use]
    pub fn pointer(
        pointer_id: i32,
        position: PanelPoint,
        button: PointerButton,
        click_count: u32,
        modifiers: KeyModifiers,
    ) -> Self {
        Self::Pointer {
            pointer_id,
            position,
            button,
            click_count,
            modifiers,
        }
    }
}

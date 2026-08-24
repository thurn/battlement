use battlement_types::{ObjectId, PointerButton};
use serde::{Deserialize, Serialize};

/// A two-dimensional panel-space position measured from the upper-left corner.
///
/// `x` increases to the right and `y` increases downward. Values are expressed
/// in panel pixels after Unity applies the panel's screen-to-panel transform.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PanelPoint {
    /// Horizontal panel coordinate, increasing to the right.
    pub x: f64,
    /// Vertical panel coordinate, increasing downward.
    pub y: f64,
}

impl PanelPoint {
    /// Creates a panel-space position from horizontal `x` and vertical `y`.
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A physical modifier key held while a native UI event occurred.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum KeyModifier {
    /// Alt on Windows and Linux, or Option on macOS.
    Alt,
    /// The physical Control key.
    Control,
    /// Command on macOS, or the Windows key on Windows and Linux.
    Command,
    /// The physical Shift key.
    Shift,
}

/// The canonical, duplicate-free physical modifiers carried by a UI event.
///
/// Values are ordered as [`KeyModifier::Alt`], [`KeyModifier::Control`],
/// [`KeyModifier::Command`], then [`KeyModifier::Shift`]. Canonical ordering
/// makes serialized event payloads deterministic regardless of native key-query
/// order.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct KeyModifiers(Vec<KeyModifier>);

impl KeyModifiers {
    /// Creates a modifier set from values already in canonical order.
    ///
    /// # Errors
    ///
    /// Returns an error when a modifier is duplicated or appears after a
    /// modifier with a greater canonical order.
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

    /// Returns `true` when the event carried no physical modifiers.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Native UI event families that an element can forward to Rust.
///
/// Adding a kind through an element's `events` builder creates a subscription;
/// unsubscribed events remain entirely inside Unity.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum UiEventKind {
    /// Activation represented by a [`ClickEvent`] payload.
    Click,
}

/// One subscribed native UI event delivered to the Rust rules engine.
///
/// `target_id` identifies the logical element on which Unity reports the event,
/// while [`Self::body`] retains the event-family-specific payload. Use
/// [`Self::kind`] to match the subscription family without inspecting the body.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct UiEvent {
    /// Logical element on which the native event originated.
    pub target_id: ObjectId,
    /// Event-family-specific payload copied from native UI state.
    pub body: UiEventBody,
}

impl UiEvent {
    /// Creates a click-family event for one logical target.
    #[must_use]
    pub fn click(target_id: ObjectId, value: ClickEvent) -> Self {
        Self {
            target_id,
            body: UiEventBody::Click(value),
        }
    }

    /// Returns the event family used for subscription checks.
    #[must_use]
    pub const fn kind(&self) -> UiEventKind {
        match self.body {
            UiEventBody::Click(_) => UiEventKind::Click,
        }
    }
}

/// Payloads for the native UI event families supported by Battlement.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum UiEventBody {
    /// Pointer, navigation-submit, or repeat-button activation.
    Click(ClickEvent),
}

/// The native mechanism that activated a clickable element.
///
/// Pointer activation preserves Unity's pointer details. Keyboard or gamepad
/// submit and repeat-button callbacks have no pointer coordinates or buttons,
/// so they use distinct payload variants rather than sentinel values.
///
/// See Unity's [`ClickEvent` reference](https://docs.unity3d.com/6000.5/Documentation/ScriptReference/UIElements.ClickEvent.html)
/// for native pointer-click behavior and propagation.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ClickEvent {
    /// Pointer down followed by pointer up on the same logical target.
    Pointer {
        /// Unity pointer identity shared by the down and up events.
        #[serde(default, skip_serializing_if = "crate::is_default")]
        pointer_id: i32,
        /// Pointer position in upper-left-origin panel coordinates.
        position: PanelPoint,
        /// Mouse-style button whose down-up sequence produced the activation.
        #[serde(default, skip_serializing_if = "crate::is_default")]
        button: PointerButton,
        /// Number of consecutive short-interval activations with this pointer and button.
        click_count: u32,
        /// Physical modifiers held when Unity produced the click.
        #[serde(default, skip_serializing_if = "KeyModifiers::is_empty")]
        modifiers: KeyModifiers,
    },
    /// Unity `NavigationSubmitEvent` activation on the focused button.
    NavigationSubmit,
    /// Callback activation emitted by a Unity repeat button.
    Repeat,
}

impl ClickEvent {
    /// Creates a pointer activation with native pointer metadata.
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

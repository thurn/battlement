use battlement_types::{ObjectId, PhysicalKey, PointerButton, Rect};
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

/// A two-dimensional displacement in upper-left-origin panel pixels.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Vector {
  /// Horizontal displacement, positive to the right.
  pub x: f32,
  /// Vertical displacement, positive downward.
  pub y: f32,
}

/// An optional dropdown selection represented by a coherent index and value pair.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Choice {
  /// Zero-based choice index, or `None` when the selection is empty.
  pub index: Option<u32>,
  /// Display value at `index`, or `None` when the selection is empty.
  pub value: Option<String>,
}

/// An ordered finite floating-point range.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct F32Range {
  /// Selected lower endpoint.
  pub min: f32,
  /// Selected upper endpoint.
  pub max: f32,
}

impl F32Range {
  /// Creates a range from ordered lower and upper endpoints.
  #[must_use]
  pub const fn new(min: f32, max: f32) -> Self {
    Self { min, max }
  }
}

impl Choice {
  /// Creates a populated selection.
  #[must_use]
  pub fn selected(index: u32, value: impl Into<String>) -> Self {
    Self {
      index: Some(index),
      value: Some(value.into()),
    }
  }

  /// Creates an explicit empty selection.
  #[must_use]
  pub const fn none() -> Self {
    Self {
      index: None,
      value: None,
    }
  }
}

impl Vector {
  /// Creates a displacement from horizontal and vertical components.
  #[must_use]
  pub const fn new(x: f32, y: f32) -> Self {
    Self { x, y }
  }
}

/// A value proposed or committed by a controlled UI component.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum UiValue {
  /// A controlled Boolean value.
  Bool(bool),
  /// An optional zero-based selected index.
  Index(Option<u32>),
  /// Unique sorted zero-based selected indices.
  Indices(Vec<u32>),
  /// A coherent optional dropdown index and display value.
  Choice(Choice),
  /// A finite floating-point control value.
  F32(f32),
  /// A controlled integer value.
  I32(i32),
  /// An ordered finite floating-point range.
  F32Range(F32Range),
  /// An arbitrary UTF-8 text control value.
  String(String),
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
  /// Caps Lock was active.
  CapsLock,
  /// Numeric Lock was active.
  Numeric,
  /// A platform function modifier was active.
  FunctionKey,
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
  /// A pointer button was pressed.
  PointerDown,
  /// A pointer moved over the panel.
  PointerMove,
  /// A pointer button was released.
  PointerUp,
  /// A pointer interaction was cancelled.
  PointerCancel,
  /// A logical activation, usually the event an application wants for a button.
  ///
  /// This is broader than Unity's pointer-only `ClickEvent`: a [`Button`](crate::Button)
  /// subscription also receives keyboard and gamepad submit as
  /// [`ClickEvent::NavigationSubmit`]. This lets one handler cover every way a user can
  /// activate a button.
  Click,
  /// A pointer entered a logical target.
  PointerEnter,
  /// A pointer left a logical target.
  PointerLeave,
  /// A pointer crossed into a target or descendant.
  PointerOver,
  /// A pointer crossed out of a target or descendant.
  PointerOut,
  /// A wheel or trackpad produced a scroll delta.
  Wheel,
  /// A logical target gained pointer capture.
  PointerCapture,
  /// A logical target lost pointer capture.
  PointerCaptureOut,
  /// A physical key was pressed while a logical target held UI focus.
  KeyDown,
  /// A physical key was released while a logical target held UI focus.
  KeyUp,
  /// UI navigation requested directional focus movement.
  NavigationMove,
  /// UI navigation requested cancellation.
  NavigationCancel,
  /// Focus moved into a logical target or one of its descendants.
  FocusIn,
  /// Focus settled on a logical target.
  Focus,
  /// Focus left a logical target or one of its descendants.
  FocusOut,
  /// A logical target lost focus.
  Blur,
  /// A logical target's panel geometry changed.
  GeometryChanged,
  /// A logical target was attached to a panel.
  AttachToPanel,
  /// A logical target was detached from a panel.
  DetachFromPanel,
  /// A transition began after its delay phase.
  TransitionStart,
  /// A transition reached its settled endpoint.
  TransitionEnd,
  /// A transition was interrupted by another style change.
  TransitionCancel,
  /// A controlled component's live local value changed during interaction.
  ValueChanging,
  /// A controlled component completed one logical value change.
  ValueCommitted,
  /// A text field's native local draft changed while editing.
  Input,
  /// A text field's caret or selection endpoints changed.
  SelectionChanged,
  /// A pointer entered a rich-text link.
  LinkEnter,
  /// A pointer left a rich-text link.
  LinkLeave,
  /// A pointer button was pressed on a rich-text link.
  LinkDown,
  /// A pointer button was released on a rich-text link.
  LinkUp,
  /// A scroll view remained motionless and uncaptured for 100 milliseconds.
  ScrollSettled,
  /// A scroll view's user-driven offset changed.
  ScrollChanged,
  /// A tab view received a proposed active-tab change.
  TabSelectionRequested,
  /// A tab view received a proposed close for one of its tabs.
  TabCloseRequested,
  /// A tab view received a proposed header reorder.
  TabReorderRequested,
}

impl UiEventKind {
  /// Returns whether strict ancestors may subscribe during trickle or bubble.
  #[must_use]
  pub const fn propagates(self) -> bool {
    matches!(
      self,
      Self::PointerDown
        | Self::PointerMove
        | Self::PointerUp
        | Self::PointerCancel
        | Self::Click
        | Self::PointerOver
        | Self::PointerOut
        | Self::Wheel
        | Self::PointerCapture
        | Self::PointerCaptureOut
        | Self::KeyDown
        | Self::KeyUp
        | Self::NavigationMove
        | Self::NavigationCancel
        | Self::FocusIn
        | Self::FocusOut
        | Self::LinkEnter
        | Self::LinkLeave
        | Self::LinkDown
        | Self::LinkUp
    )
  }
}

/// One phase at which an event subscription participates in logical routing.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum UiEventPhase {
  /// Deliver on strict ancestors from the document root toward the target.
  Trickle,
  /// Deliver on the originating logical target.
  #[default]
  Target,
  /// Deliver on strict ancestors from the target toward the document root.
  Bubble,
}

/// One event kind and logical route phase requested by an element.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct UiEventSubscription {
  /// Native event family to observe.
  pub kind: UiEventKind,
  /// Logical route phase at which to deliver it.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub phase: UiEventPhase,
}

impl UiEventSubscription {
  /// Creates a subscription for one explicit route phase.
  #[must_use]
  pub const fn new(kind: UiEventKind, phase: UiEventPhase) -> Self {
    Self { kind, phase }
  }

  /// Creates the target-phase shorthand used by element `events` builders.
  #[must_use]
  pub const fn target(kind: UiEventKind) -> Self {
    Self::new(kind, UiEventPhase::Target)
  }
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
      UiEventBody::PointerDown(_) => UiEventKind::PointerDown,
      UiEventBody::PointerMove(_) => UiEventKind::PointerMove,
      UiEventBody::PointerUp(_) => UiEventKind::PointerUp,
      UiEventBody::PointerCancel(_) => UiEventKind::PointerCancel,
      UiEventBody::Click(_) => UiEventKind::Click,
      UiEventBody::PointerEnter(_) => UiEventKind::PointerEnter,
      UiEventBody::PointerLeave(_) => UiEventKind::PointerLeave,
      UiEventBody::PointerOver(_) => UiEventKind::PointerOver,
      UiEventBody::PointerOut(_) => UiEventKind::PointerOut,
      UiEventBody::Wheel(_) => UiEventKind::Wheel,
      UiEventBody::PointerCapture(_) => UiEventKind::PointerCapture,
      UiEventBody::PointerCaptureOut(_) => UiEventKind::PointerCaptureOut,
      UiEventBody::KeyDown(_) => UiEventKind::KeyDown,
      UiEventBody::KeyUp(_) => UiEventKind::KeyUp,
      UiEventBody::NavigationMove(_) => UiEventKind::NavigationMove,
      UiEventBody::NavigationCancel(_) => UiEventKind::NavigationCancel,
      UiEventBody::FocusIn(_) => UiEventKind::FocusIn,
      UiEventBody::Focus(_) => UiEventKind::Focus,
      UiEventBody::FocusOut(_) => UiEventKind::FocusOut,
      UiEventBody::Blur(_) => UiEventKind::Blur,
      UiEventBody::GeometryChanged(_) => UiEventKind::GeometryChanged,
      UiEventBody::AttachToPanel(_) => UiEventKind::AttachToPanel,
      UiEventBody::DetachFromPanel(_) => UiEventKind::DetachFromPanel,
      UiEventBody::TransitionStart(_) => UiEventKind::TransitionStart,
      UiEventBody::TransitionEnd(_) => UiEventKind::TransitionEnd,
      UiEventBody::TransitionCancel(_) => UiEventKind::TransitionCancel,
      UiEventBody::ValueChanging(_) => UiEventKind::ValueChanging,
      UiEventBody::ValueCommitted(_) => UiEventKind::ValueCommitted,
      UiEventBody::Input(_) => UiEventKind::Input,
      UiEventBody::SelectionChanged(_) => UiEventKind::SelectionChanged,
      UiEventBody::LinkEnter(_) => UiEventKind::LinkEnter,
      UiEventBody::LinkLeave(_) => UiEventKind::LinkLeave,
      UiEventBody::LinkDown(_) => UiEventKind::LinkDown,
      UiEventBody::LinkUp(_) => UiEventKind::LinkUp,
      UiEventBody::ScrollSettled(_) => UiEventKind::ScrollSettled,
      UiEventBody::ScrollChanged(_) => UiEventKind::ScrollChanged,
      UiEventBody::TabSelectionRequested(_) => UiEventKind::TabSelectionRequested,
      UiEventBody::TabCloseRequested(_) => UiEventKind::TabCloseRequested,
      UiEventBody::TabReorderRequested(_) => UiEventKind::TabReorderRequested,
    }
  }
}

/// Payloads for the native UI event families supported by Battlement.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum UiEventBody {
  /// Pointer-button press metadata.
  PointerDown(PointerButtonEvent),
  /// Pointer-motion metadata.
  PointerMove(PointerMoveEvent),
  /// Pointer-button release metadata.
  PointerUp(PointerButtonEvent),
  /// Cancelled pointer-interaction metadata.
  PointerCancel(PointerCancelEvent),
  /// Pointer, navigation-submit, or repeat-button activation.
  Click(ClickEvent),
  /// Target boundary entry metadata.
  PointerEnter(PointerBoundaryEvent),
  /// Target boundary exit metadata.
  PointerLeave(PointerBoundaryEvent),
  /// Propagating crossing-entry metadata.
  PointerOver(PointerCrossingEvent),
  /// Propagating crossing-exit metadata.
  PointerOut(PointerCrossingEvent),
  /// Wheel or trackpad metadata.
  Wheel(WheelEvent),
  /// Pointer-capture acquisition metadata.
  PointerCapture(PointerCaptureEvent),
  /// Pointer-capture release metadata.
  PointerCaptureOut(PointerCaptureEvent),
  /// Physical-key press metadata.
  KeyDown(KeyEvent),
  /// Physical-key release metadata.
  KeyUp(KeyEvent),
  /// Directional UI navigation metadata.
  NavigationMove(NavigationMoveEvent),
  /// Navigation cancellation metadata.
  NavigationCancel(NavigationEvent),
  /// Focus entered a logical target or descendant.
  FocusIn(FocusEvent),
  /// Focus settled on a logical target.
  Focus(FocusEvent),
  /// Focus left a logical target or descendant.
  FocusOut(FocusEvent),
  /// A logical target lost focus.
  Blur(FocusEvent),
  /// Target-only old and new panel geometry.
  GeometryChanged(GeometryEvent),
  /// Target-only panel attachment notification.
  AttachToPanel(LifecycleEvent),
  /// Target-only panel detachment notification.
  DetachFromPanel(LifecycleEvent),
  /// Transition delay completed and interpolation began.
  TransitionStart(TransitionEvent),
  /// Transition interpolation reached its endpoint.
  TransitionEnd(TransitionEvent),
  /// Transition interpolation was interrupted.
  TransitionCancel(TransitionEvent),
  /// Live proposed value from a controlled component.
  ValueChanging(ValueChangingEvent),
  /// Previous committed value and proposed replacement at gesture completion.
  ValueCommitted(ValueCommitEvent),
  /// Latest native local draft from a text field.
  Input(TextInputEvent),
  /// Current caret and selection endpoints from a text field.
  SelectionChanged(SelectionEvent),
  /// A pointer entered a rich-text link.
  LinkEnter(LinkEvent),
  /// A pointer left a rich-text link.
  LinkLeave(LinkEvent),
  /// A pointer button was pressed on a rich-text link.
  LinkDown(LinkEvent),
  /// A pointer button was released on a rich-text link.
  LinkUp(LinkEvent),
  /// Final offset after the exact scroll-settlement boundary.
  ScrollSettled(ScrollEvent),
  /// Latest user-driven scroll offset.
  ScrollChanged(ScrollEvent),
  /// Proposed controlled selection change in a tab view.
  TabSelectionRequested(TabSelectionEvent),
  /// Proposed close for one tab; native removal has already been vetoed.
  TabCloseRequested(TabCloseEvent),
  /// Proposed controlled reorder in a tab view.
  TabReorderRequested(TabReorderEvent),
}

/// Proposed active-tab change reported by a controlled tab view.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TabSelectionEvent {
  /// Index currently authored by Rust.
  pub previous_index: u32,
  /// Index selected by the user.
  pub proposed_index: u32,
  /// Identity of the user-selected tab.
  pub proposed_tab_id: ObjectId,
}

/// Proposed close reported after restoring the native tab to its authored position.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TabCloseEvent {
  /// Identity of the tab whose close control was activated.
  pub tab_id: ObjectId,
  /// Authored index at which the tab was restored.
  pub index: u32,
}

/// Proposed tab-header reorder reported after restoring the authored order.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TabReorderEvent {
  /// Identity of the tab the user dragged.
  pub tab_id: ObjectId,
  /// Authored index before the gesture.
  pub previous_index: u32,
  /// Destination index proposed by the user.
  pub proposed_index: u32,
}

/// Live value proposed by a controlled component while interaction continues.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ValueChangingEvent {
  /// Native value currently proposed by the user.
  pub proposed: UiValue,
}

/// Completed proposal from a controlled component.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ValueCommitEvent {
  /// Latest value authored by Rust before the interaction.
  pub previous: UiValue,
  /// Native value proposed when the interaction completed.
  pub proposed: UiValue,
}

/// Native local draft reported by a text field.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TextInputEvent {
  /// Complete draft after the native edit.
  pub value: String,
}

/// One logical native text-selection mutation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SelectionEvent {
  /// Caret endpoint measured in UTF-16 code units, matching Unity's index model.
  pub cursor_index: u32,
  /// Selection anchor measured in UTF-16 code units, matching Unity's index model.
  pub selection_index: u32,
}

/// Old and new finite panel geometry for one logical target.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct GeometryEvent {
  /// Geometry before the native layout change.
  pub previous: Rect,
  /// Geometry after the native layout change.
  pub current: Rect,
}

/// Empty payload for target-only panel lifecycle notifications.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct LifecycleEvent {}

/// Semantic rich-text link interaction metadata.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct LinkEvent {
  /// Author-provided rich-text link identifier.
  pub link_id: String,
  /// Visible linked text.
  pub link_text: String,
  /// Native pointer identity; zero is omitted on the wire.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pointer_id: i32,
  /// Pointer position in panel pixels.
  pub position: PanelPoint,
  /// Changed button for down and up; absent for enter and leave.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub button: Option<PointerButton>,
}

/// Scroll position reported by a live or settled scroll event.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ScrollEvent {
  /// Current horizontal and vertical content displacement in panel pixels.
  pub offset: Vector,
}

/// Property names and elapsed interpolation time from a native transition event.
///
/// Unity reports elapsed time without the delay phase. Battlement converts it
/// from seconds to milliseconds and rejects native property names outside the
/// closed [`TransitionProperty`](crate::TransitionProperty) catalog.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TransitionEvent {
  /// Nonempty supported properties whose transition lifecycle changed.
  pub properties: Vec<crate::TransitionProperty>,
  /// Finite interpolation time in milliseconds, excluding the delay.
  pub elapsed_ms: f32,
}

impl TransitionEvent {
  /// Creates a transition event payload in native property order.
  #[must_use]
  pub fn new(properties: Vec<crate::TransitionProperty>, elapsed_ms: f32) -> Self {
    assert!(
      !properties.is_empty(),
      "transition events require at least one supported property"
    );
    assert!(
      elapsed_ms.is_finite(),
      "transition elapsed time must be finite"
    );
    Self {
      properties,
      elapsed_ms,
    }
  }
}

/// Native pointer device category.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PointerType {
  /// Mouse or mouse-compatible pointer.
  #[default]
  Mouse,
  /// Direct touch contact.
  Touch,
  /// Pen or stylus contact.
  Pen,
  /// A native pointer type outside the public catalog.
  Unknown,
}

/// Complete metadata for a pointer-button press or release.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PointerButtonEvent {
  /// Stable native pointer identity.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pointer_id: i32,
  /// Position in panel pixels.
  pub position: PanelPoint,
  /// Change since the preceding pointer event.
  pub delta: Vector,
  /// Button changed by this event.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub button: PointerButton,
  /// Native pressed-button bit mask.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub buttons: u32,
  /// Normalized contact pressure.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pressure: f32,
  /// Native consecutive-click count.
  #[serde(default = "one", skip_serializing_if = "is_one")]
  pub click_count: u32,
  /// Physical modifiers active at dispatch.
  #[serde(default, skip_serializing_if = "KeyModifiers::is_empty")]
  pub modifiers: KeyModifiers,
  /// Native pointer device category.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pointer_type: PointerType,
}

/// Complete metadata for pointer motion.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PointerMoveEvent {
  /// Stable native pointer identity.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pointer_id: i32,
  /// Position in panel pixels.
  pub position: PanelPoint,
  /// Change since the preceding pointer event.
  pub delta: Vector,
  /// Button associated with the motion when Unity supplies one.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub changed_button: Option<PointerButton>,
  /// Native pressed-button bit mask.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub buttons: u32,
  /// Normalized contact pressure.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pressure: f32,
  /// Native consecutive-click count, or zero when absent.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub click_count: u32,
  /// Physical modifiers active at dispatch.
  #[serde(default, skip_serializing_if = "KeyModifiers::is_empty")]
  pub modifiers: KeyModifiers,
  /// Native pointer device category.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pointer_type: PointerType,
}

/// Complete metadata for a cancelled pointer interaction.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PointerCancelEvent {
  /// Stable native pointer identity.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pointer_id: i32,
  /// Position in panel pixels.
  pub position: PanelPoint,
  /// Change since the preceding pointer event.
  pub delta: Vector,
  /// Native pressed-button bit mask.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub buttons: u32,
  /// Normalized contact pressure.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pressure: f32,
  /// Physical modifiers active at dispatch.
  #[serde(default, skip_serializing_if = "KeyModifiers::is_empty")]
  pub modifiers: KeyModifiers,
  /// Native pointer device category.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pointer_type: PointerType,
}

/// Target boundary metadata for pointer enter and leave.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct PointerBoundaryEvent {
  /// Stable native pointer identity.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pointer_id: i32,
  /// Position in panel pixels.
  pub position: PanelPoint,
  /// Native pointer device category.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pointer_type: PointerType,
}

/// Propagating pointer crossing metadata without a related target.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct PointerCrossingEvent {
  /// Stable native pointer identity.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pointer_id: i32,
  /// Position in panel pixels.
  pub position: PanelPoint,
  /// Native pointer device category.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pointer_type: PointerType,
}

/// Three-dimensional wheel or trackpad delta at a panel position.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WheelEvent {
  /// Position in panel pixels.
  pub position: PanelPoint,
  /// Native horizontal, vertical, and depth delta.
  pub delta: UiVector3,
  /// Physical modifiers active at dispatch.
  #[serde(default, skip_serializing_if = "KeyModifiers::is_empty")]
  pub modifiers: KeyModifiers,
}

/// Identity of a pointer whose capture ownership changed.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PointerCaptureEvent {
  /// Stable native pointer identity.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub pointer_id: i32,
}

/// Focus-change metadata mapped to the nearest Rust-owned related target.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct FocusEvent {
  /// Logical element focus moved from or to, when it belongs to Battlement UI.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub related_target_id: Option<ObjectId>,
  /// Public native focus-change direction.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub direction: FocusDirection,
}

/// Exact physical-key metadata exposed by UI Toolkit.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct KeyEvent {
  /// W3C physical key when Unity's public key code has a stable mapping.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub physical_key: Option<PhysicalKey>,
  /// Text produced by the key event, or an empty string for non-text keys.
  pub text: String,
  /// Physical modifiers active at dispatch.
  #[serde(default, skip_serializing_if = "KeyModifiers::is_empty")]
  pub modifiers: KeyModifiers,
}

/// Public UI navigation direction.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum NavigationDirection {
  /// No directional intent.
  #[default]
  None,
  /// Move left.
  Left,
  /// Move upward.
  Up,
  /// Move right.
  Right,
  /// Move downward.
  Down,
  /// Move to the next focusable target.
  Next,
  /// Move to the previous focusable target.
  Previous,
}

/// Direction and finite move vector from UI navigation.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct NavigationMoveEvent {
  /// Semantic navigation direction.
  pub direction: NavigationDirection,
  /// Raw native move vector.
  #[serde(rename = "move")]
  pub move_vector: Vector,
}

/// Empty payload for navigation submit and cancel.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct NavigationEvent {}

/// Public UI focus-change direction.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum FocusDirection {
  /// No direction was supplied.
  #[default]
  None,
  /// Unity reported its unspecified direction singleton.
  Unspecified,
  /// Focus moved left.
  Left,
  /// Focus moved right.
  Right,
  /// Project-defined focus direction value.
  Other(i32),
}

/// A three-dimensional displacement.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiVector3 {
  /// Horizontal component.
  pub x: f32,
  /// Vertical component.
  pub y: f32,
  /// Depth component.
  pub z: f32,
}

const fn one() -> u32 {
  1
}

fn is_one(value: &u32) -> bool {
  *value == 1
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
  /// Keyboard or gamepad submit converted into the focused Button's logical click.
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

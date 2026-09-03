//! Unity-local gesture, drag, scroll, and viewport authoring.

use std::any::TypeId;

use battlement::{
  CommandBody, MotionCallbackSubscriptions, MotionDragBounds, MotionDragConstraint,
  MotionDragControlOperation, MotionDragDescriptor, MotionDragElastic, MotionDragTransition,
  MotionGeneration, MotionGestureAxis, MotionGestureDescriptor, MotionGestureEvent,
  MotionGestureEventKind, MotionGestureSubscriptions, MotionLayer, MotionSlotDescriptor,
  MotionSlotId,
};

use crate::{
  element_ref::ElementRef,
  hook_storage::{HookKind, HookSlot},
  hooks,
  motion::{MotionProps, MotionTarget, Transition},
  motion_value::{ErasedMotionValue, MotionValue, MotionValueRuntimeHandle},
};

macro_rules! gesture_callbacks {
  ($($name:ident => $kind:ident),+ $(,)?) => {
    $(
      #[doc = concat!("Runs with native `", stringify!($kind), "` gesture event data.")]
      #[must_use]
      pub fn $name<G: 'static>(
        mut self,
        callback: impl Fn(&mut G, &MotionGestureEvent) + 'static,
      ) -> Self {
        self.callbacks = self
          .callbacks
          .gesture_event(MotionGestureEventKind::$kind, callback);
        self
      }
    )+
  };
}

/// Axes which a drag recognizer may own.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DragAxis {
  /// Horizontal movement only.
  X,
  /// Vertical movement only.
  Y,
  /// Independent movement on both axes.
  Both,
}

/// Fixed or element-backed drag constraints.
#[derive(Clone, Debug, PartialEq)]
pub enum DragConstraints {
  /// Inclusive panel-space offset bounds.
  Bounds {
    /// Minimum horizontal offset.
    min_x: f32,
    /// Maximum horizontal offset.
    max_x: f32,
    /// Minimum vertical offset.
    min_y: f32,
    /// Maximum vertical offset.
    max_y: f32,
  },
  /// Padding box of an attached host in the same panel.
  Element(ElementRef),
}

/// Per-edge elasticity beyond drag constraints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragElastic {
  left: f32,
  right: f32,
  top: f32,
  bottom: f32,
}

/// Release inertia and constraint spring behavior.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragTransition {
  velocity_retention: f32,
  rest_speed: f32,
  bounce_stiffness: f32,
  bounce_damping: f32,
}

/// Options for one externally initiated drag.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DragStartOptions {
  snap_to_cursor: bool,
}

/// Stable imperative handle which starts a bound host's drag.
#[derive(Clone)]
pub struct DragControls {
  handle: MotionValueRuntimeHandle,
  control_id: battlement::ObjectId,
}

/// Inherited gesture recognition thresholds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GestureConfig {
  /// Panel pixels required to start pan or drag.
  pub pan_threshold: f32,
  /// Panel pixels required to select one locked direction.
  pub direction_lock_threshold: f32,
  /// Mouse and pen tap slop.
  pub pointer_tap_slop: f32,
  /// Touch tap slop.
  pub touch_tap_slop: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GestureProps {
  pub(crate) hover: Option<MotionTarget>,
  pub(crate) tap: Option<MotionTarget>,
  pub(crate) focus: Option<MotionTarget>,
  pub(crate) focus_visible: Option<MotionTarget>,
  pub(crate) drag_target: Option<MotionTarget>,
  pub(crate) in_view_target: Option<MotionTarget>,
  pub(crate) config: GestureConfig,
  pub(crate) pan: bool,
  pub(crate) drag: Option<DragProps>,
  pub(crate) observe_scroll: bool,
  pub(crate) observe_in_view: bool,
  pub(crate) scroll_x_value: Option<ErasedMotionValue>,
  pub(crate) scroll_y_value: Option<ErasedMotionValue>,
  pub(crate) in_view_value: Option<ErasedMotionValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DragProps {
  axis: DragAxis,
  constraints: Option<DragConstraints>,
  elastic: DragElastic,
  momentum: bool,
  direction_lock: bool,
  listener: bool,
  snap_to_origin: Option<DragAxis>,
  control_id: Option<battlement::ObjectId>,
  propagation: bool,
  transition: DragTransition,
  x_value: Option<ErasedMotionValue>,
  y_value: Option<ErasedMotionValue>,
}

struct DragControlsSlot(DragControls);

/// Creates stable external drag controls in the current hook slot.
pub fn use_drag_controls() -> DragControls {
  hooks::use_slot(
    HookKind::DragControl,
    TypeId::of::<DragControls>(),
    |_| {
      DragControlsSlot(DragControls {
        handle: MotionValueRuntimeHandle::current(),
        control_id: battlement::ObjectId::new_v4(),
      })
    },
    |slot| slot.0.clone(),
  )
}

impl Default for GestureConfig {
  fn default() -> Self {
    Self {
      pan_threshold: 3.0,
      direction_lock_threshold: 10.0,
      pointer_tap_slop: 3.0,
      touch_tap_slop: 8.0,
    }
  }
}

impl Default for DragElastic {
  fn default() -> Self {
    Self::all(0.35)
  }
}

impl Default for DragTransition {
  fn default() -> Self {
    Self {
      velocity_retention: 0.02,
      rest_speed: 8.0,
      bounce_stiffness: 500.0,
      bounce_damping: 40.0,
    }
  }
}

impl DragElastic {
  /// Applies one elastic factor to every edge.
  #[must_use]
  pub fn all(value: f32) -> Self {
    validate_elastic(value);
    Self {
      left: value,
      right: value,
      top: value,
      bottom: value,
    }
  }

  /// Applies independent horizontal and vertical factors.
  #[must_use]
  pub fn axes(horizontal: f32, vertical: f32) -> Self {
    Self::sides(horizontal, horizontal, vertical, vertical)
  }

  /// Applies independent left, right, top, and bottom factors.
  #[must_use]
  pub fn sides(left: f32, right: f32, top: f32, bottom: f32) -> Self {
    for value in [left, right, top, bottom] {
      validate_elastic(value);
    }
    Self {
      left,
      right,
      top,
      bottom,
    }
  }
}

impl DragTransition {
  /// Creates Motion-compatible release defaults.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Sets exponential velocity retention after one second.
  #[must_use]
  pub fn velocity_retention(mut self, value: f32) -> Self {
    assert!(
      value.is_finite() && value > 0.0 && value < 1.0,
      "drag velocity retention must be between zero and one"
    );
    self.velocity_retention = value;
    self
  }

  /// Sets the terminal velocity in panel pixels per second.
  #[must_use]
  pub fn rest_speed(mut self, value: f32) -> Self {
    assert!(
      value.is_finite() && value >= 0.0,
      "drag rest speed must be finite and nonnegative"
    );
    self.rest_speed = value;
    self
  }

  /// Sets the boundary spring parameters.
  #[must_use]
  pub fn bounce(mut self, stiffness: f32, damping: f32) -> Self {
    assert!(
      stiffness.is_finite() && stiffness > 0.0,
      "drag bounce stiffness must be positive"
    );
    assert!(
      damping.is_finite() && damping > 0.0,
      "drag bounce damping must be positive"
    );
    self.bounce_stiffness = stiffness;
    self.bounce_damping = damping;
    self
  }
}

impl DragStartOptions {
  /// Centers the dragged host under the initiating pointer.
  #[must_use]
  pub fn snap_to_cursor(mut self, value: bool) -> Self {
    self.snap_to_cursor = value;
    self
  }
}

impl DragControls {
  /// Starts the bound host from one live pointer gesture event.
  pub fn start(&self, event: &MotionGestureEvent, options: DragStartOptions) {
    assert!(
      event.pointer_id >= 0,
      "external drag controls require a pointer event"
    );
    self
      .handle
      .queue(CommandBody::MotionDragControl(MotionDragControlOperation {
        control_id: self.control_id,
        pointer_id: event.pointer_id,
        device: event.device,
        point: event.point,
        snap_to_cursor: options.snap_to_cursor,
      }));
  }

  fn id(&self) -> battlement::ObjectId {
    self.control_id
  }
}

impl DragConstraints {
  /// Creates inclusive panel-space offset bounds.
  #[must_use]
  pub fn bounds(min_x: f32, max_x: f32, min_y: f32, max_y: f32) -> Self {
    assert!(
      [min_x, max_x, min_y, max_y].into_iter().all(f32::is_finite),
      "drag bounds must be finite"
    );
    assert!(min_x <= max_x && min_y <= max_y, "drag bounds are reversed");
    Self::Bounds {
      min_x,
      max_x,
      min_y,
      max_y,
    }
  }

  /// Uses the padding box of an attached host.
  #[must_use]
  pub fn element(value: ElementRef) -> Self {
    Self::Element(value)
  }
}

impl MotionProps {
  pub(crate) fn gesture_brief(
    mut self,
    kind: MotionGestureEventKind,
    callback: impl Fn() + 'static,
  ) -> Self {
    self.callbacks = self.callbacks.gesture_brief(kind, callback);
    self
  }

  /// Sets the locally activated hover target.
  #[must_use]
  pub fn while_hover(mut self, value: impl Into<MotionTarget>) -> Self {
    self.gestures.hover = Some(value.into());
    self
  }

  /// Sets the locally activated exact-focus target.
  #[must_use]
  pub fn while_focus(mut self, value: impl Into<MotionTarget>) -> Self {
    self.gestures.focus = Some(value.into());
    self
  }

  /// Sets the target activated by keyboard- or controller-visible focus.
  #[must_use]
  pub fn while_focus_visible(mut self, value: impl Into<MotionTarget>) -> Self {
    self.gestures.focus_visible = Some(value.into());
    self
  }

  /// Sets the locally activated tap target.
  #[must_use]
  pub fn while_tap(mut self, value: impl Into<MotionTarget>) -> Self {
    self.gestures.tap = Some(value.into());
    self
  }

  /// Sets the locally activated drag target.
  #[must_use]
  pub fn while_drag(mut self, value: impl Into<MotionTarget>) -> Self {
    self.gestures.drag_target = Some(value.into());
    self
  }

  /// Sets the viewport-entry target.
  #[must_use]
  pub fn while_in_view(mut self, value: impl Into<MotionTarget>) -> Self {
    self.gestures.in_view_target = Some(value.into());
    self.gestures.observe_in_view = true;
    self
  }

  /// Replaces local recognition thresholds.
  #[must_use]
  pub fn gesture_config(mut self, value: GestureConfig) -> Self {
    validate_config(value);
    self.gestures.config = value;
    self
  }

  /// Enables pan recognition without owning translation.
  #[must_use]
  pub fn pan(mut self, value: bool) -> Self {
    self.gestures.pan = value;
    self
  }

  /// Enables drag ownership on the selected axes.
  #[must_use]
  pub fn drag(mut self, axis: DragAxis) -> Self {
    self.gestures.drag = Some(DragProps::new(axis));
    self
  }

  /// Replaces drag constraints.
  #[must_use]
  pub fn drag_constraints(mut self, value: DragConstraints) -> Self {
    self.drag_mut().constraints = Some(value);
    self
  }

  /// Replaces per-edge elastic overshoot.
  #[must_use]
  pub fn drag_elastic(mut self, value: DragElastic) -> Self {
    self.drag_mut().elastic = value;
    self
  }

  /// Enables or disables release momentum.
  #[must_use]
  pub fn drag_momentum(mut self, value: bool) -> Self {
    self.drag_mut().momentum = value;
    self
  }

  /// Enables or disables ten-pixel direction locking.
  #[must_use]
  pub fn drag_direction_lock(mut self, value: bool) -> Self {
    self.drag_mut().direction_lock = value;
    self
  }

  /// Enables or disables pointer initiation on the draggable host.
  #[must_use]
  pub fn drag_listener(mut self, value: bool) -> Self {
    self.drag_mut().listener = value;
    self
  }

  /// Selects axes returned to the drag origin after release.
  #[must_use]
  pub fn drag_snap_to_origin(mut self, value: DragAxis) -> Self {
    self.drag_mut().snap_to_origin = Some(value);
    self
  }

  /// Allows an eligible ancestor to recognize the same pointer drag.
  #[must_use]
  pub fn drag_propagation(mut self, value: bool) -> Self {
    self.drag_mut().propagation = value;
    self
  }

  /// Replaces release inertia and boundary spring behavior.
  #[must_use]
  pub fn drag_transition(mut self, value: DragTransition) -> Self {
    self.drag_mut().transition = value;
    self
  }

  /// Binds native drag offsets to stable mutable motion values.
  #[must_use]
  pub fn drag_motion_values(mut self, x: MotionValue<f32>, y: MotionValue<f32>) -> Self {
    self.drag_mut().x_value = Some(x.erase());
    self.drag_mut().y_value = Some(y.erase());
    self
  }

  /// Binds one stable external drag-controls identity.
  #[must_use]
  pub fn drag_controls(mut self, value: DragControls) -> Self {
    self.drag_mut().control_id = Some(value.id());
    self
  }

  /// Observes native scroll offset through coalesced gesture samples.
  #[must_use]
  pub fn observe_scroll(mut self, value: bool) -> Self {
    self.gestures.observe_scroll = value;
    self
  }

  /// Binds native scroll offsets to stable mutable motion values.
  #[must_use]
  pub fn scroll_motion_values(mut self, x: MotionValue<f32>, y: MotionValue<f32>) -> Self {
    self.gestures.observe_scroll = true;
    self.gestures.scroll_x_value = Some(x.erase());
    self.gestures.scroll_y_value = Some(y.erase());
    self
  }

  /// Binds viewport membership to a stable mutable zero-or-one motion value.
  #[must_use]
  pub fn in_view_motion_value(mut self, value: MotionValue<f32>) -> Self {
    self.gestures.observe_in_view = true;
    self.gestures.in_view_value = Some(value.erase());
    self
  }

  gesture_callbacks! {
    on_hover_start => HoverStart,
    on_hover_end => HoverEnd,
    on_tap_start => TapStart,
    on_tap => Tap,
    on_tap_cancel => TapCancel,
    on_focus_start => FocusStart,
    on_focus_end => FocusEnd,
    on_focus_visible_start => FocusVisibleStart,
    on_focus_visible_end => FocusVisibleEnd,
    on_pan_session_start => PanSessionStart,
    on_pan_start => PanStart,
    on_pan => Pan,
    on_pan_end => PanEnd,
    on_pan_cancel => PanCancel,
    on_drag_start => DragStart,
    on_drag_direction_lock => DragDirectionLock,
    on_drag => Drag,
    on_drag_end => DragEnd,
    on_drag_cancel => DragCancel,
    on_drag_momentum_complete => DragMomentumComplete,
    on_drag_constraints_measured => DragConstraintsMeasured,
    on_scroll_motion => Scroll,
    on_viewport_enter => InViewEnter,
    on_viewport_leave => InViewLeave,
  }

  pub(crate) fn gesture_slots(
    &self,
    generation: MotionGeneration,
    transition: Option<&Transition>,
  ) -> Vec<MotionSlotDescriptor> {
    [
      (2, MotionLayer::Hover, &self.gestures.hover),
      (3, MotionLayer::Focus, &self.gestures.focus),
      (7, MotionLayer::FocusVisible, &self.gestures.focus_visible),
      (4, MotionLayer::Tap, &self.gestures.tap),
      (5, MotionLayer::Drag, &self.gestures.drag_target),
      (6, MotionLayer::InView, &self.gestures.in_view_target),
    ]
    .into_iter()
    .filter_map(|(slot, layer, target)| {
      target.as_ref().map(|target| MotionSlotDescriptor {
        slot: MotionSlotId(slot),
        generation,
        layer,
        target: target.descriptor(transition, 0),
        callbacks: MotionCallbackSubscriptions::default(),
      })
    })
    .collect()
  }

  pub(crate) fn gesture_descriptor(&self) -> Option<MotionGestureDescriptor> {
    let subscriptions = self.callbacks.gesture_subscriptions();
    let value = &self.gestures;
    let enabled = value.hover.is_some()
      || value.tap.is_some()
      || value.focus.is_some()
      || value.drag_target.is_some()
      || value.in_view_target.is_some()
      || value.pan
      || value.drag.is_some()
      || value.observe_scroll
      || value.observe_in_view
      || subscriptions != MotionGestureSubscriptions::default();
    enabled.then(|| MotionGestureDescriptor {
      pan_threshold: value.config.pan_threshold,
      direction_lock_threshold: value.config.direction_lock_threshold,
      pointer_tap_slop: value.config.pointer_tap_slop,
      touch_tap_slop: value.config.touch_tap_slop,
      pan: value.pan,
      drag: value.drag.as_ref().map(DragProps::descriptor),
      in_view: value.observe_in_view,
      scroll: value.observe_scroll,
      scroll_x_value: value.scroll_x_value.as_ref().map(ErasedMotionValue::id),
      scroll_y_value: value.scroll_y_value.as_ref().map(ErasedMotionValue::id),
      in_view_value: value.in_view_value.as_ref().map(ErasedMotionValue::id),
      subscriptions,
    })
  }

  pub(crate) fn gesture_graph_values(&self) -> Vec<battlement::MotionValueDescriptor> {
    let mut values = Vec::new();
    if let Some(drag) = &self.gestures.drag {
      for value in [&drag.x_value, &drag.y_value].into_iter().flatten() {
        value.collect(&mut values);
      }
    }
    for value in [
      &self.gestures.scroll_x_value,
      &self.gestures.scroll_y_value,
      &self.gestures.in_view_value,
    ]
    .into_iter()
    .flatten()
    {
      value.collect(&mut values);
    }
    values
  }

  pub(crate) fn gesture_value_subscriptions(&self) -> Vec<battlement::MotionValueSubscription> {
    let mut subscriptions = Vec::new();
    if let Some(drag) = &self.gestures.drag {
      for value in [&drag.x_value, &drag.y_value].into_iter().flatten() {
        value.collect_subscriptions(&mut subscriptions);
      }
    }
    for value in [
      &self.gestures.scroll_x_value,
      &self.gestures.scroll_y_value,
      &self.gestures.in_view_value,
    ]
    .into_iter()
    .flatten()
    {
      value.collect_subscriptions(&mut subscriptions);
    }
    subscriptions
  }

  fn drag_mut(&mut self) -> &mut DragProps {
    self
      .gestures
      .drag
      .as_mut()
      .expect("drag options require drag(axis)")
  }
}

impl GestureProps {
  pub(crate) const fn new() -> Self {
    Self {
      hover: None,
      tap: None,
      focus: None,
      focus_visible: None,
      drag_target: None,
      in_view_target: None,
      config: GestureConfig {
        pan_threshold: 3.0,
        direction_lock_threshold: 10.0,
        pointer_tap_slop: 3.0,
        touch_tap_slop: 8.0,
      },
      pan: false,
      drag: None,
      observe_scroll: false,
      observe_in_view: false,
      scroll_x_value: None,
      scroll_y_value: None,
      in_view_value: None,
    }
  }

  pub(crate) fn merge(mut self, value: Self) -> Self {
    if value.hover.is_some() {
      self.hover = value.hover;
    }
    if value.tap.is_some() {
      self.tap = value.tap;
    }
    if value.focus.is_some() {
      self.focus = value.focus;
    }
    if value.focus_visible.is_some() {
      self.focus_visible = value.focus_visible;
    }
    if value.drag_target.is_some() {
      self.drag_target = value.drag_target;
    }
    if value.in_view_target.is_some() {
      self.in_view_target = value.in_view_target;
    }
    if value.config != GestureConfig::default() {
      self.config = value.config;
    }
    self.pan |= value.pan;
    if value.drag.is_some() {
      self.drag = value.drag;
    }
    self.observe_scroll |= value.observe_scroll;
    self.observe_in_view |= value.observe_in_view;
    if value.scroll_x_value.is_some() {
      self.scroll_x_value = value.scroll_x_value;
    }
    if value.scroll_y_value.is_some() {
      self.scroll_y_value = value.scroll_y_value;
    }
    if value.in_view_value.is_some() {
      self.in_view_value = value.in_view_value;
    }
    self
  }

  pub(crate) fn drag_constraint_ref(&self) -> Option<&ElementRef> {
    self.drag.as_ref().and_then(|drag| {
      drag
        .constraints
        .as_ref()
        .and_then(|constraints| match constraints {
          DragConstraints::Bounds { .. } => None,
          DragConstraints::Element(element_ref) => Some(element_ref),
        })
    })
  }
}

impl Default for GestureProps {
  fn default() -> Self {
    Self::new()
  }
}

impl DragProps {
  fn new(axis: DragAxis) -> Self {
    Self {
      axis,
      constraints: None,
      elastic: DragElastic::default(),
      momentum: true,
      direction_lock: false,
      listener: true,
      snap_to_origin: None,
      control_id: None,
      propagation: false,
      transition: DragTransition::default(),
      x_value: None,
      y_value: None,
    }
  }

  fn descriptor(&self) -> MotionDragDescriptor {
    MotionDragDescriptor {
      axis: axis(self.axis),
      constraints: self.constraints.as_ref().and_then(|value| match value {
        DragConstraints::Bounds {
          min_x,
          max_x,
          min_y,
          max_y,
        } => Some(MotionDragConstraint::Bounds(MotionDragBounds {
          min_x: *min_x,
          max_x: *max_x,
          min_y: *min_y,
          max_y: *max_y,
        })),
        DragConstraints::Element(_) => None,
      }),
      elastic: MotionDragElastic {
        left: self.elastic.left,
        right: self.elastic.right,
        top: self.elastic.top,
        bottom: self.elastic.bottom,
      },
      momentum: self.momentum,
      direction_lock: self.direction_lock,
      listener: self.listener,
      snap_to_origin: self.snap_to_origin.map(axis),
      control_id: self.control_id,
      propagation: self.propagation,
      transition: MotionDragTransition {
        velocity_retention: self.transition.velocity_retention,
        rest_speed: self.transition.rest_speed,
        bounce_stiffness: self.transition.bounce_stiffness,
        bounce_damping: self.transition.bounce_damping,
      },
      x_value: self.x_value.as_ref().map(ErasedMotionValue::id),
      y_value: self.y_value.as_ref().map(ErasedMotionValue::id),
    }
  }
}

impl HookSlot for DragControlsSlot {
  fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
    self
  }

  fn clone_box(&self) -> Box<dyn HookSlot> {
    Box::new(Self(self.0.clone()))
  }

  fn commit(&mut self) {}

  fn discard_pending(&mut self) {}

  fn has_pending(&self) -> bool {
    false
  }

  fn has_pending_change(&self) -> bool {
    false
  }

  fn context_changed(&self) -> bool {
    false
  }

  fn kind(&self) -> HookKind {
    HookKind::DragControl
  }

  fn value_type(&self) -> TypeId {
    TypeId::of::<DragControls>()
  }
}

const fn axis(value: DragAxis) -> MotionGestureAxis {
  match value {
    DragAxis::X => MotionGestureAxis::X,
    DragAxis::Y => MotionGestureAxis::Y,
    DragAxis::Both => MotionGestureAxis::Both,
  }
}

fn validate_elastic(value: f32) {
  assert!(
    value.is_finite() && (0.0..=1.0).contains(&value),
    "drag elasticity must be between zero and one"
  );
}

fn validate_config(value: GestureConfig) {
  for threshold in [
    value.pan_threshold,
    value.direction_lock_threshold,
    value.pointer_tap_slop,
    value.touch_tap_slop,
  ] {
    assert!(
      threshold.is_finite() && threshold >= 0.0,
      "gesture thresholds must be finite and nonnegative"
    );
  }
}

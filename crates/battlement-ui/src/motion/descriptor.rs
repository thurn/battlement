use std::collections::HashSet;

use battlement_types::ObjectId;
use serde::{Deserialize, Serialize};

use crate::MotionVariantResolution;
use crate::{
  CssAnimationDescriptor, MotionDecorationDescriptor, MotionPseudoStyle, StyleTransitionDescriptor,
};
use crate::{
  MotionNamedTarget, MotionValueBinding, MotionValueDescriptor, MotionValueSource,
  MotionValueSubscription,
};
use crate::{MotionProperty, MotionValue, MotionValueKind, TransitionDefinition};

/// Stable animation slot identity scoped to one descriptor.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct MotionSlotId(pub u64);

/// Generation checked by Unity when updating or dispatching one animation slot.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct MotionGeneration(pub u32);

/// Monotonic reliable-event sequence scoped to one transport session.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct MotionSequence(pub u64);

/// Property and its target or typed keyframe sequence.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionPropertyTrack {
  /// Catalog property identity.
  pub property: MotionProperty,
  /// One or more property values; one value is a constant target.
  pub values: Vec<MotionValue>,
  /// Optional property-local normalized times.
  pub times: Option<Vec<f64>>,
  /// Fully resolved timing for this property.
  pub transition: TransitionDefinition,
}

impl MotionPropertyTrack {
  /// Validates finite values, property shapes, times, and transition compatibility.
  pub fn validate(&self) -> Result<(), String> {
    if self.values.is_empty() {
      return Err(format!(
        "motion property {} has an empty keyframe sequence",
        self.property.metadata().wire_name
      ));
    }
    for value in &self.values {
      value.validate_for(self.property).map_err(str::to_owned)?;
      if !value_matches(self.property.metadata().value_kind, value) {
        return Err(format!(
          "motion property {} received an incompatible value shape",
          self.property.metadata().wire_name
        ));
      }
    }
    if let Some(times) = &self.times {
      if times.len() != self.values.len() {
        return Err(format!(
          "motion property {} has mismatched keyframe times",
          self.property.metadata().wire_name
        ));
      }
      validate_times(times)?;
    }
    self.transition.validate().map_err(str::to_owned)?;
    if self.property.metadata().interpolation == crate::InterpolationCategory::Discrete
      && matches!(
        self.transition.generator,
        crate::TransitionGenerator::Spring(_)
      )
    {
      return Err(format!(
        "discrete motion property {} cannot use a spring",
        self.property.metadata().wire_name
      ));
    }
    Ok(())
  }
}

/// One flattened target layer with unique property ownership.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct MotionTargetDescriptor {
  /// Independently sampled property tracks.
  pub tracks: Vec<MotionPropertyTrack>,
  /// Values assigned atomically after successful finite completion.
  pub transition_end: Vec<MotionPropertyValue>,
}

impl MotionTargetDescriptor {
  /// Validates that each property has one owner in this layer.
  pub fn validate(&self) -> Result<(), String> {
    let mut properties = HashSet::new();
    for track in &self.tracks {
      track.validate()?;
      if !properties.insert(track.property) {
        return Err(format!(
          "motion target repeats property {}",
          track.property.metadata().wire_name
        ));
      }
    }
    properties.clear();
    for value in &self.transition_end {
      value.validate()?;
      if !properties.insert(value.property) {
        return Err(format!(
          "transition_end repeats property {}",
          value.property.metadata().wire_name
        ));
      }
    }
    Ok(())
  }
}

/// One property assignment outside a sampled timeline.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionPropertyValue {
  /// Catalog property identity.
  pub property: MotionProperty,
  /// Normalized value.
  pub value: MotionValue,
}

impl MotionPropertyValue {
  /// Validates the value and catalog-declared shape.
  pub fn validate(&self) -> Result<(), String> {
    self
      .value
      .validate_for(self.property)
      .map_err(str::to_owned)?;
    if !value_matches(self.property.metadata().value_kind, &self.value) {
      return Err(format!(
        "motion property {} received an incompatible value shape",
        self.property.metadata().wire_name
      ));
    }
    Ok(())
  }
}

/// Descriptor layer priority after variant resolution.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum MotionLayer {
  /// Declarative animate or imperative controls.
  Animate,
  /// Viewport-entry gesture layer.
  InView,
  /// Exact-focus layer.
  Focus,
  /// Exact focus entered through keyboard or controller modality.
  FocusVisible,
  /// Pointer hover layer.
  Hover,
  /// Pointer/submit tap layer.
  Tap,
  /// Active drag layer.
  Drag,
  /// Presence exit layer.
  Exit,
}

/// Axis ownership for pan and drag recognition.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum MotionGestureAxis {
  /// Horizontal movement only.
  X,
  /// Vertical movement only.
  Y,
  /// Independent movement on both axes.
  Both,
}

/// Pointer or navigation device which owns a gesture.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum MotionPointerDevice {
  /// Mouse pointer.
  Mouse,
  /// Pen pointer.
  Pen,
  /// Touch-compatible pointer.
  Touch,
  /// Keyboard submit.
  Keyboard,
  /// Gamepad submit.
  Gamepad,
}

/// Panel-space point or vector carried by a gesture event.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct MotionGestureVector {
  /// Horizontal panel pixels.
  pub x: f32,
  /// Vertical panel pixels.
  pub y: f32,
}

/// Fixed panel-space drag bounds.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionDragBounds {
  /// Minimum horizontal offset.
  pub min_x: f32,
  /// Maximum horizontal offset.
  pub max_x: f32,
  /// Minimum vertical offset.
  pub min_y: f32,
  /// Maximum vertical offset.
  pub max_y: f32,
}

/// Source used to resolve drag bounds locally.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum MotionDragConstraint {
  /// Authored panel-space bounds.
  Bounds(MotionDragBounds),
  /// Padding box of another host in the same panel.
  Element(ObjectId),
}

/// Per-edge elasticity applied beyond drag bounds.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionDragElastic {
  /// Overshoot multiplier at the left edge.
  pub left: f32,
  /// Overshoot multiplier at the right edge.
  pub right: f32,
  /// Overshoot multiplier at the top edge.
  pub top: f32,
  /// Overshoot multiplier at the bottom edge.
  pub bottom: f32,
}

/// Native release-inertia and boundary-spring settings.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionDragTransition {
  /// Exponential velocity retention per second, between zero and one.
  pub velocity_retention: f32,
  /// Velocity below which inertia completes, in panel pixels per second.
  pub rest_speed: f32,
  /// Spring stiffness used when an offset finishes beyond a constraint.
  pub bounce_stiffness: f32,
  /// Spring damping used when an offset finishes beyond a constraint.
  pub bounce_damping: f32,
}

/// Native drag behavior attached to one host.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionDragDescriptor {
  /// Axes owned by drag.
  pub axis: MotionGestureAxis,
  /// Optional fixed or measured constraints.
  pub constraints: Option<MotionDragConstraint>,
  /// Per-edge elastic overshoot.
  pub elastic: MotionDragElastic,
  /// Whether release velocity starts inertia.
  pub momentum: bool,
  /// Whether drag waits for a ten-pixel direction lock.
  pub direction_lock: bool,
  /// Whether the host itself listens for pointer initiation.
  pub listener: bool,
  /// Axes returned to their origin after release.
  pub snap_to_origin: Option<MotionGestureAxis>,
  /// Optional stable external-control binding.
  pub control_id: Option<ObjectId>,
  /// Whether an eligible ancestor may recognize the same pointer drag.
  pub propagation: bool,
  /// Release-inertia and boundary-spring settings.
  pub transition: MotionDragTransition,
  /// Optional mutable motion value receiving the horizontal offset.
  pub x_value: Option<ObjectId>,
  /// Optional mutable motion value receiving the vertical offset.
  pub y_value: Option<ObjectId>,
}

/// Explicit callback subscriptions for gesture boundaries and samples.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct MotionGestureSubscriptions {
  /// Hover entered or left.
  pub hover: bool,
  /// Tap began, completed, or cancelled.
  pub tap: bool,
  /// Exact focus changed.
  pub focus: bool,
  /// Keyboard- or controller-visible focus changed.
  pub focus_visible: bool,
  /// Pan session and threshold boundaries.
  pub pan: bool,
  /// Coalesced pan movement.
  pub pan_update: bool,
  /// Drag boundaries and direction lock.
  pub drag: bool,
  /// Coalesced pointer-driven drag movement.
  pub drag_update: bool,
  /// Momentum reached its terminal offset.
  pub momentum_complete: bool,
  /// Drag constraints were measured.
  pub constraints_measured: bool,
  /// Scroll offset or progress changed.
  pub scroll: bool,
  /// Viewport entry changed.
  pub in_view: bool,
}

/// Unity-local gesture recognition and presentation configuration.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionGestureDescriptor {
  /// Panel pixels required to start pan or drag.
  pub pan_threshold: f32,
  /// Panel pixels required to lock one drag direction.
  pub direction_lock_threshold: f32,
  /// Mouse and pen tap slop in panel pixels.
  pub pointer_tap_slop: f32,
  /// Touch tap slop in panel pixels.
  pub touch_tap_slop: f32,
  /// Whether pan recognition is enabled.
  pub pan: bool,
  /// Optional drag behavior.
  pub drag: Option<MotionDragDescriptor>,
  /// Whether viewport intersection is observed.
  pub in_view: bool,
  /// Whether native scroll progress is observed.
  pub scroll: bool,
  /// Optional mutable motion value receiving horizontal scroll offset.
  pub scroll_x_value: Option<ObjectId>,
  /// Optional mutable motion value receiving vertical scroll offset.
  pub scroll_y_value: Option<ObjectId>,
  /// Optional mutable motion value receiving zero outside and one inside the viewport.
  pub in_view_value: Option<ObjectId>,
  /// Callback subscription set.
  pub subscriptions: MotionGestureSubscriptions,
}

/// One independently identified layer slot.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionSlotDescriptor {
  /// Stable identity in the host descriptor.
  pub slot: MotionSlotId,
  /// Current generation.
  pub generation: MotionGeneration,
  /// Layer priority.
  pub layer: MotionLayer,
  /// Flattened target.
  pub target: MotionTargetDescriptor,
  /// Lifecycle events requested by Rust.
  pub callbacks: MotionCallbackSubscriptions,
}

/// Explicit lifecycle subscription set for one slot.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct MotionCallbackSubscriptions {
  /// Emits when the first track leaves delay.
  pub start: bool,
  /// Emits at most once per rendered frame.
  pub update: bool,
  /// Emits crossed repeat boundaries.
  pub repeat: bool,
  /// Emits successful finite completion.
  pub complete: bool,
  /// Emits explicit stop.
  pub stop: bool,
  /// Emits cancellation or supersession.
  pub cancel: bool,
}

/// Clock selected for one descriptor.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum MotionClockSource {
  /// Unity unscaled runtime time.
  Unscaled,
  /// Unity scaled game time.
  Scaled,
  /// A test-owned controlled clock.
  Controlled(ObjectId),
  /// A Battlement-owned audio playback operation.
  Audio(ObjectId),
}

/// Resolved reduced-motion policy sent to Unity.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ReducedMotionPolicy {
  /// Observe the supported platform bridge.
  User,
  /// Suppress spatial tracks.
  Always,
  /// Never suppress authored tracks.
  Never,
}

/// Projection axes applied after one native layout pass.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum MotionLayoutMode {
  /// Project panel-space position only.
  Position,
  /// Project rendered size only.
  Size,
  /// Project both position and size.
  Both,
}

/// Stable typed identity used by layout groups and shared layout handoffs.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct MotionLayoutIdentity {
  /// Rust type which owns the identity value.
  pub value_type: String,
  /// Stable hash of the typed value.
  pub value_hash: u64,
}

/// Native layout-projection configuration for one host.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionLayoutDescriptor {
  /// Projection axes.
  pub mode: MotionLayoutMode,
  /// Nearest logical layout group.
  pub group: MotionLayoutIdentity,
  /// Optional shared-layout identity within the group.
  pub layout_id: Option<MotionLayoutIdentity>,
  /// Whether this host contributes scroll offset to projected descendants.
  pub scroll: bool,
  /// Whether this host establishes a fixed projection root.
  pub root: bool,
  /// Whether an exiting host is removed from native layout flow.
  pub pop_layout: bool,
  /// Timing used to animate the inverse projection to identity.
  pub transition: TransitionDefinition,
}

/// Complete validated animation state installed beside one UI host.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionDescriptor {
  /// Stable descriptor identity across updates.
  pub descriptor_id: ObjectId,
  /// Host receiving the final presentation values.
  pub host_id: ObjectId,
  /// Committed descriptor generation.
  pub generation: MotionGeneration,
  /// Optional mount origin; omission starts at current presentation.
  pub initial: Option<MotionTargetDescriptor>,
  /// Whether initial animation is explicitly disabled.
  pub initial_disabled: bool,
  /// Ordered active and inactive layer slots.
  pub slots: Vec<MotionSlotDescriptor>,
  /// Inherited clock source.
  pub clock: MotionClockSource,
  /// Inherited reduced-motion policy.
  pub reduced_motion: ReducedMotionPolicy,
  /// Locally resolved pseudo-state style overlays.
  #[serde(default)]
  pub pseudo_styles: Vec<MotionPseudoStyle>,
  /// CSS transitions over resolved static style changes.
  #[serde(default)]
  pub style_transition: StyleTransitionDescriptor,
  /// Ordered reusable CSS animation slots.
  #[serde(default)]
  pub animations: Vec<CssAnimationDescriptor>,
  /// Non-interactive keyed paint layers.
  #[serde(default)]
  pub decorations: Vec<MotionDecorationDescriptor>,
  /// Inspectable logical-variant resolution facts.
  #[serde(default)]
  pub variants: Option<MotionVariantResolution>,
  /// Deduplicated value nodes required by this host's bindings and subscriptions.
  #[serde(default)]
  pub values: Vec<MotionValueDescriptor>,
  /// Host properties driven by graph values.
  #[serde(default)]
  pub value_bindings: Vec<MotionValueBinding>,
  /// Explicit Rust-side value observations.
  #[serde(default)]
  pub value_subscriptions: Vec<MotionValueSubscription>,
  /// Optional animation-controls binding.
  #[serde(default)]
  pub control_id: Option<ObjectId>,
  /// Optional animation-scope root identity.
  #[serde(default)]
  pub scope_id: Option<ObjectId>,
  /// Whether this host is the scope root.
  #[serde(default)]
  pub scope_root: bool,
  /// Optional closed selector name.
  #[serde(default)]
  pub motion_name: Option<String>,
  /// Named targets resolved for imperative starts.
  #[serde(default)]
  pub named_targets: Vec<MotionNamedTarget>,
  /// Unity-local gesture recognizers and drag behavior.
  #[serde(default)]
  pub gestures: Option<MotionGestureDescriptor>,
  /// Optional layout projection and shared-layout configuration.
  #[serde(default)]
  pub layout: Option<MotionLayoutDescriptor>,
}

impl MotionDescriptor {
  /// Validates complete descriptor identity and property ownership.
  pub fn validate(&self) -> Result<(), String> {
    if self.initial_disabled && self.initial.is_some() {
      return Err("disabled initial target cannot also carry tracks".to_owned());
    }
    if let Some(initial) = &self.initial {
      initial.validate()?;
    }
    let mut slots = HashSet::new();
    for slot in &self.slots {
      if !slots.insert(slot.slot) {
        return Err(format!("motion descriptor repeats slot {}", slot.slot.0));
      }
      slot.target.validate()?;
    }
    if let Some(variants) = &self.variants {
      variants.validate()?;
    }
    crate::validate_motion_graph(
      &self.values,
      &self.value_bindings,
      &self.value_subscriptions,
    )?;
    validate_gestures(self)?;
    validate_css(self)?;
    Ok(())
  }
}

fn validate_gestures(descriptor: &MotionDescriptor) -> Result<(), String> {
  let Some(gestures) = descriptor.gestures else {
    return Ok(());
  };
  for (name, value) in [
    ("pan threshold", gestures.pan_threshold),
    (
      "direction-lock threshold",
      gestures.direction_lock_threshold,
    ),
    ("pointer tap slop", gestures.pointer_tap_slop),
    ("touch tap slop", gestures.touch_tap_slop),
  ] {
    if !value.is_finite() || value < 0.0 {
      return Err(format!(
        "motion gesture {name} must be finite and nonnegative"
      ));
    }
  }
  if let Some(drag) = gestures.drag {
    for value in [
      drag.elastic.left,
      drag.elastic.right,
      drag.elastic.top,
      drag.elastic.bottom,
    ] {
      if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err("motion drag elasticity must be between zero and one".to_owned());
      }
    }
    if let Some(MotionDragConstraint::Bounds(bounds)) = drag.constraints
      && (!bounds.min_x.is_finite()
        || !bounds.max_x.is_finite()
        || !bounds.min_y.is_finite()
        || !bounds.max_y.is_finite()
        || bounds.min_x > bounds.max_x
        || bounds.min_y > bounds.max_y)
    {
      return Err("motion drag bounds are invalid".to_owned());
    }
    let transition = drag.transition;
    if !transition.velocity_retention.is_finite()
      || !(0.0..1.0).contains(&transition.velocity_retention)
      || !transition.rest_speed.is_finite()
      || transition.rest_speed < 0.0
      || !transition.bounce_stiffness.is_finite()
      || transition.bounce_stiffness <= 0.0
      || !transition.bounce_damping.is_finite()
      || transition.bounce_damping <= 0.0
    {
      return Err("motion drag transition is invalid".to_owned());
    }
    for value_id in [drag.x_value, drag.y_value].into_iter().flatten() {
      let value = descriptor
        .values
        .iter()
        .find(|value| value.value_id == value_id)
        .ok_or_else(|| "motion drag value is unavailable".to_owned())?;
      if !matches!(value.source, MotionValueSource::Mutable) {
        return Err("motion drag values must be mutable".to_owned());
      }
    }
  }
  for value_id in [
    gestures.scroll_x_value,
    gestures.scroll_y_value,
    gestures.in_view_value,
  ]
  .into_iter()
  .flatten()
  {
    let value = descriptor
      .values
      .iter()
      .find(|value| value.value_id == value_id)
      .ok_or_else(|| "motion gesture value is unavailable".to_owned())?;
    if !matches!(value.source, MotionValueSource::Mutable) {
      return Err("motion gesture values must be mutable".to_owned());
    }
  }
  Ok(())
}

fn validate_css(descriptor: &MotionDescriptor) -> Result<(), String> {
  let motion_properties = descriptor
    .slots
    .iter()
    .flat_map(|slot| slot.target.tracks.iter().map(|track| track.property))
    .collect::<HashSet<_>>();
  let mut transition_properties = HashSet::new();
  for entry in &descriptor.style_transition.properties {
    if !transition_properties.insert(entry.property) {
      return Err("style transitions repeat a property".to_owned());
    }
    entry.transition.validate().map_err(str::to_owned)?;
    if !matches!(
      entry.transition.generator,
      crate::TransitionGenerator::Immediate | crate::TransitionGenerator::Tween { .. }
    ) {
      return Err("style transitions accept only tween or immediate timing".to_owned());
    }
  }
  if let Some(transition) = &descriptor.style_transition.all {
    transition.validate().map_err(str::to_owned)?;
    if !matches!(
      transition.generator,
      crate::TransitionGenerator::Immediate | crate::TransitionGenerator::Tween { .. }
    ) {
      return Err("style transitions accept only tween or immediate timing".to_owned());
    }
    if !motion_properties.is_empty() {
      return Err("style transition `all` conflicts with Motion-owned properties".to_owned());
    }
  }
  let mut animation_slots = descriptor
    .slots
    .iter()
    .map(|slot| slot.slot.0)
    .collect::<HashSet<_>>();
  for animation in &descriptor.animations {
    if !animation_slots.insert(animation.slot) {
      return Err(format!("CSS animations repeat slot {}", animation.slot));
    }
    let mut properties = HashSet::new();
    for track in &animation.tracks {
      validate_css_track(track)?;
      if !properties.insert(track.property) {
        return Err("CSS animation repeats a property track".to_owned());
      }
      if motion_properties.contains(&track.property)
        || transition_properties.contains(&track.property)
        || descriptor.style_transition.all.is_some()
      {
        return Err(format!(
          "property {} has conflicting Motion and CSS owners",
          track.property.metadata().wire_name
        ));
      }
      if animation.composition != crate::AnimationComposition::Replace
        && track.property.metadata().additive == crate::AdditiveRule::None
      {
        return Err(format!(
          "property {} does not support additive animation",
          track.property.metadata().wire_name
        ));
      }
      validate_additive_track(animation, track)?;
    }
  }
  for property in transition_properties {
    if motion_properties.contains(&property) {
      return Err(format!(
        "property {} has conflicting Motion and CSS owners",
        property.metadata().wire_name
      ));
    }
  }
  let mut pseudo_states = HashSet::new();
  for style in &descriptor.pseudo_styles {
    if !pseudo_states.insert(style.state) {
      return Err("pseudo styles repeat a state".to_owned());
    }
    let mut properties = HashSet::new();
    for value in &style.values {
      value.validate()?;
      if !properties.insert(value.property) {
        return Err("pseudo style repeats a property".to_owned());
      }
    }
  }
  let mut decoration_keys = HashSet::new();
  for decoration in &descriptor.decorations {
    if !decoration_keys.insert(decoration.key) {
      return Err(format!("decorations repeat key {}", decoration.key));
    }
    let mut slots = HashSet::new();
    for animation in &decoration.animations {
      if !slots.insert(animation.slot) {
        return Err(format!(
          "decoration {} repeats animation slot {}",
          decoration.key, animation.slot
        ));
      }
      let mut properties = HashSet::new();
      for track in &animation.tracks {
        validate_css_track(track)?;
        if !properties.insert(track.property) {
          return Err("decoration animation repeats a property track".to_owned());
        }
        if animation.composition != crate::AnimationComposition::Replace
          && track.property.metadata().additive == crate::AdditiveRule::None
        {
          return Err(format!(
            "property {} does not support additive animation",
            track.property.metadata().wire_name
          ));
        }
        validate_additive_track(animation, track)?;
      }
    }
  }
  Ok(())
}

fn validate_additive_track(
  animation: &crate::CssAnimationDescriptor,
  track: &crate::CssPropertyTrack,
) -> Result<(), String> {
  if animation.composition == crate::AnimationComposition::Replace
    || track.property.metadata().additive != crate::AdditiveRule::Transform
  {
    return Ok(());
  }
  let Some(crate::MotionValue::TransformList(first)) = track.values.first() else {
    return Err("additive transform track requires transform-list values".to_owned());
  };
  for value in &track.values[1..] {
    let crate::MotionValue::TransformList(current) = value else {
      return Err("additive transform track requires transform-list values".to_owned());
    };
    if current.len() != first.len()
      || current
        .iter()
        .zip(first)
        .any(|(left, right)| std::mem::discriminant(left) != std::mem::discriminant(right))
    {
      return Err("additive transform lists require compatible operations".to_owned());
    }
  }
  Ok(())
}

fn validate_css_track(track: &crate::CssPropertyTrack) -> Result<(), String> {
  if track.values.is_empty() || track.values.len() != track.times.len() {
    return Err("CSS property values and times must be nonempty and aligned".to_owned());
  }
  track.transition.validate().map_err(str::to_owned)?;
  let mut prior = 0.0;
  for (index, time) in track.times.iter().copied().enumerate() {
    if !time.is_finite() || !(0.0..=1.0).contains(&time) || (index != 0 && time < prior) {
      return Err("CSS keyframe times must be finite and nondecreasing in 0..=1".to_owned());
    }
    prior = time;
  }
  for value in &track.values {
    value.validate_for(track.property).map_err(str::to_owned)?;
    if !value_matches(track.property.metadata().value_kind, value) {
      return Err(format!(
        "motion property {} received an incompatible value shape",
        track.property.metadata().wire_name
      ));
    }
  }
  Ok(())
}

/// Renderer declaration for one supported property and value shape.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MotionRendererCapability {
  /// Property supported by the writer.
  pub property: MotionProperty,
  /// Exact value shape accepted by the writer.
  pub value_kind: MotionValueKind,
}

/// Reliable lifecycle event kind.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum MotionEventKind {
  /// Descriptor activation acknowledgement.
  Activated,
  /// First track left delay.
  Started,
  /// One or more repeat boundaries were crossed.
  Repeated {
    /// First crossed logical iteration.
    first: u32,
    /// Last crossed logical iteration.
    last: u32,
  },
  /// All finite tracks completed.
  Completed,
  /// Playback stopped at its presentation value.
  Stopped,
  /// Playback was cancelled and its slot removed.
  Cancelled,
}

/// One reliable slot lifecycle boundary.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionLifecycleEvent {
  /// Transport sequence.
  pub sequence: MotionSequence,
  /// Descriptor identity.
  pub descriptor_id: ObjectId,
  /// Slot identity.
  pub slot: MotionSlotId,
  /// Slot generation.
  pub generation: MotionGeneration,
  /// Logical elapsed time at the boundary.
  pub elapsed_micros: u64,
  /// Boundary kind.
  pub kind: MotionEventKind,
}

/// Replaceable presentation sample excluded from reliable sequence positions.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionPresentationSample {
  /// Descriptor identity.
  pub descriptor_id: ObjectId,
  /// Slot identity.
  pub slot: MotionSlotId,
  /// Slot generation.
  pub generation: MotionGeneration,
  /// Logical elapsed time.
  pub elapsed_micros: u64,
  /// Resolved values owned by this slot.
  pub values: Vec<MotionPropertyValue>,
}

/// Reliable or replaceable native gesture event kind.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum MotionGestureEventKind {
  /// A non-touch pointer entered the host.
  HoverStart,
  /// The owning hover pointer left the host.
  HoverEnd,
  /// Pointer or submit tap recognition began.
  TapStart,
  /// Tap recognition completed.
  Tap,
  /// Tap recognition was cancelled.
  TapCancel,
  /// The exact host gained focus.
  FocusStart,
  /// The exact host lost focus.
  FocusEnd,
  /// Keyboard- or controller-visible focus began.
  FocusVisibleStart,
  /// Keyboard- or controller-visible focus ended.
  FocusVisibleEnd,
  /// A primary pointer established a pan session.
  PanSessionStart,
  /// Pan crossed its configured threshold.
  PanStart,
  /// Latest coalesced pan movement.
  Pan,
  /// Pointer ownership ended after pan.
  PanEnd,
  /// Pan was cancelled.
  PanCancel,
  /// Drag crossed its configured threshold.
  DragStart,
  /// Drag selected one locked direction.
  DragDirectionLock,
  /// Latest coalesced pointer-driven drag movement.
  Drag,
  /// Pointer ownership ended after drag.
  DragEnd,
  /// Drag or active momentum was cancelled.
  DragCancel,
  /// Release momentum reached its terminal offset.
  DragMomentumComplete,
  /// Element-backed constraints were measured.
  DragConstraintsMeasured,
  /// Native scroll offset or viewport progress changed.
  Scroll,
  /// The host entered its viewport.
  InViewEnter,
  /// The host left its viewport.
  InViewLeave,
}

/// One gesture boundary or coalesced movement sample.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionGestureEvent {
  /// Descriptor identity.
  pub descriptor_id: ObjectId,
  /// Descriptor generation.
  pub generation: MotionGeneration,
  /// Gesture event kind.
  pub kind: MotionGestureEventKind,
  /// Owning pointer identity, or `-1` for navigation and observation events.
  pub pointer_id: i32,
  /// Owning device.
  pub device: MotionPointerDevice,
  /// Current panel-space point.
  pub point: MotionGestureVector,
  /// Delta since the previous sample.
  pub delta: MotionGestureVector,
  /// Offset from the gesture origin.
  pub offset: MotionGestureVector,
  /// Panel pixels per second.
  pub velocity: MotionGestureVector,
  /// Direction selected by locking, when any.
  pub axis: Option<MotionGestureAxis>,
  /// Stable release-momentum generation.
  pub momentum_generation: u32,
  /// Whether the current offset is constrained on either axis.
  #[serde(default)]
  pub constrained: bool,
}

/// One external drag-controls start sampled from an initiating pointer event.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionDragControlOperation {
  /// Stable binding shared with one draggable host.
  pub control_id: ObjectId,
  /// Pointer which owns the initiated drag.
  pub pointer_id: i32,
  /// Initiating pointer device.
  pub device: MotionPointerDevice,
  /// Current panel-space pointer position.
  pub point: MotionGestureVector,
  /// Whether the draggable host is centered under the pointer before movement.
  pub snap_to_cursor: bool,
}

/// Terminal result for one stable imperative playback identity.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum MotionPlaybackOutcome {
  /// The terminal target was applied.
  Completed,
  /// Playback froze at its presentation value.
  Stopped,
  /// Playback was removed and exposed its lower layer.
  Cancelled,
}

/// One generation-checked terminal event for an imperative playback.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MotionPlaybackEvent {
  /// Stable playback identity.
  pub playback_id: ObjectId,
  /// Playback generation.
  pub generation: u32,
  /// Terminal outcome.
  pub outcome: MotionPlaybackOutcome,
}

/// Ordered lifecycle boundaries and partitioned replaceable samples.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionEventBatch {
  /// First reliable sequence included.
  pub first_sequence: MotionSequence,
  /// Last reliable sequence included.
  pub last_sequence: MotionSequence,
  /// Ordered non-droppable boundaries.
  pub events: Vec<MotionLifecycleEvent>,
  /// Latest coalesced presentation samples.
  pub samples: Vec<MotionPresentationSample>,
  /// Latest coalesced samples for explicit value subscriptions.
  #[serde(default)]
  pub value_samples: Vec<crate::MotionValueSample>,
  /// Terminal events for stable imperative playback handles.
  #[serde(default)]
  pub playback_events: Vec<MotionPlaybackEvent>,
  /// Reliable gesture boundaries followed by latest coalesced movement samples.
  #[serde(default)]
  pub gesture_events: Vec<MotionGestureEvent>,
}

/// Compact timeline checkpoint retained for reconnect reconstruction.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionTimelineCheckpoint {
  /// Descriptor identity.
  pub descriptor_id: ObjectId,
  /// Slot identity.
  pub slot: MotionSlotId,
  /// Slot generation.
  pub generation: MotionGeneration,
  /// Logical elapsed time.
  pub elapsed_micros: u64,
  /// Last crossed logical iteration.
  pub iteration: u32,
  /// Whether timeline advancement is paused.
  pub paused: bool,
  /// Latest presentation values.
  pub values: Vec<MotionPropertyValue>,
}

/// Direction selected for one playback generation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum MotionPlaybackDirection {
  /// Runs from origin to target.
  Forward,
  /// Runs from target to origin.
  Reverse,
  /// Alternates beginning forward.
  Alternate,
  /// Alternates beginning in reverse.
  AlternateReverse,
}

/// Generation-checked playback operation.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum MotionPlaybackCommand {
  /// Resume logical time.
  Play,
  /// Hold the current presentation.
  Pause,
  /// Restart the generation from its origin.
  Replay,
  /// Freeze and terminate without applying the target.
  Stop,
  /// Remove the slot and expose the next layer.
  Cancel,
  /// Apply the terminal target and complete.
  Complete,
  /// Sample one logical time without boundary side effects.
  Seek {
    /// Requested logical time.
    elapsed_micros: u64,
  },
  /// Change the nonnegative playback rate.
  SetSpeed {
    /// Nonnegative playback multiplier.
    value: f64,
  },
  /// Change playback direction.
  SetDirection {
    /// Replacement direction.
    value: MotionPlaybackDirection,
  },
}

/// Addressed playback operation for one current slot generation.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionPlaybackOperation {
  /// Descriptor identity.
  pub descriptor_id: ObjectId,
  /// Stable slot identity.
  pub slot: MotionSlotId,
  /// Required current generation.
  pub generation: MotionGeneration,
  /// Operation to apply.
  pub command: MotionPlaybackCommand,
}

/// Deterministic controlled-clock mutation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum MotionControlledClockCommand {
  /// Replace the elapsed logical time.
  Set {
    /// Replacement logical time.
    elapsed_micros: u64,
  },
  /// Advance by an exact logical duration.
  Advance {
    /// Exact logical-time increment.
    delta_micros: u64,
  },
}

/// Addressed mutation for a controlled motion clock.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MotionControlledClockOperation {
  /// Controlled clock identity.
  pub clock_id: ObjectId,
  /// Mutation to apply.
  pub command: MotionControlledClockCommand,
}

fn value_matches(kind: MotionValueKind, value: &MotionValue) -> bool {
  matches!(
    (kind, value),
    (MotionValueKind::Scalar, MotionValue::Scalar(_))
      | (MotionValueKind::Length, MotionValue::Length(_))
      | (MotionValueKind::Color, MotionValue::Color(_))
      | (MotionValueKind::Vector2, MotionValue::Vector2(_))
      | (MotionValueKind::Vector3, MotionValue::Vector3(_))
      | (MotionValueKind::Angle, MotionValue::Angle(_))
      | (
        MotionValueKind::TransformList,
        MotionValue::TransformList(_)
      )
      | (MotionValueKind::FilterList, MotionValue::FilterList(_))
      | (MotionValueKind::ShadowList, MotionValue::ShadowList(_))
      | (MotionValueKind::Gradient, MotionValue::Gradient(_))
      | (MotionValueKind::ClipInset, MotionValue::ClipInset(_))
      | (MotionValueKind::ClipPolygon, MotionValue::ClipPolygon(_))
      | (MotionValueKind::Discrete, MotionValue::Discrete(_))
  )
}

fn validate_times(values: &[f64]) -> Result<(), String> {
  if values.first() != Some(&0.0) || values.last() != Some(&1.0) {
    return Err("property keyframe times must begin at zero and end at one".to_owned());
  }
  if values.iter().any(|value| !value.is_finite()) {
    return Err("property keyframe times must be finite".to_owned());
  }
  if values.windows(2).any(|values| values[0] > values[1]) {
    return Err("property keyframe times must be nondecreasing".to_owned());
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use battlement_types::{Color, ObjectId};

  use crate::{
    FilterFunction, FilterList, MotionClockSource, MotionDescriptor, MotionGeneration,
    MotionProperty, MotionPropertyTrack, MotionSlotDescriptor, MotionSlotId,
    MotionTargetDescriptor, MotionValue, ReducedMotionPolicy, Shadow, TransitionDefinition,
  };

  #[test]
  fn descriptor_rejects_duplicate_slots_and_invalid_shapes() {
    let target = MotionTargetDescriptor {
      tracks: vec![MotionPropertyTrack {
        property: MotionProperty::Opacity,
        values: vec![MotionValue::Scalar(1.0)],
        times: None,
        transition: TransitionDefinition::tween(),
      }],
      transition_end: Vec::new(),
    };
    let slot = MotionSlotDescriptor {
      slot: MotionSlotId(7),
      generation: MotionGeneration(1),
      layer: crate::MotionLayer::Animate,
      target: target.clone(),
      callbacks: crate::MotionCallbackSubscriptions::default(),
    };
    let mut descriptor = MotionDescriptor {
      descriptor_id: ObjectId::new_v4(),
      host_id: ObjectId::new_v4(),
      generation: MotionGeneration(1),
      initial: None,
      initial_disabled: false,
      slots: vec![slot.clone(), slot],
      clock: MotionClockSource::Unscaled,
      reduced_motion: ReducedMotionPolicy::Never,
      pseudo_styles: Vec::new(),
      style_transition: crate::StyleTransitionDescriptor::default(),
      animations: Vec::new(),
      decorations: Vec::new(),
      variants: None,
      values: Vec::new(),
      value_bindings: Vec::new(),
      value_subscriptions: Vec::new(),
      control_id: None,
      scope_id: None,
      scope_root: false,
      motion_name: None,
      named_targets: Vec::new(),
      gestures: None,
      layout: None,
    };
    assert!(descriptor.validate().unwrap_err().contains("repeats slot"));
    descriptor.slots.truncate(1);
    descriptor.slots[0].target.tracks[0].values = vec![MotionValue::Angle(1.0)];
    assert!(
      descriptor
        .validate()
        .unwrap_err()
        .contains("incompatible value shape")
    );
  }

  #[test]
  fn filter_capabilities_are_property_specific() {
    let shadow = Shadow {
      x: 0.0,
      y: 2.0,
      blur: 8.0,
      spread: 1.0,
      color: Color::rgba(0.1, 0.4, 1.0, 0.8),
      inset: false,
    };
    let track = |property, filter| MotionPropertyTrack {
      property,
      values: vec![MotionValue::FilterList(FilterList::new([filter]))],
      times: None,
      transition: TransitionDefinition::tween(),
    };
    assert!(
      track(MotionProperty::PaintFilter, FilterFunction::Brightness(1.2))
        .validate()
        .is_ok()
    );
    assert!(
      track(
        MotionProperty::PaintFilter,
        FilterFunction::DropShadow(shadow)
      )
      .validate()
      .is_ok()
    );
    assert!(
      track(MotionProperty::Filter, FilterFunction::Brightness(1.2))
        .validate()
        .is_err()
    );
    assert!(
      track(MotionProperty::PaintFilter, FilterFunction::Saturate(1.2))
        .validate()
        .is_err()
    );
    assert!(
      track(MotionProperty::Filter, FilterFunction::Blur(-1.0))
        .validate()
        .is_err()
    );
    assert!(
      track(
        MotionProperty::PaintFilter,
        FilterFunction::Brightness(-0.1)
      )
      .validate()
      .is_err()
    );
    assert!(
      MotionPropertyTrack {
        property: MotionProperty::PaintFilter,
        values: vec![MotionValue::FilterList(FilterList::new([
          FilterFunction::DropShadow(shadow),
          FilterFunction::DropShadow(shadow),
        ]))],
        times: None,
        transition: TransitionDefinition::tween(),
      }
      .validate()
      .is_err()
    );
    assert!(
      MotionPropertyTrack {
        property: MotionProperty::BoxShadow,
        values: vec![MotionValue::ShadowList(vec![shadow])],
        times: None,
        transition: TransitionDefinition::tween(),
      }
      .validate()
      .is_ok()
    );
    assert!(
      MotionPropertyTrack {
        property: MotionProperty::BoxShadow,
        values: vec![MotionValue::ShadowList(vec![Shadow {
          blur: -1.0,
          ..shadow
        }])],
        times: None,
        transition: TransitionDefinition::tween(),
      }
      .validate()
      .is_err()
    );
  }
}

use std::collections::HashSet;

use battlement_types::ObjectId;
use serde::{Deserialize, Serialize};

use crate::{
  CssAnimationDescriptor, MotionDecorationDescriptor, MotionPseudoStyle, StyleTransitionDescriptor,
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
      value.validate().map_err(str::to_owned)?;
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
    self.value.validate().map_err(str::to_owned)?;
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
  /// Pointer hover layer.
  Hover,
  /// Pointer/submit tap layer.
  Tap,
  /// Active drag layer.
  Drag,
  /// Presence exit layer.
  Exit,
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

/// Complete validated animation state installed beside one UI host.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionDescriptor {
  /// Stable descriptor identity across updates.
  pub descriptor_id: ObjectId,
  /// Host receiving the final presentation values.
  pub host_id: ObjectId,
  /// Committed descriptor generation.
  pub generation: MotionGeneration,
  /// Static values beneath every animated layer.
  pub static_baseline: Vec<MotionPropertyValue>,
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
}

impl MotionDescriptor {
  /// Validates complete descriptor identity and property ownership.
  pub fn validate(&self) -> Result<(), String> {
    if self.initial_disabled && self.initial.is_some() {
      return Err("disabled initial target cannot also carry tracks".to_owned());
    }
    let mut baseline = HashSet::new();
    for value in &self.static_baseline {
      value.validate()?;
      if !baseline.insert(value.property) {
        return Err(format!(
          "static baseline repeats property {}",
          value.property.metadata().wire_name
        ));
      }
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
    validate_css(self)?;
    Ok(())
  }
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
    value.validate().map_err(str::to_owned)?;
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
  use battlement_types::ObjectId;

  use crate::{
    MotionClockSource, MotionDescriptor, MotionGeneration, MotionProperty, MotionPropertyTrack,
    MotionSlotDescriptor, MotionSlotId, MotionTargetDescriptor, MotionValue, ReducedMotionPolicy,
    TransitionDefinition,
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
      static_baseline: Vec::new(),
      initial: None,
      initial_disabled: false,
      slots: vec![slot.clone(), slot],
      clock: MotionClockSource::Unscaled,
      reduced_motion: ReducedMotionPolicy::Never,
      pseudo_styles: Vec::new(),
      style_transition: crate::StyleTransitionDescriptor::default(),
      animations: Vec::new(),
      decorations: Vec::new(),
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
}

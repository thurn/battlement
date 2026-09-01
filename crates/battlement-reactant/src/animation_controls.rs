//! Stable animation controls, typed scopes, selectors, and sequences.

use std::{
  any::{Any, TypeId},
  collections::HashMap,
  marker::PhantomData,
  time::Duration,
};

use battlement::{
  CommandBody, MotionControlCommand, MotionControlOperation, MotionScopeCommand,
  MotionScopeOperation, MotionSequenceStep,
};

use crate::{
  element_ref::ElementRef,
  hook_storage::{HookKind, HookSlot},
  hooks,
  motion::{MotionStyle, MotionTarget},
  motion_value::{AnimationPlayback, MotionValueRuntimeHandle},
  variant_map::{VariantKey, variant_label},
};

/// Stable broadcast controls bound to hosts using the same variant-name type.
pub struct AnimationControls<Name: VariantKey> {
  handle: MotionValueRuntimeHandle,
  control_id: battlement::ObjectId,
  marker: PhantomData<Name>,
}

/// Concrete or named target accepted by [`AnimationControls`].
pub enum ControlTarget<Name: VariantKey> {
  /// Fully authored target.
  Target(Box<MotionTarget>),
  /// Named target from each bound host's compatible variant map.
  Variant(Name),
}

/// Stable root for closed selector-based animation operations.
#[derive(Clone)]
pub struct AnimationScope {
  handle: MotionValueRuntimeHandle,
  scope_id: battlement::ObjectId,
}

/// Closed selector resolved atomically inside one animation scope.
#[derive(Clone)]
pub enum MotionSelector {
  /// One attached host ref.
  Element(ElementRef),
  /// Hosts carrying a stable motion name.
  Name(String),
  /// The scope root.
  ScopeRoot,
  /// Direct visual children of the scope root.
  Children,
  /// Every visual descendant of the scope root.
  Descendants,
}

/// Ordered selector-target steps on one native playhead.
#[derive(Clone, Default)]
pub struct AnimationSequence {
  steps: Vec<SequenceStep>,
  labels: HashMap<String, Duration>,
}

/// Placement for the most recently appended sequence step.
#[derive(Clone, Debug, PartialEq)]
pub enum SequencePosition {
  /// Starts after the previous step's finite duration.
  AfterPrevious,
  /// Starts with the previous step plus a signed offset in seconds.
  WithPrevious(f64),
  /// Starts at an absolute sequence time.
  Absolute(Duration),
  /// Starts at a named label plus a signed offset in seconds.
  Label(String, f64),
}

#[derive(Clone)]
struct SequenceStep {
  selector: MotionSelector,
  target: MotionTarget,
  start: Duration,
  duration: Duration,
}

struct ControlsSlot<Name: VariantKey>(AnimationControls<Name>);

struct ScopeSlot(AnimationScope);

/// Creates stable typed animation controls in the current hook slot.
pub fn use_animation_controls<Name: VariantKey>() -> AnimationControls<Name> {
  hooks::use_slot(
    HookKind::AnimationControl,
    TypeId::of::<Name>(),
    |_| {
      ControlsSlot(AnimationControls {
        handle: MotionValueRuntimeHandle::current(),
        control_id: battlement::ObjectId::new_v4(),
        marker: PhantomData,
      })
    },
    |slot| slot.0.clone(),
  )
}

/// Creates one stable animation scope in the current hook slot.
pub fn use_animation_scope() -> AnimationScope {
  hooks::use_slot(
    HookKind::AnimationScope,
    TypeId::of::<AnimationScope>(),
    |_| {
      ScopeSlot(AnimationScope {
        handle: MotionValueRuntimeHandle::current(),
        scope_id: battlement::ObjectId::new_v4(),
      })
    },
    |slot| slot.0.clone(),
  )
}

impl<Name: VariantKey> AnimationControls<Name> {
  /// Starts one target on the current binding snapshot.
  pub fn start(&self, target: impl Into<ControlTarget<Name>>) -> AnimationPlayback {
    let playback = AnimationPlayback::from_handle(&self.handle);
    let (playback_id, generation) = playback.protocol_identity();
    self.queue(MotionControlCommand::Start {
      playback_id,
      generation,
      target: target.into().into_protocol(),
    });
    playback
  }

  /// Applies one target immediately to every current binding.
  pub fn set(&self, target: impl Into<ControlTarget<Name>>) {
    self.queue(MotionControlCommand::Set(target.into().into_protocol()));
  }

  /// Freezes every active controlled track.
  pub fn stop(&self) {
    self.queue(MotionControlCommand::Stop);
  }

  /// Removes the imperative control layer.
  pub fn clear(&self) {
    self.queue(MotionControlCommand::Clear);
  }

  pub(crate) fn id(&self) -> battlement::ObjectId {
    self.control_id
  }

  fn queue(&self, command: MotionControlCommand) {
    self
      .handle
      .queue(CommandBody::MotionControl(MotionControlOperation {
        control_id: self.control_id,
        command,
      }));
  }
}

impl<Name: VariantKey> Clone for AnimationControls<Name> {
  fn clone(&self) -> Self {
    Self {
      handle: self.handle.clone(),
      control_id: self.control_id,
      marker: PhantomData,
    }
  }
}

impl<Name: VariantKey> From<MotionTarget> for ControlTarget<Name> {
  fn from(value: MotionTarget) -> Self {
    Self::Target(Box::new(value))
  }
}

impl<Name: VariantKey> From<MotionStyle> for ControlTarget<Name> {
  fn from(value: MotionStyle) -> Self {
    Self::Target(Box::new(value.into()))
  }
}

impl<Name: VariantKey> From<Name> for ControlTarget<Name> {
  fn from(value: Name) -> Self {
    Self::Variant(value)
  }
}

impl<Name: VariantKey> ControlTarget<Name> {
  fn into_protocol(self) -> battlement::MotionControlTarget {
    match self {
      Self::Target(value) => battlement::MotionControlTarget::Target(value.descriptor(None, 0)),
      Self::Variant(value) => battlement::MotionControlTarget::Variant(variant_label(&value)),
    }
  }
}

impl AnimationScope {
  /// Starts one selector-snapshotted sequence.
  pub fn start(&self, sequence: AnimationSequence) -> AnimationPlayback {
    let playback = AnimationPlayback::from_handle(&self.handle);
    let (playback_id, generation) = playback.protocol_identity();
    self.queue(MotionScopeCommand::Start {
      playback_id,
      generation,
      steps: sequence.into_protocol(),
    });
    playback
  }

  /// Applies one target immediately to a selector snapshot.
  pub fn set(&self, selector: MotionSelector, target: MotionStyle) {
    self.queue(MotionScopeCommand::Set {
      selector: selector.into_protocol(),
      target: MotionTarget::new(target).descriptor(None, 0),
    });
  }

  /// Freezes active tracks in a selector snapshot.
  pub fn stop(&self, selector: MotionSelector) {
    self.queue(MotionScopeCommand::Stop(selector.into_protocol()));
  }

  pub(crate) fn id(&self) -> battlement::ObjectId {
    self.scope_id
  }

  fn queue(&self, command: MotionScopeCommand) {
    self
      .handle
      .queue(CommandBody::MotionScope(MotionScopeOperation {
        scope_id: self.scope_id,
        command,
      }));
  }
}

impl MotionSelector {
  /// Selects one exact attached element ref.
  pub fn element(value: ElementRef) -> Self {
    Self::Element(value)
  }

  /// Selects hosts with one nonempty stable name.
  pub fn name(value: impl Into<String>) -> Self {
    let value = value.into();
    assert!(!value.trim().is_empty(), "motion selector name is empty");
    Self::Name(value)
  }

  fn into_protocol(self) -> battlement::MotionSelector {
    match self {
      Self::Element(value) => battlement::MotionSelector::Element(
        value
          .geometry_identity()
          .2
          .expect("motion selector element ref is not attached"),
      ),
      Self::Name(value) => battlement::MotionSelector::Name(value),
      Self::ScopeRoot => battlement::MotionSelector::ScopeRoot,
      Self::Children => battlement::MotionSelector::Children,
      Self::Descendants => battlement::MotionSelector::Descendants,
    }
  }
}

impl AnimationSequence {
  /// Creates an empty sequence.
  pub fn new() -> Self {
    Self::default()
  }

  /// Appends a step after the current sequence end.
  pub fn animate(
    mut self,
    selector: MotionSelector,
    target: MotionStyle,
    transition: crate::motion::Transition,
  ) -> Self {
    let target = MotionTarget::new(target).transition(transition);
    let start = self
      .steps
      .last()
      .map_or(Duration::ZERO, |value| value.start + value.duration);
    let duration = Duration::from_micros(target.total_duration_micros(None));
    self.steps.push(SequenceStep {
      selector,
      target,
      start,
      duration,
    });
    self
  }

  /// Appends a step after the current sequence end.
  pub fn then(
    self,
    selector: MotionSelector,
    target: MotionStyle,
    transition: crate::motion::Transition,
  ) -> Self {
    self.animate(selector, target, transition)
  }

  /// Adds a label at the current sequence end.
  pub fn label(mut self, name: impl Into<String>) -> Self {
    let name = name.into();
    assert!(!name.trim().is_empty(), "animation sequence label is empty");
    let end = self
      .steps
      .last()
      .map_or(Duration::ZERO, |value| value.start + value.duration);
    assert!(
      self.labels.insert(name, end).is_none(),
      "duplicate sequence label"
    );
    self
  }

  /// Repositions the most recently appended step.
  pub fn at(mut self, position: SequencePosition) -> Self {
    let index = self
      .steps
      .len()
      .checked_sub(1)
      .expect("sequence has no step");
    let previous = index
      .checked_sub(1)
      .map(|value| self.steps[value].start)
      .unwrap_or(Duration::ZERO);
    let previous_end = index
      .checked_sub(1)
      .map(|value| self.steps[value].start + self.steps[value].duration)
      .unwrap_or(Duration::ZERO);
    self.steps[index].start = match position {
      SequencePosition::AfterPrevious => previous_end,
      SequencePosition::WithPrevious(offset) => signed_offset(previous, offset),
      SequencePosition::Absolute(value) => value,
      SequencePosition::Label(name, offset) => signed_offset(
        *self
          .labels
          .get(&name)
          .expect("animation sequence label is missing"),
        offset,
      ),
    };
    self
  }

  fn into_protocol(self) -> Vec<MotionSequenceStep> {
    self
      .steps
      .into_iter()
      .map(|value| MotionSequenceStep {
        selector: value.selector.into_protocol(),
        target: value.target.descriptor(None, 0),
        start_micros: value
          .start
          .as_micros()
          .try_into()
          .expect("sequence time overflow"),
      })
      .collect()
  }
}

impl<Name: VariantKey> HookSlot for ControlsSlot<Name> {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
  fn clone_box(&self) -> Box<dyn HookSlot> {
    Box::new(ControlsSlot(self.0.clone()))
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
    HookKind::AnimationControl
  }
  fn value_type(&self) -> TypeId {
    TypeId::of::<Name>()
  }
}

impl HookSlot for ScopeSlot {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
  fn clone_box(&self) -> Box<dyn HookSlot> {
    Box::new(ScopeSlot(self.0.clone()))
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
    HookKind::AnimationScope
  }
  fn value_type(&self) -> TypeId {
    TypeId::of::<AnimationScope>()
  }
}

fn signed_offset(value: Duration, seconds: f64) -> Duration {
  assert!(seconds.is_finite(), "sequence offset must be finite");
  Duration::from_secs_f64((value.as_secs_f64() + seconds).max(0.0))
}

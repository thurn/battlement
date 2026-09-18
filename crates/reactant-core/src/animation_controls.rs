//! Stable animation controls, typed scopes, selectors, and sequences.

use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  time::Duration,
};

use battlement::{
  CommandBody, MotionControlCommand, MotionControlOperation, MotionReferenceResolution,
  MotionScopeCommand, MotionScopeOperation, MotionSequenceConflict, MotionSequenceEntry,
  MotionSequenceSchedule,
};

use crate::{
  element_ref::ElementRef,
  hook_storage::{HookKind, HookSlot},
  hooks,
  motion::{MotionTarget, StyleTarget},
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
  /// One exact stable presentation identity.
  Identified(battlement::ObjectId),
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
  entries: Vec<SequenceEntry>,
}

/// A concrete property target with an optional typed position reference.
#[derive(Clone)]
pub struct SequenceTarget {
  target: MotionTarget,
  position: Option<MotionPositionRef>,
}

/// A typed host or named world-anchor position used by a sequence target.
#[derive(Clone)]
pub struct MotionPositionRef {
  source: MotionPositionSource,
  anchor: Option<String>,
  resolution: MotionReferenceResolution,
}

#[derive(Clone)]
enum MotionPositionSource {
  Element(ElementRef),
  Identified(battlement::ObjectId),
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
enum SequenceEntry {
  Animate {
    selector: MotionSelector,
    target: Box<MotionTarget>,
    position: Option<MotionPositionRef>,
    schedule: MotionSequenceSchedule,
    conflict: MotionSequenceConflict,
  },
  Label {
    name: String,
    schedule: MotionSequenceSchedule,
  },
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

impl<Name: VariantKey> From<StyleTarget> for ControlTarget<Name> {
  fn from(value: StyleTarget) -> Self {
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
      entries: sequence.into_protocol(),
    });
    playback
  }

  /// Applies one target immediately to a selector snapshot.
  pub fn set(&self, selector: MotionSelector, target: StyleTarget) {
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

  /// Selects one host by its stable presentation identity.
  pub fn identified(value: battlement::ObjectId) -> Self {
    Self::Identified(value)
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
      Self::Identified(value) => battlement::MotionSelector::Element(value),
      Self::Name(value) => battlement::MotionSelector::Name(value),
      Self::ScopeRoot => battlement::MotionSelector::ScopeRoot,
      Self::Children => battlement::MotionSelector::Children,
      Self::Descendants => battlement::MotionSelector::Descendants,
    }
  }
}

impl SequenceTarget {
  /// Creates a sequence target from concrete style properties.
  #[must_use]
  pub fn new(target: impl Into<MotionTarget>) -> Self {
    Self {
      target: target.into(),
      position: None,
    }
  }

  /// Moves to one typed host or named world-anchor position.
  #[must_use]
  pub fn position(mut self, value: impl Into<MotionPositionRef>) -> Self {
    self.position = Some(value.into());
    self
  }
}

impl From<StyleTarget> for SequenceTarget {
  fn from(value: StyleTarget) -> Self {
    Self::new(value)
  }
}

impl From<MotionTarget> for SequenceTarget {
  fn from(value: MotionTarget) -> Self {
    Self::new(value)
  }
}

impl MotionPositionRef {
  /// References the attached host origin.
  #[must_use]
  pub fn element(value: ElementRef) -> Self {
    Self {
      source: MotionPositionSource::Element(value),
      anchor: None,
      resolution: MotionReferenceResolution::CaptureAtStart,
    }
  }

  /// References a host by stable presentation identity.
  #[doc(hidden)]
  #[must_use]
  pub fn identified(value: battlement::ObjectId) -> Self {
    Self {
      source: MotionPositionSource::Identified(value),
      anchor: None,
      resolution: MotionReferenceResolution::CaptureAtStart,
    }
  }

  /// References one named anchor inside an attached prepared world visual.
  #[must_use]
  pub fn named_anchor(value: ElementRef, anchor: impl Into<String>) -> Self {
    let anchor = anchor.into();
    assert!(!anchor.is_empty(), "Motion position anchor is empty");
    Self {
      source: MotionPositionSource::Element(value),
      anchor: Some(anchor),
      resolution: MotionReferenceResolution::CaptureAtStart,
    }
  }

  /// Resolves once when the sequence starts.
  #[must_use]
  pub fn capture_at_start(mut self) -> Self {
    self.resolution = MotionReferenceResolution::CaptureAtStart;
    self
  }

  /// Follows the referenced presentation position while the entry is active.
  #[must_use]
  pub fn follow(mut self) -> Self {
    self.resolution = MotionReferenceResolution::Follow;
    self
  }

  fn into_protocol(self) -> battlement::MotionPositionReference {
    battlement::MotionPositionReference {
      object_id: match self.source {
        MotionPositionSource::Element(value) => value
          .geometry_identity()
          .2
          .expect("Motion position ref is not attached"),
        MotionPositionSource::Identified(value) => value,
      },
      anchor: self.anchor,
      resolution: self.resolution,
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
    target: impl Into<SequenceTarget>,
    transition: crate::motion::Transition,
  ) -> Self {
    let target = target.into();
    let schedule = self.after_previous();
    self.entries.push(SequenceEntry::Animate {
      selector,
      target: Box::new(target.target.transition(transition)),
      position: target.position,
      schedule,
      conflict: MotionSequenceConflict::Reject,
    });
    self
  }

  /// Appends a step after the current sequence end.
  pub fn then(
    self,
    selector: MotionSelector,
    target: impl Into<SequenceTarget>,
    transition: crate::motion::Transition,
  ) -> Self {
    self.animate(selector, target, transition)
  }

  /// Adds a label at the current sequence end.
  pub fn label(mut self, name: impl Into<String>) -> Self {
    let name = name.into();
    assert!(!name.trim().is_empty(), "animation sequence label is empty");
    let schedule = self.after_previous();
    self.entries.push(SequenceEntry::Label { name, schedule });
    self
  }

  /// Adds a label at an explicit graph position.
  pub fn label_at(mut self, name: impl Into<String>, position: SequencePosition) -> Self {
    let name = name.into();
    assert!(!name.trim().is_empty(), "animation sequence label is empty");
    let schedule = self.schedule(position, self.entries.len());
    self.entries.push(SequenceEntry::Label { name, schedule });
    self
  }

  /// Repositions the most recently appended step.
  pub fn at(mut self, position: SequencePosition) -> Self {
    let index = self
      .entries
      .iter()
      .rposition(|entry| matches!(entry, SequenceEntry::Animate { .. }))
      .expect("sequence has no animation entry");
    let schedule = self.schedule(position, index);
    let SequenceEntry::Animate {
      schedule: current, ..
    } = &mut self.entries[index]
    else {
      unreachable!()
    };
    *current = schedule;
    self
  }

  /// Allows the most recent animation entry to interrupt a prior property writer.
  pub fn replace(mut self) -> Self {
    let entry = self
      .entries
      .iter_mut()
      .rev()
      .find(|entry| matches!(entry, SequenceEntry::Animate { .. }))
      .expect("sequence has no animation entry");
    let SequenceEntry::Animate { conflict, .. } = entry else {
      unreachable!()
    };
    *conflict = MotionSequenceConflict::Replace;
    self
  }

  fn into_protocol(self) -> Vec<MotionSequenceEntry> {
    self
      .entries
      .into_iter()
      .map(|value| match value {
        SequenceEntry::Animate {
          selector,
          target,
          position,
          schedule,
          conflict,
        } => {
          let target = *target;
          MotionSequenceEntry::Animate {
            selector: selector.into_protocol(),
            position_transition: Box::new(target.sequence_position_transition()),
            target: target.descriptor(None, 0),
            position: position.map(MotionPositionRef::into_protocol),
            schedule,
            conflict,
          }
        }
        SequenceEntry::Label { name, schedule } => MotionSequenceEntry::Label { name, schedule },
      })
      .collect()
  }

  fn after_previous(&self) -> MotionSequenceSchedule {
    self
      .entries
      .last()
      .map_or(MotionSequenceSchedule::Absolute(0), |_| {
        MotionSequenceSchedule::AfterCompletion {
          entry: u32::try_from(self.entries.len() - 1).expect("sequence has too many entries"),
          offset_micros: 0,
        }
      })
  }

  fn schedule(&self, position: SequencePosition, index: usize) -> MotionSequenceSchedule {
    match position {
      SequencePosition::AfterPrevious => {
        index
          .checked_sub(1)
          .map_or(MotionSequenceSchedule::Absolute(0), |entry| {
            MotionSequenceSchedule::AfterCompletion {
              entry: u32::try_from(entry).expect("sequence has too many entries"),
              offset_micros: 0,
            }
          })
      }
      SequencePosition::WithPrevious(offset) => index.checked_sub(1).map_or_else(
        || MotionSequenceSchedule::Absolute(offset_micros(Duration::ZERO, offset)),
        |entry| MotionSequenceSchedule::RelativeStart {
          entry: u32::try_from(entry).expect("sequence has too many entries"),
          offset_micros: signed_micros(offset),
        },
      ),
      SequencePosition::Absolute(value) => MotionSequenceSchedule::Absolute(duration_micros(value)),
      SequencePosition::Label(name, offset) => MotionSequenceSchedule::Label {
        name,
        offset_micros: signed_micros(offset),
      },
    }
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

fn signed_micros(seconds: f64) -> i64 {
  assert!(seconds.is_finite(), "sequence offset must be finite");
  let micros = seconds * 1_000_000.0;
  assert!(
    micros >= i64::MIN as f64 && micros <= i64::MAX as f64,
    "sequence offset overflow"
  );
  micros.round_ties_even() as i64
}

fn duration_micros(value: Duration) -> u64 {
  value
    .as_micros()
    .try_into()
    .expect("sequence time overflow")
}

fn offset_micros(value: Duration, seconds: f64) -> u64 {
  duration_micros(value).saturating_add_signed(signed_micros(seconds))
}

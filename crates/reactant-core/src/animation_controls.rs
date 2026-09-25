//! Stable animation controls, typed scopes, selectors, and sequences.

use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  rc::Rc,
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
  local_point::LocalPointTarget,
  motion::{MotionTarget, StyleTarget},
  motion_value::{AnimationPlayback, MotionValueRuntimeHandle},
  native_host::ObjectRef,
  native_identity_lease::NativeIdentityLease,
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
  /// One attached world-object ref.
  Object(ObjectRef),
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
  LocalPoint(LocalPointTarget),
  Identified(battlement::ObjectId),
}

struct PreparedSequence {
  entries: Vec<MotionSequenceEntry>,
  native_identities: Vec<Rc<NativeIdentityLease>>,
  scope_selectors: Vec<battlement::MotionSelector>,
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
  Sound {
    address: battlement::AudioClipAddress,
    options: SequenceSoundOptions,
    schedule: MotionSequenceSchedule,
  },
  Particle {
    address: battlement::PrefabAddress,
    position: MotionPositionRef,
    lifetime: Duration,
    schedule: MotionSequenceSchedule,
  },
}

/// Captured playback parameters for one sequence sound occurrence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SequenceSoundOptions {
  /// Shared routing category.
  pub bus: battlement::AudioBus,
  /// Initial linear volume.
  pub volume: f64,
  /// Positive playback pitch.
  pub pitch: f64,
  /// Whether playback loops independently after it starts.
  pub looping: bool,
  /// Optional fade-in duration.
  pub fade_in: Duration,
}

impl Default for SequenceSoundOptions {
  fn default() -> Self {
    Self {
      bus: battlement::AudioBus::Effects,
      volume: 1.0,
      pitch: 1.0,
      looping: false,
      fade_in: Duration::ZERO,
    }
  }
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
    self.start_with_blocking(sequence, false)
  }

  /// Starts one selector-snapshotted sequence that blocks later gameplay work.
  pub fn start_blocking(&self, sequence: AnimationSequence) -> AnimationPlayback {
    self.start_with_blocking(sequence, true)
  }

  fn start_with_blocking(&self, sequence: AnimationSequence, blocking: bool) -> AnimationPlayback {
    let mut sequence = sequence.into_protocol();
    sequence.native_identities.extend(
      self
        .handle
        .retain_scope_targets(self.scope_id, &sequence.scope_selectors),
    );
    let playback =
      AnimationPlayback::from_handle_with_retainers(&self.handle, sequence.native_identities);
    let (playback_id, generation) = playback.protocol_identity();
    let command = MotionScopeCommand::Start {
      playback_id,
      generation,
      entries: sequence.entries,
    };
    if blocking {
      self.queue_blocking(command);
    } else {
      self.queue(command);
    }
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

  fn queue_blocking(&self, command: MotionScopeCommand) {
    self
      .handle
      .queue_blocking(CommandBody::MotionScope(MotionScopeOperation {
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

  /// Selects one exact attached world object ref.
  pub fn object(value: ObjectRef) -> Self {
    Self::Object(value)
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
      Self::Object(value) => battlement::MotionSelector::Element(
        value
          .object_id()
          .expect("motion selector object ref is not attached"),
      ),
      Self::Identified(value) => battlement::MotionSelector::Element(value),
      Self::Name(value) => battlement::MotionSelector::Name(value),
      Self::ScopeRoot => battlement::MotionSelector::ScopeRoot,
      Self::Children => battlement::MotionSelector::Children,
      Self::Descendants => battlement::MotionSelector::Descendants,
    }
  }

  fn into_protocol_retaining(
    self,
    native_identities: &mut Vec<Rc<NativeIdentityLease>>,
  ) -> battlement::MotionSelector {
    match self {
      Self::Element(value) => {
        let object_id = value
          .geometry_identity()
          .2
          .expect("motion selector element ref is not attached");
        native_identities.push(value.retain_native_identity(object_id));
        battlement::MotionSelector::Element(object_id)
      }
      Self::Object(value) => {
        let object_id = value
          .object_id()
          .expect("motion selector object ref is not attached");
        native_identities.push(value.retain_native_identity(object_id));
        battlement::MotionSelector::Element(object_id)
      }
      value => value.into_protocol(),
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

  fn into_protocol(
    self,
    native_identities: &mut Vec<Rc<NativeIdentityLease>>,
  ) -> battlement::MotionPositionReference {
    let (object_id, offset) = match self.source {
      MotionPositionSource::Element(value) => {
        let object_id = value
          .geometry_identity()
          .2
          .expect("Motion position ref is not attached");
        native_identities.push(value.retain_native_identity(object_id));
        (object_id, battlement::Vector3::ZERO)
      }
      MotionPositionSource::LocalPoint(value) => {
        let resolved = value.resolve();
        native_identities.push(resolved.lease());
        (resolved.object_id(), resolved.offset())
      }
      MotionPositionSource::Identified(value) => (value, battlement::Vector3::ZERO),
    };
    battlement::MotionPositionReference {
      object_id,
      anchor: self.anchor,
      offset,
      resolution: self.resolution,
    }
  }
}

impl From<LocalPointTarget> for MotionPositionRef {
  fn from(value: LocalPointTarget) -> Self {
    let resolution = match value.tracking() {
      crate::local_point::PointTracking::FollowLive => MotionReferenceResolution::Follow,
      crate::local_point::PointTracking::CaptureAtStart => {
        MotionReferenceResolution::CaptureAtStart
      }
    };
    Self {
      source: MotionPositionSource::LocalPoint(value),
      anchor: None,
      resolution,
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

  /// Appends one prepared audio occurrence after the current sequence end.
  pub fn play_sound(self, address: impl Into<battlement::AudioClipAddress>) -> Self {
    self.play_sound_with(address, SequenceSoundOptions::default())
  }

  /// Appends one prepared audio occurrence with captured playback parameters.
  pub fn play_sound_with(
    mut self,
    address: impl Into<battlement::AudioClipAddress>,
    options: SequenceSoundOptions,
  ) -> Self {
    let address = address.into();
    assert!(
      !address.as_str().is_empty(),
      "sequence sound address is empty"
    );
    assert!(
      options.volume.is_finite() && (0.0..=1.0).contains(&options.volume),
      "sequence sound volume is invalid"
    );
    assert!(
      options.pitch.is_finite() && options.pitch > 0.0 && options.pitch <= 3.0,
      "sequence sound pitch is invalid"
    );
    let schedule = self.after_previous();
    self.entries.push(SequenceEntry::Sound {
      address,
      options,
      schedule,
    });
    self
  }

  /// Appends one prepared particle burst after the current sequence end.
  pub fn particle(
    self,
    address: impl Into<battlement::PrefabAddress>,
    position: impl Into<MotionPositionRef>,
  ) -> Self {
    self.particle_for(address, position, Duration::from_secs(1))
  }

  /// Appends one prepared particle burst with a captured lifetime.
  pub fn particle_for(
    mut self,
    address: impl Into<battlement::PrefabAddress>,
    position: impl Into<MotionPositionRef>,
    lifetime: Duration,
  ) -> Self {
    let address = address.into();
    assert!(
      !address.as_str().is_empty(),
      "sequence particle address is empty"
    );
    assert!(!lifetime.is_zero(), "sequence particle lifetime is zero");
    let schedule = self.after_previous();
    self.entries.push(SequenceEntry::Particle {
      address,
      position: position.into(),
      lifetime,
      schedule,
    });
    self
  }

  /// Repositions the most recently appended step.
  pub fn at(mut self, position: SequencePosition) -> Self {
    let index = self
      .entries
      .iter()
      .rposition(|entry| !matches!(entry, SequenceEntry::Label { .. }))
      .expect("sequence has no schedulable entry");
    let schedule = self.schedule(position, index);
    let current = match &mut self.entries[index] {
      SequenceEntry::Animate { schedule, .. }
      | SequenceEntry::Sound { schedule, .. }
      | SequenceEntry::Particle { schedule, .. } => schedule,
      SequenceEntry::Label { .. } => unreachable!(),
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

  fn into_protocol(self) -> PreparedSequence {
    let mut native_identities = Vec::new();
    let mut scope_selectors = Vec::new();
    let entries = self
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
          let selector = selector.into_protocol_retaining(&mut native_identities);
          if !matches!(selector, battlement::MotionSelector::Element(_)) {
            scope_selectors.push(selector.clone());
          }
          MotionSequenceEntry::Animate {
            selector,
            position_transition: Box::new(target.sequence_position_transition()),
            target: target.descriptor(None, 0),
            position: position.map(|value| value.into_protocol(&mut native_identities)),
            schedule,
            conflict,
          }
        }
        SequenceEntry::Label { name, schedule } => MotionSequenceEntry::Label { name, schedule },
        SequenceEntry::Sound {
          address,
          options,
          schedule,
        } => MotionSequenceEntry::Sound {
          sound: battlement::MotionSoundOccurrence {
            address: address.as_str().to_owned(),
            bus: options.bus,
            volume: options.volume,
            pitch: options.pitch,
            looping: options.looping,
            fade_in_ms: u64::try_from(options.fade_in.as_millis())
              .expect("sequence sound fade-in is too long"),
          },
          schedule,
        },
        SequenceEntry::Particle {
          address,
          position,
          lifetime,
          schedule,
        } => MotionSequenceEntry::Particle {
          particle: battlement::MotionParticleOccurrence {
            address: address.as_str().to_owned(),
            position: position.into_protocol(&mut native_identities),
            lifetime_ms: u64::try_from(lifetime.as_millis())
              .expect("sequence particle lifetime is too long"),
          },
          schedule,
        },
      })
      .collect();
    PreparedSequence {
      entries,
      native_identities,
      scope_selectors,
    }
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

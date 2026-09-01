//! Typed logical variants and parent/child orchestration.

use std::{cell::RefCell, rc::Rc};

use battlement::{
  MotionCallbackSubscriptions, MotionClockSource, MotionDescriptor, MotionGeneration, MotionLayer,
  MotionRepeat, MotionSlotDescriptor, MotionSlotId, MotionVariantResolution, ObjectId,
  ReducedMotionPolicy, StaggerDirection, VariantWhen,
};

use crate::{
  motion::{InitialTarget, MotionProps, MotionTarget, Transition},
  motion_lifecycle::MotionCallbacks,
  variant_map::{
    ErasedVariantData, ErasedVariantSelection, ErasedVariants, VariantData, VariantKey,
    VariantTarget, Variants,
  },
};

/// Parent/child sequencing for one resolved variant target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VariantOrchestration {
  delay_children_micros: u64,
  stagger_micros: u64,
  direction: StaggerDirection,
  when: VariantWhen,
  child_count: Option<u32>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct VariantScope {
  initial_selection: Option<ErasedVariantSelection>,
  animate_selection: Option<ErasedVariantSelection>,
  exit_selection: Option<ErasedVariantSelection>,
  custom: Option<ErasedVariantData>,
  schedule: VariantSchedule,
  progress: Rc<RefCell<VariantProgress>>,
}

#[derive(Clone, Copy, Debug)]
struct VariantSchedule {
  delay_children_micros: u64,
  stagger_micros: u64,
  direction: StaggerDirection,
  when: VariantWhen,
  child_count: Option<u32>,
}

#[derive(Clone, Copy, Debug, Default)]
struct VariantProgress {
  next_child: u32,
  max_completion_micros: u64,
}

pub(crate) struct ResolvedVariants {
  pub(crate) initial_target: Option<MotionTarget>,
  pub(crate) target: Option<MotionTarget>,
  pub(crate) descriptor: Option<MotionVariantResolution>,
  pub(crate) child_scope: VariantScope,
  base_delay_micros: u64,
  when: VariantWhen,
}

#[derive(Clone, Debug)]
pub(crate) struct ExitBlueprint {
  props: MotionProps,
  scope: VariantScope,
}

impl VariantOrchestration {
  /// Creates concurrent parent and child playback.
  #[must_use]
  pub const fn new() -> Self {
    Self {
      delay_children_micros: 0,
      stagger_micros: 0,
      direction: StaggerDirection::Forward,
      when: VariantWhen::Together,
      child_count: None,
    }
  }

  /// Delays every child from the orchestration origin.
  #[must_use]
  pub fn delay_children_secs(mut self, value: f64) -> Self {
    self.delay_children_micros = crate::motion::micros(value, true);
    self
  }

  /// Assigns a delay step between direct logical children.
  #[must_use]
  pub fn stagger_secs(mut self, value: f64) -> Self {
    self.stagger_micros = crate::motion::micros(value, true);
    self
  }

  /// Selects forward or reverse stagger ordering.
  #[must_use]
  pub const fn stagger_direction(mut self, value: StaggerDirection) -> Self {
    self.direction = value;
    self
  }

  /// Supplies the direct-child count required by reverse staggering.
  #[must_use]
  pub fn child_count(mut self, value: u32) -> Self {
    assert!(value > 0, "variant child count must be positive");
    self.child_count = Some(value);
    self
  }

  /// Selects concurrent, parent-first, or children-first playback.
  #[must_use]
  pub const fn when(mut self, value: VariantWhen) -> Self {
    self.when = value;
    self
  }
}

impl Default for VariantOrchestration {
  fn default() -> Self {
    Self::new()
  }
}

impl Default for VariantSchedule {
  fn default() -> Self {
    Self {
      delay_children_micros: 0,
      stagger_micros: 0,
      direction: StaggerDirection::Forward,
      when: VariantWhen::Together,
      child_count: None,
    }
  }
}

impl MotionProps {
  /// Replaces the named target definitions available to this host.
  #[must_use]
  pub fn variants<Name, Custom>(mut self, value: Variants<Name, Custom>) -> Self
  where
    Name: VariantKey,
    Custom: VariantData,
  {
    self.variants = value.erase();
    self
  }

  /// Selects one named variant target.
  #[must_use]
  pub fn animate_variant<Name: VariantKey>(mut self, value: Name) -> Self {
    self.variant_selection = Some(ErasedVariantSelection::new([value]));
    self
  }

  /// Selects one named mount origin.
  #[must_use]
  pub fn initial_variant<Name: VariantKey>(mut self, value: Name) -> Self {
    self.initial_variant_selection = Some(ErasedVariantSelection::new([value]));
    self
  }

  /// Selects an ordered named mount-origin list.
  #[must_use]
  pub fn initial_variants<Name: VariantKey>(
    mut self,
    values: impl IntoIterator<Item = Name>,
  ) -> Self {
    self.initial_variant_selection = Some(ErasedVariantSelection::new(values));
    self
  }

  /// Selects one named presence-exit target.
  #[must_use]
  pub fn exit_variant<Name: VariantKey>(mut self, value: Name) -> Self {
    self.exit_variant_selection = Some(ErasedVariantSelection::new([value]));
    self
  }

  /// Selects an ordered named presence-exit list.
  #[must_use]
  pub fn exit_variants<Name: VariantKey>(mut self, values: impl IntoIterator<Item = Name>) -> Self {
    self.exit_variant_selection = Some(ErasedVariantSelection::new(values));
    self
  }

  /// Selects an ordered variant list; later targets override earlier properties.
  #[must_use]
  pub fn animate_variants<Name: VariantKey>(
    mut self,
    values: impl IntoIterator<Item = Name>,
  ) -> Self {
    self.variant_selection = Some(ErasedVariantSelection::new(values));
    self
  }

  /// Supplies custom data to computed variants.
  #[must_use]
  pub fn custom<T: VariantData>(mut self, value: T) -> Self {
    self.variant_data = Some(ErasedVariantData::new(value));
    self
  }

  /// Enables or disables logical parent variant propagation.
  #[must_use]
  pub fn inherit_variants(mut self, value: bool) -> Self {
    self.inherit_variants = value;
    self
  }

  pub(crate) fn descriptor(
    &self,
    host_id: ObjectId,
    generation: MotionGeneration,
    resolved: &ResolvedVariants,
    previous: Option<&MotionDescriptor>,
  ) -> MotionDescriptor {
    let transition = self.transition.as_ref();
    let (initial, initial_disabled) = if let Some(value) = &resolved.initial_target {
      (
        Some(value.descriptor(Some(&Transition::immediate()), 0)),
        false,
      )
    } else {
      match &self.initial {
        Some(InitialTarget::Target(value)) => (
          Some(value.descriptor(Some(&Transition::immediate()), 0)),
          false,
        ),
        Some(InitialTarget::Disabled) => (None, true),
        None => (None, false),
      }
    };
    let delay_micros = resolved.final_delay_micros();
    let target = resolved.target.as_ref().or(self.animate.as_ref());
    let reuse_variant = previous.is_some_and(|value| {
      value.variants.as_ref().map(|value| &value.names)
        == resolved.descriptor.as_ref().map(|value| &value.names)
        && resolved.descriptor.is_some()
    });
    let mut slots: Vec<MotionSlotDescriptor> = if reuse_variant {
      previous
        .expect("reused variant descriptor exists")
        .slots
        .iter()
        .filter(|slot| slot.layer == MotionLayer::Animate)
        .cloned()
        .map(|mut slot| {
          slot.generation = generation;
          slot
        })
        .collect()
    } else {
      target
        .iter()
        .map(|target| MotionSlotDescriptor {
          slot: MotionSlotId(1),
          generation,
          layer: MotionLayer::Animate,
          target: target.descriptor(transition, delay_micros),
          callbacks: self.callbacks(resolved).subscriptions(),
        })
        .collect()
    };
    slots.extend(self.gesture_slots(generation, transition));
    let mut values = target.map_or_else(Vec::new, MotionTarget::graph_values);
    for value in self.gesture_graph_values() {
      if !values
        .iter()
        .any(|existing| existing.value_id == value.value_id)
      {
        values.push(value);
      }
    }
    let mut value_subscriptions = target.map_or_else(Vec::new, MotionTarget::value_subscriptions);
    for subscription in self.gesture_value_subscriptions() {
      if !value_subscriptions
        .iter()
        .any(|existing| existing.subscription_id == subscription.subscription_id)
      {
        value_subscriptions.push(subscription);
      }
    }
    MotionDescriptor {
      descriptor_id: host_id,
      host_id,
      generation,
      static_baseline: Vec::new(),
      initial,
      initial_disabled,
      slots,
      clock: MotionClockSource::Unscaled,
      reduced_motion: ReducedMotionPolicy::Never,
      pseudo_styles: self.css.pseudo_descriptors(),
      style_transition: self.css.transition_descriptor(),
      animations: self.css.animation_descriptors(generation),
      decorations: self.css.decoration_descriptors(generation),
      variants: resolved.descriptor.as_ref().map(|value| {
        let mut value = value.clone();
        value.delay_micros = delay_micros;
        if reuse_variant {
          value.custom_snapshot = previous
            .and_then(|value| value.variants.as_ref())
            .expect("reused variant facts exist")
            .custom_snapshot;
        }
        value
      }),
      values,
      value_bindings: target.map_or_else(Vec::new, MotionTarget::value_bindings),
      value_subscriptions,
      control_id: self.control_id,
      scope_id: self.scope_id,
      scope_root: self.scope_root,
      motion_name: self.motion_name.clone(),
      named_targets: self.control_id.map_or_else(Vec::new, |_| {
        self
          .variants
          .named_targets(self.variant_data.as_ref(), transition)
      }),
      gestures: self.gesture_descriptor(),
    }
  }

  pub(crate) fn resolved_duration_micros(&self, resolved: &ResolvedVariants) -> u64 {
    resolved
      .target
      .as_ref()
      .or(self.animate.as_ref())
      .map_or(0, |value| {
        value.total_duration_micros(self.transition.as_ref())
      })
  }

  pub(crate) fn callbacks(&self, resolved: &ResolvedVariants) -> MotionCallbacks {
    resolved
      .target
      .as_ref()
      .or(self.animate.as_ref())
      .map_or_else(MotionCallbacks::default, |target| target.callbacks.clone())
      .merge(self.callbacks.clone())
  }
}

fn effective_selection(
  local: &Option<ErasedVariantSelection>,
  inherited: &Option<ErasedVariantSelection>,
  inherit: bool,
) -> Option<ErasedVariantSelection> {
  local
    .clone()
    .or_else(|| inherit.then(|| inherited.clone()).flatten())
}

fn resolve_layer(
  variants: &ErasedVariants,
  selection: Option<&ErasedVariantSelection>,
  custom: Option<&ErasedVariantData>,
  explicit: bool,
) -> Option<VariantTarget> {
  selection.and_then(|selection| {
    (!selection.is_empty())
      .then(|| variants.resolve(selection, custom, explicit))
      .flatten()
  })
}

impl VariantScope {
  pub(crate) fn resolve(&self, props: &MotionProps) -> ResolvedVariants {
    let initial_explicit = props.initial_variant_selection.is_some();
    let animate_explicit = props.variant_selection.is_some();
    let exit_explicit = props.exit_variant_selection.is_some();
    let initial_selection = effective_selection(
      &props.initial_variant_selection,
      &self.initial_selection,
      props.inherit_variants,
    );
    let animate_selection = effective_selection(
      &props.variant_selection,
      &self.animate_selection,
      props.inherit_variants,
    );
    let exit_selection = effective_selection(
      &props.exit_variant_selection,
      &self.exit_selection,
      props.inherit_variants,
    );
    let custom = props.variant_data.clone().or_else(|| {
      props
        .inherit_variants
        .then(|| self.custom.clone())
        .flatten()
    });
    let initial_resolved = resolve_layer(
      &props.variants,
      initial_selection.as_ref(),
      custom.as_ref(),
      initial_explicit,
    );
    let animate_resolved = resolve_layer(
      &props.variants,
      animate_selection.as_ref(),
      custom.as_ref(),
      animate_explicit,
    );
    let exit_resolved = resolve_layer(
      &props.variants,
      exit_selection.as_ref(),
      custom.as_ref(),
      exit_explicit,
    );
    let participates = animate_resolved.is_some();
    let (child_index, inherited_delay) = if animate_explicit || !participates {
      (0, 0)
    } else {
      self.claim_child()
    };
    let orchestration = animate_resolved
      .as_ref()
      .map_or_else(VariantOrchestration::new, |value| value.orchestration);
    if orchestration.direction == StaggerDirection::Reverse && orchestration.stagger_micros > 0 {
      assert!(
        orchestration.child_count.is_some(),
        "reverse variant staggering requires child_count"
      );
    }
    let custom_snapshot = custom.as_ref().map_or(0, ErasedVariantData::snapshot);
    let initial_target = initial_resolved.map(|value| value.target);
    let target = animate_resolved.map(|value| value.target);
    let exit_participates = exit_resolved.is_some();
    let before_delay = if orchestration.when == VariantWhen::BeforeChildren {
      target.as_ref().map_or(0, |value| {
        value.total_duration_micros(props.transition.as_ref())
      })
    } else {
      0
    };
    let establishes_scope = [
      initial_explicit,
      animate_explicit,
      exit_explicit,
      initial_target.is_some(),
      participates,
      exit_participates,
    ]
    .into_iter()
    .any(|value| value);
    let child_scope = if establishes_scope {
      Self {
        initial_selection: initial_selection.clone(),
        animate_selection: animate_selection.clone(),
        exit_selection: exit_selection.clone(),
        custom: custom.clone(),
        schedule: VariantSchedule {
          delay_children_micros: orchestration
            .delay_children_micros
            .checked_add(before_delay)
            .expect("variant child delay exhausted"),
          stagger_micros: orchestration.stagger_micros,
          direction: orchestration.direction,
          when: orchestration.when,
          child_count: orchestration.child_count,
        },
        progress: Rc::new(RefCell::new(VariantProgress::default())),
      }
    } else if props.inherit_variants {
      let mut child_scope = self.clone();
      child_scope.custom = custom.clone();
      child_scope
    } else {
      Self::default()
    };
    let descriptor = participates.then(|| MotionVariantResolution {
      names: animate_selection
        .as_ref()
        .expect("participating variant has a selection")
        .labels(),
      inherited: !animate_explicit,
      custom_snapshot,
      child_index,
      delay_micros: inherited_delay,
      when: if animate_explicit {
        orchestration.when
      } else {
        self.schedule.when
      },
      stagger_direction: if animate_explicit {
        orchestration.direction
      } else {
        self.schedule.direction
      },
    });
    ResolvedVariants {
      initial_target,
      target,
      descriptor,
      child_scope,
      base_delay_micros: inherited_delay,
      when: orchestration.when,
    }
  }

  fn claim_child(&self) -> (u32, u64) {
    let mut progress = self.progress.borrow_mut();
    let index = progress.next_child;
    progress.next_child = progress
      .next_child
      .checked_add(1)
      .expect("variant child index exhausted");
    let stagger_index = match self.schedule.direction {
      StaggerDirection::Forward => index,
      StaggerDirection::Reverse => self
        .schedule
        .child_count
        .expect("reverse variant staggering requires child_count")
        .checked_sub(index + 1)
        .expect("variant child count is smaller than rendered children"),
    };
    (
      index,
      self
        .schedule
        .delay_children_micros
        .checked_add(self.schedule.stagger_micros * u64::from(stagger_index))
        .expect("variant stagger delay exhausted"),
    )
  }

  fn record_completion(&self, value: u64) {
    let mut progress = self.progress.borrow_mut();
    progress.max_completion_micros = progress.max_completion_micros.max(value);
  }

  fn resolve_exit(
    &self,
    props: &MotionProps,
    custom_override: Option<&ErasedVariantData>,
  ) -> (Option<MotionTarget>, Option<MotionVariantResolution>) {
    let explicit = props.exit_variant_selection.is_some();
    let selection = effective_selection(
      &props.exit_variant_selection,
      &self.exit_selection,
      props.inherit_variants,
    );
    let custom = custom_override
      .cloned()
      .or_else(|| props.variant_data.clone())
      .or_else(|| {
        props
          .inherit_variants
          .then(|| self.custom.clone())
          .flatten()
      });
    let resolved = resolve_layer(
      &props.variants,
      selection.as_ref(),
      custom.as_ref(),
      explicit,
    );
    let descriptor = resolved.as_ref().map(|_| MotionVariantResolution {
      names: selection
        .as_ref()
        .expect("resolved exit variant has a selection")
        .labels(),
      inherited: !explicit,
      custom_snapshot: custom.as_ref().map_or(0, ErasedVariantData::snapshot),
      child_index: 0,
      delay_micros: 0,
      when: VariantWhen::Together,
      stagger_direction: StaggerDirection::Forward,
    });
    (resolved.map(|value| value.target), descriptor)
  }
}

impl ExitBlueprint {
  pub(crate) fn new(props: MotionProps, scope: VariantScope) -> Option<Self> {
    let has_exit = props.exit.is_some()
      || props.exit_variant_selection.is_some()
      || scope.exit_selection.is_some();
    has_exit.then_some(Self { props, scope })
  }

  pub(crate) fn descriptor(
    &self,
    host_id: ObjectId,
    previous: &MotionDescriptor,
    custom: Option<&ErasedVariantData>,
  ) -> Option<MotionDescriptor> {
    let (variant, variants) = self.scope.resolve_exit(&self.props, custom);
    let target = variant.as_ref().or(self.props.exit.as_ref())?;
    let generation = MotionGeneration(
      previous
        .generation
        .0
        .checked_add(1)
        .expect("motion generation exhausted"),
    );
    let target = target.descriptor(self.props.transition.as_ref(), 0);
    assert!(
      target
        .tracks
        .iter()
        .all(|track| track.transition.repeat != MotionRepeat::Forever),
      "presence exit tracks must be finite"
    );
    let mut descriptor = previous.clone();
    descriptor.descriptor_id = host_id;
    descriptor.host_id = host_id;
    descriptor.generation = generation;
    descriptor.initial = None;
    descriptor.initial_disabled = true;
    descriptor.slots = if !target.tracks.is_empty() || !target.transition_end.is_empty() {
      vec![MotionSlotDescriptor {
        slot: MotionSlotId(1),
        generation,
        layer: MotionLayer::Exit,
        target,
        callbacks: MotionCallbackSubscriptions {
          complete: true,
          cancel: true,
          ..self.callbacks(custom).subscriptions()
        },
      }]
    } else {
      Vec::new()
    };
    descriptor.variants = variants;
    Some(descriptor)
  }

  pub(crate) fn callbacks(&self, custom: Option<&ErasedVariantData>) -> MotionCallbacks {
    let (variant, _) = self.scope.resolve_exit(&self.props, custom);
    variant
      .as_ref()
      .or(self.props.exit.as_ref())
      .map_or_else(MotionCallbacks::default, |target| target.callbacks.clone())
      .merge(self.props.callbacks.clone())
  }
}

impl ResolvedVariants {
  pub(crate) fn final_delay_micros(&self) -> u64 {
    if self.when == VariantWhen::AfterChildren {
      self
        .base_delay_micros
        .checked_add(self.child_scope.progress.borrow().max_completion_micros)
        .expect("variant after-children delay exhausted")
    } else {
      self.base_delay_micros
    }
  }

  pub(crate) fn complete(&self, parent: &VariantScope, duration_micros: u64) {
    parent.record_completion(
      self
        .final_delay_micros()
        .checked_add(duration_micros)
        .expect("variant completion time exhausted"),
    );
  }
}

use std::collections::{HashMap, HashSet};

use battlement_types::ObjectId;

use crate::{
  MotionClockSource, MotionProperty, MotionTargetDescriptor, MotionValue, SpringConfiguration,
  TransitionDefinition,
};

/// One closed operation evaluated by Unity's motion-value graph.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MotionExpressionOperation {
  /// Adds the two inputs.
  Add,
  /// Subtracts the second input from the first.
  Subtract,
  /// Multiplies the two inputs channel by channel.
  Multiply,
  /// Divides the first input by the second.
  Divide,
  /// Raises the scalar input to a finite power.
  Power(f64),
  /// Takes the nonnegative scalar square root.
  SquareRoot,
  /// Takes the scalar absolute value.
  Absolute,
  /// Selects the lesser scalar input.
  Minimum,
  /// Selects the greater scalar input.
  Maximum,
  /// Clamps a scalar to inclusive bounds.
  Clamp {
    /// Inclusive lower bound.
    min: f64,
    /// Inclusive upper bound.
    max: f64,
  },
  /// Computes a Euclidean remainder with a positive modulus.
  Modulo(f64),
  /// Wraps a scalar into the half-open range `[min, max)`.
  Wrap {
    /// Inclusive lower bound.
    min: f64,
    /// Exclusive upper bound.
    max: f64,
  },
  /// Applies exponential decay to a scalar input.
  ExponentialDecay {
    /// Nonnegative decay rate.
    rate: f64,
  },
  /// Mixes two compatible values using the third scalar input.
  Mix,
}

/// Native source or derived operation for one stable motion value.
#[derive(Clone, Debug, PartialEq)]
pub enum MotionValueSource {
  /// A mutable value changed only by addressed commands.
  Mutable,
  /// Seconds read from one Unity-local clock.
  Time(MotionClockSource),
  /// Per-second velocity of another value.
  Velocity {
    /// Observed source value.
    source: ObjectId,
  },
  /// Piecewise interpolation through aligned typed ranges.
  Range {
    /// Value mapped through the range.
    source: ObjectId,
    /// Ordered compatible input values.
    input: Vec<MotionValue>,
    /// Ordered compatible output values.
    output: Vec<MotionValue>,
    /// Whether values beyond the input endpoints clamp.
    clamp: bool,
  },
  /// A passive spring following another value.
  Spring {
    /// Value followed by the spring.
    source: ObjectId,
    /// Physical spring parameters.
    configuration: SpringConfiguration,
  },
  /// One operation over graph inputs.
  Expression {
    /// Closed native operation.
    operation: MotionExpressionOperation,
    /// Ordered input identities.
    inputs: Vec<ObjectId>,
  },
}

/// One stable node in the Unity-local motion-value graph.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionValueDescriptor {
  /// Runtime-unique value identity.
  pub value_id: ObjectId,
  /// Typed mount or reconstruction value.
  pub initial: MotionValue,
  /// Native source or derived operation.
  pub source: MotionValueSource,
}

/// How a graph value participates in a host property.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MotionBindingComposition {
  /// Replaces the property with the graph sample.
  #[default]
  Replace,
  /// Multiplies scale after local style and interaction animation.
  Compose,
}

/// One host property driven directly by a graph value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MotionValueBinding {
  /// Property receiving the sampled value.
  pub property: MotionProperty,
  /// Value whose shape must match the property catalog.
  pub value_id: ObjectId,
  /// Relationship to the host's local style and animation.
  pub composition: MotionBindingComposition,
}

/// Explicit replaceable event requested for one value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionValueEventKind {
  /// Latest presentation value.
  Change,
  /// Latest per-second velocity.
  Velocity,
  /// Presentation value at the rendered-frame boundary.
  AnimationFrame,
}

/// One explicit Rust-side graph observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MotionValueSubscription {
  /// Stable subscription identity used for coalescing.
  pub subscription_id: ObjectId,
  /// Observed graph value.
  pub value_id: ObjectId,
  /// Requested sample channel.
  pub event: MotionValueEventKind,
}

/// Mutable-value operation issued outside render.
#[derive(Clone, Debug, PartialEq)]
pub enum MotionValueCommand {
  /// Changes the source while preserving a passive effect.
  Set(MotionValue),
  /// Changes the source, clears velocity, and detaches passive effects.
  Jump(MotionValue),
  /// Freezes the current presentation value and clears velocity.
  Stop,
  /// Starts one independently controlled transition.
  Animate {
    /// Stable identity returned to the caller.
    playback_id: ObjectId,
    /// Playback generation.
    generation: u32,
    /// Typed terminal value.
    target: Box<MotionValue>,
    /// Sampling transition.
    transition: TransitionDefinition,
  },
}

/// Addressed mutable-value operation.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionValueOperation {
  /// Target mutable value.
  pub value_id: ObjectId,
  /// Operation to apply.
  pub command: MotionValueCommand,
}

/// Generation-checked operation for a motion-value playback.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionValuePlaybackOperation {
  /// Stable start identity.
  pub playback_id: ObjectId,
  /// Required current generation.
  pub generation: u32,
  /// Playback mutation.
  pub command: crate::MotionPlaybackCommand,
}

/// One coalesced value sample returned only for an explicit subscription.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionValueSample {
  /// Stable subscription identity.
  pub subscription_id: ObjectId,
  /// Sampled value identity.
  pub value_id: ObjectId,
  /// Rendered-frame token used to prove coalescing.
  pub frame: u64,
  /// Presentation value.
  pub value: MotionValue,
  /// Per-second velocity using the same value shape.
  pub velocity: MotionValue,
  /// Whether this sample follows a seek, loop, replacement, or reconnect jump.
  pub discontinuity: bool,
}

/// Concrete or named target broadcast through animation controls.
#[derive(Clone, Debug, PartialEq)]
pub enum MotionControlTarget {
  /// Fully lowered imperative target.
  Target(MotionTargetDescriptor),
  /// Named target resolved from the bound host's variant map.
  Variant(String),
}

/// One named target retained for imperative variant starts.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionNamedTarget {
  /// Stable variant label.
  pub name: String,
  /// Fully resolved target for the descriptor's custom-data snapshot.
  pub target: MotionTargetDescriptor,
}

/// Broadcast operation for one typed animation-controls identity.
#[derive(Clone, Debug, PartialEq)]
pub enum MotionControlCommand {
  /// Starts one imperative generation on the current binding snapshot.
  Start {
    /// Stable playback identity.
    playback_id: ObjectId,
    /// Playback generation.
    generation: u32,
    /// Concrete or named target.
    target: MotionControlTarget,
  },
  /// Applies one target immediately.
  Set(MotionControlTarget),
  /// Freezes every active controlled host.
  Stop,
  /// Removes the imperative layer.
  Clear,
}

/// Addressed animation-controls operation.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionControlOperation {
  /// Stable controls identity.
  pub control_id: ObjectId,
  /// Broadcast operation.
  pub command: MotionControlCommand,
}

/// Closed selector resolved inside one animation scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MotionSelector {
  /// One exact Reactant host identity.
  Element(ObjectId),
  /// Hosts carrying one motion name.
  Name(String),
  /// The scope root itself.
  ScopeRoot,
  /// Direct visual children of the scope root.
  Children,
  /// Every visual descendant of the scope root.
  Descendants,
}

/// How one sequence entry becomes eligible.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MotionSequenceSchedule {
  /// Starts at a fixed sequence playhead time.
  Absolute(u64),
  /// Starts at a signed offset from another entry's start.
  RelativeStart {
    /// Referenced declaration-order entry.
    entry: u32,
    /// Signed offset from its actual start.
    offset_micros: i64,
  },
  /// Starts at a signed offset from another entry's actual completion.
  AfterCompletion {
    /// Referenced declaration-order entry.
    entry: u32,
    /// Signed offset from its actual completion.
    offset_micros: i64,
  },
  /// Starts at a signed offset from a named label event.
  Label {
    /// Referenced label name.
    name: String,
    /// Signed offset from the label occurrence.
    offset_micros: i64,
  },
}

/// Whether an accidental property collision is rejected or explicitly replaced.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MotionSequenceConflict {
  /// Reject a graph which can write the same property concurrently.
  #[default]
  Reject,
  /// Interrupt the previous writer when this entry becomes eligible.
  Replace,
}

/// When a referenced position is sampled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionReferenceResolution {
  /// Resolve once when the sequence starts.
  CaptureAtStart,
  /// Follow the referenced position while the entry is active.
  Follow,
}

/// One typed host or named world-anchor position used by a sequence target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MotionPositionReference {
  /// Referenced host identity.
  pub object_id: ObjectId,
  /// Optional named anchor inside a prepared world visual.
  pub anchor: Option<String>,
  /// Capture or live-follow behavior.
  pub resolution: MotionReferenceResolution,
}

/// Immutable audio parameters captured when a sequence is submitted.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionSoundOccurrence {
  /// Prepared audio-clip address.
  pub address: String,
  /// Initial linear volume in the inclusive range zero through one.
  pub volume: f64,
  /// Positive playback pitch.
  pub pitch: f64,
  /// Whether playback loops independently after the occurrence.
  pub looping: bool,
  /// Optional fade-in duration in milliseconds.
  pub fade_in_ms: u64,
}

/// Immutable particle-burst parameters captured when a sequence is submitted.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionParticleOccurrence {
  /// Prepared particle-effect address.
  pub address: String,
  /// Typed world host or anchor used for placement.
  pub position: MotionPositionReference,
  /// Effect lifetime in milliseconds.
  pub lifetime_ms: u64,
}

/// One immutable declaration-order entry in a scoped sequence graph.
#[derive(Clone, Debug, PartialEq)]
pub enum MotionSequenceEntry {
  /// Starts a selector-target animation when its dependency becomes eligible.
  Animate {
    /// Selector snapshotted when this entry starts.
    selector: MotionSelector,
    /// Fully lowered property target.
    target: MotionTargetDescriptor,
    /// Optional typed position destination.
    position: Option<MotionPositionReference>,
    /// Timing used by generated position tracks.
    position_transition: Box<TransitionDefinition>,
    /// Eligibility dependency.
    schedule: MotionSequenceSchedule,
    /// Property conflict behavior.
    conflict: MotionSequenceConflict,
  },
  /// Emits one correlated label event when its dependency becomes eligible.
  Label {
    /// Stable authoring label.
    name: String,
    /// Eligibility dependency.
    schedule: MotionSequenceSchedule,
  },
  /// Starts one prepared audio occurrence when eligible.
  Sound {
    /// Captured audio parameters.
    sound: MotionSoundOccurrence,
    /// Eligibility dependency.
    schedule: MotionSequenceSchedule,
  },
  /// Starts one prepared particle burst when eligible.
  Particle {
    /// Captured effect parameters and placement.
    particle: MotionParticleOccurrence,
    /// Eligibility dependency.
    schedule: MotionSequenceSchedule,
  },
}

/// Scoped animation operation.
#[derive(Clone, Debug, PartialEq)]
pub enum MotionScopeCommand {
  /// Starts one scheduled sequence.
  Start {
    /// Stable playback identity.
    playback_id: ObjectId,
    /// Playback generation.
    generation: u32,
    /// Immutable declaration-order dependency graph.
    entries: Vec<MotionSequenceEntry>,
  },
  /// Applies one scoped target immediately.
  Set {
    /// Target selector.
    selector: MotionSelector,
    /// Immediate target.
    target: MotionTargetDescriptor,
  },
  /// Freezes active tracks selected inside the scope.
  Stop(MotionSelector),
}

/// Addressed animation-scope operation.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionScopeOperation {
  /// Stable scope identity.
  pub scope_id: ObjectId,
  /// Scoped operation.
  pub command: MotionScopeCommand,
}

/// Validates identities, dependencies, cycles, ranges, and bindings.
pub fn validate_motion_graph(
  nodes: &[MotionValueDescriptor],
  bindings: &[MotionValueBinding],
  subscriptions: &[MotionValueSubscription],
) -> Result<(), String> {
  let mut by_id = HashMap::new();
  for (index, node) in nodes.iter().enumerate() {
    node.initial.validate().map_err(str::to_owned)?;
    if by_id.insert(node.value_id, index).is_some() {
      return Err("motion-value graph repeats a value identity".to_owned());
    }
    validate_source(node)?;
  }
  let dependencies = nodes.iter().map(dependencies).collect::<Vec<_>>();
  for values in &dependencies {
    if values.iter().any(|value| !by_id.contains_key(value)) {
      return Err("motion-value graph references an unavailable input".to_owned());
    }
  }
  let mut visiting = HashSet::new();
  let mut visited = HashSet::new();
  for node in nodes {
    visit(
      node.value_id,
      &by_id,
      &dependencies,
      &mut visiting,
      &mut visited,
    )?;
  }
  let mut properties = HashSet::new();
  for binding in bindings {
    if binding.composition == MotionBindingComposition::Compose
      && binding.property != MotionProperty::Scale
    {
      return Err("graph composition supports scale factors".to_owned());
    }
    if !by_id.contains_key(&binding.value_id) {
      return Err("motion-value binding references an unavailable value".to_owned());
    }
    if !properties.insert(binding.property) {
      return Err("motion-value bindings repeat a host property".to_owned());
    }
  }
  let mut subscription_ids = HashSet::new();
  for subscription in subscriptions {
    if !by_id.contains_key(&subscription.value_id) {
      return Err("motion-value subscription references an unavailable value".to_owned());
    }
    if !subscription_ids.insert(subscription.subscription_id) {
      return Err("motion-value subscriptions repeat an identity".to_owned());
    }
  }
  Ok(())
}

/// Validates one immutable sequence graph before any host property changes.
pub fn validate_motion_sequence(entries: &[MotionSequenceEntry]) -> Result<(), String> {
  let mut labels = HashMap::new();
  for (index, entry) in entries.iter().enumerate() {
    match entry {
      MotionSequenceEntry::Animate {
        selector,
        target,
        position,
        position_transition,
        ..
      } => {
        validate_selector(selector)?;
        target.validate()?;
        position_transition.validate().map_err(str::to_owned)?;
        if position
          .as_ref()
          .and_then(|value| value.anchor.as_ref())
          .is_some_and(String::is_empty)
        {
          return Err("Motion sequence position anchor is empty".to_owned());
        }
      }
      MotionSequenceEntry::Label { name, .. } => {
        if name.is_empty() {
          return Err("Motion sequence label is empty".to_owned());
        }
        if labels.insert(name.as_str(), index).is_some() {
          return Err(format!("Motion sequence repeats label {name}"));
        }
      }
      MotionSequenceEntry::Sound { sound, .. } => {
        if sound.address.is_empty()
          || !sound.volume.is_finite()
          || !(0.0..=1.0).contains(&sound.volume)
          || !sound.pitch.is_finite()
          || sound.pitch <= 0.0
          || sound.pitch > 3.0
        {
          return Err("Motion sequence sound is invalid".to_owned());
        }
      }
      MotionSequenceEntry::Particle { particle, .. } => {
        if particle.address.is_empty() || particle.lifetime_ms == 0 {
          return Err("Motion sequence particle burst is invalid".to_owned());
        }
        if particle
          .position
          .anchor
          .as_ref()
          .is_some_and(String::is_empty)
        {
          return Err("Motion sequence particle anchor is empty".to_owned());
        }
      }
    }
  }
  let dependencies = entries
    .iter()
    .map(|entry| dependency(entry, &labels, entries.len()))
    .collect::<Result<Vec<_>, _>>()?;
  for (index, dependency) in dependencies.iter().enumerate() {
    if dependency.is_some_and(|value| value == index) {
      return Err("Motion sequence contains a dependency cycle".to_owned());
    }
    if matches!(
      schedule(&entries[index]),
      MotionSequenceSchedule::AfterCompletion { .. }
    ) && dependency.is_some_and(|value| infinite(&entries[value]))
    {
      return Err("Motion sequence requires completion from an infinite entry".to_owned());
    }
  }
  let mut visiting = HashSet::new();
  let mut visited = HashSet::new();
  for index in 0..entries.len() {
    visit_sequence(index, &dependencies, &mut visiting, &mut visited)?;
  }
  Ok(())
}

fn validate_selector(selector: &MotionSelector) -> Result<(), String> {
  if matches!(selector, MotionSelector::Name(value) if value.is_empty()) {
    return Err("Motion sequence selector name is empty".to_owned());
  }
  Ok(())
}

fn schedule(entry: &MotionSequenceEntry) -> &MotionSequenceSchedule {
  match entry {
    MotionSequenceEntry::Animate { schedule, .. }
    | MotionSequenceEntry::Label { schedule, .. }
    | MotionSequenceEntry::Sound { schedule, .. }
    | MotionSequenceEntry::Particle { schedule, .. } => schedule,
  }
}

fn dependency(
  entry: &MotionSequenceEntry,
  labels: &HashMap<&str, usize>,
  count: usize,
) -> Result<Option<usize>, String> {
  let value = match schedule(entry) {
    MotionSequenceSchedule::Absolute(_) => return Ok(None),
    MotionSequenceSchedule::RelativeStart { entry, .. }
    | MotionSequenceSchedule::AfterCompletion { entry, .. } => usize::try_from(*entry)
      .ok()
      .filter(|value| *value < count)
      .ok_or_else(|| "Motion sequence references a missing entry".to_owned())?,
    MotionSequenceSchedule::Label { name, .. } => *labels
      .get(name.as_str())
      .ok_or_else(|| format!("Motion sequence references missing label {name}"))?,
  };
  Ok(Some(value))
}

fn infinite(entry: &MotionSequenceEntry) -> bool {
  let MotionSequenceEntry::Animate {
    target,
    position,
    position_transition,
    ..
  } = entry
  else {
    return false;
  };
  target
    .tracks
    .iter()
    .any(|track| track.transition.repeat == crate::MotionRepeat::Forever)
    || (position.is_some() && position_transition.repeat == crate::MotionRepeat::Forever)
}

fn visit_sequence(
  index: usize,
  dependencies: &[Option<usize>],
  visiting: &mut HashSet<usize>,
  visited: &mut HashSet<usize>,
) -> Result<(), String> {
  if visited.contains(&index) {
    return Ok(());
  }
  if !visiting.insert(index) {
    return Err("Motion sequence contains a dependency cycle".to_owned());
  }
  if let Some(dependency) = dependencies[index] {
    visit_sequence(dependency, dependencies, visiting, visited)?;
  }
  visiting.remove(&index);
  visited.insert(index);
  Ok(())
}

fn validate_source(node: &MotionValueDescriptor) -> Result<(), String> {
  match &node.source {
    MotionValueSource::Range { input, output, .. } => {
      if input.len() < 2 || input.len() != output.len() {
        return Err("motion-value ranges require aligned ranges of at least two values".to_owned());
      }
      for value in input.iter().chain(output) {
        value.validate().map_err(str::to_owned)?;
      }
    }
    MotionValueSource::Spring { configuration, .. } => {
      configuration.validate().map_err(str::to_owned)?;
    }
    MotionValueSource::Expression { operation, inputs } => {
      let expected = match operation {
        MotionExpressionOperation::Power(_)
        | MotionExpressionOperation::SquareRoot
        | MotionExpressionOperation::Absolute
        | MotionExpressionOperation::Clamp { .. }
        | MotionExpressionOperation::Modulo(_)
        | MotionExpressionOperation::Wrap { .. }
        | MotionExpressionOperation::ExponentialDecay { .. } => 1,
        MotionExpressionOperation::Mix => 3,
        _ => 2,
      };
      if inputs.len() != expected {
        return Err("motion expression has the wrong input arity".to_owned());
      }
      validate_operation(*operation)?;
    }
    MotionValueSource::Mutable
    | MotionValueSource::Time(_)
    | MotionValueSource::Velocity { .. } => {}
  }
  Ok(())
}

fn validate_operation(operation: MotionExpressionOperation) -> Result<(), String> {
  let finite = match operation {
    MotionExpressionOperation::Power(value)
    | MotionExpressionOperation::Modulo(value)
    | MotionExpressionOperation::ExponentialDecay { rate: value } => value.is_finite(),
    MotionExpressionOperation::Clamp { min, max }
    | MotionExpressionOperation::Wrap { min, max } => {
      min.is_finite() && max.is_finite() && min < max
    }
    _ => true,
  };
  if !finite {
    return Err("motion expression contains invalid finite bounds".to_owned());
  }
  if matches!(operation, MotionExpressionOperation::Modulo(value) if value <= 0.0) {
    return Err("motion modulo requires a positive modulus".to_owned());
  }
  if matches!(operation, MotionExpressionOperation::ExponentialDecay { rate } if rate < 0.0) {
    return Err("motion exponential decay requires a nonnegative rate".to_owned());
  }
  Ok(())
}

fn dependencies(node: &MotionValueDescriptor) -> Vec<ObjectId> {
  match &node.source {
    MotionValueSource::Mutable | MotionValueSource::Time(_) => Vec::new(),
    MotionValueSource::Velocity { source }
    | MotionValueSource::Range { source, .. }
    | MotionValueSource::Spring { source, .. } => vec![*source],
    MotionValueSource::Expression { inputs, .. } => inputs.clone(),
  }
}

fn visit(
  value: ObjectId,
  by_id: &HashMap<ObjectId, usize>,
  dependencies: &[Vec<ObjectId>],
  visiting: &mut HashSet<ObjectId>,
  visited: &mut HashSet<ObjectId>,
) -> Result<(), String> {
  if visited.contains(&value) {
    return Ok(());
  }
  if !visiting.insert(value) {
    return Err("motion-value graph contains a cycle".to_owned());
  }
  for dependency in &dependencies[by_id[&value]] {
    visit(*dependency, by_id, dependencies, visiting, visited)?;
  }
  visiting.remove(&value);
  visited.insert(value);
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{MotionReferenceResolution, MotionRepeat};

  #[test]
  fn sequence_validation_rejects_cycles_and_missing_labels() {
    let cycle = vec![
      label(
        "a",
        MotionSequenceSchedule::Label {
          name: "b".into(),
          offset_micros: 0,
        },
      ),
      label(
        "b",
        MotionSequenceSchedule::Label {
          name: "a".into(),
          offset_micros: 0,
        },
      ),
    ];
    assert!(
      validate_motion_sequence(&cycle)
        .unwrap_err()
        .contains("cycle")
    );
    assert!(
      validate_motion_sequence(&[label(
        "a",
        MotionSequenceSchedule::Label {
          name: "missing".into(),
          offset_micros: 0,
        },
      )])
      .unwrap_err()
      .contains("missing label")
    );
  }

  #[test]
  fn sequence_validation_rejects_completion_dependency_on_infinite_position() {
    let mut transition = TransitionDefinition::tween();
    transition.repeat = MotionRepeat::Forever;
    let entries = vec![
      MotionSequenceEntry::Animate {
        selector: MotionSelector::ScopeRoot,
        target: MotionTargetDescriptor {
          tracks: Vec::new(),
          transition_end: Vec::new(),
        },
        position: Some(MotionPositionReference {
          object_id: ObjectId::new_v4(),
          anchor: None,
          resolution: MotionReferenceResolution::Follow,
        }),
        position_transition: Box::new(transition),
        schedule: MotionSequenceSchedule::Absolute(0),
        conflict: MotionSequenceConflict::Reject,
      },
      label(
        "never",
        MotionSequenceSchedule::AfterCompletion {
          entry: 0,
          offset_micros: 0,
        },
      ),
    ];
    assert!(
      validate_motion_sequence(&entries)
        .unwrap_err()
        .contains("infinite")
    );
  }

  fn label(name: &str, schedule: MotionSequenceSchedule) -> MotionSequenceEntry {
    MotionSequenceEntry::Label {
      name: name.into(),
      schedule,
    }
  }
}

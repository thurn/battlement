use std::collections::{HashMap, HashSet};

use battlement_types::ObjectId;
use serde::{Deserialize, Serialize};

use crate::{
  MotionClockSource, MotionProperty, MotionTargetDescriptor, MotionValue, SpringConfiguration,
  TransitionDefinition,
};

/// One closed operation evaluated by Unity's motion-value graph.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionValueDescriptor {
  /// Runtime-unique value identity.
  pub value_id: ObjectId,
  /// Typed mount or reconstruction value.
  pub initial: MotionValue,
  /// Native source or derived operation.
  pub source: MotionValueSource,
}

/// One host property driven directly by a graph value.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MotionValueBinding {
  /// Property receiving the sampled value.
  pub property: MotionProperty,
  /// Value whose shape must match the property catalog.
  pub value_id: ObjectId,
}

/// Explicit replaceable event requested for one value.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum MotionValueEventKind {
  /// Latest presentation value.
  Change,
  /// Latest per-second velocity.
  Velocity,
  /// Presentation value at the rendered-frame boundary.
  AnimationFrame,
}

/// One explicit Rust-side graph observation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MotionValueSubscription {
  /// Stable subscription identity used for coalescing.
  pub subscription_id: ObjectId,
  /// Observed graph value.
  pub value_id: ObjectId,
  /// Requested sample channel.
  pub event: MotionValueEventKind,
}

/// Mutable-value operation issued outside render.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionValueOperation {
  /// Target mutable value.
  pub value_id: ObjectId,
  /// Operation to apply.
  pub command: MotionValueCommand,
}

/// Generation-checked operation for a motion-value playback.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionValuePlaybackOperation {
  /// Stable start identity.
  pub playback_id: ObjectId,
  /// Required current generation.
  pub generation: u32,
  /// Playback mutation.
  pub command: crate::MotionPlaybackCommand,
}

/// One coalesced value sample returned only for an explicit subscription.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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
  #[serde(default)]
  pub discontinuity: bool,
}

/// Concrete or named target broadcast through animation controls.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum MotionControlTarget {
  /// Fully lowered imperative target.
  Target(MotionTargetDescriptor),
  /// Named target resolved from the bound host's variant map.
  Variant(String),
}

/// One named target retained for imperative variant starts.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionNamedTarget {
  /// Stable variant label.
  pub name: String,
  /// Fully resolved target for the descriptor's custom-data snapshot.
  pub target: MotionTargetDescriptor,
}

/// Broadcast operation for one typed animation-controls identity.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionControlOperation {
  /// Stable controls identity.
  pub control_id: ObjectId,
  /// Broadcast operation.
  pub command: MotionControlCommand,
}

/// Closed selector resolved inside one animation scope.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

/// One scheduled scoped animation step.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionSequenceStep {
  /// Selector snapshotted when the step becomes eligible.
  pub selector: MotionSelector,
  /// Fully lowered target.
  pub target: MotionTargetDescriptor,
  /// Absolute start offset from sequence activation.
  pub start_micros: u64,
}

/// Scoped animation operation.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum MotionScopeCommand {
  /// Starts one scheduled sequence.
  Start {
    /// Stable playback identity.
    playback_id: ObjectId,
    /// Playback generation.
    generation: u32,
    /// Ordered sequence steps.
    steps: Vec<MotionSequenceStep>,
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
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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

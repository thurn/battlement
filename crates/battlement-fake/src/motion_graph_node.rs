//! Derived Motion values evaluated in dependency order on the presentation clock.

use std::collections::HashMap;

use battlement::{
  Color, Length, MotionClockSource, MotionExpressionOperation, MotionValue, MotionValueDescriptor,
  MotionValueSource, ObjectId, TransitionDefinition, TransitionGenerator,
};

use crate::{motion_mix, motion_scalar};

pub(crate) struct Node {
  pub(crate) definition: MotionValueDescriptor,
  pub(crate) value: MotionValue,
  pub(crate) velocity: MotionValue,
  pub(crate) changed: bool,
  pub(crate) discontinuity: bool,
  previous_micros: u64,
  spring_velocity: f64,
  spring: Option<(MotionValue, MotionValue, i128)>,
}

impl Node {
  pub(crate) fn new(definition: MotionValueDescriptor) -> Self {
    Self {
      value: definition.initial.clone(),
      velocity: zero(&definition.initial),
      definition,
      changed: true,
      discontinuity: false,
      previous_micros: 0,
      spring_velocity: 0.0,
      spring: None,
    }
  }

  pub(crate) fn set(&mut self, value: MotionValue, discontinuity: bool) {
    self.velocity = zero(&value);
    self.value = value;
    self.changed = true;
    self.discontinuity |= discontinuity;
    self.spring = None;
  }

  pub(crate) fn stop(&mut self) {
    self.velocity = zero(&self.value);
    self.changed = true;
    self.spring = None;
  }

  pub(crate) fn rebase(&mut self, now: u64) {
    self.previous_micros = 0;
    if let Some((_, _, anchor)) = &mut self.spring {
      *anchor -= i128::from(now);
    }
    self.discontinuity = true;
  }

  pub(crate) fn deadline(&self, now: u64) -> Option<u64> {
    let (origin, target, anchor) = self.spring.as_ref()?;
    if self.value == *target {
      return None;
    }
    let MotionValueSource::Spring { configuration, .. } = self.definition.source else {
      return None;
    };
    let duration = match (origin, target) {
      (MotionValue::Scalar(origin), MotionValue::Scalar(target)) => motion_scalar::spring_duration(
        f64::from(*origin),
        f64::from(*target),
        self.spring_velocity,
        configuration,
      ),
      _ => 300_000,
    };
    Some(
      u64::try_from((*anchor + i128::from(duration)).max(i128::from(now) + 1))
        .expect("Motion spring deadline overflow"),
    )
  }

  pub(crate) fn evaluate(
    &mut self,
    graph: &HashMap<ObjectId, Node>,
    mut clock: impl FnMut(MotionClockSource) -> u64,
  ) -> bool {
    let inputs_changed = dependencies(&self.definition.source)
      .iter()
      .any(|id| graph[id].changed || graph[id].discontinuity);
    let continuous = matches!(self.definition.source, MotionValueSource::Time(_))
      || self
        .spring
        .as_ref()
        .is_some_and(|(_, target, _)| *target != self.value);
    if !self.changed && !inputs_changed && !continuous {
      return true;
    }
    let now = clock(match self.definition.source {
      MotionValueSource::Time(source) => source,
      _ => MotionClockSource::Unscaled,
    });
    let next = match &self.definition.source {
      MotionValueSource::Mutable => self.value.clone(),
      MotionValueSource::Time(_) => MotionValue::Scalar((now as f64 / 1_000_000.0) as f32),
      MotionValueSource::Velocity { source } => graph[source].velocity.clone(),
      MotionValueSource::Range {
        source,
        input,
        output,
        clamp,
      } => {
        let value = scalar(&graph[source].value);
        let segment = (0..input.len() - 1)
          .find(|i| value <= scalar(&input[i + 1]))
          .unwrap_or(input.len() - 2);
        let left = scalar(&input[segment]);
        let mut progress = (value - left) / (scalar(&input[segment + 1]) - left);
        if *clamp {
          progress = progress.clamp(0.0, 1.0);
        }
        match (&output[segment], &output[segment + 1]) {
          (MotionValue::Scalar(a), MotionValue::Scalar(b)) => {
            MotionValue::Scalar((f64::from(*a) + (f64::from(*b) - f64::from(*a)) * progress) as f32)
          }
          (a, b) => motion_mix::mix(a, b, progress),
        }
      }
      MotionValueSource::Expression { operation, inputs } => {
        let value = |i: usize| &graph[&inputs[i]].value;
        if matches!(operation, MotionExpressionOperation::Mix)
          && !matches!(value(0), MotionValue::Scalar(_))
        {
          motion_mix::mix(value(0), value(1), scalar(value(2)))
        } else {
          MotionValue::Scalar(expression(
            *operation,
            scalar(value(0)),
            if inputs.len() > 1 {
              scalar(value(1))
            } else {
              0.0
            },
            if inputs.len() > 2 {
              scalar(value(2))
            } else {
              0.0
            },
          ) as f32)
        }
      }
      MotionValueSource::Spring {
        source,
        configuration,
      } => {
        let target = &graph[source].value;
        if self
          .spring
          .as_ref()
          .is_none_or(|(_, previous, _)| previous != target)
        {
          self.spring_velocity = match self.velocity {
            MotionValue::Scalar(value) => f64::from(value),
            _ => 0.0,
          };
          self.spring = Some((self.value.clone(), target.clone(), i128::from(now)));
        }
        let (origin, target, anchor) = self.spring.as_ref().unwrap();
        if let (MotionValue::Scalar(a), MotionValue::Scalar(b)) = (origin, target) {
          let mut transition = TransitionDefinition::spring();
          transition.generator = TransitionGenerator::Spring(*configuration);
          MotionValue::Scalar(
            motion_scalar::sample(
              f64::from(*a),
              f64::from(*b),
              self.spring_velocity,
              &transition,
              (i128::from(now) - *anchor).max(0) as u64,
            )
            .value as f32,
          )
        } else {
          motion_mix::mix(
            origin,
            target,
            (i128::from(now) - *anchor).max(0) as f64 / 300_000.0,
          )
        }
      }
    };
    if next.validate().is_err() {
      return false;
    }
    let delta = now.saturating_sub(self.previous_micros);
    self.velocity = match (&next, &self.value) {
      (MotionValue::Scalar(a), MotionValue::Scalar(b)) if delta > 0 && !self.discontinuity => {
        MotionValue::Scalar(((f64::from(*a) - f64::from(*b)) / (delta as f64 / 1_000_000.0)) as f32)
      }
      _ => zero(&next),
    };
    self.changed |= next != self.value;
    self.value = next;
    self.previous_micros = now;
    true
  }
}

pub(crate) fn dependencies(source: &MotionValueSource) -> Vec<ObjectId> {
  match source {
    MotionValueSource::Mutable | MotionValueSource::Time(_) => Vec::new(),
    MotionValueSource::Velocity { source }
    | MotionValueSource::Range { source, .. }
    | MotionValueSource::Spring { source, .. } => vec![*source],
    MotionValueSource::Expression { inputs, .. } => inputs.clone(),
  }
}

pub(crate) fn zero(value: &MotionValue) -> MotionValue {
  match value {
    MotionValue::Scalar(_) => MotionValue::Scalar(0.0),
    MotionValue::Length(_) => MotionValue::Length(Length::Px(0.0)),
    MotionValue::Angle(_) => MotionValue::Angle(0.0),
    MotionValue::Color(_) => MotionValue::Color(Color::rgba(0.0, 0.0, 0.0, 0.0)),
    MotionValue::Vector2(_) => MotionValue::Vector2([0.0; 2]),
    MotionValue::Vector3(_) => MotionValue::Vector3([0.0; 3]),
    _ => MotionValue::Scalar(0.0),
  }
}

fn scalar(value: &MotionValue) -> f64 {
  let MotionValue::Scalar(value) = value else {
    panic!("Motion graph expects a scalar input");
  };
  f64::from(*value)
}

fn expression(operation: MotionExpressionOperation, a: f64, b: f64, t: f64) -> f64 {
  match operation {
    MotionExpressionOperation::Add => a + b,
    MotionExpressionOperation::Subtract => a - b,
    MotionExpressionOperation::Multiply => a * b,
    MotionExpressionOperation::Divide => a / b,
    MotionExpressionOperation::Power(value) => a.powf(value),
    MotionExpressionOperation::SquareRoot => a.sqrt(),
    MotionExpressionOperation::Absolute => a.abs(),
    MotionExpressionOperation::Minimum => a.min(b),
    MotionExpressionOperation::Maximum => a.max(b),
    MotionExpressionOperation::Clamp { min, max } => a.clamp(min, max),
    MotionExpressionOperation::Modulo(value) => a.rem_euclid(value),
    MotionExpressionOperation::Wrap { min, max } => min + (a - min).rem_euclid(max - min),
    MotionExpressionOperation::ExponentialDecay { rate } => (-rate * a).exp(),
    MotionExpressionOperation::Mix => a + (b - a) * t,
  }
}

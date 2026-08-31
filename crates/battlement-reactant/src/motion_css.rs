//! CSS-style transitions, reusable animations, and decoration layers.

use std::hash::{DefaultHasher, Hash, Hasher};

use battlement::{
  AdditiveRule, AnimationComposition as ProtocolComposition,
  AnimationDirection as ProtocolDirection, AnimationFill as ProtocolFill,
  AnimationPlayState as ProtocolPlayState, CssAnimationDescriptor, CssPropertyTrack,
  DecorationOverflow as ProtocolOverflow, DecorationPlacement,
  DecorationPosition as ProtocolPosition, MotionDecorationDescriptor, MotionGeneration,
  MotionProperty, MotionPseudoState, MotionPseudoStyle, MotionRepeat, MotionRepeatType, Prop,
  Style, StylePropertyTransition, StyleTransitionDescriptor, StyleValue, TransitionGenerator,
};

use crate::motion::{Easing, Keyframes, MotionStyle, Transition};

/// A typed pseudo-state style input.
pub trait IntoPseudoStyle {
  #[doc(hidden)]
  fn into_pseudo_style(self) -> MotionStyle;
}

impl IntoPseudoStyle for MotionStyle {
  fn into_pseudo_style(self) -> MotionStyle {
    self
  }
}

impl IntoPseudoStyle for Style {
  fn into_pseudo_style(self) -> MotionStyle {
    let mut result = MotionStyle::new();
    if let Prop::Set(StyleValue::Value(value)) = self.opacity {
      result = result.opacity(value.0);
    }
    if let Prop::Set(StyleValue::Value(value)) = self.background_color {
      result = result.background_color(battlement::MotionColor::new(
        value.r as f32,
        value.g as f32,
        value.b as f32,
        value.a as f32,
      ));
    }
    if let Prop::Set(StyleValue::Value(value)) = self.color {
      result = result.color(battlement::MotionColor::new(
        value.r as f32,
        value.g as f32,
        value.b as f32,
        value.a as f32,
      ));
    }
    if let Prop::Set(StyleValue::Value(value)) = self.scale {
      result = result.scale_x(value.x).scale_y(value.y);
    }
    if let Prop::Set(StyleValue::Value(value)) = self.rotate {
      assert!(
        value.x == 0.0 && value.y == 0.0 && value.z == 1.0,
        "pseudo-style rotation must use the panel axis"
      );
      result = result.rotate(value.degrees);
    }
    result
  }
}

/// A CSS-transition property identity.
pub type StyleProperty = MotionProperty;

/// CSS animation composition behavior.
pub use battlement::AnimationComposition;
/// CSS animation playback direction.
pub use battlement::AnimationDirection;
/// CSS animation fill behavior.
pub use battlement::AnimationFill;
/// Exact CSS iteration count.
pub use battlement::AnimationIterations;
/// CSS animation play state.
pub use battlement::AnimationPlayState;
/// Decoration clip policy.
pub use battlement::DecorationOverflow;
/// Decoration geometry policy.
pub use battlement::DecorationPosition;

/// Typed transition rules over static and pseudo-state style changes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleTransition {
  properties: Vec<(StyleProperty, Transition)>,
  all: Option<Transition>,
  allow_discrete: bool,
}

/// One reusable CSS-style keyframe animation.
#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
  frames: Keyframes<MotionStyle>,
  duration_micros: u64,
  delay_micros: i64,
  easings: Vec<Easing>,
  iterations: AnimationIterations,
  direction: AnimationDirection,
  fill: AnimationFill,
  play_state: AnimationPlayState,
  composition: AnimationComposition,
  key: Option<u64>,
  diagnostic_name: Option<String>,
}

/// One keyed non-interactive paint layer.
#[derive(Clone, Debug, PartialEq)]
pub struct Decoration {
  style: Style,
  position: DecorationPosition,
  overflow: DecorationOverflow,
  key: Option<u64>,
  animations: Vec<Animation>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CssProps {
  pub(crate) hover: Option<MotionStyle>,
  pub(crate) focus: Option<MotionStyle>,
  pub(crate) active: Option<MotionStyle>,
  pub(crate) disabled: Option<MotionStyle>,
  pub(crate) transition: StyleTransition,
  pub(crate) animations: Vec<Animation>,
  pub(crate) before: Vec<Decoration>,
  pub(crate) after: Vec<Decoration>,
}

impl StyleTransition {
  /// Creates an empty transition declaration.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Assigns complete timing to one property.
  #[must_use]
  pub fn property(mut self, property: StyleProperty, transition: Transition) -> Self {
    validate_transition(&transition);
    self.properties.retain(|entry| entry.0 != property);
    self.properties.push((property, transition));
    self
  }

  /// Assigns default timing to every changed interpolable property.
  #[must_use]
  pub fn all(mut self, transition: Transition) -> Self {
    validate_transition(&transition);
    self.all = Some(transition);
    self
  }

  /// Enables midpoint switching for explicitly selected discrete properties.
  #[must_use]
  pub fn allow_discrete(mut self, value: bool) -> Self {
    self.allow_discrete = value;
    self
  }
}

impl Animation {
  /// Creates a one-second, once-running keyframe animation.
  #[must_use]
  pub fn new(frames: Keyframes<MotionStyle>) -> Self {
    assert!(
      frames.values.len() >= 2,
      "CSS keyframes require at least two frames"
    );
    if let Some(times) = &frames.times {
      crate::motion::validate_times(times);
      assert_eq!(
        times.len(),
        frames.values.len(),
        "CSS frame times must match frames"
      );
    }
    Self {
      frames,
      duration_micros: 1_000_000,
      delay_micros: 0,
      easings: vec![Easing::EaseInOut],
      iterations: AnimationIterations::Once,
      direction: AnimationDirection::Normal,
      fill: AnimationFill::None,
      play_state: AnimationPlayState::Running,
      composition: AnimationComposition::Replace,
      key: None,
      diagnostic_name: None,
    }
  }

  /// Sets active duration per iteration.
  #[must_use]
  pub fn duration_secs(mut self, value: f64) -> Self {
    self.duration_micros = crate::motion::micros(value, true);
    self
  }

  /// Sets signed start delay; a negative delay seeks into playback.
  #[must_use]
  pub fn delay_secs(mut self, value: f64) -> Self {
    assert!(value.is_finite(), "animation delay must be finite");
    self.delay_micros = (value * 1_000_000.0).round() as i64;
    self
  }

  /// Sets segment easing.
  #[must_use]
  pub fn ease(mut self, value: Easing) -> Self {
    self.easings = vec![value];
    self
  }

  /// Sets one easing per complete keyframe segment.
  #[must_use]
  pub fn easings(mut self, values: impl IntoIterator<Item = Easing>) -> Self {
    self.easings = values.into_iter().collect();
    assert!(
      self.easings.len() == 1 || self.easings.len() + 1 == self.frames.values.len(),
      "CSS easings require one value or one value per frame segment"
    );
    self
  }
  /// Sets exact CSS iteration behavior.
  #[must_use]
  pub fn iterations(mut self, value: AnimationIterations) -> Self {
    self.iterations = value;
    self
  }
  /// Sets iteration direction.
  #[must_use]
  pub fn direction(mut self, value: AnimationDirection) -> Self {
    self.direction = value;
    self
  }
  /// Sets fill behavior.
  #[must_use]
  pub fn fill(mut self, value: AnimationFill) -> Self {
    self.fill = value;
    self
  }
  /// Sets running or paused state.
  #[must_use]
  pub fn play_state(mut self, value: AnimationPlayState) -> Self {
    self.play_state = value;
    self
  }
  /// Sets property composition.
  #[must_use]
  pub fn composition(mut self, value: AnimationComposition) -> Self {
    self.composition = value;
    self
  }

  /// Assigns stable dynamic identity and deliberately restarts when it changes.
  #[must_use]
  pub fn animation_key<K: Hash>(mut self, value: K) -> Self {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    self.key = Some(hasher.finish());
    self
  }

  /// Adds a developer-facing trace name.
  #[must_use]
  pub fn diagnostic_name(mut self, value: impl Into<String>) -> Self {
    self.diagnostic_name = Some(value.into());
    self
  }

  fn descriptor(
    &self,
    index: usize,
    generation: MotionGeneration,
  ) -> Option<CssAnimationDescriptor> {
    let plays = match self.iterations {
      AnimationIterations::Count(0) => return None,
      AnimationIterations::Once => MotionRepeat::None,
      AnimationIterations::Count(value) => MotionRepeat::Count(value - 1),
      AnimationIterations::Forever => MotionRepeat::Forever,
    };
    let mut properties = Vec::new();
    for frame in &self.frames.values {
      for entry in &frame.entries {
        if !properties.contains(&entry.property) {
          properties.push(entry.property);
        }
      }
    }
    let times = self.frames.times.clone().unwrap_or_else(|| {
      (0..self.frames.values.len())
        .map(|i| i as f64 / (self.frames.values.len() - 1) as f64)
        .collect()
    });
    let mut tracks = Vec::new();
    for property in properties {
      let mut values = Vec::new();
      let mut property_times = Vec::new();
      let mut frame_indices = Vec::new();
      for (frame_index, (frame, time)) in self.frames.values.iter().zip(&times).enumerate() {
        if let Some(entry) = frame
          .entries
          .iter()
          .find(|entry| entry.property == property)
        {
          values.push(entry.values.last().expect("motion entry is empty").clone());
          property_times.push(*time);
          frame_indices.push(frame_index);
        }
      }
      let track_easings = self.track_easings(&frame_indices);
      let transition = Transition::tween()
        .duration_secs(self.duration_micros as f64 / 1_000_000.0)
        .easings(track_easings);
      let mut definition = transition.for_property(property);
      definition.delay_micros = self.delay_micros;
      definition.repeat = plays;
      definition.repeat_type = match self.direction {
        AnimationDirection::Normal | AnimationDirection::Reverse => MotionRepeatType::Loop,
        AnimationDirection::Alternate => MotionRepeatType::Reverse,
        AnimationDirection::AlternateReverse => MotionRepeatType::Reverse,
      };
      tracks.push(CssPropertyTrack {
        property,
        values,
        times: property_times,
        transition: definition,
      });
    }
    if self.composition != AnimationComposition::Replace {
      for track in &tracks {
        assert!(
          track.property.metadata().additive != AdditiveRule::None,
          "property does not support additive animation"
        );
      }
    }
    let restart_key = animation_restart_key(
      self.key.unwrap_or(1_000 + index as u64),
      &tracks,
      self.direction,
      self.fill,
      self.composition,
    );
    Some(CssAnimationDescriptor {
      slot: self.key.unwrap_or(1_000 + index as u64),
      generation: generation.0,
      restart_key,
      tracks,
      direction: map_direction(self.direction),
      fill: map_fill(self.fill),
      play_state: map_play_state(self.play_state),
      composition: map_composition(self.composition),
      diagnostic_name: self.diagnostic_name.clone(),
    })
  }

  fn track_easings(&self, frame_indices: &[usize]) -> Vec<Easing> {
    if self.easings.len() == 1 {
      return self.easings.clone();
    }
    let last_frame = self.frames.values.len() - 1;
    let mut starts = Vec::new();
    if frame_indices[0] != 0 {
      starts.push(0);
    }
    for index in frame_indices.iter().take(frame_indices.len() - 1) {
      starts.push(*index);
    }
    if *frame_indices.last().expect("CSS property has no keyframe") != last_frame {
      starts.push(*frame_indices.last().expect("CSS property has no keyframe"));
    }
    starts
      .into_iter()
      .map(|index| self.easings[index])
      .collect()
  }
}

impl Decoration {
  /// Creates a static decoration covering its host.
  #[must_use]
  pub fn new() -> Self {
    Self {
      style: Style::new(),
      position: DecorationPosition::Fill,
      overflow: DecorationOverflow::Hidden,
      key: None,
      animations: Vec::new(),
    }
  }
  /// Sets static decoration style.
  #[must_use]
  pub fn style(mut self, value: Style) -> Self {
    self.style = value;
    self
  }
  /// Sets decoration geometry.
  #[must_use]
  pub fn position(mut self, value: DecorationPosition) -> Self {
    self.position = value;
    self
  }
  /// Sets decoration overflow.
  #[must_use]
  pub fn overflow(mut self, value: DecorationOverflow) -> Self {
    self.overflow = value;
    self
  }
  /// Assigns stable dynamic identity.
  #[must_use]
  pub fn key<K: Hash>(mut self, value: K) -> Self {
    let mut h = DefaultHasher::new();
    value.hash(&mut h);
    self.key = Some(h.finish());
    self
  }
  /// Adds one CSS animation.
  #[must_use]
  pub fn animation(mut self, value: Animation) -> Self {
    self.animations.push(value);
    self
  }
}

impl Default for Decoration {
  fn default() -> Self {
    Self::new()
  }
}

impl CssProps {
  pub(crate) const fn new() -> Self {
    Self {
      hover: None,
      focus: None,
      active: None,
      disabled: None,
      transition: StyleTransition {
        properties: Vec::new(),
        all: None,
        allow_discrete: false,
      },
      animations: Vec::new(),
      before: Vec::new(),
      after: Vec::new(),
    }
  }
  pub(crate) fn merge(mut self, value: Self) -> Self {
    if value.hover.is_some() {
      self.hover = value.hover;
    }
    if value.focus.is_some() {
      self.focus = value.focus;
    }
    if value.active.is_some() {
      self.active = value.active;
    }
    if value.disabled.is_some() {
      self.disabled = value.disabled;
    }
    if value.transition != StyleTransition::default() {
      self.transition = value.transition;
    }
    if !value.animations.is_empty() {
      self.animations = value.animations;
    }
    if !value.before.is_empty() {
      self.before = value.before;
    }
    if !value.after.is_empty() {
      self.after = value.after;
    }
    self
  }
  pub(crate) fn pseudo_descriptors(&self) -> Vec<MotionPseudoStyle> {
    [
      (MotionPseudoState::Hover, &self.hover),
      (MotionPseudoState::Focus, &self.focus),
      (MotionPseudoState::Active, &self.active),
      (MotionPseudoState::Disabled, &self.disabled),
    ]
    .into_iter()
    .filter_map(|(state, style)| {
      style.as_ref().map(|style| MotionPseudoStyle {
        state,
        values: style.values(),
      })
    })
    .collect()
  }
  pub(crate) fn transition_descriptor(&self) -> StyleTransitionDescriptor {
    StyleTransitionDescriptor {
      properties: self
        .transition
        .properties
        .iter()
        .map(|(property, transition)| StylePropertyTransition {
          property: *property,
          transition: transition.for_property(*property),
        })
        .collect(),
      all: self
        .transition
        .all
        .as_ref()
        .map(|value| value.for_property(MotionProperty::Opacity)),
      allow_discrete: self.transition.allow_discrete,
    }
  }
  pub(crate) fn animation_descriptors(
    &self,
    generation: MotionGeneration,
  ) -> Vec<CssAnimationDescriptor> {
    self
      .animations
      .iter()
      .enumerate()
      .filter_map(|(index, value)| value.descriptor(index, generation))
      .collect()
  }
  pub(crate) fn decoration_descriptors(
    &self,
    generation: MotionGeneration,
  ) -> Vec<MotionDecorationDescriptor> {
    self
      .before
      .iter()
      .enumerate()
      .map(|(index, value)| {
        decoration_descriptor(value, index, DecorationPlacement::Before, generation)
      })
      .chain(self.after.iter().enumerate().map(|(index, value)| {
        decoration_descriptor(value, index, DecorationPlacement::After, generation)
      }))
      .collect()
  }
}

fn decoration_descriptor(
  value: &Decoration,
  index: usize,
  placement: DecorationPlacement,
  generation: MotionGeneration,
) -> MotionDecorationDescriptor {
  MotionDecorationDescriptor {
    key: value.key.unwrap_or(index as u64),
    placement,
    position: match value.position {
      DecorationPosition::Fill => ProtocolPosition::Fill,
      DecorationPosition::Border => ProtocolPosition::Border,
    },
    overflow: match value.overflow {
      DecorationOverflow::Hidden => ProtocolOverflow::Hidden,
      DecorationOverflow::Visible => ProtocolOverflow::Visible,
    },
    style: value.style.clone(),
    animations: value
      .animations
      .iter()
      .enumerate()
      .filter_map(|(index, value)| value.descriptor(index, generation))
      .collect(),
  }
}

fn animation_restart_key(
  slot: u64,
  tracks: &[CssPropertyTrack],
  direction: AnimationDirection,
  fill: AnimationFill,
  composition: AnimationComposition,
) -> u64 {
  let bytes = serde_json::to_vec(&(slot, tracks, direction, fill, composition))
    .expect("CSS animation restart settings serialize");
  let mut hasher = DefaultHasher::new();
  hasher.write(&bytes);
  hasher.finish()
}

fn validate_transition(value: &Transition) {
  assert!(
    matches!(
      value.default.generator,
      TransitionGenerator::Immediate | TransitionGenerator::Tween { .. }
    ),
    "style transitions accept only tween or immediate timing"
  );
}
fn map_direction(value: AnimationDirection) -> ProtocolDirection {
  value
}
fn map_fill(value: AnimationFill) -> ProtocolFill {
  value
}
fn map_play_state(value: AnimationPlayState) -> ProtocolPlayState {
  value
}
fn map_composition(value: AnimationComposition) -> ProtocolComposition {
  value
}

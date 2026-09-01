//! Typed Motion authoring for Reactant hosts and forwarding components.

use battlement::{
  MotionColor, MotionFilter, MotionGradient, MotionLength, MotionProperty, MotionPropertyTrack,
  MotionPropertyValue, MotionRepeat, MotionRepeatType, MotionShadow, MotionTargetDescriptor,
  MotionTransform, MotionValue, ObjectId, SpringConfiguration, StepPosition, TransitionDefinition,
  TransitionGenerator, Visibility,
};

use crate::{
  animation_controls::{AnimationControls, AnimationScope},
  gesture::GestureProps,
  motion_lifecycle::MotionCallbacks,
  motion_value::{ErasedMotionValue, MotionValue as TypedMotionValue},
  motion_variants::VariantOrchestration,
  variant_map::{ErasedVariantData, ErasedVariantSelection, ErasedVariants},
};

/// A property-local sequence of typed Motion keyframes.
#[derive(Clone, Debug, PartialEq)]
pub struct Keyframes<T> {
  pub(crate) values: Vec<T>,
  pub(crate) times: Option<Vec<f64>>,
}

/// A collection of typed property targets.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MotionStyle {
  pub(crate) entries: Vec<MotionStyleEntry>,
}

/// A style target with optional timing and terminal assignments.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionTarget {
  style: MotionStyle,
  transition: Option<Transition>,
  transition_end: MotionStyle,
  pub(crate) callbacks: MotionCallbacks,
}

/// Complete Motion props forwarded to one host façade.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MotionProps {
  pub(crate) initial: Option<InitialTarget>,
  pub(crate) animate: Option<MotionTarget>,
  pub(crate) exit: Option<MotionTarget>,
  pub(crate) transition: Option<Transition>,
  pub(crate) variants: ErasedVariants,
  pub(crate) variant_data: Option<ErasedVariantData>,
  pub(crate) initial_variant_selection: Option<ErasedVariantSelection>,
  pub(crate) variant_selection: Option<ErasedVariantSelection>,
  pub(crate) exit_variant_selection: Option<ErasedVariantSelection>,
  pub(crate) inherit_variants: bool,
  pub(crate) callbacks: MotionCallbacks,
  pub(crate) css: crate::motion_css::CssProps,
  pub(crate) control_id: Option<ObjectId>,
  pub(crate) scope_id: Option<ObjectId>,
  pub(crate) scope_root: bool,
  pub(crate) motion_name: Option<String>,
  pub(crate) gestures: GestureProps,
}

/// A concrete or disabled mount origin.
#[derive(Clone, Debug, PartialEq)]
pub enum InitialTarget {
  /// A concrete mount origin.
  Target(Box<MotionTarget>),
  /// Suppresses mount animation.
  Disabled,
}

/// Sealed input accepted by [`MotionProps::initial`].
pub trait InitialValue: private::InitialValueSealed {
  #[doc(hidden)]
  fn into_initial(self) -> InitialTarget;
}

/// Tween easing functions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Easing {
  /// Constant normalized velocity.
  Linear,
  /// Motion's accelerating curve.
  EaseIn,
  /// Motion's decelerating curve.
  EaseOut,
  /// Motion's symmetric curve.
  EaseInOut,
  /// A cubic Bézier curve.
  CubicBezier([f32; 4]),
  /// A finite stepped curve.
  Steps {
    /// Number of steps.
    count: u32,
    /// Placement of each jump.
    position: StepPosition,
  },
}

/// Number of additional iterations after the first.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Repeat {
  /// A finite number of additional iterations.
  Count(u32),
  /// No terminal iteration.
  Forever,
}

/// How repeated iterations derive their endpoints.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepeatType {
  /// Restart from the origin.
  Loop,
  /// Alternate playback direction.
  Reverse,
  /// Swap origin and target before sampling.
  Mirror,
}

/// Serializable target adjustment for an inertia transition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InertiaTarget {
  /// Leaves the projected target unchanged.
  Identity,
  /// Rounds the projected target to the nearest multiple.
  NearestMultiple(f64),
  /// Rounds the projected target down to a multiple.
  FloorMultiple(f64),
  /// Rounds the projected target up to a multiple.
  CeilingMultiple(f64),
  /// Clamps the projected target to inclusive bounds.
  Clamp {
    /// Inclusive lower target.
    min: f64,
    /// Inclusive upper target.
    max: f64,
  },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SpringAuthoring {
  NotSpring,
  Unconfigured,
  Physical,
  Duration,
  VisualDuration,
}

/// Timing shared by a target with optional property replacements.
#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
  pub(crate) default: TransitionDefinition,
  pub(crate) properties: Vec<(MotionProperty, TransitionDefinition)>,
  pub(crate) spring_authoring: SpringAuthoring,
  pub(crate) orchestration: VariantOrchestration,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MotionStyleEntry {
  pub(crate) property: MotionProperty,
  pub(crate) values: Vec<MotionValue>,
  pub(crate) times: Option<Vec<f64>>,
  pub(crate) binding: Option<ErasedMotionValue>,
}

impl<T> Keyframes<T> {
  /// Creates a sequence with evenly distributed times.
  #[must_use]
  pub fn new(values: impl IntoIterator<Item = T>) -> Self {
    let values = values.into_iter().collect::<Vec<_>>();
    assert!(!values.is_empty(), "motion keyframes cannot be empty");
    Self {
      values,
      times: None,
    }
  }

  /// Assigns normalized property-local times.
  #[must_use]
  pub fn times(mut self, values: impl IntoIterator<Item = f64>) -> Self {
    let times = values.into_iter().collect::<Vec<_>>();
    assert_eq!(
      times.len(),
      self.values.len(),
      "motion keyframe times must match values"
    );
    validate_times(&times);
    self.times = Some(times);
    self
  }
}

impl MotionStyle {
  /// Creates an empty target style.
  #[must_use]
  pub const fn new() -> Self {
    Self {
      entries: Vec::new(),
    }
  }

  pub(crate) fn merge(mut self, value: Self) -> Self {
    for entry in value.entries {
      self = self.set(entry.property, entry.values, entry.times);
    }
    self
  }

  /// Sets opacity.
  #[must_use]
  pub fn opacity(self, value: f32) -> Self {
    self.scalar(MotionProperty::Opacity, value)
  }

  /// Sets opacity keyframes.
  #[must_use]
  pub fn opacity_keyframes(self, value: Keyframes<f32>) -> Self {
    self.scalar_keyframes(MotionProperty::Opacity, value)
  }

  /// Binds opacity to a stable Unity-local motion value.
  #[must_use]
  pub fn opacity_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::Opacity, value.erase())
  }

  /// Sets horizontal translation in panel pixels.
  #[must_use]
  pub fn x(self, value: f32) -> Self {
    self.length(MotionProperty::X, value.into())
  }

  /// Sets horizontal translation keyframes in panel pixels.
  #[must_use]
  pub fn x_keyframes(self, value: Keyframes<f32>) -> Self {
    self.length_keyframes(MotionProperty::X, map_keyframes(value, MotionLength::px))
  }

  /// Binds horizontal translation to a scalar pixel motion value.
  #[must_use]
  pub fn x_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::X, value.erase())
  }

  /// Binds horizontal translation to a typed length value.
  #[must_use]
  pub fn x_length_value(self, value: TypedMotionValue<MotionLength>) -> Self {
    self.bind(MotionProperty::X, value.erase())
  }

  /// Sets vertical translation in panel pixels.
  #[must_use]
  pub fn y(self, value: f32) -> Self {
    self.length(MotionProperty::Y, value.into())
  }

  /// Sets vertical translation keyframes in panel pixels.
  #[must_use]
  pub fn y_keyframes(self, value: Keyframes<f32>) -> Self {
    self.length_keyframes(MotionProperty::Y, map_keyframes(value, MotionLength::px))
  }

  /// Binds vertical translation to a scalar pixel motion value.
  #[must_use]
  pub fn y_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::Y, value.erase())
  }

  /// Sets two-axis scale.
  #[must_use]
  pub fn scale(self, value: f32) -> Self {
    self.vector2(MotionProperty::Scale, [value, value])
  }

  /// Sets two-axis scale keyframes.
  #[must_use]
  pub fn scale_keyframes(self, value: Keyframes<f32>) -> Self {
    self.vector2_keyframes(
      MotionProperty::Scale,
      map_keyframes(value, |value| [value, value]),
    )
  }

  /// Binds uniform scale to a scalar motion value.
  #[must_use]
  pub fn scale_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::Scale, value.erase())
  }

  /// Sets horizontal scale.
  #[must_use]
  pub fn scale_x(self, value: f32) -> Self {
    self.scalar(MotionProperty::ScaleX, value)
  }

  /// Sets horizontal scale keyframes.
  #[must_use]
  pub fn scale_x_keyframes(self, value: Keyframes<f32>) -> Self {
    self.scalar_keyframes(MotionProperty::ScaleX, value)
  }

  /// Sets vertical scale.
  #[must_use]
  pub fn scale_y(self, value: f32) -> Self {
    self.scalar(MotionProperty::ScaleY, value)
  }

  /// Sets vertical scale keyframes.
  #[must_use]
  pub fn scale_y_keyframes(self, value: Keyframes<f32>) -> Self {
    self.scalar_keyframes(MotionProperty::ScaleY, value)
  }

  /// Sets structured two-axis background size.
  #[must_use]
  pub fn background_size(self, value: [f32; 2]) -> Self {
    self.vector2(MotionProperty::BackgroundSize, value)
  }

  /// Sets the background color.
  #[must_use]
  pub fn background_color(self, value: MotionColor) -> Self {
    self.color_property(MotionProperty::BackgroundColor, value)
  }

  /// Sets background-color keyframes.
  #[must_use]
  pub fn background_color_keyframes(self, value: Keyframes<MotionColor>) -> Self {
    self.color_keyframes(MotionProperty::BackgroundColor, value)
  }

  /// Binds background color to a color motion value.
  #[must_use]
  pub fn background_color_value(self, value: TypedMotionValue<MotionColor>) -> Self {
    self.bind(MotionProperty::BackgroundColor, value.erase())
  }

  /// Sets text color.
  #[must_use]
  pub fn color(self, value: MotionColor) -> Self {
    self.color_property(MotionProperty::Color, value)
  }

  /// Binds text color to a color motion value.
  #[must_use]
  pub fn color_value(self, value: TypedMotionValue<MotionColor>) -> Self {
    self.bind(MotionProperty::Color, value.erase())
  }

  /// Binds the ordered transform list to a native motion value.
  #[must_use]
  pub fn transform_list_value(self, value: TypedMotionValue<Vec<MotionTransform>>) -> Self {
    self.bind(MotionProperty::TransformList, value.erase())
  }

  /// Binds the ordered filter list to a native motion value.
  #[must_use]
  pub fn filter_value(self, value: TypedMotionValue<Vec<MotionFilter>>) -> Self {
    self.bind(MotionProperty::Filter, value.erase())
  }

  /// Sets panel-plane rotation in degrees.
  #[must_use]
  pub fn rotate(self, value: f32) -> Self {
    self.set(
      MotionProperty::Rotate,
      vec![MotionValue::Angle(value)],
      None,
    )
  }

  /// Sets chrome-plane rotation around the horizontal axis in degrees.
  #[must_use]
  pub fn rotate_x(self, value: f32) -> Self {
    self.set(
      MotionProperty::RotateX,
      vec![MotionValue::Angle(value)],
      None,
    )
  }

  /// Sets chrome-plane rotation around the vertical axis in degrees.
  #[must_use]
  pub fn rotate_y(self, value: f32) -> Self {
    self.set(
      MotionProperty::RotateY,
      vec![MotionValue::Angle(value)],
      None,
    )
  }

  /// Sets horizontal chrome skew in degrees.
  #[must_use]
  pub fn skew_x(self, value: f32) -> Self {
    self.set(MotionProperty::SkewX, vec![MotionValue::Angle(value)], None)
  }

  /// Sets vertical chrome skew in degrees.
  #[must_use]
  pub fn skew_y(self, value: f32) -> Self {
    self.set(MotionProperty::SkewY, vec![MotionValue::Angle(value)], None)
  }

  /// Sets an ordered transform operation list.
  #[must_use]
  pub fn transform_list(self, value: impl IntoIterator<Item = MotionTransform>) -> Self {
    self.set(
      MotionProperty::TransformList,
      vec![MotionValue::TransformList(value.into_iter().collect())],
      None,
    )
  }

  /// Sets ordered filter operations.
  #[must_use]
  pub fn filter(self, value: impl IntoIterator<Item = MotionFilter>) -> Self {
    self.set(
      MotionProperty::Filter,
      vec![MotionValue::FilterList(value.into_iter().collect())],
      None,
    )
  }

  /// Sets filter-list keyframes.
  #[must_use]
  pub fn filter_keyframes(self, value: Keyframes<Vec<MotionFilter>>) -> Self {
    self.set(
      MotionProperty::Filter,
      value
        .values
        .into_iter()
        .map(MotionValue::FilterList)
        .collect(),
      value.times,
    )
  }

  /// Sets a typed background gradient.
  #[must_use]
  pub fn background_gradient(self, value: MotionGradient) -> Self {
    self.set(
      MotionProperty::BackgroundGradient,
      vec![MotionValue::Gradient(value)],
      None,
    )
  }

  /// Sets outer or inset box shadows.
  #[must_use]
  pub fn box_shadow(self, value: impl IntoIterator<Item = MotionShadow>) -> Self {
    self.set(
      MotionProperty::BoxShadow,
      vec![MotionValue::ShadowList(value.into_iter().collect())],
      None,
    )
  }

  /// Sets rectangular clip insets in top-right-bottom-left order.
  #[must_use]
  pub fn clip_inset(self, value: [MotionLength; 4]) -> Self {
    self.set(
      MotionProperty::ClipInset,
      vec![MotionValue::ClipInset(value)],
      None,
    )
  }

  /// Sets polygon clip geometry with a stable vertex count.
  #[must_use]
  pub fn clip_polygon(self, value: impl IntoIterator<Item = [MotionLength; 2]>) -> Self {
    self.set(
      MotionProperty::ClipPolygon,
      vec![MotionValue::ClipPolygon(value.into_iter().collect())],
      None,
    )
  }

  /// Sets a discrete prepared texture selection.
  #[must_use]
  pub fn prepared_texture(self, address: impl Into<String>) -> Self {
    self.set(
      MotionProperty::BackgroundImage,
      vec![MotionValue::Discrete(address.into().into())],
      None,
    )
  }

  /// Sets a discrete prepared shader material selection.
  #[must_use]
  pub fn shader_material(self, address: impl Into<String>) -> Self {
    self.set(
      MotionProperty::UnityMaterial,
      vec![MotionValue::Discrete(address.into().into())],
      None,
    )
  }

  /// Sets a discrete mask selection.
  #[must_use]
  pub fn mask(self, address: impl Into<String>) -> Self {
    self.set(
      MotionProperty::Mask,
      vec![MotionValue::Discrete(address.into().into())],
      None,
    )
  }

  /// Sets discrete visibility.
  #[must_use]
  pub fn visibility(self, value: Visibility) -> Self {
    self.set(
      MotionProperty::Visibility,
      vec![MotionValue::Discrete(visibility_value(value).into())],
      None,
    )
  }

  /// Sets discrete visibility keyframes.
  #[must_use]
  pub fn visibility_keyframes(self, value: Keyframes<Visibility>) -> Self {
    self.set(
      MotionProperty::Visibility,
      value
        .values
        .into_iter()
        .map(|value| MotionValue::Discrete(visibility_value(value).into()))
        .collect(),
      value.times,
    )
  }

  fn scalar(self, property: MotionProperty, value: f32) -> Self {
    self.set(property, vec![MotionValue::Scalar(value)], None)
  }

  fn scalar_keyframes(self, property: MotionProperty, value: Keyframes<f32>) -> Self {
    self.set(
      property,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  fn length(self, property: MotionProperty, value: MotionLength) -> Self {
    self.set(property, vec![MotionValue::Length(value)], None)
  }

  fn length_keyframes(self, property: MotionProperty, value: Keyframes<MotionLength>) -> Self {
    self.set(
      property,
      value.values.into_iter().map(MotionValue::Length).collect(),
      value.times,
    )
  }

  fn vector2(self, property: MotionProperty, value: [f32; 2]) -> Self {
    self.set(property, vec![MotionValue::Vector2(value)], None)
  }

  fn vector2_keyframes(self, property: MotionProperty, value: Keyframes<[f32; 2]>) -> Self {
    self.set(
      property,
      value.values.into_iter().map(MotionValue::Vector2).collect(),
      value.times,
    )
  }

  fn color_property(self, property: MotionProperty, value: MotionColor) -> Self {
    self.set(property, vec![MotionValue::Color(value)], None)
  }

  fn color_keyframes(self, property: MotionProperty, value: Keyframes<MotionColor>) -> Self {
    self.set(
      property,
      value.values.into_iter().map(MotionValue::Color).collect(),
      value.times,
    )
  }

  pub(crate) fn set(
    mut self,
    property: MotionProperty,
    values: Vec<MotionValue>,
    times: Option<Vec<f64>>,
  ) -> Self {
    let entry = MotionStyleEntry {
      property,
      values,
      times,
      binding: None,
    };
    if let Some(index) = self
      .entries
      .iter()
      .position(|value| value.property == property)
    {
      self.entries[index] = entry;
    } else {
      self.entries.push(entry);
    }
    self
  }

  fn bind(mut self, property: MotionProperty, binding: ErasedMotionValue) -> Self {
    let entry = MotionStyleEntry {
      property,
      values: Vec::new(),
      times: None,
      binding: Some(binding),
    };
    if let Some(index) = self
      .entries
      .iter()
      .position(|value| value.property == property)
    {
      self.entries[index] = entry;
    } else {
      self.entries.push(entry);
    }
    self
  }

  fn target(&self, transition: Option<&Transition>) -> MotionTargetDescriptor {
    MotionTargetDescriptor {
      tracks: self
        .entries
        .iter()
        .filter(|entry| entry.binding.is_none())
        .map(|entry| MotionPropertyTrack {
          property: entry.property,
          values: entry.values.clone(),
          times: entry.times.clone(),
          transition: transition.map_or_else(
            || implicit_transition(entry),
            |transition| transition.for_property(entry.property),
          ),
        })
        .collect(),
      transition_end: Vec::new(),
    }
  }

  pub(crate) fn values(&self) -> Vec<MotionPropertyValue> {
    self
      .entries
      .iter()
      .filter(|entry| entry.binding.is_none())
      .map(|entry| MotionPropertyValue {
        property: entry.property,
        value: entry
          .values
          .last()
          .expect("motion style entry has a value")
          .clone(),
      })
      .collect()
  }

  pub(crate) fn value_bindings(&self) -> Vec<battlement::MotionValueBinding> {
    self
      .entries
      .iter()
      .filter_map(|entry| {
        entry
          .binding
          .as_ref()
          .map(|value| battlement::MotionValueBinding {
            property: entry.property,
            value_id: value.id(),
          })
      })
      .collect()
  }

  pub(crate) fn graph_values(&self) -> Vec<battlement::MotionValueDescriptor> {
    let mut values = Vec::new();
    for binding in self
      .entries
      .iter()
      .filter_map(|entry| entry.binding.as_ref())
    {
      binding.collect(&mut values);
    }
    values
  }

  pub(crate) fn value_subscriptions(&self) -> Vec<battlement::MotionValueSubscription> {
    let mut subscriptions = Vec::new();
    for binding in self
      .entries
      .iter()
      .filter_map(|entry| entry.binding.as_ref())
    {
      binding.collect_subscriptions(&mut subscriptions);
    }
    subscriptions
  }
}

impl MotionTarget {
  /// Creates a target from typed style values.
  #[must_use]
  pub fn new(style: MotionStyle) -> Self {
    Self {
      style,
      transition: None,
      transition_end: MotionStyle::new(),
      callbacks: MotionCallbacks::new(),
    }
  }

  /// Replaces timing for this target.
  #[must_use]
  pub fn transition(mut self, value: Transition) -> Self {
    self.transition = Some(value);
    self
  }

  /// Assigns values atomically after successful finite completion.
  #[must_use]
  pub fn transition_end(mut self, value: MotionStyle) -> Self {
    self.transition_end = value;
    self
  }

  pub(crate) fn descriptor(
    &self,
    inherited: Option<&Transition>,
    extra_delay_micros: u64,
  ) -> MotionTargetDescriptor {
    let transition = self.transition.as_ref().or(inherited);
    let mut target = self.style.target(transition);
    for track in &mut target.tracks {
      track.transition.delay_micros = track
        .transition
        .delay_micros
        .checked_add_unsigned(extra_delay_micros)
        .expect("variant delay exhausted");
    }
    target.transition_end = self.transition_end.values();
    target
  }

  pub(crate) fn value_bindings(&self) -> Vec<battlement::MotionValueBinding> {
    self.style.value_bindings()
  }

  pub(crate) fn graph_values(&self) -> Vec<battlement::MotionValueDescriptor> {
    self.style.graph_values()
  }

  pub(crate) fn value_subscriptions(&self) -> Vec<battlement::MotionValueSubscription> {
    self.style.value_subscriptions()
  }

  pub(crate) fn merge(mut self, value: Self) -> Self {
    self.style = self.style.merge(value.style);
    self.transition_end = self.transition_end.merge(value.transition_end);
    if value.transition.is_some() {
      self.transition = value.transition;
    }
    self.callbacks = self.callbacks.merge(value.callbacks);
    self
  }

  pub(crate) fn total_duration_micros(&self, inherited: Option<&Transition>) -> u64 {
    self
      .descriptor(inherited, 0)
      .tracks
      .iter()
      .map(|track| transition_duration_micros(&track.transition))
      .max()
      .unwrap_or(0)
  }

  pub(crate) fn variant_orchestration(&self) -> VariantOrchestration {
    self
      .transition
      .as_ref()
      .map_or_else(VariantOrchestration::new, |value| value.orchestration)
  }
}

impl From<MotionStyle> for MotionTarget {
  fn from(value: MotionStyle) -> Self {
    Self::new(value)
  }
}

impl MotionProps {
  pub(crate) fn drag_constraint_ref(&self) -> Option<&crate::element_ref::ElementRef> {
    self.gestures.drag_constraint_ref()
  }

  /// Creates empty Motion props.
  #[must_use]
  pub const fn new() -> Self {
    Self {
      initial: None,
      animate: None,
      exit: None,
      transition: None,
      variants: ErasedVariants::new(),
      variant_data: None,
      initial_variant_selection: None,
      variant_selection: None,
      exit_variant_selection: None,
      inherit_variants: true,
      callbacks: MotionCallbacks::new(),
      css: crate::motion_css::CssProps::new(),
      control_id: None,
      scope_id: None,
      scope_root: false,
      motion_name: None,
      gestures: GestureProps::new(),
    }
  }

  /// Selects the mount origin.
  #[must_use]
  pub fn initial(mut self, value: impl InitialValue) -> Self {
    self.initial = Some(value.into_initial());
    self
  }

  /// Selects the base animation target.
  #[must_use]
  pub fn animate(mut self, value: impl Into<MotionTarget>) -> Self {
    self.animate = Some(value.into());
    self
  }

  /// Selects the stored presence-exit target.
  #[must_use]
  pub fn exit(mut self, value: impl Into<MotionTarget>) -> Self {
    self.exit = Some(value.into());
    self
  }

  /// Replaces target timing inherited by properties without local overrides.
  #[must_use]
  pub fn transition(mut self, value: Transition) -> Self {
    self.transition = Some(value);
    self
  }

  /// Binds this host to one stable typed animation-controls identity.
  #[must_use]
  pub fn animation_controls<Name: crate::variant_map::VariantKey>(
    mut self,
    value: AnimationControls<Name>,
  ) -> Self {
    self.control_id = Some(value.id());
    self
  }

  /// Marks this host as one animation-scope root.
  #[must_use]
  pub fn animation_scope(mut self, value: AnimationScope) -> Self {
    self.scope_id = Some(value.id());
    self.scope_root = true;
    self
  }

  /// Assigns one stable closed selector name.
  #[must_use]
  pub fn motion_name(mut self, value: impl Into<String>) -> Self {
    let value = value.into();
    assert!(!value.trim().is_empty(), "motion name is empty");
    self.motion_name = Some(value);
    self
  }

  pub(crate) fn merge(mut self, value: Self) -> Self {
    if value.initial.is_some() {
      self.initial = value.initial;
    }
    if value.animate.is_some() {
      self.animate = value.animate;
    }
    if value.exit.is_some() {
      self.exit = value.exit;
    }
    if value.transition.is_some() {
      self.transition = value.transition;
    }
    if value.variants != ErasedVariants::new() {
      self.variants = value.variants;
    }
    if value.variant_data.is_some() {
      self.variant_data = value.variant_data;
    }
    if value.variant_selection.is_some() {
      self.variant_selection = value.variant_selection;
    }
    if value.initial_variant_selection.is_some() {
      self.initial_variant_selection = value.initial_variant_selection;
    }
    if value.exit_variant_selection.is_some() {
      self.exit_variant_selection = value.exit_variant_selection;
    }
    if !value.inherit_variants {
      self.inherit_variants = false;
    }
    self.callbacks = self.callbacks.merge(value.callbacks);
    self.css = self.css.merge(value.css);
    if value.control_id.is_some() {
      self.control_id = value.control_id;
    }
    if value.scope_id.is_some() {
      self.scope_id = value.scope_id;
      self.scope_root = value.scope_root;
    }
    if value.motion_name.is_some() {
      self.motion_name = value.motion_name;
    }
    self.gestures = self.gestures.merge(value.gestures);
    self
  }
}

impl InitialValue for bool {
  fn into_initial(self) -> InitialTarget {
    assert!(!self, "initial(true) has no Motion meaning");
    InitialTarget::Disabled
  }
}

impl InitialValue for MotionStyle {
  fn into_initial(self) -> InitialTarget {
    InitialTarget::Target(Box::new(self.into()))
  }
}

impl InitialValue for MotionTarget {
  fn into_initial(self) -> InitialTarget {
    InitialTarget::Target(Box::new(self))
  }
}

fn map_keyframes<T, U>(value: Keyframes<T>, map: impl Fn(T) -> U) -> Keyframes<U> {
  Keyframes {
    values: value.values.into_iter().map(map).collect(),
    times: value.times,
  }
}

fn implicit_transition(entry: &MotionStyleEntry) -> TransitionDefinition {
  if entry.values.len() > 2 {
    return Transition::tween().duration_secs(0.8).default;
  }
  match entry.property {
    MotionProperty::X
    | MotionProperty::Y
    | MotionProperty::Z
    | MotionProperty::Translate
    | MotionProperty::Rotate
    | MotionProperty::RotateX
    | MotionProperty::RotateY => physical_default(500.0, 25.0),
    MotionProperty::Scale | MotionProperty::ScaleX | MotionProperty::ScaleY => {
      let damping = if targets_zero(entry) {
        2.0 * 550.0_f64.sqrt()
      } else {
        30.0
      };
      physical_default(550.0, damping)
    }
    _ => {
      Transition::tween()
        .ease(Easing::CubicBezier([0.25, 0.1, 0.35, 1.0]))
        .default
    }
  }
}

fn physical_default(stiffness: f64, damping: f64) -> TransitionDefinition {
  TransitionDefinition {
    generator: TransitionGenerator::Spring(SpringConfiguration::Physical {
      stiffness,
      damping,
      mass: 1.0,
      initial_velocity: None,
      rest_speed: Some(10.0),
      rest_delta: None,
    }),
    delay_micros: 0,
    repeat: MotionRepeat::None,
    repeat_delay_micros: 0,
    repeat_type: MotionRepeatType::Loop,
  }
}

fn targets_zero(entry: &MotionStyleEntry) -> bool {
  match entry.values.last() {
    Some(MotionValue::Scalar(value)) => *value == 0.0,
    Some(MotionValue::Vector2(value)) => value.iter().all(|value| *value == 0.0),
    _ => false,
  }
}

fn visibility_value(value: Visibility) -> &'static str {
  match value {
    Visibility::Visible => "visible",
    Visibility::Hidden => "hidden",
  }
}

fn transition_duration_micros(value: &TransitionDefinition) -> u64 {
  let active = match &value.generator {
    TransitionGenerator::Immediate => 0,
    TransitionGenerator::Tween {
      duration_micros, ..
    } => *duration_micros,
    TransitionGenerator::Spring(SpringConfiguration::Duration {
      duration_micros, ..
    })
    | TransitionGenerator::Spring(SpringConfiguration::VisualDuration {
      duration_micros, ..
    }) => *duration_micros,
    TransitionGenerator::Spring(SpringConfiguration::Physical { .. }) => 1_000_000,
    TransitionGenerator::Inertia {
      time_constant_micros,
      ..
    } => time_constant_micros.saturating_mul(8),
  };
  let plays = match value.repeat {
    MotionRepeat::None => 1,
    MotionRepeat::Count(count) => u64::from(count) + 1,
    MotionRepeat::Forever => panic!("infinite variants cannot sequence parent and children"),
  };
  value.delay_micros.max(0) as u64
    + active.saturating_mul(plays)
    + value.repeat_delay_micros.saturating_mul(plays - 1)
}

pub(crate) fn micros(value: f64, allow_zero: bool) -> u64 {
  assert!(
    value.is_finite() && value >= 0.0,
    "motion duration must be finite and nonnegative"
  );
  let value = (value * 1_000_000.0).round() as u64;
  assert!(allow_zero || value > 0, "motion duration must be positive");
  value
}

pub(crate) fn validate_times(values: &[f64]) {
  assert!(
    values.len() >= 2,
    "motion times require at least two entries"
  );
  assert_eq!(
    values.first(),
    Some(&0.0),
    "motion times must begin at zero"
  );
  assert_eq!(values.last(), Some(&1.0), "motion times must end at one");
  assert!(
    values.iter().all(|value| value.is_finite()),
    "motion times must be finite"
  );
  assert!(
    values.windows(2).all(|pair| pair[0] <= pair[1]),
    "motion times must be nondecreasing"
  );
}

mod private {
  pub trait InitialValueSealed {}

  impl InitialValueSealed for bool {}
  impl InitialValueSealed for super::MotionStyle {}
  impl InitialValueSealed for super::MotionTarget {}
}

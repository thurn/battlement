//! Motion forwarding for custom components with one stable host.

use std::any::TypeId;

use crate::{
  component::Component,
  motion::{InitialValue, MotionProps, MotionTarget, Transition},
  render::{Render, RenderSink},
  render_value::Sealed,
  variant_map::{VariantData, VariantKey, Variants},
};

/// Rust-only adapter that forwards one complete Motion value without a host wrapper.
#[doc(hidden)]
pub struct ForwardedMotion<C> {
  component: C,
  motion: MotionProps,
}

/// Components that forward one complete Motion value to a stable host.
pub trait MotionComponent: Component + Sized {
  /// Applies the complete forwarded Motion value.
  fn with_motion(self, motion: MotionProps) -> Self;
}

/// Motion builders available on forwarding components.
pub trait MotionComponentExt: MotionComponent + Clone {
  /// Applies a complete Motion value.
  #[must_use]
  fn motion(self, value: MotionProps) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, value)
  }

  /// Selects the mount origin.
  #[must_use]
  fn initial(self, value: impl InitialValue) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, MotionProps::new().initial(value))
  }

  /// Selects the base animation target.
  #[must_use]
  fn animate(self, value: impl Into<MotionTarget>) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, MotionProps::new().animate(value))
  }

  /// Selects the presence-exit target.
  #[must_use]
  fn exit(self, value: impl Into<MotionTarget>) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, MotionProps::new().exit(value))
  }

  /// Replaces the default transition.
  #[must_use]
  fn transition(self, value: Transition) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, MotionProps::new().transition(value))
  }

  /// Replaces the typed target definitions available to the forwarded host.
  #[must_use]
  fn variants<Name, Custom>(self, value: Variants<Name, Custom>) -> ForwardedMotion<Self>
  where
    Name: VariantKey,
    Custom: VariantData,
  {
    ForwardedMotion::new(self, MotionProps::new().variants(value))
  }

  /// Selects one named animate target on the forwarded host.
  #[must_use]
  fn animate_variant<Name: VariantKey>(self, value: Name) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, MotionProps::new().animate_variant(value))
  }

  /// Selects an ordered named animate list on the forwarded host.
  #[must_use]
  fn animate_variants<Name: VariantKey>(
    self,
    values: impl IntoIterator<Item = Name>,
  ) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, MotionProps::new().animate_variants(values))
  }

  /// Selects one named mount origin on the forwarded host.
  #[must_use]
  fn initial_variant<Name: VariantKey>(self, value: Name) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, MotionProps::new().initial_variant(value))
  }

  /// Selects an ordered named mount-origin list on the forwarded host.
  #[must_use]
  fn initial_variants<Name: VariantKey>(
    self,
    values: impl IntoIterator<Item = Name>,
  ) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, MotionProps::new().initial_variants(values))
  }

  /// Selects one named presence-exit target on the forwarded host.
  #[must_use]
  fn exit_variant<Name: VariantKey>(self, value: Name) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, MotionProps::new().exit_variant(value))
  }

  /// Selects an ordered named presence-exit list on the forwarded host.
  #[must_use]
  fn exit_variants<Name: VariantKey>(
    self,
    values: impl IntoIterator<Item = Name>,
  ) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, MotionProps::new().exit_variants(values))
  }

  /// Supplies custom data to computed variants.
  #[must_use]
  fn custom<T: VariantData>(self, value: T) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, MotionProps::new().custom(value))
  }

  /// Enables or disables inherited variant layers.
  #[must_use]
  fn inherit_variants(self, value: bool) -> ForwardedMotion<Self> {
    ForwardedMotion::new(self, MotionProps::new().inherit_variants(value))
  }
}

impl<T: MotionComponent + Clone> MotionComponentExt for T {}

impl<C> ForwardedMotion<C> {
  fn new(component: C, motion: MotionProps) -> Self {
    Self { component, motion }
  }

  /// Selects the mount origin.
  #[must_use]
  pub fn initial(mut self, value: impl InitialValue) -> Self {
    self.motion = self.motion.merge(MotionProps::new().initial(value));
    self
  }

  /// Selects the base animation target.
  #[must_use]
  pub fn animate(mut self, value: impl Into<MotionTarget>) -> Self {
    self.motion = self.motion.merge(MotionProps::new().animate(value));
    self
  }

  /// Selects the presence-exit target.
  #[must_use]
  pub fn exit(mut self, value: impl Into<MotionTarget>) -> Self {
    self.motion = self.motion.merge(MotionProps::new().exit(value));
    self
  }

  /// Replaces inherited target timing.
  #[must_use]
  pub fn transition(mut self, value: Transition) -> Self {
    self.motion = self.motion.merge(MotionProps::new().transition(value));
    self
  }

  /// Replaces the typed target definitions available to the forwarded host.
  #[must_use]
  pub fn variants<Name, Custom>(mut self, value: Variants<Name, Custom>) -> Self
  where
    Name: VariantKey,
    Custom: VariantData,
  {
    self.motion = self.motion.merge(MotionProps::new().variants(value));
    self
  }

  /// Selects one named animate target on the forwarded host.
  #[must_use]
  pub fn animate_variant<Name: VariantKey>(mut self, value: Name) -> Self {
    self.motion = self.motion.merge(MotionProps::new().animate_variant(value));
    self
  }

  /// Selects an ordered named animate list.
  #[must_use]
  pub fn animate_variants<Name: VariantKey>(
    mut self,
    values: impl IntoIterator<Item = Name>,
  ) -> Self {
    self.motion = self
      .motion
      .merge(MotionProps::new().animate_variants(values));
    self
  }

  /// Selects one named mount origin.
  #[must_use]
  pub fn initial_variant<Name: VariantKey>(mut self, value: Name) -> Self {
    self.motion = self.motion.merge(MotionProps::new().initial_variant(value));
    self
  }

  /// Selects an ordered named mount-origin list.
  #[must_use]
  pub fn initial_variants<Name: VariantKey>(
    mut self,
    values: impl IntoIterator<Item = Name>,
  ) -> Self {
    self.motion = self
      .motion
      .merge(MotionProps::new().initial_variants(values));
    self
  }

  /// Selects one named presence-exit target.
  #[must_use]
  pub fn exit_variant<Name: VariantKey>(mut self, value: Name) -> Self {
    self.motion = self.motion.merge(MotionProps::new().exit_variant(value));
    self
  }

  /// Selects an ordered named presence-exit list.
  #[must_use]
  pub fn exit_variants<Name: VariantKey>(mut self, values: impl IntoIterator<Item = Name>) -> Self {
    self.motion = self.motion.merge(MotionProps::new().exit_variants(values));
    self
  }

  /// Supplies custom data to computed variants.
  #[must_use]
  pub fn custom<T: VariantData>(mut self, value: T) -> Self {
    self.motion = self.motion.merge(MotionProps::new().custom(value));
    self
  }

  /// Enables or disables inherited variant layers.
  #[must_use]
  pub fn inherit_variants(mut self, value: bool) -> Self {
    self.motion = self
      .motion
      .merge(MotionProps::new().inherit_variants(value));
    self
  }
}

impl<C> Render for ForwardedMotion<C> where C: MotionComponent + Clone {}

#[allow(private_interfaces)]
impl<C> Sealed for ForwardedMotion<C>
where
  C: MotionComponent + Clone,
{
  fn descriptor(&self) -> TypeId {
    TypeId::of::<Self>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    sink.push_motion_component::<Self, C>(self.component.clone(), self.motion.clone());
  }
}

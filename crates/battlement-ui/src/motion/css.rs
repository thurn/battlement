use serde::{Deserialize, Serialize};

use crate::{MotionProperty, MotionPropertyValue, MotionValue, Style, TransitionDefinition};

/// A locally resolved UI pseudo-state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum MotionPseudoState {
  /// Pointer hover.
  Hover,
  /// Keyboard or programmatic focus.
  Focus,
  /// Pointer or submit activation.
  Active,
  /// Disabled in the local hierarchy.
  Disabled,
}

/// Typed properties contributed by one pseudo-state.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionPseudoStyle {
  /// Pseudo-state whose properties are active.
  pub state: MotionPseudoState,
  /// Sparse property overlay.
  pub values: Vec<MotionPropertyValue>,
}

/// One property-specific CSS transition.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StylePropertyTransition {
  /// Property whose resolved static changes are sampled.
  pub property: MotionProperty,
  /// Tween or immediate timing.
  pub transition: TransitionDefinition,
}

/// CSS transition behavior for resolved static and pseudo styles.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct StyleTransitionDescriptor {
  /// Property-specific transition definitions.
  pub properties: Vec<StylePropertyTransition>,
  /// Default timing for changed interpolable properties.
  pub all: Option<TransitionDefinition>,
  /// Whether explicitly selected discrete properties switch at a boundary.
  pub allow_discrete: bool,
}

/// CSS animation playback direction.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AnimationDirection {
  /// Every iteration runs from zero to one.
  Normal,
  /// Every iteration runs from one to zero.
  Reverse,
  /// Odd iterations run forward and even iterations run backward.
  Alternate,
  /// Odd iterations run backward and even iterations run forward.
  AlternateReverse,
}

/// CSS animation fill behavior outside its active interval.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AnimationFill {
  /// Reveals the lower property owner before and after playback.
  None,
  /// Retains the terminal sample after completion.
  Forwards,
  /// Applies the initial sample during delay.
  Backwards,
  /// Applies both backwards and forwards fill.
  Both,
}

/// CSS animation play state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AnimationPlayState {
  /// Logical time advances.
  Running,
  /// Logical time is held.
  Paused,
}

/// CSS animation property composition.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AnimationComposition {
  /// The animation replaces the lower resolved value.
  Replace,
  /// The sample combines with the lower resolved value.
  Add,
  /// Completed iteration deltas accumulate before the current sample.
  Accumulate,
}

/// Exact CSS iteration count.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AnimationIterations {
  /// One play.
  Once,
  /// An exact number of plays; zero installs no tracks.
  Count(u32),
  /// No terminal iteration.
  Forever,
}

/// One property-local CSS keyframe sequence.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CssPropertyTrack {
  /// Animated property.
  pub property: MotionProperty,
  /// Keyframes declared for this property.
  pub values: Vec<MotionValue>,
  /// Normalized positions of the declared keyframes.
  pub times: Vec<f64>,
  /// Complete playback schedule.
  pub transition: TransitionDefinition,
}

/// One reusable CSS-style animation slot.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CssAnimationDescriptor {
  /// Stable list or authored key identity.
  pub slot: u64,
  /// Generation incremented when restart-affecting settings change.
  pub generation: u32,
  /// Stable fingerprint of settings that require playback to restart.
  pub restart_key: u64,
  /// Normalized property tracks.
  pub tracks: Vec<CssPropertyTrack>,
  /// Playback direction.
  pub direction: AnimationDirection,
  /// Fill outside the active interval.
  pub fill: AnimationFill,
  /// Current play state.
  pub play_state: AnimationPlayState,
  /// Property composition rule.
  pub composition: AnimationComposition,
  /// Optional developer-facing trace name.
  pub diagnostic_name: Option<String>,
}

/// Paint order for one decoration layer.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DecorationPlacement {
  /// Paints behind host content and above its background.
  Before,
  /// Paints above host content.
  After,
}

/// Geometry policy for one decoration layer.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DecorationPosition {
  /// Covers the host padding box.
  Fill,
  /// Covers the host border box.
  Border,
}

/// Clip policy for one decoration layer.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DecorationOverflow {
  /// Clips to the host.
  Hidden,
  /// Permits paint outside the host when its panel permits it.
  Visible,
}

/// One non-interactive paint layer associated with a host.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionDecorationDescriptor {
  /// Stable list or authored key identity.
  pub key: u64,
  /// Paint order relative to host content.
  pub placement: DecorationPlacement,
  /// Geometry policy.
  pub position: DecorationPosition,
  /// Clip policy.
  pub overflow: DecorationOverflow,
  /// Typed static decoration style.
  pub style: Style,
  /// CSS animations scoped to this decoration.
  pub animations: Vec<CssAnimationDescriptor>,
}

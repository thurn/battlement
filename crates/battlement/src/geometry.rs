//! Canonical geometry observation protocol and registry validation.

#![allow(missing_docs)]

use std::{
  collections::{HashMap, HashSet},
  num::NonZeroU64,
};

use serde::{Deserialize, Serialize};

use crate::{ObjectId, Rect};

/// Identifies one observation epoch.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct GeometryObservationId(pub ObjectId);

/// Identifies one complete native sampling pass within a session.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct GeometryGeneration(pub NonZeroU64);

/// Identifies one physical display.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct DisplayId(pub u32);

/// Identifies the UI document panel that owns an observed element.
pub type PanelId = ObjectId;

/// Selects the camera used to project a world target.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum CameraTarget {
  /// Use the session's selected input camera.
  Input,
  /// Use the enabled camera component on this object.
  Object(ObjectId),
}

/// Names one authored world-space geometry anchor.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct AnchorName(pub String);

impl From<String> for AnchorName {
  fn from(value: String) -> Self {
    Self(value)
  }
}

impl From<&str> for AnchorName {
  fn from(value: &str) -> Self {
    Self(value.to_owned())
  }
}

/// A row-major three-by-three projective transform.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Projective2 {
  pub m11: f64,
  pub m12: f64,
  pub m13: f64,
  pub m21: f64,
  pub m22: f64,
  pub m23: f64,
  pub m31: f64,
  pub m32: f64,
  pub m33: f64,
}

/// A two-dimensional point in an explicitly selected coordinate space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
  pub x: f64,
  pub y: f64,
}

impl Point {
  /// Creates a point from its components.
  #[must_use]
  pub const fn new(x: f64, y: f64) -> Self {
    Self { x, y }
  }
}

/// A point in upper-left-origin physical display coordinates.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ViewportPoint {
  pub x: f64,
  pub y: f64,
  pub display_id: DisplayId,
}

/// A rectangle in upper-left-origin physical display coordinates.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ViewportRect {
  pub x: f64,
  pub y: f64,
  pub width: f64,
  pub height: f64,
  pub display_id: DisplayId,
}

/// Geometry measured for one UI element.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ElementGeometry {
  pub layout: Rect,
  pub viewport_bound: ViewportRect,
  pub viewport_from_local: Projective2,
  pub viewport_from_parent: Projective2,
  pub panel_id: PanelId,
}

impl ElementGeometry {
  /// Returns this element's local bounds in another element's local space.
  #[must_use]
  pub fn bounds_in(&self, coordinate_space: &Self) -> Option<Rect> {
    if self.viewport_bound.display_id != coordinate_space.viewport_bound.display_id {
      return None;
    }
    let inverse = self::inverse(coordinate_space.viewport_from_local)?;
    let points = [
      Point::new(0.0, 0.0),
      Point::new(self.layout.width, 0.0),
      Point::new(0.0, self.layout.height),
      Point::new(self.layout.width, self.layout.height),
    ];
    let transform = self::multiply(inverse, self.viewport_from_local);
    if self::crosses_horizon(self.viewport_from_local, points)
      || self::crosses_horizon(transform, points)
    {
      return None;
    }
    let corners = points.map(|point| self::map(transform, point));
    let [Some(first), Some(second), Some(third), Some(fourth)] = corners else {
      return None;
    };
    let min_x = first.x.min(second.x).min(third.x).min(fourth.x);
    let max_x = first.x.max(second.x).max(third.x).max(fourth.x);
    let min_y = first.y.min(second.y).min(third.y).min(fourth.y);
    let max_y = first.y.max(second.y).max(third.y).max(fourth.y);
    Some(Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
  }

  /// Converts a same-display viewport point into element-local coordinates.
  #[must_use]
  pub fn viewport_point_to_local(&self, point: ViewportPoint) -> Option<Point> {
    self::same_display(self.viewport_bound.display_id, point).and_then(|point| {
      self::inverse(self.viewport_from_local).and_then(|matrix| self::map(matrix, point))
    })
  }

  /// Converts a same-display viewport point into parent-local coordinates.
  #[must_use]
  pub fn viewport_point_to_parent(&self, point: ViewportPoint) -> Option<Point> {
    self::same_display(self.viewport_bound.display_id, point).and_then(|point| {
      self::inverse(self.viewport_from_parent).and_then(|matrix| self::map(matrix, point))
    })
  }

  /// Converts an element-local point into viewport coordinates.
  #[must_use]
  pub fn local_point_to_viewport(&self, point: Point) -> Option<ViewportPoint> {
    self::map(self.viewport_from_local, point).map(|point| ViewportPoint {
      x: point.x,
      y: point.y,
      display_id: self.viewport_bound.display_id,
    })
  }
}

/// Physical display orientation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DisplayOrientation {
  Landscape,
  LandscapeFlipped,
  Portrait,
  PortraitFlipped,
}

/// Geometry measured for one display viewport.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ViewportGeometry {
  pub viewport: ViewportRect,
  pub safe_area: ViewportRect,
  pub scale: f64,
  pub dpi: Option<f64>,
  pub orientation: DisplayOrientation,
}

/// Geometry measured for a projected world point.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorldPointGeometry {
  pub point: ViewportPoint,
  pub depth: f64,
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub is_inside_viewport: bool,
}

/// Geometry measured for projected rendered bounds.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorldBoundsGeometry {
  pub bound: ViewportRect,
  pub nearest_depth: f64,
  pub farthest_depth: f64,
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub is_inside_viewport: bool,
}

/// One target installed in the native observation registry.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum GeometryObservationTarget {
  UiElement {
    object_id: ObjectId,
  },
  Viewport {
    display_id: DisplayId,
  },
  WorldOrigin {
    object_id: ObjectId,
    camera: CameraTarget,
  },
  WorldAnchor {
    object_id: ObjectId,
    anchor: AnchorName,
    camera: CameraTarget,
  },
  WorldRenderedBounds {
    object_id: ObjectId,
    camera: CameraTarget,
  },
}

/// Associates one observation epoch with its target.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GeometryObservation {
  pub observation_id: GeometryObservationId,
  pub target: GeometryObservationTarget,
}

/// One atomic registry update.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GeometryObservationUpdate {
  pub added: Vec<GeometryObservation>,
  pub removed: Vec<GeometryObservationId>,
}

/// A successfully sampled geometry value.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum GeometryValue {
  Element(ElementGeometry),
  Viewport(ViewportGeometry),
  WorldPoint(WorldPointGeometry),
  WorldBounds(WorldBoundsGeometry),
}

/// A temporary reason an observation could not be sampled.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum GeometryUnavailable {
  Detached,
  Hidden,
  ObjectMissing,
  CameraDisabled,
  DisplayUnavailable,
  NoRenderers,
  BehindCamera,
  NoViewportMapping,
  ProjectionUnavailable,
}

/// The result of sampling one observation.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum GeometryObservationResult {
  Current(GeometryValue),
  Unavailable(GeometryUnavailable),
}

/// One changed observation in a sampling pass.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct GeometryObservationValue {
  pub observation_id: GeometryObservationId,
  pub result: GeometryObservationResult,
}

/// Changed values from one complete native sampling pass.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GeometryObservationBatch {
  pub generation: GeometryGeneration,
  pub changed: Vec<GeometryObservationValue>,
}

/// A geometry registry or batch invariant that was violated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeometryValidationError {
  DuplicateId,
  UnknownId,
  EmptyAnchor,
  NonFiniteNumber,
  InvalidProjective,
  WrongValueKind,
  InvalidGeneration,
}

/// Validates registry updates and batches before mutating its state.
#[derive(Clone, Debug, Default)]
pub struct GeometryRegistry {
  targets: HashMap<GeometryObservationId, GeometryObservationTarget>,
  generation: Option<GeometryGeneration>,
}

impl GeometryRegistry {
  /// Returns the active target for an observation.
  #[must_use]
  pub fn get(&self, id: GeometryObservationId) -> Option<&GeometryObservationTarget> {
    self.targets.get(&id)
  }

  /// Atomically applies a validated registry update.
  pub fn apply_update(
    &mut self,
    update: &GeometryObservationUpdate,
  ) -> Result<(), GeometryValidationError> {
    let mut next = self.targets.clone();
    let mut removed = HashSet::with_capacity(update.removed.len());
    for id in &update.removed {
      if !removed.insert(*id) {
        return Err(GeometryValidationError::DuplicateId);
      }
      if next.remove(id).is_none() {
        return Err(GeometryValidationError::UnknownId);
      }
    }
    for observation in &update.added {
      if removed.contains(&observation.observation_id) {
        return Err(GeometryValidationError::DuplicateId);
      }
      validate_target(&observation.target)?;
      if next
        .insert(observation.observation_id, observation.target.clone())
        .is_some()
      {
        return Err(GeometryValidationError::DuplicateId);
      }
    }
    self.targets = next;
    Ok(())
  }

  /// Accepts a batch only after every changed value validates.
  pub fn accept_batch(
    &mut self,
    batch: &GeometryObservationBatch,
  ) -> Result<(), GeometryValidationError> {
    if self
      .generation
      .is_some_and(|value| batch.generation <= value)
    {
      return Err(GeometryValidationError::InvalidGeneration);
    }
    let mut seen = HashMap::with_capacity(batch.changed.len());
    for value in &batch.changed {
      let target = self
        .targets
        .get(&value.observation_id)
        .ok_or(GeometryValidationError::UnknownId)?;
      if seen.insert(value.observation_id, ()).is_some() {
        return Err(GeometryValidationError::DuplicateId);
      }
      validate_result(target, value.result)?;
    }
    self.generation = Some(batch.generation);
    Ok(())
  }
}

fn validate_target(target: &GeometryObservationTarget) -> Result<(), GeometryValidationError> {
  if let GeometryObservationTarget::WorldAnchor { anchor, .. } = target
    && anchor.0.is_empty()
  {
    return Err(GeometryValidationError::EmptyAnchor);
  }
  Ok(())
}

fn validate_result(
  target: &GeometryObservationTarget,
  result: GeometryObservationResult,
) -> Result<(), GeometryValidationError> {
  let GeometryObservationResult::Current(value) = result else {
    return Ok(());
  };
  let matches = matches!(
    (target, value),
    (
      GeometryObservationTarget::UiElement { .. },
      GeometryValue::Element(_)
    ) | (
      GeometryObservationTarget::Viewport { .. },
      GeometryValue::Viewport(_)
    ) | (
      GeometryObservationTarget::WorldOrigin { .. } | GeometryObservationTarget::WorldAnchor { .. },
      GeometryValue::WorldPoint(_)
    ) | (
      GeometryObservationTarget::WorldRenderedBounds { .. },
      GeometryValue::WorldBounds(_)
    )
  );
  if !matches {
    return Err(GeometryValidationError::WrongValueKind);
  }
  validate_numbers(value)
}

fn validate_numbers(value: GeometryValue) -> Result<(), GeometryValidationError> {
  let finite = match value {
    GeometryValue::Element(value) => {
      projective_valid(value.viewport_from_local)?;
      projective_valid(value.viewport_from_parent)?;
      rect_finite(value.layout) && viewport_rect_finite(value.viewport_bound)
    }
    GeometryValue::Viewport(value) => {
      viewport_rect_finite(value.viewport)
        && viewport_rect_finite(value.safe_area)
        && value.scale.is_finite()
        && value.dpi.is_none_or(f64::is_finite)
    }
    GeometryValue::WorldPoint(value) => {
      value.point.x.is_finite() && value.point.y.is_finite() && value.depth.is_finite()
    }
    GeometryValue::WorldBounds(value) => {
      viewport_rect_finite(value.bound)
        && value.nearest_depth.is_finite()
        && value.farthest_depth.is_finite()
    }
  };
  if finite {
    Ok(())
  } else {
    Err(GeometryValidationError::NonFiniteNumber)
  }
}

fn rect_finite(value: Rect) -> bool {
  [value.x, value.y, value.width, value.height]
    .into_iter()
    .all(f64::is_finite)
}

fn viewport_rect_finite(value: ViewportRect) -> bool {
  [value.x, value.y, value.width, value.height]
    .into_iter()
    .all(f64::is_finite)
}

fn projective_valid(value: Projective2) -> Result<(), GeometryValidationError> {
  let entries = [
    value.m11, value.m12, value.m13, value.m21, value.m22, value.m23, value.m31, value.m32,
    value.m33,
  ];
  if !entries.into_iter().all(f64::is_finite) {
    return Err(GeometryValidationError::NonFiniteNumber);
  }
  let determinant = value.m11 * (value.m22 * value.m33 - value.m23 * value.m32)
    - value.m12 * (value.m21 * value.m33 - value.m23 * value.m31)
    + value.m13 * (value.m21 * value.m32 - value.m22 * value.m31);
  if !determinant.is_finite() || determinant == 0.0 {
    Err(GeometryValidationError::InvalidProjective)
  } else {
    Ok(())
  }
}

fn same_display(display_id: DisplayId, point: ViewportPoint) -> Option<Point> {
  (display_id == point.display_id).then_some(Point::new(point.x, point.y))
}

fn map(matrix: Projective2, point: Point) -> Option<Point> {
  let divisor = matrix.m31 * point.x + matrix.m32 * point.y + matrix.m33;
  if !divisor.is_finite() || divisor == 0.0 {
    return None;
  }
  let point = Point::new(
    (matrix.m11 * point.x + matrix.m12 * point.y + matrix.m13) / divisor,
    (matrix.m21 * point.x + matrix.m22 * point.y + matrix.m23) / divisor,
  );
  (point.x.is_finite() && point.y.is_finite()).then_some(point)
}

fn crosses_horizon(matrix: Projective2, points: [Point; 4]) -> bool {
  let divisors = points.map(|point| matrix.m31 * point.x + matrix.m32 * point.y + matrix.m33);
  !divisors.iter().all(|divisor| divisor.is_finite())
    || !(divisors.iter().all(|divisor| *divisor > 0.0)
      || divisors.iter().all(|divisor| *divisor < 0.0))
}

fn multiply(left: Projective2, right: Projective2) -> Projective2 {
  Projective2 {
    m11: left.m11 * right.m11 + left.m12 * right.m21 + left.m13 * right.m31,
    m12: left.m11 * right.m12 + left.m12 * right.m22 + left.m13 * right.m32,
    m13: left.m11 * right.m13 + left.m12 * right.m23 + left.m13 * right.m33,
    m21: left.m21 * right.m11 + left.m22 * right.m21 + left.m23 * right.m31,
    m22: left.m21 * right.m12 + left.m22 * right.m22 + left.m23 * right.m32,
    m23: left.m21 * right.m13 + left.m22 * right.m23 + left.m23 * right.m33,
    m31: left.m31 * right.m11 + left.m32 * right.m21 + left.m33 * right.m31,
    m32: left.m31 * right.m12 + left.m32 * right.m22 + left.m33 * right.m32,
    m33: left.m31 * right.m13 + left.m32 * right.m23 + left.m33 * right.m33,
  }
}

fn inverse(matrix: Projective2) -> Option<Projective2> {
  let c11 = matrix.m22 * matrix.m33 - matrix.m23 * matrix.m32;
  let c12 = matrix.m23 * matrix.m31 - matrix.m21 * matrix.m33;
  let c13 = matrix.m21 * matrix.m32 - matrix.m22 * matrix.m31;
  let determinant = matrix.m11 * c11 + matrix.m12 * c12 + matrix.m13 * c13;
  if !determinant.is_finite() || determinant == 0.0 {
    return None;
  }
  let inverse = Projective2 {
    m11: c11 / determinant,
    m12: (matrix.m13 * matrix.m32 - matrix.m12 * matrix.m33) / determinant,
    m13: (matrix.m12 * matrix.m23 - matrix.m13 * matrix.m22) / determinant,
    m21: c12 / determinant,
    m22: (matrix.m11 * matrix.m33 - matrix.m13 * matrix.m31) / determinant,
    m23: (matrix.m13 * matrix.m21 - matrix.m11 * matrix.m23) / determinant,
    m31: c13 / determinant,
    m32: (matrix.m12 * matrix.m31 - matrix.m11 * matrix.m32) / determinant,
    m33: (matrix.m11 * matrix.m22 - matrix.m12 * matrix.m21) / determinant,
  };
  [
    inverse.m11,
    inverse.m12,
    inverse.m13,
    inverse.m21,
    inverse.m22,
    inverse.m23,
    inverse.m31,
    inverse.m32,
    inverse.m33,
  ]
  .into_iter()
  .all(f64::is_finite)
  .then_some(inverse)
}

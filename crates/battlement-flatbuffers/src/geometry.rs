use std::collections::HashSet;

use crate::{
  ProtocolError, geometry_generated::battlement::flat_buffers::generated as wire,
  ui_event::uuid_bytes,
};

const MAXIMUM_CHANGED_VALUES: usize = 262_144;

/// A verified borrowed geometry sampling pass.
#[derive(Clone, Copy)]
pub struct GeometryObservationBatchView<'a> {
  value: wire::GeometryObservationBatch<'a>,
}

impl<'a> GeometryObservationBatchView<'a> {
  pub(crate) fn new(value: wire::GeometryObservationBatch<'a>) -> Result<Self, ProtocolError> {
    validate_batch(value)?;
    Ok(Self { value })
  }

  /// Returns the nonzero sampling generation.
  #[must_use]
  pub fn generation(self) -> u64 {
    self.value.generation()
  }

  /// Returns the number of changed observations.
  #[must_use]
  pub fn changed_count(self) -> usize {
    self.value.changed().len()
  }

  /// Returns one changed observation, if the index is in range.
  #[must_use]
  pub fn changed(self, index: usize) -> Option<GeometryObservationValueView<'a>> {
    (index < self.changed_count()).then(|| GeometryObservationValueView {
      value: self.value.changed().get(index),
    })
  }

  /// Iterates through changed observations without materializing a collection.
  pub fn changed_values(self) -> impl ExactSizeIterator<Item = GeometryObservationValueView<'a>> {
    self
      .value
      .changed()
      .iter()
      .map(|value| GeometryObservationValueView { value })
  }
}

/// One borrowed changed geometry observation.
#[derive(Clone, Copy)]
pub struct GeometryObservationValueView<'a> {
  value: wire::GeometryObservationValue<'a>,
}

impl<'a> GeometryObservationValueView<'a> {
  /// Returns the observation UUID bytes.
  #[must_use]
  pub fn observation_id(self) -> [u8; 16] {
    uuid_bytes(self.value.observation_id())
  }

  /// Returns the current value or temporary-unavailability result.
  #[must_use]
  pub fn result(self) -> GeometryObservationResultView<'a> {
    result(self.value).expect("geometry view stores a validated result union")
  }

  /// Copies this one changed value for a registry that must retain it after input unpinning.
  #[must_use]
  pub fn copy_for_retention(self) -> battlement::GeometryObservationValue {
    battlement::GeometryObservationValue {
      observation_id: battlement::GeometryObservationId(object_id(self.observation_id())),
      result: match self.result() {
        GeometryObservationResultView::Current(value) => {
          battlement::GeometryObservationResult::Current(value.copy_for_retention())
        }
        GeometryObservationResultView::Unavailable(reason) => {
          battlement::GeometryObservationResult::Unavailable(match reason {
            0 => battlement::GeometryUnavailable::Detached,
            1 => battlement::GeometryUnavailable::Hidden,
            2 => battlement::GeometryUnavailable::ObjectMissing,
            3 => battlement::GeometryUnavailable::CameraDisabled,
            4 => battlement::GeometryUnavailable::DisplayUnavailable,
            5 => battlement::GeometryUnavailable::NoRenderers,
            6 => battlement::GeometryUnavailable::BehindCamera,
            7 => battlement::GeometryUnavailable::NoViewportMapping,
            8 => battlement::GeometryUnavailable::ProjectionUnavailable,
            _ => unreachable!("geometry view validates unavailable reasons"),
          })
        }
      },
    }
  }
}

/// A borrowed geometry observation result.
#[derive(Clone, Copy)]
pub enum GeometryObservationResultView<'a> {
  /// A current sampled value.
  Current(GeometryValueView<'a>),
  /// A temporary unavailable-reason ordinal.
  Unavailable(u8),
}

/// A borrowed current geometry value.
#[derive(Clone, Copy)]
pub enum GeometryValueView<'a> {
  /// UI element geometry.
  Element(ElementGeometryView<'a>),
  /// Display viewport geometry.
  Viewport(ViewportGeometryView<'a>),
  /// Projected world point geometry.
  WorldPoint(WorldPointGeometryView<'a>),
  /// Projected rendered bounds geometry.
  WorldBounds(WorldBoundsGeometryView<'a>),
  /// Renderer bounds in the observed object's local XY space.
  WorldRestBounds(WorldRestBoundsGeometryView<'a>),
  /// Native queue and blocking-work counts.
  PresentationWork(PresentationWorkGeometryView<'a>),
}

impl GeometryValueView<'_> {
  fn copy_for_retention(self) -> battlement::GeometryValue {
    match self {
      Self::Element(value) => {
        let [x, y, width, height] = value.layout();
        let (viewport, display_id) = value.viewport_bound();
        battlement::GeometryValue::Element(battlement::ElementGeometry {
          layout: battlement::Rect::new(x, y, width, height),
          viewport_bound: viewport_rect_owned(viewport, display_id),
          viewport_from_local: projective_owned(value.viewport_from_local()),
          viewport_from_parent: projective_owned(value.viewport_from_parent()),
          panel_id: object_id(value.panel_id()),
        })
      }
      Self::Viewport(value) => {
        let (viewport, viewport_display) = value.viewport();
        let (safe_area, safe_display) = value.safe_area();
        battlement::GeometryValue::Viewport(battlement::ViewportGeometry {
          viewport: viewport_rect_owned(viewport, viewport_display),
          safe_area: viewport_rect_owned(safe_area, safe_display),
          scale: value.scale(),
          dpi: value.dpi(),
          orientation: match value.orientation() {
            0 => battlement::DisplayOrientation::Landscape,
            1 => battlement::DisplayOrientation::LandscapeFlipped,
            2 => battlement::DisplayOrientation::Portrait,
            3 => battlement::DisplayOrientation::PortraitFlipped,
            _ => unreachable!("geometry view validates display orientations"),
          },
        })
      }
      Self::WorldPoint(value) => {
        let ([x, y], display_id) = value.point();
        battlement::GeometryValue::WorldPoint(battlement::WorldPointGeometry {
          point: battlement::ViewportPoint {
            x,
            y,
            display_id: battlement::DisplayId(display_id),
          },
          depth: value.depth(),
          is_inside_viewport: value.is_inside_viewport(),
        })
      }
      Self::WorldBounds(value) => {
        let (bound, display_id) = value.bound();
        battlement::GeometryValue::WorldBounds(battlement::WorldBoundsGeometry {
          bound: viewport_rect_owned(bound, display_id),
          nearest_depth: value.nearest_depth(),
          farthest_depth: value.farthest_depth(),
          is_inside_viewport: value.is_inside_viewport(),
        })
      }
      Self::WorldRestBounds(value) => {
        let [x, y, width, height] = value.bound();
        battlement::GeometryValue::WorldRestBounds(battlement::WorldRestBoundsGeometry {
          bound: battlement::Rect::new(x, y, width, height),
        })
      }
      Self::PresentationWork(value) => {
        battlement::GeometryValue::PresentationWork(battlement::PresentationWorkGeometry {
          queued_batches: value.queued_batches(),
          blocking_operations: value.blocking_operations(),
          paused_scopes: value.paused_scopes(),
        })
      }
    }
  }
}

fn object_id(bytes: [u8; 16]) -> battlement::ObjectId {
  battlement::ObjectId::from_uuid(uuid::Uuid::from_bytes(bytes))
    .expect("geometry view validates nonzero UUIDs")
}

fn viewport_rect_owned(values: [f64; 4], display_id: u32) -> battlement::ViewportRect {
  battlement::ViewportRect {
    x: values[0],
    y: values[1],
    width: values[2],
    height: values[3],
    display_id: battlement::DisplayId(display_id),
  }
}

fn projective_owned(values: [f64; 9]) -> battlement::Projective2 {
  battlement::Projective2 {
    m11: values[0],
    m12: values[1],
    m13: values[2],
    m21: values[3],
    m22: values[4],
    m23: values[5],
    m31: values[6],
    m32: values[7],
    m33: values[8],
  }
}

/// Borrowed UI element geometry.
#[derive(Clone, Copy)]
pub struct ElementGeometryView<'a> {
  value: wire::ElementGeometry<'a>,
}

impl ElementGeometryView<'_> {
  /// Returns local layout as x, y, width, and height.
  #[must_use]
  pub fn layout(self) -> [f64; 4] {
    let value = self.value.layout();
    [value.x(), value.y(), value.width(), value.height()]
  }
  /// Returns viewport bounds as x, y, width, height, and display ID.
  #[must_use]
  pub fn viewport_bound(self) -> ([f64; 4], u32) {
    viewport_rect(self.value.viewport_bound())
  }
  /// Returns the row-major local-to-viewport projective matrix.
  #[must_use]
  pub fn viewport_from_local(self) -> [f64; 9] {
    projective(self.value.viewport_from_local())
  }
  /// Returns the row-major parent-to-viewport projective matrix.
  #[must_use]
  pub fn viewport_from_parent(self) -> [f64; 9] {
    projective(self.value.viewport_from_parent())
  }
  /// Returns the owning panel UUID bytes.
  #[must_use]
  pub fn panel_id(self) -> [u8; 16] {
    uuid_bytes(self.value.panel_id())
  }
}

/// Borrowed viewport geometry.
#[derive(Clone, Copy)]
pub struct ViewportGeometryView<'a> {
  value: wire::ViewportGeometry<'a>,
}

impl ViewportGeometryView<'_> {
  /// Returns viewport bounds and display ID.
  #[must_use]
  pub fn viewport(self) -> ([f64; 4], u32) {
    viewport_rect(self.value.viewport())
  }
  /// Returns safe-area bounds and display ID.
  #[must_use]
  pub fn safe_area(self) -> ([f64; 4], u32) {
    viewport_rect(self.value.safe_area())
  }
  /// Returns the display scale.
  #[must_use]
  pub fn scale(self) -> f64 {
    self.value.scale()
  }
  /// Returns optional display DPI.
  #[must_use]
  pub fn dpi(self) -> Option<f64> {
    self.value.dpi()
  }
  /// Returns the stable orientation ordinal.
  #[must_use]
  pub fn orientation(self) -> u8 {
    self.value.orientation().0
  }
}

/// Borrowed projected world point geometry.
#[derive(Clone, Copy)]
pub struct WorldPointGeometryView<'a> {
  value: wire::WorldPointGeometry<'a>,
}

impl WorldPointGeometryView<'_> {
  /// Returns x, y, and display ID.
  #[must_use]
  pub fn point(self) -> ([f64; 2], u32) {
    let value = self.value.point();
    ([value.x(), value.y()], value.display_id())
  }
  /// Returns projection depth.
  #[must_use]
  pub fn depth(self) -> f64 {
    self.value.depth()
  }
  /// Returns whether the point is inside the viewport.
  #[must_use]
  pub fn is_inside_viewport(self) -> bool {
    self.value.is_inside_viewport()
  }
}

/// Borrowed projected rendered bounds geometry.
#[derive(Clone, Copy)]
pub struct WorldBoundsGeometryView<'a> {
  value: wire::WorldBoundsGeometry<'a>,
}

/// Borrowed renderer bounds in an object's local XY space.
#[derive(Clone, Copy)]
pub struct WorldRestBoundsGeometryView<'a> {
  value: wire::WorldRestBoundsGeometry<'a>,
}

/// Borrowed native presentation-work counts.
#[derive(Clone, Copy)]
pub struct PresentationWorkGeometryView<'a> {
  value: wire::PresentationWorkGeometry<'a>,
}

impl PresentationWorkGeometryView<'_> {
  /// Returns retained native batches.
  #[must_use]
  pub fn queued_batches(self) -> u32 {
    self.value.queued_batches()
  }

  /// Returns finite operations blocking batch progress.
  #[must_use]
  pub fn blocking_operations(self) -> u32 {
    self.value.blocking_operations()
  }

  /// Returns paused gameplay presentation scopes.
  #[must_use]
  pub fn paused_scopes(self) -> u32 {
    self.value.paused_scopes()
  }
}

impl WorldRestBoundsGeometryView<'_> {
  /// Returns x, y, width, and height in local units.
  #[must_use]
  pub fn bound(self) -> [f64; 4] {
    let value = self.value.bound();
    [value.x(), value.y(), value.width(), value.height()]
  }
}

impl WorldBoundsGeometryView<'_> {
  /// Returns bounds and display ID.
  #[must_use]
  pub fn bound(self) -> ([f64; 4], u32) {
    viewport_rect(self.value.bound())
  }
  /// Returns nearest projection depth.
  #[must_use]
  pub fn nearest_depth(self) -> f64 {
    self.value.nearest_depth()
  }
  /// Returns farthest projection depth.
  #[must_use]
  pub fn farthest_depth(self) -> f64 {
    self.value.farthest_depth()
  }
  /// Returns whether the bounds intersect the viewport.
  #[must_use]
  pub fn is_inside_viewport(self) -> bool {
    self.value.is_inside_viewport()
  }
}

fn validate_batch(value: wire::GeometryObservationBatch<'_>) -> Result<(), ProtocolError> {
  if value.generation() == 0 {
    return Err(error("geometry generation must be nonzero"));
  }
  let changed = value.changed();
  if changed.len() > MAXIMUM_CHANGED_VALUES {
    return Err(error("geometry batch has too many changed values"));
  }
  let mut identities = HashSet::with_capacity(changed.len());
  for observation in changed {
    let id = uuid_bytes(observation.observation_id());
    if id.iter().all(|byte| *byte == 0) || !identities.insert(id) {
      return Err(error(
        "geometry observation UUIDs must be nonzero and unique",
      ));
    }
    result(observation)?;
  }
  Ok(())
}

fn result(
  value: wire::GeometryObservationValue<'_>,
) -> Result<GeometryObservationResultView<'_>, ProtocolError> {
  match value.result_type() {
    wire::GeometryResult::CurrentGeometry => {
      let current = value
        .result_as_current_geometry()
        .ok_or_else(|| error("current geometry payload is missing"))?;
      Ok(GeometryObservationResultView::Current(current_value(
        current,
      )?))
    }
    wire::GeometryResult::UnavailableGeometry => {
      let unavailable = value
        .result_as_unavailable_geometry()
        .ok_or_else(|| error("unavailable geometry payload is missing"))?;
      if unavailable.reason().0 > wire::GeometryUnavailable::ProjectionUnavailable.0 {
        return Err(error("unknown geometry-unavailable reason"));
      }
      Ok(GeometryObservationResultView::Unavailable(
        unavailable.reason().0,
      ))
    }
    _ => Err(error("unknown geometry result union tag")),
  }
}

fn current_value(value: wire::CurrentGeometry<'_>) -> Result<GeometryValueView<'_>, ProtocolError> {
  match value.value_type() {
    wire::GeometryValue::ElementGeometry => {
      let value = value
        .value_as_element_geometry()
        .ok_or_else(|| error("element geometry payload is missing"))?;
      validate_element(value)?;
      Ok(GeometryValueView::Element(ElementGeometryView { value }))
    }
    wire::GeometryValue::ViewportGeometry => {
      let value = value
        .value_as_viewport_geometry()
        .ok_or_else(|| error("viewport geometry payload is missing"))?;
      validate_viewport(value)?;
      Ok(GeometryValueView::Viewport(ViewportGeometryView { value }))
    }
    wire::GeometryValue::WorldPointGeometry => {
      let value = value
        .value_as_world_point_geometry()
        .ok_or_else(|| error("world-point geometry payload is missing"))?;
      validate_world_point(value)?;
      Ok(GeometryValueView::WorldPoint(WorldPointGeometryView {
        value,
      }))
    }
    wire::GeometryValue::WorldBoundsGeometry => {
      let value = value
        .value_as_world_bounds_geometry()
        .ok_or_else(|| error("world-bounds geometry payload is missing"))?;
      validate_world_bounds(value)?;
      Ok(GeometryValueView::WorldBounds(WorldBoundsGeometryView {
        value,
      }))
    }
    wire::GeometryValue::WorldRestBoundsGeometry => {
      let value = value
        .value_as_world_rest_bounds_geometry()
        .ok_or_else(|| error("world-rest-bounds geometry payload is missing"))?;
      validate_world_rest_bounds(value)?;
      Ok(GeometryValueView::WorldRestBounds(
        WorldRestBoundsGeometryView { value },
      ))
    }
    wire::GeometryValue::PresentationWorkGeometry => {
      let value = value
        .value_as_presentation_work_geometry()
        .ok_or_else(|| error("presentation-work geometry payload is missing"))?;
      Ok(GeometryValueView::PresentationWork(
        PresentationWorkGeometryView { value },
      ))
    }
    _ => Err(error("unknown geometry value union tag")),
  }
}

fn validate_element(value: wire::ElementGeometry<'_>) -> Result<(), ProtocolError> {
  if uuid_bytes(value.panel_id()).iter().all(|byte| *byte == 0)
    || !rect(value.layout())
    || !valid_viewport_rect(value.viewport_bound())
  {
    return Err(error("invalid element geometry"));
  }
  valid_projective(value.viewport_from_local())?;
  valid_projective(value.viewport_from_parent())
}

fn validate_viewport(value: wire::ViewportGeometry<'_>) -> Result<(), ProtocolError> {
  if !valid_viewport_rect(value.viewport())
    || !valid_viewport_rect(value.safe_area())
    || !value.scale().is_finite()
    || value.dpi().is_some_and(|dpi| !dpi.is_finite())
    || value.orientation().0 > wire::DisplayOrientation::PortraitFlipped.0
  {
    return Err(error("invalid viewport geometry"));
  }
  Ok(())
}

fn validate_world_point(value: wire::WorldPointGeometry<'_>) -> Result<(), ProtocolError> {
  if !value.point().x().is_finite() || !value.point().y().is_finite() || !value.depth().is_finite()
  {
    return Err(error("invalid world-point geometry"));
  }
  Ok(())
}

fn validate_world_bounds(value: wire::WorldBoundsGeometry<'_>) -> Result<(), ProtocolError> {
  if !valid_viewport_rect(value.bound())
    || !value.nearest_depth().is_finite()
    || !value.farthest_depth().is_finite()
  {
    return Err(error("invalid world-bounds geometry"));
  }
  Ok(())
}

fn validate_world_rest_bounds(
  value: wire::WorldRestBoundsGeometry<'_>,
) -> Result<(), ProtocolError> {
  if !rect(value.bound()) || value.bound().width() <= 0.0 || value.bound().height() <= 0.0 {
    return Err(error("invalid world-rest-bounds geometry"));
  }
  Ok(())
}

fn rect(value: &crate::ui_event_generated::Rect) -> bool {
  [value.x(), value.y(), value.width(), value.height()]
    .into_iter()
    .all(f64::is_finite)
}

fn valid_viewport_rect(value: &wire::ViewportRect) -> bool {
  [value.x(), value.y(), value.width(), value.height()]
    .into_iter()
    .all(f64::is_finite)
}

fn valid_projective(value: &wire::Projective2) -> Result<(), ProtocolError> {
  let entries = projective(value);
  if !entries.into_iter().all(f64::is_finite) {
    return Err(error("projective matrix contains a non-finite number"));
  }
  let determinant = value.m11() * (value.m22() * value.m33() - value.m23() * value.m32())
    - value.m12() * (value.m21() * value.m33() - value.m23() * value.m31())
    + value.m13() * (value.m21() * value.m32() - value.m22() * value.m31());
  if !determinant.is_finite() || determinant == 0.0 {
    return Err(error("projective matrix must be invertible"));
  }
  Ok(())
}

fn projective(value: &wire::Projective2) -> [f64; 9] {
  [
    value.m11(),
    value.m12(),
    value.m13(),
    value.m21(),
    value.m22(),
    value.m23(),
    value.m31(),
    value.m32(),
    value.m33(),
  ]
}

fn viewport_rect(value: &wire::ViewportRect) -> ([f64; 4], u32) {
  (
    [value.x(), value.y(), value.width(), value.height()],
    value.display_id(),
  )
}

fn error(message: impl Into<String>) -> ProtocolError {
  ProtocolError::new(message)
}

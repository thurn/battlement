use std::num::NonZeroU64;

use battlement::{
  DisplayId, DisplayOrientation, GeometryGeneration, GeometryObservationBatch,
  GeometryObservationResult, GeometryObservationTarget, GeometryObservationValue, GeometryValue,
  Rect, ScreenSize, ViewportGeometry, ViewportRect,
};
use reactant_testing::Display;

pub(crate) fn observe_viewport(
  display: &mut Display,
  generation: u64,
  size: ScreenSize,
  safe: Rect,
) {
  let viewport = ViewportRect {
    x: 0.0,
    y: 0.0,
    width: f64::from(size.width),
    height: f64::from(size.height),
    display_id: DisplayId(0),
  };
  let changed = display
    .geometry_registry()
    .iter()
    .filter_map(|(id, target)| {
      matches!(target, GeometryObservationTarget::Viewport { .. }).then_some(
        GeometryObservationValue {
          observation_id: *id,
          result: GeometryObservationResult::Current(GeometryValue::Viewport(ViewportGeometry {
            viewport,
            safe_area: ViewportRect {
              x: safe.x,
              y: safe.y,
              width: safe.width,
              height: safe.height,
              ..viewport
            },
            scale: 1.0,
            dpi: None,
            orientation: if size.width < size.height {
              DisplayOrientation::Portrait
            } else {
              DisplayOrientation::Landscape
            },
          })),
        },
      )
    })
    .collect();
  display.deliver_geometry(GeometryObservationBatch {
    generation: GeometryGeneration(NonZeroU64::new(generation).unwrap()),
    changed,
  });
  display.flush();
}

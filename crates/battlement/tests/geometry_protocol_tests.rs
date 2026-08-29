use std::num::NonZeroU64;

use battlement::*;

const OBJECT: &str = "10000000-0000-0000-0000-000000000001";
const PANEL: &str = "10000000-0000-0000-0000-000000000002";
const OBSERVATIONS: [&str; 5] = [
  "20000000-0000-0000-0000-000000000001",
  "20000000-0000-0000-0000-000000000002",
  "20000000-0000-0000-0000-000000000003",
  "20000000-0000-0000-0000-000000000004",
  "20000000-0000-0000-0000-000000000005",
];

#[test]
fn element_geometry_converts_round_trip_and_rejects_cross_display_points() {
  let geometry = ElementGeometry {
    viewport_from_local: Projective2 {
      m11: 2.0,
      m12: 0.0,
      m13: 10.0,
      m21: 0.0,
      m22: 3.0,
      m23: 20.0,
      m31: 0.01,
      m32: 0.02,
      m33: 1.0,
    },
    viewport_from_parent: Projective2 {
      m11: 1.0,
      m12: 0.0,
      m13: 10.0,
      m21: 0.0,
      m22: 1.0,
      m23: 20.0,
      m31: 0.0,
      m32: 0.0,
      m33: 1.0,
    },
    ..element_geometry()
  };
  let local = Point::new(4.0, 6.0);
  let viewport = geometry.local_point_to_viewport(local).unwrap();
  let round_trip = geometry.viewport_point_to_local(viewport).unwrap();
  assert!((round_trip.x - local.x).abs() < 1e-10);
  assert!((round_trip.y - local.y).abs() < 1e-10);
  assert_eq!(
    geometry
      .viewport_point_to_parent(ViewportPoint {
        x: 15.0,
        y: 27.0,
        display_id: DisplayId(0),
      })
      .unwrap(),
    Point::new(5.0, 7.0)
  );
  assert!(
    geometry
      .viewport_point_to_local(ViewportPoint {
        display_id: DisplayId(1),
        ..viewport
      })
      .is_none()
  );
}

#[test]
fn element_bounds_transform_original_corners_into_the_destination_space() {
  let source = ElementGeometry {
    layout: Rect::new(9.0, 12.0, 10.0, 20.0),
    viewport_from_local: Projective2 {
      m11: 0.0,
      m12: -1.0,
      m13: 100.0,
      m21: 1.0,
      m22: 0.0,
      m23: 50.0,
      m31: 0.0,
      m32: 0.0,
      m33: 1.0,
    },
    ..element_geometry()
  };
  let destination = ElementGeometry {
    viewport_from_local: identity(),
    ..element_geometry()
  };
  assert_eq!(
    source.bounds_in(&destination),
    Some(Rect::new(80.0, 50.0, 20.0, 10.0))
  );
  let different_display = ElementGeometry {
    viewport_bound: ViewportRect {
      display_id: DisplayId(1),
      ..destination.viewport_bound
    },
    ..destination
  };
  assert!(source.bounds_in(&different_display).is_none());
}

#[test]
fn element_bounds_reject_a_projective_horizon_crossing() {
  let mut source = self::element_geometry();
  source.layout = Rect::new(0.0, 0.0, 1.0, 1.0);
  source.viewport_from_local = Projective2 {
    m31: 2.0,
    m33: -1.0,
    ..self::identity()
  };

  assert_eq!(source.bounds_in(&self::element_geometry()), None);
}

#[test]
fn geometry_json_round_trips_every_target_value_and_unavailable_case() {
  let update = GeometryObservationUpdate {
    added: targets(),
    removed: Vec::new(),
  };
  let command = CommandBody::GeometryObservationUpdate(update.clone());
  assert_eq!(
    json::from_slice::<CommandBody>(&json::to_vec(&command).unwrap()).unwrap(),
    command
  );

  let mut changed = values();
  for (index, reason) in [
    GeometryUnavailable::Detached,
    GeometryUnavailable::Hidden,
    GeometryUnavailable::ObjectMissing,
    GeometryUnavailable::CameraDisabled,
    GeometryUnavailable::DisplayUnavailable,
    GeometryUnavailable::NoRenderers,
    GeometryUnavailable::BehindCamera,
    GeometryUnavailable::NoViewportMapping,
    GeometryUnavailable::ProjectionUnavailable,
  ]
  .into_iter()
  .enumerate()
  {
    changed.push(GeometryObservationValue {
      observation_id: observation(index % OBSERVATIONS.len()),
      result: GeometryObservationResult::Unavailable(reason),
    });
  }
  let body = ActionBody::GeometryObservations(GeometryObservationBatch {
    generation: generation(1),
    changed,
  });
  assert_eq!(
    json::from_slice::<ActionBody>(&json::to_vec(&body).unwrap()).unwrap(),
    body
  );
}

#[test]
fn registry_rejects_complete_invalid_inputs_without_partial_acceptance() {
  let mut registry = GeometryRegistry::default();
  registry
    .apply_update(&GeometryObservationUpdate {
      added: targets(),
      removed: Vec::new(),
    })
    .unwrap();

  for (mut changed, expected) in [
    (
      vec![values()[0], values()[0]],
      GeometryValidationError::DuplicateId,
    ),
    (
      vec![GeometryObservationValue {
        observation_id: observation(0),
        result: GeometryObservationResult::Current(GeometryValue::Viewport(viewport_geometry())),
      }],
      GeometryValidationError::WrongValueKind,
    ),
    (
      vec![GeometryObservationValue {
        observation_id: observation(2),
        result: GeometryObservationResult::Current(GeometryValue::WorldPoint(WorldPointGeometry {
          point: ViewportPoint {
            x: f64::NAN,
            y: 2.0,
            display_id: DisplayId(0),
          },
          depth: 3.0,
          is_inside_viewport: true,
        })),
      }],
      GeometryValidationError::NonFiniteNumber,
    ),
    (
      vec![GeometryObservationValue {
        observation_id: observation(0),
        result: GeometryObservationResult::Current(GeometryValue::Element(ElementGeometry {
          viewport_from_local: overflowing_projective(),
          ..element_geometry()
        })),
      }],
      GeometryValidationError::InvalidProjective,
    ),
  ] {
    assert_eq!(
      registry.accept_batch(&GeometryObservationBatch {
        generation: generation(1),
        changed: std::mem::take(&mut changed),
      }),
      Err(expected)
    );
  }

  assert_eq!(
    registry.accept_batch(&GeometryObservationBatch {
      generation: generation(1),
      changed: values(),
    }),
    Ok(())
  );
  assert_eq!(
    registry.accept_batch(&GeometryObservationBatch {
      generation: generation(1),
      changed: Vec::new(),
    }),
    Err(GeometryValidationError::InvalidGeneration)
  );
}

#[test]
fn malformed_generations_and_registry_updates_are_rejected() {
  assert!(json::from_slice::<GeometryGeneration>(b"0").is_err());
  let mut registry = GeometryRegistry::default();
  let duplicate = targets()[0].clone();
  assert_eq!(
    registry.apply_update(&GeometryObservationUpdate {
      added: vec![duplicate.clone(), duplicate],
      removed: Vec::new(),
    }),
    Err(GeometryValidationError::DuplicateId)
  );
  assert!(registry.get(observation(0)).is_none());

  registry
    .apply_update(&GeometryObservationUpdate {
      added: vec![targets()[0].clone()],
      removed: Vec::new(),
    })
    .unwrap();
  assert_eq!(
    registry.apply_update(&GeometryObservationUpdate {
      added: vec![GeometryObservation {
        observation_id: observation(0),
        target: GeometryObservationTarget::Viewport {
          display_id: DisplayId(0),
        },
      }],
      removed: vec![observation(0)],
    }),
    Err(GeometryValidationError::DuplicateId)
  );
  assert!(matches!(
    registry.get(observation(0)),
    Some(GeometryObservationTarget::UiElement { .. })
  ));
}

fn targets() -> Vec<GeometryObservation> {
  let object_id = OBJECT.parse().unwrap();
  vec![
    GeometryObservation {
      observation_id: observation(0),
      target: GeometryObservationTarget::UiElement { object_id },
    },
    GeometryObservation {
      observation_id: observation(1),
      target: GeometryObservationTarget::Viewport {
        display_id: DisplayId(0),
      },
    },
    GeometryObservation {
      observation_id: observation(2),
      target: GeometryObservationTarget::WorldOrigin {
        object_id,
        camera: CameraTarget::Input,
      },
    },
    GeometryObservation {
      observation_id: observation(3),
      target: GeometryObservationTarget::WorldAnchor {
        object_id,
        anchor: AnchorName("head".into()),
        camera: CameraTarget::Object(PANEL.parse().unwrap()),
      },
    },
    GeometryObservation {
      observation_id: observation(4),
      target: GeometryObservationTarget::WorldRenderedBounds {
        object_id,
        camera: CameraTarget::Input,
      },
    },
  ]
}

fn values() -> Vec<GeometryObservationValue> {
  vec![
    value(0, GeometryValue::Element(element_geometry())),
    value(1, GeometryValue::Viewport(viewport_geometry())),
    value(
      2,
      GeometryValue::WorldPoint(WorldPointGeometry {
        point: ViewportPoint {
          x: 1.0,
          y: 2.0,
          display_id: DisplayId(0),
        },
        depth: 3.0,
        is_inside_viewport: true,
      }),
    ),
    value(
      3,
      GeometryValue::WorldPoint(WorldPointGeometry {
        point: ViewportPoint {
          x: 4.0,
          y: 5.0,
          display_id: DisplayId(0),
        },
        depth: 6.0,
        is_inside_viewport: false,
      }),
    ),
    value(
      4,
      GeometryValue::WorldBounds(WorldBoundsGeometry {
        bound: viewport_rect(),
        nearest_depth: 1.0,
        farthest_depth: 8.0,
        is_inside_viewport: true,
      }),
    ),
  ]
}

fn value(index: usize, value: GeometryValue) -> GeometryObservationValue {
  GeometryObservationValue {
    observation_id: observation(index),
    result: GeometryObservationResult::Current(value),
  }
}

fn element_geometry() -> ElementGeometry {
  ElementGeometry {
    layout: Rect::new(1.0, 2.0, 30.0, 40.0),
    viewport_bound: viewport_rect(),
    viewport_from_local: identity(),
    viewport_from_parent: identity(),
    panel_id: PANEL.parse().unwrap(),
  }
}

fn viewport_geometry() -> ViewportGeometry {
  ViewportGeometry {
    viewport: viewport_rect(),
    safe_area: viewport_rect(),
    scale: 2.0,
    dpi: Some(144.0),
    orientation: DisplayOrientation::Landscape,
  }
}

fn viewport_rect() -> ViewportRect {
  ViewportRect {
    x: 0.0,
    y: 0.0,
    width: 1920.0,
    height: 1080.0,
    display_id: DisplayId(0),
  }
}

fn identity() -> Projective2 {
  Projective2 {
    m11: 1.0,
    m12: 0.0,
    m13: 0.0,
    m21: 0.0,
    m22: 1.0,
    m23: 0.0,
    m31: 0.0,
    m32: 0.0,
    m33: 1.0,
  }
}

fn overflowing_projective() -> Projective2 {
  Projective2 {
    m11: 1e308,
    m12: 1e308,
    m13: 1e308,
    m21: 1e308,
    m22: 1e308,
    m23: 1e308,
    m31: 1e308,
    m32: 1e308,
    m33: 1e308,
  }
}

fn observation(index: usize) -> GeometryObservationId {
  GeometryObservationId(OBSERVATIONS[index].parse().unwrap())
}

fn generation(value: u64) -> GeometryGeneration {
  GeometryGeneration(NonZeroU64::new(value).unwrap())
}

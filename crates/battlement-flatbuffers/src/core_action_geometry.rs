use std::collections::HashSet;

use battlement::{
  GeometryObservationBatch, GeometryObservationResult, GeometryUnavailable, GeometryValue,
  Projective2, Rect, ViewportPoint, ViewportRect,
};
use flatbuffers::{FlatBufferBuilder, UnionWIPOffset, WIPOffset};

use crate::{
  ProtocolError, client_message_generated::battlement::flat_buffers::generated as client,
  common_generated as common, geometry_generated::battlement::flat_buffers::generated as wire,
  ui_event_generated::Rect as WireRect,
};

const MAXIMUM_CHANGED_VALUES: usize = 262_144;

pub(crate) fn write<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  batch: &GeometryObservationBatch,
) -> Result<WIPOffset<client::GeometryAction<'a>>, ProtocolError> {
  if batch.changed.len() > MAXIMUM_CHANGED_VALUES {
    return Err(error("a geometry batch has too many changed values"));
  }
  let mut identities = HashSet::with_capacity(batch.changed.len());
  let changed = batch
    .changed
    .iter()
    .map(|changed| {
      let identity = *changed.observation_id.0.as_uuid().as_bytes();
      if !identities.insert(identity) {
        return Err(error("geometry observation identities must be unique"));
      }
      let (result_type, result) = write_result(builder, changed.result)?;
      let observation_id = common::Uuid(identity);
      Ok(wire::GeometryObservationValue::create(
        builder,
        &wire::GeometryObservationValueArgs {
          observation_id: Some(&observation_id),
          result_type,
          result: Some(result),
        },
      ))
    })
    .collect::<Result<Vec<_>, ProtocolError>>()?;
  let changed = builder.create_vector(&changed);
  let batch = wire::GeometryObservationBatch::create(
    builder,
    &wire::GeometryObservationBatchArgs {
      generation: batch.generation.0.get(),
      changed: Some(changed),
    },
  );
  Ok(client::GeometryAction::create(
    builder,
    &client::GeometryActionArgs { value: Some(batch) },
  ))
}

fn write_result<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  result: GeometryObservationResult,
) -> Result<(wire::GeometryResult, WIPOffset<UnionWIPOffset>), ProtocolError> {
  Ok(match result {
    GeometryObservationResult::Current(value) => {
      let (value_type, value) = write_value(builder, value)?;
      let current = wire::CurrentGeometry::create(
        builder,
        &wire::CurrentGeometryArgs {
          value_type,
          value: Some(value),
        },
      );
      (
        wire::GeometryResult::CurrentGeometry,
        current.as_union_value(),
      )
    }
    GeometryObservationResult::Unavailable(reason) => {
      let reason = match reason {
        GeometryUnavailable::Detached => wire::GeometryUnavailable::Detached,
        GeometryUnavailable::Hidden => wire::GeometryUnavailable::Hidden,
        GeometryUnavailable::ObjectMissing => wire::GeometryUnavailable::ObjectMissing,
        GeometryUnavailable::CameraDisabled => wire::GeometryUnavailable::CameraDisabled,
        GeometryUnavailable::DisplayUnavailable => wire::GeometryUnavailable::DisplayUnavailable,
        GeometryUnavailable::NoRenderers => wire::GeometryUnavailable::NoRenderers,
        GeometryUnavailable::BehindCamera => wire::GeometryUnavailable::BehindCamera,
        GeometryUnavailable::NoViewportMapping => wire::GeometryUnavailable::NoViewportMapping,
        GeometryUnavailable::ProjectionUnavailable => {
          wire::GeometryUnavailable::ProjectionUnavailable
        }
      };
      let unavailable =
        wire::UnavailableGeometry::create(builder, &wire::UnavailableGeometryArgs { reason });
      (
        wire::GeometryResult::UnavailableGeometry,
        unavailable.as_union_value(),
      )
    }
  })
}

fn write_value<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: GeometryValue,
) -> Result<(wire::GeometryValue, WIPOffset<UnionWIPOffset>), ProtocolError> {
  Ok(match value {
    GeometryValue::Element(value) => {
      finite_rect(value.layout)?;
      finite_viewport_rect(value.viewport_bound)?;
      finite_projective(value.viewport_from_local)?;
      finite_projective(value.viewport_from_parent)?;
      let layout = WireRect::new(
        value.layout.x,
        value.layout.y,
        value.layout.width,
        value.layout.height,
      );
      let bound = viewport_rect(value.viewport_bound);
      let local = projective(value.viewport_from_local);
      let parent = projective(value.viewport_from_parent);
      let panel = common::Uuid(*value.panel_id.as_uuid().as_bytes());
      let value = wire::ElementGeometry::create(
        builder,
        &wire::ElementGeometryArgs {
          layout: Some(&layout),
          viewport_bound: Some(&bound),
          viewport_from_local: Some(&local),
          viewport_from_parent: Some(&parent),
          panel_id: Some(&panel),
        },
      );
      (wire::GeometryValue::ElementGeometry, value.as_union_value())
    }
    GeometryValue::Viewport(value) => {
      finite_viewport_rect(value.viewport)?;
      finite_viewport_rect(value.safe_area)?;
      finite(value.scale)?;
      if let Some(dpi) = value.dpi {
        finite(dpi)?;
      }
      let viewport = viewport_rect(value.viewport);
      let safe_area = viewport_rect(value.safe_area);
      let orientation = wire::DisplayOrientation(value.orientation as u8);
      let value = wire::ViewportGeometry::create(
        builder,
        &wire::ViewportGeometryArgs {
          viewport: Some(&viewport),
          safe_area: Some(&safe_area),
          scale: value.scale,
          dpi: value.dpi,
          orientation,
        },
      );
      (
        wire::GeometryValue::ViewportGeometry,
        value.as_union_value(),
      )
    }
    GeometryValue::WorldPoint(value) => {
      finite_viewport_point(value.point)?;
      finite(value.depth)?;
      let point = viewport_point(value.point);
      let value = wire::WorldPointGeometry::create(
        builder,
        &wire::WorldPointGeometryArgs {
          point: Some(&point),
          depth: value.depth,
          is_inside_viewport: value.is_inside_viewport,
        },
      );
      (
        wire::GeometryValue::WorldPointGeometry,
        value.as_union_value(),
      )
    }
    GeometryValue::WorldBounds(value) => {
      finite_viewport_rect(value.bound)?;
      finite(value.nearest_depth)?;
      finite(value.farthest_depth)?;
      let bound = viewport_rect(value.bound);
      let value = wire::WorldBoundsGeometry::create(
        builder,
        &wire::WorldBoundsGeometryArgs {
          bound: Some(&bound),
          nearest_depth: value.nearest_depth,
          farthest_depth: value.farthest_depth,
          is_inside_viewport: value.is_inside_viewport,
        },
      );
      (
        wire::GeometryValue::WorldBoundsGeometry,
        value.as_union_value(),
      )
    }
    GeometryValue::WorldRestBounds(value) => {
      finite_rect(value.bound)?;
      if value.bound.width <= 0.0 || value.bound.height <= 0.0 {
        return Err(error("world rest bounds must have positive dimensions"));
      }
      let bound = WireRect::new(
        value.bound.x,
        value.bound.y,
        value.bound.width,
        value.bound.height,
      );
      let value = wire::WorldRestBoundsGeometry::create(
        builder,
        &wire::WorldRestBoundsGeometryArgs {
          bound: Some(&bound),
        },
      );
      (
        wire::GeometryValue::WorldRestBoundsGeometry,
        value.as_union_value(),
      )
    }
    GeometryValue::PresentationWork(value) => {
      let value = wire::PresentationWorkGeometry::create(
        builder,
        &wire::PresentationWorkGeometryArgs {
          queued_batches: value.queued_batches,
          blocking_operations: value.blocking_operations,
          paused_scopes: value.paused_scopes,
        },
      );
      (
        wire::GeometryValue::PresentationWorkGeometry,
        value.as_union_value(),
      )
    }
  })
}

fn viewport_point(value: ViewportPoint) -> wire::ViewportPoint {
  wire::ViewportPoint::new(value.x, value.y, value.display_id.0)
}

fn viewport_rect(value: ViewportRect) -> wire::ViewportRect {
  wire::ViewportRect::new(
    value.x,
    value.y,
    value.width,
    value.height,
    value.display_id.0,
  )
}

fn projective(value: Projective2) -> wire::Projective2 {
  wire::Projective2::new(
    value.m11, value.m12, value.m13, value.m21, value.m22, value.m23, value.m31, value.m32,
    value.m33,
  )
}

fn finite_rect(value: Rect) -> Result<(), ProtocolError> {
  finite_values(&[value.x, value.y, value.width, value.height])
}

fn finite_viewport_point(value: ViewportPoint) -> Result<(), ProtocolError> {
  finite_values(&[value.x, value.y])
}

fn finite_viewport_rect(value: ViewportRect) -> Result<(), ProtocolError> {
  finite_values(&[value.x, value.y, value.width, value.height])
}

fn finite_projective(value: Projective2) -> Result<(), ProtocolError> {
  finite_values(&[
    value.m11, value.m12, value.m13, value.m21, value.m22, value.m23, value.m31, value.m32,
    value.m33,
  ])?;
  let determinant = value.m11 * (value.m22 * value.m33 - value.m23 * value.m32)
    - value.m12 * (value.m21 * value.m33 - value.m23 * value.m31)
    + value.m13 * (value.m21 * value.m32 - value.m22 * value.m31);
  if determinant == 0.0 || !determinant.is_finite() {
    return Err(error("geometry projective transforms must be invertible"));
  }
  Ok(())
}

fn finite_values(values: &[f64]) -> Result<(), ProtocolError> {
  for value in values {
    finite(*value)?;
  }
  Ok(())
}

fn finite(value: f64) -> Result<(), ProtocolError> {
  if !value.is_finite() {
    return Err(error("geometry numbers must be finite"));
  }
  Ok(())
}

fn error(message: impl Into<String>) -> ProtocolError {
  ProtocolError::new(message)
}
